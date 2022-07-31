extern crate proc_macro2;
extern crate quote;
extern crate syn;

use syn::{Data, DeriveInput};

mod fields;
mod structs;
mod enums;

const PREFIX: &str = "__serde_";

#[proc_macro_derive(Serial, attributes(key, serial_if, nbt, json, default))]
pub fn derive_mc_serde(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let derive_input = syn::parse_macro_input!(item as DeriveInput);

    match &derive_input.data {
        Data::Struct(data_struct) => proc_macro::TokenStream::from(structs::expand_serial_struct(&derive_input, data_struct)),
        Data::Enum(data_enum) => proc_macro::TokenStream::from(enums::expand_serial_enum(&derive_input, data_enum)),
        Data::Union(_) => panic!("Cannot apply mc serde derive on unions."),
    }
}
