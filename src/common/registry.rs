use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq, Ord, PartialOrd)]
pub enum RegistryKey {
    Blocks,
    BlockStates,
    Items,
}

pub struct GlobalRegistry {
    pub registries: HashMap<RegistryKey, Registry>,
}

impl GlobalRegistry {
    pub fn create() -> GlobalRegistry {
        let mut registries = HashMap::new();
        registries.insert(
            RegistryKey::Blocks,
            Registry::from_json_bytes(include_bytes!("./registry/blocks.json")),
        );
        registries.insert(
            RegistryKey::BlockStates,
            Registry::from_json_bytes(include_bytes!("./registry/block_states.json")),
        );
        registries.insert(
            RegistryKey::Items,
            Registry::from_json_bytes(include_bytes!("./registry/items.json")),
        );
        GlobalRegistry { registries }
    }

    pub fn get_id(&self, registry: RegistryKey, key: &str) -> Option<&i32> {
        self.registries.get(&registry)?.get_id(key)
    }

    pub fn get_key(&self, registry: RegistryKey, idx: i32) -> Option<&String> {
        self.registries.get(&registry)?.get_key(idx)
    }
}

#[derive(Serialize, Deserialize)]
struct RegistryItem {
    key: String,
    idx: i32,
}

#[derive(Default)]
pub struct Registry {
    to_id: HashMap<String, i32>,
    to_key: HashMap<i32, String>,
}

impl Registry {
    pub fn register(&mut self, key: String, idx: i32) {
        self.to_id.insert(key.clone(), idx);
        self.to_key.insert(idx, key);
    }

    pub fn get_id(&self, key: &str) -> Option<&i32> {
        self.to_id.get(key)
    }

    pub fn get_key(&self, idx: i32) -> Option<&String> {
        self.to_key.get(&idx)
    }

    pub fn from_json_bytes(slice: &[u8]) -> Registry {
        let mut reg = Registry::default();
        let items: Vec<RegistryItem> = serde_json::from_slice(slice).unwrap();
        for item in items {
            reg.register(format!("{}", item.key), item.idx);
        }
        reg
    }
}
