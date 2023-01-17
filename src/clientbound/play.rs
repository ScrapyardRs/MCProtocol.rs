use crate::common::bit_set::{BitSet, FixedBitSet};
use crate::common::chat::Chat;
use crate::common::chunk::Chunk;
use crate::common::play::{
    BlockPos, ChatBind, CommandNode, Difficulty, GlobalPos, InteractionHand, ItemStack, Location,
    MapColorPatch, MessageSignature, PackedMessageBody, PackedMessageSignature, SimpleLocation,
};
use crate::common::play::{RecipeBookType, RemoteChatSession};
use crate::common::{GameProfile, GameProfileProperty};
use crate::serverbound::play::ServerboundPlayRegistry::Chat;
use drax::nbt::EnsuredCompoundTag;
use drax::prelude::{
    AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, DraxReadExt, DraxWriteExt, PacketComponent,
    Size, Uuid,
};
use drax::transport::packet::option::Maybe;
use drax::transport::packet::primitive::{VarInt, VarLong};
use drax::transport::packet::serde_json::JsonDelegate;
use drax::transport::packet::string::LimitedString;
use drax::transport::packet::vec::{ByteDrain, LimitedVec};
use drax::{err_explain, throw_explain, PinnedLivelyResult};
use std::collections::HashMap;

impl RelativeArgument {
    pub const X_BIT: u8 = 0x00;
    pub const Y_BIT: u8 = 0x01;
    pub const Z_BIT: u8 = 0x02;
    pub const Y_ROT_BIT: u8 = 0x03;
    pub const X_ROT_BIT: u8 = 0x04;

    pub fn new(bit: u8) -> Self {
        Self { bit }
    }

    pub fn is_set(&self, mask: u8) -> bool {
        self.bit & (1 << mask) != 0
    }

    pub fn set(&mut self, mask: u8) {
        self.bit |= 1 << mask;
    }
}

impl Default for RelativeArgument {
    fn default() -> Self {
        Self::new(0)
    }
}

#[derive(Debug)]
pub struct PlayerInfoEntry {
    pub profile_id: Uuid,
    pub profile: Option<GameProfile>,
    pub latency: Option<i32>,
    pub listed: Option<bool>,
    pub game_mode: Option<i32>,
    pub display_name: Option<Chat>,
    pub chat_session: Option<RemoteChatSession>,
}

impl PlayerInfoEntry {
    pub fn new(profile_id: Uuid) -> Self {
        Self {
            profile_id,
            profile: None,
            latency: None,
            listed: None,
            game_mode: None,
            display_name: None,
            chat_session: None,
        }
    }
}

#[derive(Debug)]
pub enum PlayerInfoAction {
    AddPlayer,
    InitializeChat,
    UpdateGameMode,
    UpdateListed,
    UpdateLatency,
    UpdateDisplayName,
}

union PlayerInfoActionContext<'a> {
    decode_context: (&'a BitSet, &'a mut PlayerInfoEntry),
    encode_context: (&'a BitSet, &'a PlayerInfoEntry),
}

impl<'b> PacketComponent<PlayerInfoActionContext<'b>> for PlayerInfoAction {
    type ComponentType = ();

    fn decode<'a, A: AsyncRead + Unpin + Send + Sync + ?Sized>(
        context: &'a mut PlayerInfoActionContext<'b>,
        read: &'a mut A,
    ) -> PinnedLivelyResult<'a, Self::ComponentType> {
        Box::pin(async move {
            unsafe {
                let (in_set, entry) = &mut context.decode_context;
                let ctx = &mut ();

                if in_set.get(0)? {
                    entry.profile = Some(GameProfile {
                        id: entry.profile_id.clone(),
                        name: String::decode(ctx, read).await?,
                        properties: Vec::<GameProfileProperty>::decode(ctx, read).await?,
                    })
                }

                if in_set.get(1)? {
                    entry.chat_session = Some(RemoteChatSession::decode(ctx, read).await?)
                }

                if in_set.get(2)? {
                    entry.game_mode = Some(read.read_var_int().await?)
                }

                if in_set.get(3)? {
                    entry.listed = Some(bool::decode(ctx, read).await?)
                }

                if in_set.get(4)? {
                    entry.latency = Some(read.read_var_int().await?)
                }

                if in_set.get(5)? {
                    entry.display_name = Maybe::<JsonDelegate<Chat>>::decode(ctx, read).await?
                }
                Ok(())
            }
        })
    }

    fn encode<'a, A: AsyncWrite + Unpin + Send + Sync + ?Sized>(
        _: &'a Self::ComponentType,
        context: &'a mut PlayerInfoActionContext<'b>,
        write: &'a mut A,
    ) -> PinnedLivelyResult<'a, ()> {
        Box::pin(async move {
            unsafe {
                let (in_set, entry) = context.encode_context;
                let ctx = &mut ();

                if in_set.get(0)? {
                    if let Some(profile) = entry.profile.as_ref() {
                        String::encode(&profile.name, ctx, write).await?;
                        Vec::<GameProfileProperty>::encode(&profile.properties, ctx, write).await?;
                    } else {
                        throw_explain!("Missing profile with add player bit set.")
                    }
                }

                if in_set.get(1)? {
                    if let Some(chat_session) = entry.chat_session.as_ref() {
                        RemoteChatSession::encode(chat_session, ctx, write).await?;
                    } else {
                        throw_explain!("Missing chat session with initialize chat bit set.")
                    }
                }

                if in_set.get(2)? {
                    if let Some(game_mode) = entry.game_mode {
                        write.write_var_int(game_mode).await?;
                    } else {
                        throw_explain!("Missing game mode with update game mode bit set.")
                    }
                }

                if in_set.get(3)? {
                    if let Some(listed) = entry.listed {
                        bool::encode(&listed, ctx, write).await?;
                    } else {
                        throw_explain!("Missing listed with update listed bit set.")
                    }
                }

                if in_set.get(4)? {
                    if let Some(latency) = entry.latency {
                        write.write_var_int(latency).await?;
                    } else {
                        throw_explain!("Missing latency with update latency bit set.")
                    }
                }

                if in_set.get(5)? {
                    Maybe::<JsonDelegate<Chat>>::encode(&entry.display_name, ctx, write).await?;
                }

                Ok(())
            }
        })
    }

    fn size(
        _: &Self::ComponentType,
        context: &mut PlayerInfoActionContext<'b>,
    ) -> drax::prelude::Result<Size> {
        unsafe {
            let ctx = &mut ();
            let (in_set, entry) = context.encode_context;

            let mut counter = Uuid::size(&entry.profile_id, ctx)?;

            if in_set.get(0)? {
                if let Some(profile) = entry.profile.as_ref() {
                    counter = counter
                        + String::size(&profile.name, ctx)?
                        + Vec::<GameProfileProperty>::size(&profile.properties, ctx)?;
                } else {
                    throw_explain!("Missing profile with add player bit set.")
                }
            }

            if in_set.get(1)? {
                if let Some(chat_session) = entry.chat_session.as_ref() {
                    counter = counter + RemoteChatSession::size(chat_session, ctx)?;
                } else {
                    throw_explain!("Missing chat session with initialize chat bit set.")
                }
            }

            if in_set.get(2)? {
                if let Some(game_mode) = entry.game_mode {
                    counter = counter + VarInt::size(&game_mode, ctx)?;
                } else {
                    throw_explain!("Missing game mode with update game mode bit set.")
                }
            }

            if in_set.get(3)? {
                if let Some(listed) = entry.listed {
                    counter = counter + bool::size(&listed, ctx)?;
                } else {
                    throw_explain!("Missing listed with update listed bit set.")
                }
            }

            if in_set.get(4)? {
                if let Some(latency) = entry.latency {
                    counter = counter + VarInt::size(&latency, ctx)?;
                } else {
                    throw_explain!("Missing latency with update latency bit set.")
                }
            }

            if in_set.get(5)? {
                counter += Maybe::<JsonDelegate<Chat>>::size(&entry.display_name, ctx)?;
            }

            Ok(match counter {
                Size::Dynamic(size) | Size::Constant(size) => Size::Dynamic(size),
            })
        }
    }
}

