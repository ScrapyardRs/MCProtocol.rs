pub use flume;
pub use typemap::{Key, ShareMap};

#[macro_export]
macro_rules! assign_key {
    ($key_name:ident, $ty:ty) => {
        pub struct $key_name;
        impl $crate::prelude::Key for $key_name {
            type Value = $ty;
        }
    };
}
