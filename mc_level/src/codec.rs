#[derive(serde_derive::Serialize, serde_derive::Deserialize, Debug)]
pub struct MonsterSpawnLightLevelRange {
    pub max_inclusive: i8,
    pub min_inclusive: i8,
}

#[derive(
    mc_serializer_derive::Contextual, serde_derive::Serialize, serde_derive::Deserialize, Debug,
)]
#[serde(untagged)]
pub enum MonsterSpawnLightLevel {
    Complex {
        #[serde(rename = "type")]
        type_inner: String,
        #[serde(rename = "value")]
        range: MonsterSpawnLightLevelRange,
    },
    ByteCoverage(i8),
}

#[derive(
    mc_serializer_derive::Contextual, serde_derive::Serialize, serde_derive::Deserialize, Debug,
)]
pub struct MinecraftDimensionTypeElement {
    pub respawn_anchor_works: i8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fixed_time: Option<i16>,
    pub has_raids: i8,
    pub effects: String,
    pub natural: i8,
    pub ambient_light: f32,
    pub has_skylight: i8,
    pub ultrawarm: i8,
    pub coordinate_scale: f32,
    pub infiniburn: String,
    pub monster_spawn_block_light_limit: i8,
    pub has_ceiling: i8,
    pub monster_spawn_light_level: MonsterSpawnLightLevel,
    pub bed_works: i8,
    pub piglin_safe: i8,
    pub logical_height: i16,
    pub min_y: i8,
    pub height: i16,
}

#[derive(
    mc_serializer_derive::Contextual, serde_derive::Serialize, serde_derive::Deserialize, Debug,
)]
pub struct MinecraftDimensionTypeEntry {
    pub id: i8,
    pub name: String,
    pub element: MinecraftDimensionTypeElement,
}
pub type MinecraftDimensionTypeEntries = Vec<MinecraftDimensionTypeEntry>;

#[derive(
    mc_serializer_derive::Contextual, serde_derive::Serialize, serde_derive::Deserialize, Debug,
)]
pub struct MinecraftDimensionType {
    pub value: MinecraftDimensionTypeEntries,
    #[serde(rename = "type")]
    pub type_inner: String,
}

#[derive(
    mc_serializer_derive::Contextual, serde_derive::Serialize, serde_derive::Deserialize, Debug,
)]
pub struct MinecraftWorldgenBiomeOptions {
    #[serde(rename = "type")]
    pub type_inner: String,
}

#[derive(
    mc_serializer_derive::Contextual, serde_derive::Serialize, serde_derive::Deserialize, Debug,
)]
pub struct MinecraftWorldgenBiomeParticle {
    pub options: MinecraftWorldgenBiomeOptions,
    pub probability: f32,
}

#[derive(
    mc_serializer_derive::Contextual, serde_derive::Serialize, serde_derive::Deserialize, Debug,
)]
pub struct MinecraftWorldgenBiomeMusic {
    pub min_delay: i16,
    pub sound: String,
    pub replace_current_music: i8,
    pub max_delay: i16,
}

#[derive(
    mc_serializer_derive::Contextual, serde_derive::Serialize, serde_derive::Deserialize, Debug,
)]
pub struct MinecraftWorldgenBiomeAdditionsSound {
    pub tick_chance: f32,
    pub sound: String,
}

#[derive(
    mc_serializer_derive::Contextual, serde_derive::Serialize, serde_derive::Deserialize, Debug,
)]
#[serde(untagged)]
pub enum MinecraftWorldgenBiomeSkyColor {
    IntCoverage(i32),
    ByteCoverage(i8),
}

#[derive(
    mc_serializer_derive::Contextual, serde_derive::Serialize, serde_derive::Deserialize, Debug,
)]
pub struct MinecraftWorldgenBiomeMoodSound {
    pub sound: String,
    pub block_search_extent: i8,
    pub offset: f32,
    pub tick_delay: i16,
}

