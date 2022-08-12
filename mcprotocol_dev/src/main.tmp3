use futures::AsyncWriteExt;
use mc_level::codec::MinecraftWorldgenBiomeSkyColor::IntCoverage;
use mc_level::codec::{
    Codec, MinecraftChatType, MinecraftChatTypeChat, MinecraftChatTypeElement,
    MinecraftChatTypeElementNarration, MinecraftChatTypeEntry, MinecraftChatTypeStyle,
    MinecraftDimensionType, MinecraftDimensionTypeElement, MinecraftDimensionTypeEntry,
    MinecraftWorldgenBiome, MinecraftWorldgenBiomeEffects, MinecraftWorldgenBiomeElement,
    MinecraftWorldgenBiomeEntry, MinecraftWorldgenBiomeMoodSound, MonsterSpawnLightLevel,
    MonsterSpawnLightLevelRange,
};
use mc_registry::client_bound::play::JoinGame;
use mc_registry::shared_types::play::{GameType, ResourceLocation};
use mc_serializer::contextual;
use mc_serializer::primitive::VarInt;
use mc_serializer::serde::Deserialize;
use mc_serializer::serde::ProtocolVersion;
use std::io::{Cursor, Read, Write};
use tokio::io::AsyncReadExt;

#[derive(mc_serializer_derive::Serial, Debug)]
pub struct ItemStack {
    pub item_id: VarInt,
    pub count: u8,
    #[nbt]
    pub item_tag: nbt::Blob,
}

pub struct Buffer {
    write: Vec<u8>,
}

impl Write for Buffer {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let written = Write::write(&mut self.write, buf)?;
        println!("Writing buf: {}: {:?}", written, buf);
        Ok(written)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Write::flush(&mut self.write)
    }
}

