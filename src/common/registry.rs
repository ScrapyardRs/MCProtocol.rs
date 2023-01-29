use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;

#[macro_export]
macro_rules! lock_static {
    ($ident:ident -> $ty:ty => $fn_create:ident) => {
        pub static $ident: std::sync::LazyLock<$ty> =
            std::sync::LazyLock::new(|| <$ty>::$fn_create());
    };
}

lock_static!(GLOBAL_REGISTRIES -> GlobalRegistry => create);

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq, Ord, PartialOrd)]
pub enum RegistryKey {
    Blocks,
    BlockStates,
    Items,
}

impl RegistryKey {
    pub fn global<I: InheritKeyType>(self, key: I) -> Option<I::OtherIdentifier> {
        GLOBAL_REGISTRIES.get(self, key)
    }
}

pub struct GlobalRegistry {
    pub registries: HashMap<RegistryKey, Registry>,
}

impl GlobalRegistry {
    pub fn create() -> GlobalRegistry {
        let mut registries = HashMap::new();
        macro_rules! inherit_registry {
            ($ident:ident -> $from:literal) => {
                registries.insert(
                    RegistryKey::$ident,
                    Registry::from_json_bytes(include_bytes!($from)),
                );
            };
        }
        inherit_registry!(Blocks -> "./registry/blocks.json");
        inherit_registry!(BlockStates -> "./registry/block_states.json");
        inherit_registry!(Items -> "./registry/items.json");
        GlobalRegistry { registries }
    }

    pub fn get_id(&self, registry: RegistryKey, key: &str) -> Option<i32> {
        self.registries.get(&registry)?.get_id(key)
    }

    pub fn get_key(&self, registry: RegistryKey, idx: i32) -> Option<String> {
        self.registries.get(&registry)?.get_key(idx)
    }

    pub fn get<I: InheritKeyType>(
        &self,
        registry: RegistryKey,
        key_type: I,
    ) -> Option<I::OtherIdentifier> {
        self.registries.get(&registry)?.get(key_type)
    }
}

#[derive(Serialize, Deserialize)]
pub struct RegistryItem {
    key: String,
    idx: i32,
}

#[derive(Default)]
pub struct Registry {
    to_id: HashMap<String, i32>,
    to_key: HashMap<i32, String>,
}

pub enum KeyType {
    String(String),
    Int(i32),
}

pub trait InheritKeyType {
    type OtherIdentifier;

    fn parse_out(&self, other: KeyType) -> Self::OtherIdentifier;

    fn create_key_type(&self) -> KeyType;
}

macro_rules! impl_str_key_type {
    ($string_ref:ty) => {
        impl InheritKeyType for $string_ref {
            type OtherIdentifier = i32;

            fn parse_out(&self, other: KeyType) -> Self::OtherIdentifier {
                match other {
                    KeyType::Int(idx) => idx,
                    _ => panic!("KeyType::parse_out: expected KeyType::Int"),
                }
            }

            fn create_key_type(&self) -> KeyType {
                KeyType::String(self.to_string())
            }
        }
    };
}

impl_str_key_type!(String);
impl_str_key_type!(&str);

impl InheritKeyType for i32 {
    type OtherIdentifier = String;

    fn parse_out(&self, other: KeyType) -> Self::OtherIdentifier {
        match other {
            KeyType::String(key) => key,
            _ => panic!("KeyType::parse_out: expected KeyType::String"),
        }
    }

    fn create_key_type(&self) -> KeyType {
        KeyType::Int(*self)
    }
}

impl From<i32> for KeyType {
    fn from(value: i32) -> Self {
        Self::Int(value)
    }
}

impl Registry {
    pub fn register(&mut self, key: String, idx: i32) {
        self.to_id.insert(key.clone(), idx);
        self.to_key.insert(idx, key);
    }

    pub fn get_id(&self, key: &str) -> Option<i32> {
        self.to_id.get(key).map(|x| *x)
    }

    pub fn get_key(&self, idx: i32) -> Option<String> {
        self.to_key.get(&idx).cloned()
    }

    pub fn get<I: InheritKeyType>(&self, key_type: I) -> Option<I::OtherIdentifier> {
        match key_type.create_key_type() {
            KeyType::String(key) => self
                .get_id(&key)
                .map(|idx| key_type.parse_out(KeyType::Int(idx))),
            KeyType::Int(idx) => self
                .get_key(idx)
                .map(|key| key_type.parse_out(KeyType::String(key))),
        }
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