#[derive(Debug)]
pub struct PlayerInfoUpsert {
    pub actions: BitSet,
    pub entries: Vec<PlayerInfoEntry>,
}

impl<C: Send + Sync> PacketComponent<C> for PlayerInfoUpsert {
    type ComponentType = Self;

    fn decode<'a, A: AsyncRead + Unpin + Send + Sync + ?Sized>(
        context: &'a mut C,
        read: &'a mut A,
    ) -> PinnedLivelyResult<'a, Self::ComponentType> {
        Box::pin(async move {
            let actions = FixedBitSet::<6>::decode(context, read).await?;

            let entry_length = read.read_var_int().await?;
            let mut entries = Vec::with_capacity(entry_length as usize);

            for _ in 0..entry_length {
                let mut entry = PlayerInfoEntry::new(Uuid::decode(context, read).await?);
                let mut ctx = PlayerInfoActionContext {
                    decode_context: (&actions, &mut entry),
                };
                PlayerInfoAction::decode(&mut ctx, read).await?;
                entries.push(entry);
            }

            Ok(PlayerInfoUpsert { actions, entries })
        })
    }

    fn encode<'a, A: AsyncWrite + Unpin + Send + Sync + ?Sized>(
        component_ref: &'a Self::ComponentType,
        context: &'a mut C,
        write: &'a mut A,
    ) -> PinnedLivelyResult<'a, ()> {
        Box::pin(async move {
            FixedBitSet::<6>::encode(&component_ref.actions, context, write).await?;

            write
                .write_var_int(component_ref.entries.len() as i32)
                .await?;

            for entry in component_ref.entries.iter() {
                Uuid::encode(&entry.profile_id, context, write).await?;

                let mut ctx = PlayerInfoActionContext {
                    encode_context: (&component_ref.actions, entry),
                };

                PlayerInfoAction::encode(&(), &mut ctx, write).await?;
            }
            Ok(())
        })
    }

    fn size(input: &Self::ComponentType, context: &mut C) -> drax::prelude::Result<Size> {
        let mut size = FixedBitSet::<6>::size(&input.actions, context)?
            + VarInt::size(&(input.entries.len() as i32), context)?;

        for entry in input.entries.iter() {
            size = size
                + Uuid::size(&entry.profile_id, context)?
                + PlayerInfoAction::size(
                    &(),
                    &mut PlayerInfoActionContext {
                        encode_context: (&input.actions, entry),
                    },
                )?;
        }

        Ok(size)
    }
}

#[derive(Debug)]
pub struct RecipeBookSettings {
    pub settings: HashMap<RecipeBookType, RecipeBookSetting>,
}

impl<C: Send + Sync> PacketComponent<C> for RecipeBookSettings {
    type ComponentType = RecipeBookSettings;

    fn decode<'a, A: AsyncRead + Unpin + Send + Sync + ?Sized>(
        context: &'a mut C,
        read: &'a mut A,
    ) -> PinnedLivelyResult<'a, Self::ComponentType> {
        Box::pin(async move {
            let mut settings = RecipeBookSettings {
                settings: HashMap::with_capacity(4),
            };
            for variant in [
                RecipeBookType::Crafting,
                RecipeBookType::Furnace,
                RecipeBookType::BlastFurnace,
                RecipeBookType::Smoker,
            ] {
                let open = bool::decode(context, read).await?;
                let filtering = bool::decode(context, read).await?;
                settings
                    .settings
                    .insert(variant, RecipeBookSetting { open, filtering });
            }
            Ok(settings)
        })
    }

    fn encode<'a, A: AsyncWrite + Unpin + Send + Sync + ?Sized>(
        component_ref: &'a Self::ComponentType,
        context: &'a mut C,
        write: &'a mut A,
    ) -> PinnedLivelyResult<'a, ()> {
        Box::pin(async move {
            for variant in [
                RecipeBookType::Crafting,
                RecipeBookType::Furnace,
                RecipeBookType::BlastFurnace,
                RecipeBookType::Smoker,
            ] {
                let setting = component_ref.settings.get(&variant).ok_or_else(|| {
                    err_explain!(format!(
                        "Missing recipe book setting for variant: {:?}",
                        variant
                    ))
                })?;
                bool::encode(&setting.open, context, write).await?;
                bool::encode(&setting.filtering, context, write).await?;
            }
            Ok(())
        })
    }

    fn size(_: &Self::ComponentType, _: &mut C) -> drax::prelude::Result<Size> {
        Ok(Size::Constant(8))
    }
}

impl EquipmentSlot {
    const VARIANTS: [EquipmentSlot; 6] = [
        EquipmentSlot::MainHand,
        EquipmentSlot::OffHand,
        EquipmentSlot::Feet,
        EquipmentSlot::Legs,
        EquipmentSlot::Chest,
        EquipmentSlot::Head,
    ];

