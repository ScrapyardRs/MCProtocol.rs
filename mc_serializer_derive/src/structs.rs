use proc_macro2::TokenStream;
use syn::{DataStruct, DeriveInput};

pub fn expand_serial_struct(derive_input: &DeriveInput, syn_struct: &DataStruct) -> TokenStream {
    let struct_ident = &derive_input.ident;
    let fields_wrapper = super::fields::FieldsWrapper::new(
        &syn_struct.fields,
        quote::quote!(stringify!(#struct_ident)),
    );

    let serializer = fields_wrapper.serializer();
    let deserializer = fields_wrapper.deserializer();
    let sizer = fields_wrapper.sizer();
    let creation_def = fields_wrapper.creation_def();
    let simple_let_map = fields_wrapper.simple_let_map();

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
                #simple_let_map
                #serializer
                Ok(())
            }

            fn size(&self, protocol_version: mc_serializer::serde::ProtocolVersion) -> mc_serializer::serde::Result<i32> {
                let mut size = 0;
                #simple_let_map
                #sizer
                Ok(size)
            }
        }

        impl mc_serializer::serde::Deserialize for #struct_ident {
            fn deserialize<R: std::io::Read>(
                reader: &mut R,
                protocol_version: mc_serializer::serde::ProtocolVersion,
            ) -> mc_serializer::serde::Result<Self> {
                #deserializer
                Ok(#struct_ident #creation_def)
            }
        }
    }
}
