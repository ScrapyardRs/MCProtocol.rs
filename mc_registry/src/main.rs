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
use mc_serializer::primitive::VarInt;
use mc_serializer::serde::ProtocolVersion;
use std::io::Cursor;

fn main() {
    let join_game = Codec {
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
            value: vec![MinecraftChatTypeEntry {
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
            }],
            type_inner: "minecraft:chat_type".to_string(),
        },
    };

    let t = T { join_game };
    let mut cursor = Cursor::new(Vec::new());
    use mc_serializer::serde::{Deserialize, Serialize};
    t.serialize(&mut cursor, ProtocolVersion::V119_1).unwrap();
    let mut cursor = Cursor::new(cursor.into_inner());
    let t: T = Deserialize::deserialize(&mut cursor, ProtocolVersion::V119_1).unwrap();
    println!("From all transport we get {:?}", t);
}

#[derive(mc_serializer_derive::Serial, Debug)]
pub struct T {
    #[nbt(inject_header)]
    join_game: Codec,
}