    pub fn ordinal(self) -> usize {
        match self {
            EquipmentSlot::MainHand => 0,
            EquipmentSlot::OffHand => 1,
            EquipmentSlot::Feet => 2,
            EquipmentSlot::Legs => 3,
            EquipmentSlot::Chest => 4,
            EquipmentSlot::Head => 5,
        }
    }
}

#[derive(Debug)]
pub struct SetEquipmentList;

impl<C: Send + Sync> PacketComponent<C> for SetEquipmentList {
    type ComponentType = Vec<(EquipmentSlot, Option<ItemStack>)>;

    fn decode<'a, A: AsyncRead + Unpin + Send + Sync + ?Sized>(
        context: &'a mut C,
        read: &'a mut A,
    ) -> PinnedLivelyResult<'a, Self::ComponentType> {
        Box::pin(async move {
            let mut slots = Vec::new();

            let mut b = read.read_u8().await?;
            let mut item = Maybe::<ItemStack>::decode(context, read).await?;
            slots.push((EquipmentSlot::VARIANTS[(b & 0x7F) as usize], item));
            while b & 0x80 != 0 {
                b = read.read_u8().await?;
                item = Maybe::<ItemStack>::decode(context, read).await?;
                slots.push((EquipmentSlot::VARIANTS[(b & 0x7F) as usize], item));
            }
            Ok(slots)
        })
    }

    fn encode<'a, A: AsyncWrite + Unpin + Send + Sync + ?Sized>(
        component_ref: &'a Self::ComponentType,
        context: &'a mut C,
        write: &'a mut A,
    ) -> PinnedLivelyResult<'a, ()> {
        Box::pin(async move {
            for (i, (slot, item)) in component_ref.iter().enumerate() {
                let mut b = slot.ordinal() as u8;
                if i != component_ref.len() - 1 {
                    b |= 0x80;
                }
                write.write_u8(b as u8).await?;
                Maybe::<ItemStack>::encode(item, context, write).await?;
            }
            Ok(())
        })
    }

    fn size(input: &Self::ComponentType, context: &mut C) -> drax::prelude::Result<Size> {
        let mut size = Size::Constant(0);
        for (_, item) in input.iter() {
            size = size + Size::Constant(1) + Maybe::<ItemStack>::size(item, context)?;
        }
        Ok(size)
    }
}

#[derive(Debug)]
pub enum SoundEvent {
    Direct {
        location: String,
        range: Option<f32>,
    },
    Generic(i32),
}

impl<C: Send + Sync> PacketComponent<C> for SoundEvent {
    type ComponentType = SoundEvent;

    fn decode<'a, A: AsyncRead + Unpin + Send + Sync + ?Sized>(
        context: &'a mut C,
        read: &'a mut A,
    ) -> PinnedLivelyResult<'a, Self::ComponentType> {
        Box::pin(async move {
            let v = read.read_var_int().await?;
            if v == 0 {
                return Ok(SoundEvent::Direct {
                    location: String::decode(context, read).await?,
                    range: Maybe::<f32>::decode(context, read).await?,
                });
            }
            Ok(SoundEvent::Generic(v))
        })
    }

    fn encode<'a, A: AsyncWrite + Unpin + Send + Sync + ?Sized>(
        component_ref: &'a Self::ComponentType,
        context: &'a mut C,
        write: &'a mut A,
    ) -> PinnedLivelyResult<'a, ()> {
        Box::pin(async move {
            match component_ref {
                SoundEvent::Direct { location, range } => {
                    write.write_var_int(0).await?;
                    String::encode(location, context, write).await?;
                    Maybe::<f32>::encode(range, context, write).await?;
                }
                SoundEvent::Generic(v) => {
                    write.write_var_int(*v).await?;
                }
            }
            Ok(())
        })
    }

    fn size(input: &Self::ComponentType, context: &mut C) -> drax::prelude::Result<Size> {
        match input {
            SoundEvent::Direct { location, range } => Ok(Size::Constant(1)
                + String::size(location, context)?
                + Maybe::<f32>::size(range, context)?),
            SoundEvent::Generic(x) => VarInt::size(&x, context),
        }
    }
}

#[derive(Debug)]
pub struct DisplayInfoFlags {
    pub show_toast: bool,
    pub hidden: bool,
    pub background: Option<String>,
}

impl<C: Send + Sync> PacketComponent<C> for DisplayInfoFlags {
    type ComponentType = DisplayInfoFlags;

    fn decode<'a, A: AsyncRead + Unpin + Send + Sync + ?Sized>(
        context: &'a mut C,
        read: &'a mut A,
    ) -> PinnedLivelyResult<'a, Self::ComponentType> {
        Box::pin(async move {
            let flag = i32::decode(context, read).await?;
            let background = if flag & 0x01 != 0 {
                Some(String::decode(context, read).await?)
            } else {
                None
            };
            Ok(DisplayInfoFlags {
                show_toast: flag & 0x02 != 0,
                hidden: flag & 0x04 != 0,
                background,
            })
        })
    }

    fn encode<'a, A: AsyncWrite + Unpin + Send + Sync + ?Sized>(
        component_ref: &'a Self::ComponentType,
        context: &'a mut C,
        write: &'a mut A,
    ) -> PinnedLivelyResult<'a, ()> {
        Box::pin(async move {
            let mut flag = 0;
            if component_ref.background.is_some() {
                flag |= 0x01;
            }
            if component_ref.show_toast {
                flag |= 0x02;
            }
            if component_ref.hidden {
                flag |= 0x04;
            }
            i32::encode(&flag, context, write).await?;
            if let Some(background) = &component_ref.background {
                String::encode(background, context, write).await?;
            }
            Ok(())
        })
    }

    fn size(input: &Self::ComponentType, context: &mut C) -> drax::prelude::Result<Size> {
        Ok(Size::Constant(4)
            + if input.background.is_some() {
                String::size(input.background.as_ref().unwrap(), context)?
            } else {
                Size::Constant(0)
            })
    }
}

pub struct DelegateStr;

impl<C: Send + Sync> PacketComponent<C> for DelegateStr {
    type ComponentType = &'static str;

    fn decode<'a, A: AsyncRead + Unpin + Send + Sync + ?Sized>(
        _: &'a mut C,
        _: &'a mut A,
    ) -> PinnedLivelyResult<'a, Self::ComponentType> {
        unimplemented!("This is a delegate serializer type.")
    }

    fn encode<'a, A: AsyncWrite + Unpin + Send + Sync + ?Sized>(
        component_ref: &'a Self::ComponentType,
        _: &'a mut C,
        write: &'a mut A,
    ) -> PinnedLivelyResult<'a, ()> {
        Box::pin(async move {
            let ref_str = component_ref.to_string();
            String::encode(&ref_str, &mut (), write).await
        })
    }

    fn size(input: &Self::ComponentType, context: &mut C) -> drax::prelude::Result<Size> {
        String::size(&input.to_string(), context)
    }
}

