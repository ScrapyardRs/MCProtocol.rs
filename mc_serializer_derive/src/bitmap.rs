use proc_macro2::TokenStream;
use syn::{DataStruct, DeriveInput};

pub fn expand_serial_bitmap(derive_input: &DeriveInput, syn_struct: &DataStruct) -> TokenStream {
    let struct_ident = &derive_input.ident;

    let (mut ser, mut de, mut make) = (Vec::new(), Vec::new(), Vec::new());

    let mut bit_marker = 1u8;

    for field in syn_struct.fields.iter() {
        let field_ident = field.ident.as_ref().unwrap();
        ser.push(quote::quote!(if self.#field_ident { by |= #bit_marker; }));
        de.push(quote::quote!(let #field_ident = (by & #bit_marker) != 0;));
        make.push(quote::quote!(#field_ident,));
        bit_marker *= 2;
    }

    quote::quote! {
        impl mc_serializer::serde::Contextual for #struct_ident {
            fn context() -> String {
                format!("{}", stringify!(#struct_ident))
            }
        }

        impl mc_serializer::serde::Serialize for #struct_ident {
            fn serialize<W: std::io::Write>(
                    &self,
                    writer: &mut W,
                    protocol_version: mc_serializer::serde::ProtocolVersion,
                ) -> mc_serializer::serde::Result<()> {
                let mut by = 0u8;
                #(#ser)*
                u8::serialize(&by, writer, protocol_version)
            }

            fn size(&self, protocol_version: mc_serializer::serde::ProtocolVersion) -> mc_serializer::serde::Result<i32> {
                Ok(1)
            }
        }

        impl mc_serializer::serde::Deserialize for #struct_ident {
            fn deserialize<R: std::io::Read>(
                reader: &mut R,
                protocol_version: mc_serializer::serde::ProtocolVersion,
            ) -> mc_serializer::serde::Result<Self> {
                let by = u8::deserialize(reader, protocol_version)?;
                #(#de)*
                Ok(Self { #(#make)* })
            }
        }
    }
}