#[derive(
    mc_serializer_derive::Contextual, serde_derive::Serialize, serde_derive::Deserialize, Debug,
)]
pub struct MinecraftWorldgenBiomeEffects {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub particle: Option<MinecraftWorldgenBiomeParticle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ambient_sound: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub music: Option<MinecraftWorldgenBiomeMusic>,
    pub water_fog_color: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grass_color: Option<i32>,
    pub fog_color: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grass_color_modifier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub foliage_color: Option<i32>,
    pub water_color: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additions_sound: Option<MinecraftWorldgenBiomeAdditionsSound>,
    pub sky_color: MinecraftWorldgenBiomeSkyColor,
    pub mood_sound: MinecraftWorldgenBiomeMoodSound,
}

#[derive(
    mc_serializer_derive::Contextual, serde_derive::Serialize, serde_derive::Deserialize, Debug,
)]
pub struct MinecraftWorldgenBiomeElement {
    pub downfall: f32,
    pub temperature: f32,
    pub precipitation: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature_modifier: Option<String>,
    pub effects: MinecraftWorldgenBiomeEffects,
}

#[derive(
    mc_serializer_derive::Contextual, serde_derive::Serialize, serde_derive::Deserialize, Debug,
)]
pub struct MinecraftWorldgenBiomeEntry {
    pub element: MinecraftWorldgenBiomeElement,
    pub name: String,
    pub id: i8,
}
pub type MinecraftWorldgenBiomeEntries = Vec<MinecraftWorldgenBiomeEntry>;

#[derive(
    mc_serializer_derive::Contextual, serde_derive::Serialize, serde_derive::Deserialize, Debug,
)]
pub struct MinecraftWorldgenBiome {
    pub value: MinecraftWorldgenBiomeEntries,
    #[serde(rename = "type")]
    pub type_inner: String,
}

#[derive(
    mc_serializer_derive::Contextual, serde_derive::Serialize, serde_derive::Deserialize, Debug,
)]
pub struct MinecraftChatTypeStyle {
    pub italic: i8,
    pub color: String,
}

pub type MinecraftChatTypeParameters = Vec<String>;

#[derive(
    mc_serializer_derive::Contextual, serde_derive::Serialize, serde_derive::Deserialize, Debug,
)]
pub struct MinecraftChatTypeChat {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<MinecraftChatTypeStyle>,
    pub translation_key: String,
    pub parameters: MinecraftChatTypeParameters,
}

pub type MinecraftChatTypeElementParameters = Vec<String>;

#[derive(
    mc_serializer_derive::Contextual, serde_derive::Serialize, serde_derive::Deserialize, Debug,
)]
pub struct MinecraftChatTypeElementNarration {
    pub parameters: MinecraftChatTypeElementParameters,
    pub translation_key: String,
}

#[derive(
    mc_serializer_derive::Contextual, serde_derive::Serialize, serde_derive::Deserialize, Debug,
)]
pub struct MinecraftChatTypeElement {
    pub chat: MinecraftChatTypeChat,
    pub narration: MinecraftChatTypeElementNarration,
}

#[derive(
    mc_serializer_derive::Contextual, serde_derive::Serialize, serde_derive::Deserialize, Debug,
)]
pub struct MinecraftChatTypeEntry {
    pub element: MinecraftChatTypeElement,
    pub id: i8,
    pub name: String,
}
pub type MinecraftChatTypeEntries = Vec<MinecraftChatTypeEntry>;

#[derive(
    mc_serializer_derive::Contextual, serde_derive::Serialize, serde_derive::Deserialize, Debug,
)]
pub struct MinecraftChatType {
    pub value: MinecraftChatTypeEntries,
    #[serde(rename = "type")]
    pub type_inner: String,
}

#[derive(
    mc_serializer_derive::Contextual, serde_derive::Serialize, serde_derive::Deserialize, Debug,
)]
pub struct Codec {
    #[serde(rename = "minecraft:dimension_type")]
    pub dimension_registry: MinecraftDimensionType,
    #[serde(rename = "minecraft:worldgen/biome")]
    pub biome_registry: MinecraftWorldgenBiome,
    #[serde(rename = "minecraft:chat_type")]
    pub chat_registry: MinecraftChatType,
}