#[derive(Debug)]
pub struct ShapedRecipeBase {
    pub width: i32,
    pub height: i32,
    pub name: String,
    pub category: CraftingBookCategory,
    pub ingredients: Vec<Vec<Option<ItemStack>>>,
    pub result: Option<ItemStack>,
}

impl<C: Send + Sync> PacketComponent<C> for ShapedRecipeBase {
    type ComponentType = ShapedRecipeBase;

    fn decode<'a, A: AsyncRead + Unpin + Send + Sync + ?Sized>(
        context: &'a mut C,
        read: &'a mut A,
    ) -> PinnedLivelyResult<'a, Self::ComponentType> {
        Box::pin(async move {
            let width = read.read_var_int().await?;
            let height = read.read_var_int().await?;
            let name = String::decode(context, read).await?;
            let category = CraftingBookCategory::decode(context, read).await?;
            let mut ingredients = Vec::with_capacity((width * height) as usize);
            for _ in 0..width * height {
                ingredients.push(Vec::<Maybe<ItemStack>>::decode(context, read).await?);
            }
            let result = Maybe::<ItemStack>::decode(context, read).await?;
            Ok(ShapedRecipeBase {
                width,
                height,
                name,
                category,
                ingredients,
                result,
            })
        })
    }

    fn encode<'a, A: AsyncWrite + Unpin + Send + Sync + ?Sized>(
        component_ref: &'a Self::ComponentType,
        context: &'a mut C,
        write: &'a mut A,
    ) -> PinnedLivelyResult<'a, ()> {
        Box::pin(async move {
            write.write_var_int(component_ref.width).await?;
            write.write_var_int(component_ref.height).await?;
            String::encode(&component_ref.name, context, write).await?;
            CraftingBookCategory::encode(&component_ref.category, context, write).await?;
            for ingredient in &component_ref.ingredients {
                Vec::<Maybe<ItemStack>>::encode(ingredient, context, write).await?;
            }
            Maybe::<ItemStack>::encode(&component_ref.result, context, write).await?;
            Ok(())
        })
    }

    fn size(input: &Self::ComponentType, context: &mut C) -> drax::prelude::Result<Size> {
        Ok(VarInt::size(&input.height, context)?
            + VarInt::size(&input.width, context)?
            + String::size(&input.name, context)?
            + CraftingBookCategory::size(&input.category, context)?
            + input
                .ingredients
                .iter()
                .map(|ingredient| Vec::<Maybe<ItemStack>>::size(ingredient, context))
                .collect::<drax::prelude::Result<Vec<Size>>>()?
                .into_iter()
                .fold(Size::Constant(0), |acc, size| acc + size)
            + Maybe::<ItemStack>::size(&input.result, context)?)
    }
}

