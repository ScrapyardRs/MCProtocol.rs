extern crate proc_macro2;
extern crate quote;
extern crate syn;

use syn::{Data, DeriveInput};

mod bitmap;
mod directives;
mod enums;
mod fields;
mod structs;

const PREFIX: &str = "__serde_";

#[proc_macro_derive(SerialBitMap)]
pub fn derive_serial_bitmap(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let derive_input = syn::parse_macro_input!(item as DeriveInput);

    if let Data::Struct(data_struct) = &derive_input.data {
        proc_macro::TokenStream::from(bitmap::expand_serial_bitmap(&derive_input, data_struct))
    } else {
        panic!("Cannot construct a bitmap from an enum or tuple type.");
    }
}

#[proc_macro_derive(
    Serial,
    attributes(
        key,
        serial_if,
        serialize_if,
        deserialize_if,
        nbt,
        json,
        default,
        after
    )
)]
pub fn derive_mc_serde(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let derive_input = syn::parse_macro_input!(item as DeriveInput);

    match &derive_input.data {
        Data::Struct(data_struct) => {
            proc_macro::TokenStream::from(structs::expand_serial_struct(&derive_input, data_struct))
        }
        Data::Enum(data_enum) => {
            proc_macro::TokenStream::from(enums::expand_serial_enum(&derive_input, data_enum))
        }
        Data::Union(_) => panic!("Cannot apply mc serde derive on unions."),
    }
}

#[proc_macro_derive(Contextual)]
pub fn derive_contextual(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let derive_input = syn::parse_macro_input!(item as DeriveInput);
    let struct_ident = derive_input.ident;

    proc_macro::TokenStream::from(quote::quote! {
        impl mc_serializer::serde::Contextual for #struct_ident {
            fn context() -> String {
                format!("{}", stringify!(#struct_ident))
            }
        }
    })
}

#[proc_macro_derive(
    MCDeserialize,
    attributes(
        key,
        serial_if,
        serialize_if,
        deserialize_if,
        nbt,
        json,
        default,
        after
    )
)]
pub fn derive_deserialize(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let derive_input = syn::parse_macro_input!(item as DeriveInput);

    match &derive_input.data {
        Data::Struct(data_struct) => proc_macro::TokenStream::from(
            structs::expand_deserialize_struct(&derive_input, data_struct),
        ),
        Data::Enum(data_enum) => {
            proc_macro::TokenStream::from(enums::expand_deserialize_enum(&derive_input, data_enum))
        }
        Data::Union(_) => panic!("Cannot apply mc serde derive on unions."),
    }
}

#[proc_macro_derive(
    MCSerialize,
    attributes(
        key,
        serial_if,
        serialize_if,
        deserialize_if,
        nbt,
        json,
        default,
        after
    )
)]
pub fn derive_serialize(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let derive_input = syn::parse_macro_input!(item as DeriveInput);

    match &derive_input.data {
        Data::Struct(data_struct) => proc_macro::TokenStream::from(
            structs::expand_serialize_struct(&derive_input, data_struct),
        ),
        Data::Enum(data_enum) => {
            proc_macro::TokenStream::from(enums::expand_serialize_enum(&derive_input, data_enum))
        }
        Data::Union(_) => panic!("Cannot apply mc serde derive on unions."),
    }
}