fn main() {
    let join_game = JoinGame {
        player_id: 12,
        hardcore: false,
        game_type: GameType::Survival,
        previous_game_type: GameType::None,
        levels: (VarInt::from(1), vec![ResourceLocation::from("test")]),
        codec: Codec {
            dimension_registry: MinecraftDimensionType {
                value: vec![MinecraftDimensionTypeEntry {
                    id: 0,
                    name: "test".to_string(),
                    element: MinecraftDimensionTypeElement {
                        respawn_anchor_works: 0,
                        fixed_time: None,
                        has_raids: 0,
                        effects: "minecraft:overworld".to_string(),
                        natural: 1,
                        ambient_light: 0.0,
                        has_skylight: 1,
                        ultrawarm: 0,
                        coordinate_scale: 1.0,
                        infiniburn: "#minecraft:infiniburn_overworld".to_string(),
                        monster_spawn_block_light_limit: 0,
                        has_ceiling: 0,
                        monster_spawn_light_level: MonsterSpawnLightLevel::Complex {
                            type_inner: "minecraft:uniform".to_string(),
                            range: MonsterSpawnLightLevelRange {
                                min_inclusive: 0,
                                max_inclusive: 7,
                            },
                        },
                        bed_works: 1,
                        piglin_safe: 0,
                        logical_height: 384,
                        min_y: -64,
                        height: 384,
                    },
                }],
                type_inner: "minecraft:dimension_type".to_string(),
            },
            biome_registry: MinecraftWorldgenBiome {
                value: vec![MinecraftWorldgenBiomeEntry {
                    element: MinecraftWorldgenBiomeElement {
                        downfall: 0.4,
                        temperature: 0.8,
                        precipitation: "rain".to_string(),
                        temperature_modifier: None,
                        effects: MinecraftWorldgenBiomeEffects {
                            particle: None,
                            ambient_sound: None,
                            music: None,
                            water_fog_color: 329011,
                            grass_color: None,
                            fog_color: 12638463,
                            grass_color_modifier: None,
                            foliage_color: None,
                            water_color: 4159204,
                            additions_sound: None,
                            sky_color: IntCoverage(7907327),
                            mood_sound: MinecraftWorldgenBiomeMoodSound {
                                sound: "minecraft:ambient.cave".to_string(),
                                block_search_extent: 8,
                                offset: 2.0,
                                tick_delay: 6000,
                            },
                        },
                    },
                    name: "test".to_string(),
                    id: 0,
                }],
                type_inner: "minecraft:worldgen/biome".to_string(),
            },
            chat_registry: MinecraftChatType {
                value: vec![
                    MinecraftChatTypeEntry {
                        element: MinecraftChatTypeElement {
                            chat: MinecraftChatTypeChat {
                                style: None,
                                translation_key: "chat.type.text".to_string(),
                                parameters: vec!["sender".to_string(), "content".to_string()],
                            },
                            narration: MinecraftChatTypeElementNarration {
                                parameters: vec!["sender".to_string(), "content".to_string()],
                                translation_key: "chat.type.text.narrate".to_string(),
                            },
                        },
                        id: 0,
                        name: "minecraft:chat".to_string(),
                    },
                    MinecraftChatTypeEntry {
                        element: MinecraftChatTypeElement {
                            chat: MinecraftChatTypeChat {
                                style: None,
                                translation_key: "chat.type.announcement".to_string(),
                                parameters: vec!["sender".to_string(), "content".to_string()],
                            },
                            narration: MinecraftChatTypeElementNarration {
                                parameters: vec!["sender".to_string(), "content".to_string()],
                                translation_key: "chat.type.text.narrate".to_string(),
                            },
                        },
                        id: 1,
                        name: "minecraft:say_command".to_string(),
                    },
                    MinecraftChatTypeEntry {
                        element: MinecraftChatTypeElement {
                            chat: MinecraftChatTypeChat {
                                style: Some(MinecraftChatTypeStyle {
                                    italic: 1,
                                    color: "gray".to_string(),
                                }),
                                translation_key: "commands.message.display.incoming".to_string(),
                                parameters: vec!["sender".to_string(), "content".to_string()],
                            },
                            narration: MinecraftChatTypeElementNarration {
                                parameters: vec!["sender".to_string(), "content".to_string()],
                                translation_key: "chat.type.text.narrate".to_string(),
                            },
                        },
                        id: 2,
                        name: "minecraft:msg_command_incoming".to_string(),
                    },
                    MinecraftChatTypeEntry {
                        element: MinecraftChatTypeElement {
                            chat: MinecraftChatTypeChat {
                                style: Some(MinecraftChatTypeStyle {
                                    italic: 1,
                                    color: "gray".to_string(),
                                }),
                                translation_key: "commands.message.display.outgoing".to_string(),
                                parameters: vec!["target".to_string(), "content".to_string()],
                            },
                            narration: MinecraftChatTypeElementNarration {
                                parameters: vec!["sender".to_string(), "content".to_string()],
                                translation_key: "chat.type.text.narrate".to_string(),
                            },
                        },
                        id: 3,
                        name: "minecraft:msg_command_outgoing".to_string(),
                    },
                    MinecraftChatTypeEntry {
                        element: MinecraftChatTypeElement {
                            chat: MinecraftChatTypeChat {
                                style: None,
                                translation_key: "chat.type.team.text".to_string(),
                                parameters: vec![
                                    "target".to_string(),
                                    "sender".to_string(),
                                    "content".to_string(),
                                ],
                            },
                            narration: MinecraftChatTypeElementNarration {
                                parameters: vec!["sender".to_string(), "content".to_string()],
                                translation_key: "chat.type.text.narrate".to_string(),
                            },
                        },
                        id: 4,
                        name: "minecraft:team_msg_command_incoming".to_string(),
                    },
                    MinecraftChatTypeEntry {
                        element: MinecraftChatTypeElement {
                            chat: MinecraftChatTypeChat {
                                style: None,
                                translation_key: "chat.type.team.sent".to_string(),
                                parameters: vec![
                                    "target".to_string(),
                                    "sender".to_string(),
                                    "content".to_string(),
                                ],
                            },
                            narration: MinecraftChatTypeElementNarration {
                                parameters: vec!["sender".to_string(), "content".to_string()],
                                translation_key: "chat.type.text.narrate".to_string(),
                            },
                        },
                        id: 5,
                        name: "minecraft:team_msg_command_outgoing".to_string(),
                    },
                    MinecraftChatTypeEntry {
                        element: MinecraftChatTypeElement {
                            chat: MinecraftChatTypeChat {
                                style: None,
                                translation_key: "chat.type.emote".to_string(),
                                parameters: vec!["sender".to_string(), "content".to_string()],
                            },
                            narration: MinecraftChatTypeElementNarration {
                                parameters: vec!["sender".to_string(), "content".to_string()],
                                translation_key: "chat.type.emote".to_string(),
                            },
                        },
                        id: 6,
                        name: "minecraft:emote_command".to_string(),
                    },
                    MinecraftChatTypeEntry {
                        element: MinecraftChatTypeElement {
                            chat: MinecraftChatTypeChat {
                                style: None,
                                translation_key: "%s".to_string(),
                                parameters: vec!["content".to_string()],
                            },
                            narration: MinecraftChatTypeElementNarration {
                                parameters: vec!["content".to_string()],
                                translation_key: "%s".to_string(),
                            },
                        },
                        id: 7,
                        name: "minecraft:raw".to_string(),
                    },
                ],
                type_inner: "minecraft:chat_type".to_string(),
            },
        },
        dimension_type: ResourceLocation::from("dimension:overworld"),
        dimension: ResourceLocation::from("dimension:overworld"),
        seed: 0,
        max_players: 20.into(),
        chunk_radius: 11.into(),
        simulation_distance: 10.into(),
        reduced_debug_info: false,
        show_death_screen: true,
        is_debug: false,
        is_flat: true,
        last_death_location: (false, None),
    };

    println!("{:#?}", join_game);

    let mut buffer = Buffer { write: Vec::new() };
    mc_serializer::serde::Serialize::serialize(&join_game, &mut buffer, ProtocolVersion::V119_1)
        .unwrap();

    println!("Buffer: {:?}", buffer.write);

    let mut buffer = Cursor::new(buffer.write);
    let out =
        JoinGame::deserialize(&mut Buffer2 { read: buffer }, ProtocolVersion::V119_1).unwrap();

    println!("{:#?}", out)
}

struct Buffer2 {
    read: Cursor<Vec<u8>>,
}

impl Read for Buffer2 {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let read = Read::read(&mut self.read, buf)?;
        println!("Read: {} => {:?}", read, buf);
        Ok(read)
    }
}