registry! {
    components {
        struct StatsEntry {
            stat_id: VarInt,
            stat_cap: VarInt,
            stat_value: VarInt
        },

        enum BossBarColor<key: VarInt> {
            Pink {},
            Blue {},
            Red {},
            Green {},
            Yellow {},
            Purple {},
            White {}
        },

        enum BossBarOverlay<key: VarInt> {
            Progress {},
            Notched6 {},
            Notched10 {},
            Notched20 {}
        },

        enum BossEventOperationType<key: VarInt> {
            Add {
                name: JsonDelegate<Chat>,
                progress: f32,
                color: BossBarColor,
                overlay: BossBarOverlay,
                mask: u8
            },
            Remove {},
            UpdateProgress {
                progress: f32
            },
            UpdateName {
                name: JsonDelegate<Chat>
            },
            UpdateStyle {
                color: BossBarColor,
                overlay: BossBarOverlay
            },
            UpdateProperties {
                mask: u8
            }
        },

        struct CommandSuggestion {
            text: String,
            tooltip: Maybe<JsonDelegate<Chat>>
        },

        enum CustomChatCompletionsAction<key: VarInt> {
            Add {},
            Remove {},
            Set {}
        },

        enum MapDecorationType<key: VarInt> {
            Player {},
            Frame {},
            RedMarker {},
            BlueMarker {},
            TargetX {},
            TargetPoint {},
            PlayerOffMap {},
            PlayerOffLimits {},
            Mansion {},
            Monument {},
            BannerWhite {},
            BannerOrange {},
            BannerMagenta {},
            BannerLightBlue {},
            BannerYellow {},
            BannerLime {},
            BannerPink {},
            BannerGray {},
            BannerLightGray {},
            BannerCyan {},
            BannerPurple {},
            BannerBlue {},
            BannerBrown {},
            BannerGreen {},
            BannerRed {},
            BannerBlack {},
            RedX {}
        },

        struct MapDecoration {
            decoration_type: MapDecorationType,
            x: u8,
            y: u8,
            rot: u8,
            name: Maybe<JsonDelegate<Chat>>
        },

        struct MerchantOffer {
            base_cost_a: Maybe<ItemStack>,
            cost_b: Maybe<ItemStack>,
            result: Maybe<ItemStack>,
            bl: bool,
            uses: i32,
            max_uses: i32,
            xp: i32,
            special_price_diff: i32,
            demand: f32,
            price_multiplier: i32
        },

        enum FilterMask<key: VarInt> {
            PassThrough {},
            FullyFiltered {},
            PartiallyFiltered {
                bit_set: BitSet
            }
        },

        enum Anchor<key: VarInt> {
            Feet {},
            Eyes {}
        },

        struct EntityAnchor {
            entity: VarInt,
            to_anchor: Anchor
        },

        struct RelativeArgument {
            bit: u8
        },

        struct RecipeBase {
            recipes: Vec<String>,
            recipe_book_settings: RecipeBookSettings
        },

        enum RecipeState<key: VarInt> {
            Init {
                base: RecipeBase,
                to_highlight: Vec<String>
            },
            Add {
                base: RecipeBase
            },
            Remove {
                base: RecipeBase
            }
        },

        struct RecipeBookSetting {
            open: bool,
            filtering: bool
        },

        #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
        enum EquipmentSlot<key: VarInt> {
            MainHand {},
            OffHand {},
            Feet {},
            Legs {},
            Chest {},
            Head {}
        },

        enum RenderType<key: VarInt> {
            Integer {},
            Hearts {}
        },

        enum SetObjectiveMethod<key: u8> {
            Add {
                display_name: JsonDelegate<Chat>,
                render_type: RenderType
            },
            Remove {},
            Change {
                display_name: JsonDelegate<Chat>,
                render_type: RenderType
            }
        },

        enum ChatFormatting<key: VarInt> {
            Black {},
            DarkBlue {},
            DarkGreen {},
            DarkAqua {},
            DarkRed {},
            DarkPurple {},
            Gold {},
            Gray {},
            DarkGray {},
            Blue {},
            Green {},
            Aqua {},
            Red {},
            LightPurple {},
            Yellow {},
            White {},
            Obfuscated {},
            Bold {},
            StrikeThrough {},
            Underline {},
            Italic {},
            Reset {}
        },

        struct TeamParameters {
            display_name: JsonDelegate<Chat>,
            options: u8,
            name_tag_visibility: LimitedString<40>,
            collision_rule: LimitedString<40>,
            color: ChatFormatting,
            player_prefix: JsonDelegate<Chat>,
            player_suffix: JsonDelegate<Chat>
        },

        enum SetPlayerTeamMethod<key: u8> {
            Add {
                players: Vec<String>,
                parameters: TeamParameters
            },
            Remove {
            },
            Change {
                parameters: TeamParameters
            },
            Join {
                players: Vec<String>
            },
            Leave {
                players: Vec<String>
            }
        },

        enum SetScoreMethod<key: VarInt> {
            Change {
                objective_name: String,
                score: VarInt
            },
            Remove {
                objective_name: String
            }
        },

        enum SoundSource<key: VarInt> {
            Master {},
            Music {},
            Records {},
            Weather {},
            Blocks {},
            Hostile {},
            Neutral {},
            Players {},
            Ambient {},
            Voice {}
        },

        enum StopSoundAction<key: u8> {
            Generic {},
            StopSource {
                source: SoundSource
            },
            StopResource {
                location: String
            },
            StopSourceAndResource {
                source: SoundSource,
                location: String
            }
        },

        enum FrameType<key: u8> {
            Task {},
            Challenge {},
            Goal {}
        },

        struct DisplayInfo {
            title: JsonDelegate<Chat>,
            description: JsonDelegate<Chat>,
            icon: Maybe<ItemStack>,
            frame_type: FrameType,
            flags: DisplayInfoFlags,
            location_x: f32,
            location_y: f32
        },

        struct Advancement {
            location: String,
            parent: Maybe<String>,
            display_info: Maybe<DisplayInfo>,
            criteria: Vec<String>,
            requirements: Vec<Vec<String>>
        },

        struct CriterionProgress {
            criteria: String,
            obtained: Maybe<u64>
        },

        struct AdvancementProgress {
            location: String,
            progress: Vec<CriterionProgress>
        },

        enum AttributeSnapshotModifierOperation<key: u8> {
            Addition {},
            MulitplyBase {},
            MultiplyTotal {}
        },

        struct AttributeSnapshotModifier {
            uuid: Uuid,
            amount: f64,
            operation: AttributeSnapshotModifierOperation
        },

        struct AttributeSnapshot {
            attribute_key: String,
            base: f64,
            modifiers: Vec<AttributeSnapshotModifier>
        },

        enum CraftingBookCategory<key: VarInt> {
            Building {},
            Redstone {},
            Equipment {},
            Misc {}
        },

        struct SimpleCook {
            name: String,
            category: CraftingBookCategory,
            ingredient: Vec<Maybe<ItemStack>>,
            result: Maybe<ItemStack>,
            experience: f32,
            cooking_time: VarInt
        },

        struct ShapelessRecipeBase {
            group: String,
            category: CraftingBookCategory,
            ingredients: Vec<Vec<Maybe<ItemStack>>>,
            result: Maybe<ItemStack>
        },

        enum RecipeRegistry<key: String> {
            @ser_delegate DelegateStr,
            @match {key.as_str()},
            ShapedRecipe {
                @key("minecraft:crafting_shaped");
                base: ShapedRecipeBase
            },
            ShapelessRecipe {
                @key("minecraft:crafting_shapeless");
                base: ShapelessRecipeBase
            },
            ArmorDye {
                @key("minecraft:crafting_special_armordye");
                crafting_group: CraftingBookCategory
            },
            BookCloning {
                @key("minecraft:crafting_special_bookcloning");
                crafting_group: CraftingBookCategory
            },
            MapCloning {
                @key("minecraft:crafting_special_mapcloning");
                crafting_group: CraftingBookCategory
            },
            MapExtending {
                @key("minecraft:crafting_special_mapextending");
                crafting_group: CraftingBookCategory
            },
            FireworkRocket {
                @key("minecraft:crafting_special_firework_rocket");
                crafting_group: CraftingBookCategory
            },
            FireworkStar {
                @key("minecraft:crafting_special_firework_star");
                crafting_group: CraftingBookCategory
            },
            FireworkStarFade {
                @key("minecraft:crafting_special_firework_star_fade");
                crafting_group: CraftingBookCategory
            },
            TippedArrow {
                @key("minecraft:crafting_special_tippedarrow");
                crafting_group: CraftingBookCategory
            },
            BannerDuplicate {
                @key("minecraft:crafting_special_bannerduplicate");
                crafting_group: CraftingBookCategory
            },
            ShieldDecoration {
                @key("minecraft:crafting_special_shielddecoration");
                crafting_group: CraftingBookCategory
            },
            ShulkerBoxColoring {
                @key("minecraft:crafting_special_shulkerboxcoloring");
                crafting_group: CraftingBookCategory
            },
            SuspiciousStew {
                @key("minecraft:crafting_special_suspiciousstew");
                crafting_group: CraftingBookCategory
            },
            RepairItem {
                @key("minecraft:crafting_special_repairitem");
                crafting_group: CraftingBookCategory
            },
            SmeltingRecipe {
                @key("minecraft:smelting");
                simple_cook: SimpleCook
            },
            BlastingRecipe {
                @key("minecraft:blasting");
                simple_cook: SimpleCook
            },
            SmokingRecipe {
                @key("minecraft:smoking");
                simple_cook: SimpleCook
            },
            CampfireCookingRecipe {
                @key("minecraft:campfire_cooking");
                simple_cook: SimpleCook
            },
            Stonecutting {
                @key("minecraft:stonecutting");
                group: String,
                ingredient: Vec<Maybe<ItemStack>>,
                result: Maybe<ItemStack>
            },
            Smithing {
                @key("minecraft:smithing");
                base: Vec<Maybe<ItemStack>>,
                addition: Vec<Maybe<ItemStack>>,
                result: Maybe<ItemStack>
            }
        },

        struct RecipeUpdate {
            loc_1: String,
            loc_2: String,
            reg_ref: RecipeRegistry
        },

        struct TagUpdatePayload {
            key: String,
            values: Vec<VarInt>
        },

        struct TagUpdate {
            key: String,
            payloads: Vec<TagUpdatePayload>
        },

        struct BlockEntityInfo {
            packed_xz: u8,
            y: i16,
            block_type: VarInt,
            tag: EnsuredCompoundTag<0>
        },

        struct LevelChunkData {
            chunk: Chunk,
            block_entities: Vec<BlockEntityInfo>
        },

        struct LightUpdateData {
            trust_edges: bool,
            sky_y_mask: BitSet,
            block_y_mask: BitSet,
            empty_sky_y_mask: BitSet,
            empty_block_y_mask: BitSet,
            sky_updates: Vec<LimitedVec<u8, 2048>>,
            block_updates: Vec<LimitedVec<u8, 2048>>
        },

        struct ParticleBase {
            override_limiter: bool,
            location: SimpleLocation,
            x_dist: f32,
            y_dist: f32,
            z_dist: f32,
            max_speed: f32,
            count: i32
        },

        enum PositionSource<key: String> {
            @ser_delegate DelegateStr,
            @match {key.as_str()},
            Block {
                @key("minecraft:block");
                pos: BlockPos
            },
            Entity {
                @key("minecraft:entity");
                entity_id: VarInt,
                y_offset: f32
            }
        },

        enum ParticleType<key: VarInt> {
            AmbientEntityEffect {
                base: ParticleBase
            },
            AngryVillager {
                base: ParticleBase
            },
            Block {
                base: ParticleBase,
                block_id: VarInt
            },
            BlockMarker {
                base: ParticleBase,
                block_id: VarInt
            },
            Bubble {
                base: ParticleBase
            },
            Cloud {
                base: ParticleBase
            },
            Crit {
                base: ParticleBase
            },
            DamageIndicator {
                base: ParticleBase
            },
            DragonBreath {
                base: ParticleBase
            },
            DrippingLava {
                base: ParticleBase
            },
            FallingLava {
                base: ParticleBase
            },
            LandingLava {
                base: ParticleBase
            },
            DrippingWater {
                base: ParticleBase
            },
            FallingWater {
                base: ParticleBase
            },
            Dust {
                base: ParticleBase,
                xa: f32,
                ya: f32,
                za: f32,
                scale: f32
            },
            DustColorTransition {
                base: ParticleBase,
                xa: f32,
                ya: f32,
                za: f32,
                scale: f32,
                to_xa: f32,
                to_ya: f32,
                to_za: f32
            },
            Effect {
                base: ParticleBase
            },
            ElderGuardian {
                base: ParticleBase
            },
            EnchantedHit {
                base: ParticleBase
            },
            Enchant {
                base: ParticleBase
            },
            EndRod {
                base: ParticleBase
            },
            EntityEffect {
                base: ParticleBase
            },
            ExplosionEmitter {
                base: ParticleBase
            },
            Explosion {
                base: ParticleBase
            },
            SonicBoom {
                base: ParticleBase
            },
            FallingDust {
                base: ParticleBase,
                block_id: VarInt
            },
            Firework {
                base: ParticleBase
            },
            Fishing {
                base: ParticleBase
            },
            Flame {
                base: ParticleBase
            },
            SculkSoul {
                base: ParticleBase
            },
            SculkCharge {
                base: ParticleBase,
                scale: f32
            },
            SculkChargePop {
                base: ParticleBase
            },
            SoulFireFlame {
                base: ParticleBase
            },
            Soul {
                base: ParticleBase
            },
            Flash {
                base: ParticleBase
            },
            HappyVillager {
                base: ParticleBase
            },
            Composter {
                base: ParticleBase
            },
            Heart {
                base: ParticleBase
            },
            InstantEffect {
                base: ParticleBase
            },
            Item {
                base: ParticleBase,
                item: Maybe<ItemStack>
            },
            Vibration {
                base: ParticleBase,
                source: PositionSource,
                arrival_in_ticks: VarInt
            },
            ItemSlime {
                base: ParticleBase
            },
            ItemSnowball {
                base: ParticleBase
            },
            LargeSmoke {
                base: ParticleBase
            },
            Lava {
                base: ParticleBase
            },
            Mycelium {
                base: ParticleBase
            },
            Note {
                base: ParticleBase
            },
            Poof {
                base: ParticleBase
            },
            Portal {
                base: ParticleBase
            },
            Rain {
                base: ParticleBase
            },
            Smoke {
                base: ParticleBase
            },
            Sneeze {
                base: ParticleBase
            },
            Spit {
                base: ParticleBase
            },
            SquidInk {
                base: ParticleBase
            },
            SweepAttack {
                base: ParticleBase
            },
            TotemOfUndying {
                base: ParticleBase
            },
            Underwater {
                base: ParticleBase
            },
            Splash {
                base: ParticleBase
            },
            Witch {
                base: ParticleBase
            },
            BubblePop {
                base: ParticleBase
            },
            CurrentDown {
                base: ParticleBase
            },
            BubbleColumnUp {
                base: ParticleBase
            },
            Nautilus {
                base: ParticleBase
            },
            Dolphin {
                base: ParticleBase
            },
            CampfireCosySmoke {
                base: ParticleBase
            },
            CampfireSignalSmoke {
                base: ParticleBase
            },
            DrippingHoney {
                base: ParticleBase
            },
            FallingHoney {
                base: ParticleBase
            },
            LandingHoney {
                base: ParticleBase
            },
            FallingNectar {
                base: ParticleBase
            },
            FallingSporeBlossom {
                base: ParticleBase
            },
            Ash {
                base: ParticleBase
            },
            CrimsonSpore {
                base: ParticleBase
            },
            WarpedSpore {
                base: ParticleBase
            },
            SporeBlossomAir {
                base: ParticleBase
            },
            DrippingObsidianTear {
                base: ParticleBase
            },
            FallingObsidianTear {
                base: ParticleBase
            },
            LandingObsidianTear {
                base: ParticleBase
            },
            ReversePortal {
                base: ParticleBase
            },
            WhiteAsh {
                base: ParticleBase
            },
            SmallFlame {
                base: ParticleBase
            },
            Snowflake {
                base: ParticleBase
            },
            DrippingDripstoneLava {
                base: ParticleBase
            },
            FallingDripstoneLava {
                base: ParticleBase
            },
            DrippingDripstoneWater {
                base: ParticleBase
            },
            FallingDripstoneWater {
                base: ParticleBase
            },
            GlowSquidInk {
                base: ParticleBase
            },
            Glow {
                base: ParticleBase
            },
            WaxOn {
                base: ParticleBase
            },
            WaxOff {
                base: ParticleBase
            },
            ElectricSpark {
                base: ParticleBase
            },
            Scrape {
                base: ParticleBase
            },
            Shriek {
                base: ParticleBase,
                delay: VarInt
            }
        }
    }

    registry ClientboundPlayRegistry {
        struct AddEntity {
            id: VarInt,
            uuid: Uuid,
            entity_type: VarInt,
            location: SimpleLocation,
            x_rot: u8,
            y_rot: u8,
            data: VarInt,
            xa: u16,
            ya: u16,
            za: u16
        },

        struct AddExperienceOrb {
            entity_id: VarInt,
            location: SimpleLocation,
            value: u16
        },

        struct AddPlayer {
            entity_id: VarInt,
            player_id: Uuid,
            location: SimpleLocation,
            y_rot: u8,
            x_rot: u8
        },

        struct Animate {
            id: VarInt,
            action: u8
        },

        struct AwardStats {
            stat_entries: Vec<StatsEntry>
        },

        struct BlockChangedAck {
            sequence_id: VarInt
        },

        struct BlockDestruction {
            id: VarInt,
            pos: BlockPos,
            progress: u8
        },

        struct BlockEntityData {
            pos: BlockPos,
            block_entity_type: VarInt,
            tag: EnsuredCompoundTag<0>
        },

        struct BlockEvent {
            pos: BlockPos,
            b0: u8,
            b1: u8,
            block: VarInt
        },

        struct BlockUpdate {
            pos: BlockPos,
            state: VarInt
        },

        struct BossEvent {
            id: Uuid,
            operation: BossEventOperationType
        },

        struct ChangeDifficulty {
            difficulty: Difficulty,
            locked: bool
        },

        struct ClearTitles {
            reset_times: bool
        },

        struct CommandSuggestions {
            transaction_id: VarInt,
            lower_bound: VarInt,
            upper_bound_offset: VarInt,
            suggestions: Vec<CommandSuggestion>
        },

        struct Commands {
            commands: Vec<CommandNode>,
            root_index: VarInt
        },

        struct ContainerClose {
            container_id: u8
        },

        struct ContainerSetContent {
            container_id: u8,
            state_id: VarInt,
            items: Vec<Maybe<ItemStack>>,
            carried_item: Maybe<ItemStack>
        },

        struct ContainerSetData {
            container_id: u8,
            id: u16,
            value: u16
        },

        struct ContainerSetSlot {
            container_id: u8,
            state_id: VarInt,
            slot: u16,
            item: Maybe<ItemStack>
        },

        struct Cooldown {
            item_id: VarInt,
            duration: VarInt
        },

        struct CustomChatCompletions {
            action: CustomChatCompletionsAction,
            completions: Vec<String>
        },

        struct CustomPayload {
            identifier: String,
            data: ByteDrain
        },

        struct DeleteChat {
            packed_signature: PackedMessageSignature
        },

        struct Disconnect {
            reason: JsonDelegate<Chat>
        },

        struct DisguisedChat {
            message: JsonDelegate<Chat>,
            chat_bind: ChatBind
        },

        struct EntityEvent {
            entity_id: i32,
            event_id: u8
        },

        struct Explode {
            location: SimpleLocation,
            power: f32,
            offsets: Vec<[u8; 3]>,
            knockback_offsets: [f32; 3]
        },

        struct ForgetLevelChunk {
            x: i32,
            z: i32
        },

        struct GameEvent {
            event: u8,
            param: f32
        },

        struct HorseScreenOpen {
            container_id: u8,
            size: VarInt,
            entity_id: i32
        },

        struct InitializeBorder {
            new_center_x: f64,
            new_center_z: f64,
            old_size: f64,
            new_size: f64,
            lerp_time: VarLong,
            new_absolute_max_size: VarInt,
            warning_blocks: VarInt,
            warning_time: VarInt
        },

        struct KeepAlive {
            id: u64
        },

        struct LevelChunkWithLight {
            chunk_data: LevelChunkData,
            light_data: LightUpdateData
        },

        struct LevelEvent {
            event_type: i32,
            pos: BlockPos,
            data: i32,
            global_event: bool
        },

        struct LevelParticles {
            particle: ParticleType
        },

        struct LightUpdate {
            pos_x: VarInt,
            pos_z: VarInt,
            data: LightUpdateData
        },

        struct ClientLogin {
            player_id: i32,
            hardcore: bool,
            game_type: u8,
            previous_game_type: u8,
            levels: Vec<String>,
            codec: EnsuredCompoundTag<0>,
            dimension_type: String,
            dimension: String,
            seed: u64,
            max_players: VarInt,
            chunk_radius: VarInt,
            simulation_distance: VarInt,
            reduced_debug_info: bool,
            show_death_screen: bool,
            is_debug: bool,
            is_flat: bool,
            last_death_location: Maybe<GlobalPos>
        },

        struct MapItemData {
            map_id: VarInt,
            scale: u8,
            locked: bool,
            decorations: Maybe<Vec<MapDecoration>>,
            color_patch: MapColorPatch
        },

        struct MerchantOffers {
            container_id: VarInt,
            offers: Vec<MerchantOffer>,
            villager_level: VarInt,
            villager_xp: VarInt,
            show_progress_bar: bool,
            can_restock: bool
        },

        struct MoveEntityPos {
            id: VarInt,
            xa: i16,
            ya: i16,
            za: i16,
            on_ground: bool
        },

        struct MoveEntityPosRot {
            id: VarInt,
            xa: i16,
            ya: i16,
            za: i16,
            y_rot: u8,
            x_rot: u8,
            on_ground: bool
        },

        struct MoveEntityRot {
            entity_id: VarInt,
            y_rot: u8,
            x_rot: u8,
            on_ground: bool
        },

        struct MoveVehicle {
            location: Location
        },

        struct OpenBook {
            interaction_hand: InteractionHand
        },

        struct OpenScreen {
            container_id: VarInt,
            container_type: VarInt,
            title: JsonDelegate<Chat>
        },

        struct OpenSignEditor {
            pos: BlockPos
        },

        struct Ping {
            ping_id: i32
        },

        struct PlaceGhostRecipe {
            container_id: u8,
            recipe: String
        },

        struct PlayerAbilities {
            flags: u8,
            flying_speed: f32,
            walking_speed: f32
        },

        struct PlayerChat {
            sender: Uuid,
            index: VarInt,
            signature: Maybe<MessageSignature>,
            body: PackedMessageBody,
            unsigned_content: Maybe<JsonDelegate<Chat>>,
            filter_mask: FilterMask,
            chat_bind: ChatBind
        },

        struct PlayerCombatEnd {
            duration: VarInt,
            killer_id: i32
        },

        struct PlayerCombatEnter {
        },

        struct PlayerCombatKill {
            player_id: VarInt,
            killer_id: i32,
            message: JsonDelegate<Chat>
        },

        struct PlayerInfoRemove {
            profile_ids: Vec<Uuid>
        },

        struct PlayerInfoUpdate {
            upsert: PlayerInfoUpsert
        },

        struct PlayerLookAt {
            from_anchor: Anchor,
            location: SimpleLocation,
            at_entity: Maybe<EntityAnchor>
        },

        struct PlayerPosition {
            location: Location,
            relative_arguments: RelativeArgument,
            id: VarInt,
            dismount: bool
        },

        struct Recipe {
            recipe_state: RecipeState
        },

        struct RemoveEntities {
            entity_ids: Vec<VarInt>
        },

        struct RemoveMobEffect {
            entity_id: VarInt,
            effect_id: VarInt
        },

        struct ResourcePack {
            url: String,
            hash: String,
            required: bool,
            prompt: Maybe<JsonDelegate<Chat>>
        },

        struct Respawn {
            dimension_type: String,
            dimension: String,
            seed: u64,
            game_type: u8,
            previous_game_type: u8,
            is_debug: bool,
            is_flat: bool,
            data_to_keep: u8,
            last_death_location: Maybe<GlobalPos>
        },

        struct RotateHead {
            entity_id: VarInt,
            y_head_rot: u8
        },

        struct SectionBlocksUpdate {
            section_pos: u64,
            suppress_light_update: bool,
            update_info: Vec<VarLong>
        },

        struct SelectAdvancementsTab {
            tab: String
        },

        struct ServerData {
            motd: Maybe<JsonDelegate<Chat>>,
            icon_base_64: Maybe<String>,
            enforces_secure_chat: bool
        },

        struct SetActionBarText {
            text: JsonDelegate<Chat>
        },

        struct SetBorderCenter {
            new_center_x: f64,
            new_center_z: f64
        },

        struct SetBorderLerpSize {
            old_size: f64,
            new_size: f64,
            lerp_time: VarLong
        },

        struct SetBorderSize {
            size: f64
        },

        struct SetBorderWarningDelay {
            warning_time: VarInt
        },

        struct SetBorderWarningDistance {
            warning_blocks: VarInt
        },

        struct SetCamera {
            camera_id: VarInt
        },

        struct SetCarriedItem {
            slot: u8
        },

        struct SetChunkCacheCenter {
            x: i32,
            z: i32
        },

        struct SetChunkCacheRadius {
            radius: VarInt
        },

        struct SetDefaultSpawnPosition {
            pos: BlockPos,
            angle: f32
        },

        struct SetDisplayObjective {
            slot: u8,
            objective: String
        },

        struct SetEntityData {
            entity_id: VarInt,
            temp_drain: ByteDrain
        },

        struct SetEntityLink {
            source_id: i32,
            dest_id: i32
        },

        struct SetEntityMotion {
            entity_id: VarInt,
            xa: u16,
            ya: u16,
            za: u16
        },

        struct SetEquipment {
            entity_id: VarInt,
            equipment_list: SetEquipmentList
        },

        struct SetExperience {
            experience_progress: f32,
            experience_level: VarInt,
            total_experience: VarInt
        },

        struct SetHealth {
            health: f32,
            food: VarInt,
            saturation: f32
        },

        struct SetObjective {
            objective_name: String,
            method: SetObjectiveMethod
        },

        struct SetPassengers {
            vehicle: VarInt,
            passengers: Vec<VarInt>
        },

        struct SetPlayerTeam {
            team_name: String,
            method: SetPlayerTeamMethod
        },

        struct SetScore {
            owner: String,
            method: SetScoreMethod
        },

        struct SetSimulationDistance {
            simulation_distance: VarInt
        },

        struct SetSubtitleText {
            text: JsonDelegate<Chat>
        },

        struct SetTime {
            game_time: VarLong,
            day_time: VarLong
        },

        struct SetTitleText {
            text: JsonDelegate<Chat>
        },

        struct SetTitlesAnimation {
            fade_in: VarInt,
            stay: VarInt,
            fade_out: VarInt
        },

        struct SoundEntity {
            sound: VarInt,
            source: VarInt,
            id: VarInt,
            volume: f32,
            seed: u64
        },

        struct Sound {
            sound_event: SoundEvent,
            source: SoundSource,
            block_x: i32,
            block_y: i32,
            block_z: i32,
            volume: f32,
            pitch: f32,
            seed: u64
        },

        struct StopSound {
            action: StopSoundAction
        },

        struct SystemChat {
            content: JsonDelegate<Chat>,
            overlay: bool
        },

        struct TabList {
            header: JsonDelegate<Chat>,
            footer: JsonDelegate<Chat>
        },

        struct TagQuery {
            transaction_id: VarInt,
            tag: EnsuredCompoundTag<0>
        },

        struct TakeItemEntity {
            item_id: VarInt,
            player_id: VarInt,
            amount: VarInt
        },

        struct TeleportEntity {
            entity_id: VarInt,
            location: SimpleLocation,
            y_rot: u8,
            x_rot: u8,
            on_ground: bool
        },

        struct UpdateAdvancements {
            reset: bool,
            added: Vec<Advancement>,
            removed: Vec<String>,
            progress: Vec<AdvancementProgress>
        },

        struct UpdateAttributes {
            entity_id: VarInt,
            attributes: Vec<AttributeSnapshot>
        },

        struct UpdateEnabledFeatures {
            features: Vec<String>
        },

        struct UpdateMobEffect {
            entity_id: VarInt,
            effect_id: VarInt,
            amplifier: u8,
            duration: VarInt,
            flags: u8,
            factor_data: Maybe<EnsuredCompoundTag<0>>
        },

        struct UpdateRecipes {
            updates: Vec<RecipeUpdate>
        },

        struct UpdateTags {
            updates: Vec<TagUpdate>
        }
    }
}
