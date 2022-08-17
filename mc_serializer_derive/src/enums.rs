use crate::fields::FieldsWrapper;
use proc_macro2::{Ident, Punct, Spacing, TokenStream, TokenTree};
use quote::ToTokens;
use syn::{Attribute, DataEnum, DeriveInput, Variant};

pub fn parse_key_type(attributes: &Vec<Attribute>) -> Option<TokenStream> {
    for attr in attributes {
        if let Some(segment) = attr.path.segments.first() {
            if segment.ident == "key" {
                return Some(
                    attr.parse_args()
                        .expect("A key should provide an associated value."),
                );
            }
        }
    }
    None
}

pub fn parse_variant_key_type(attributes: &Vec<Attribute>, super_key: TokenStream) -> TokenStream {
    parse_key_type(attributes).unwrap_or(quote::quote!(#super_key::from(ordinal)))
}

pub fn is_default_variant(attributes: &Vec<Attribute>) -> bool {
    for attr in attributes {
        if let Some(segment) = attr.path.segments.first() {
            if segment.ident == "default" {
                return true;
            }
        }
    }
    false
}

struct VariantWrapper {
    ordinal: i32,
    key: TokenStream,
    fields: FieldsWrapper,
    full_path: TokenStream,
    is_default: bool,
}

impl VariantWrapper {
    pub fn new(
        ordinal: i32,
        super_key: TokenStream,
        struct_ident: &Ident,
        variant: &Variant,
    ) -> Self {
        let key = parse_variant_key_type(&variant.attrs, super_key);
        let is_default = is_default_variant(&variant.attrs);
        let mut struct_context = TokenStream::new();
        struct_ident.to_tokens(&mut struct_context);
        let sep = TokenTree::Punct(Punct::new(':', Spacing::Joint));
        sep.to_tokens(&mut struct_context);
        sep.to_tokens(&mut struct_context);
        variant.ident.to_tokens(&mut struct_context);
        let fields =
            FieldsWrapper::new(&variant.fields, quote::quote!(stringify!(#struct_context)));
        Self {
            ordinal,
            key,
            fields,
            full_path: quote::quote!(#struct_context),
            is_default,
        }
    }

    fn key_err(&self) -> TokenStream {
        let self_ident = &self.full_path;
        quote::quote! {
            .map_err(|err|
                err.update_context(|ctx| {
                    ctx.current_struct(format!("{}", stringify!(#self_ident))).current_field(format!("{}", stringify!(key)));
                })
            )?
        }
    }

    fn enum_variant_def(&self, ignore_default: bool) -> TokenStream {
        let fields_variant_def = self.fields.enum_variant_def();
        let self_ident = &self.full_path;
        if self.is_default && !ignore_default {
            quote::quote!(_ =>)
        } else {
            quote::quote!(#self_ident #fields_variant_def =>)
        }
    }

    pub fn serializer(&self, passthrough: bool) -> TokenStream {
        let enum_variant_def = self.enum_variant_def(true);
        let variant_let_map = self.fields.variant_let_map();
        let serializer_stmt = self.fields.serializer();
        let key_err = self.key_err();
        let key = &self.key;
        let ordinal = self.ordinal;
        let key_ser = if passthrough {
            None
        } else {
            Some(
                quote::quote!(mc_serializer::serde::Serialize::serialize(&#key, writer, protocol_version)#key_err;),
            )
        };
        quote::quote! {
            #enum_variant_def {
                let ordinal = #ordinal;
                #key_ser
                #variant_let_map
                #serializer_stmt
                Ok(())
            }
        }
    }

    pub fn sizer(&self, passthrough: bool) -> TokenStream {
        let enum_variant_def = self.enum_variant_def(true);
        let variant_let_map = self.fields.variant_let_map();
        let sizer_stmt = self.fields.sizer();
        let key_err = self.key_err();
        let key = &self.key;
        let ordinal = self.ordinal;
        let key_ser = if passthrough {
            None
        } else {
            Some(
                quote::quote!(mc_serializer::serde::Serialize::size(&#key, protocol_version)#key_err;),
            )
        };
        quote::quote! {
            #enum_variant_def {
                let ordinal: i32 = #ordinal;
                let mut size = 0;
                #key_ser
                #variant_let_map
                #sizer_stmt
                Ok(size)
            }
        }
    }

    pub fn deserializer_raw(&self) -> TokenStream {
        let self_ident = &self.full_path;
        let variant_make = self.fields.creation_def();
        let deserializer = self.fields.deserializer();
        quote::quote! {
            #deserializer
            return Ok(#self_ident #variant_make)
        }
    }

    pub fn deserializer(&self) -> TokenStream {
        let deserializer = self.deserializer_raw();
        let key = &self.key;
        let ordinal = self.ordinal;
        quote::quote! {
            let ordinal: i32 = #ordinal;
            if key_value == #key {
                #deserializer
            }
        }
    }
}

macro_rules! variant_def {
    ($ident:ident) => {
        let mut $ident = Some(quote::quote!(
            return Err(mc_serializer::serde::Error::Generic(
                mc_serializer::serde::SerializerContext::new(
                    Self::context(),
                    format!("Failed to read key {:?} as a valid option.", key_value)
                )
            ))
        ));
    };
}

macro_rules! key_deser {
    ($ident:ident, $passthrough:ident, $key_type:ident, $enum_ident:ident) => {
        let $ident = if $passthrough {
            None
        } else {
            Some(quote::quote! {
                let key_value = <#$key_type>::deserialize(reader, protocol_version).map_err(|err|
                    err.update_context(|ctx| {
                        ctx.current_struct(format!("{}", stringify!(#$enum_ident))).current_field(format!("{}", stringify!(key)));
                    })
                )?;
            })
        };
    }
}

pub fn expand_deserialize_enum(derive_input: &DeriveInput, syn_enum: &DataEnum) -> TokenStream {
    let enum_ident = &derive_input.ident;

    let key_type = parse_key_type(&derive_input.attrs)
        .expect(format!("No key defined for enum {:?}", derive_input.ident).as_str());

    let passthrough_key = key_type.to_string().eq("pass");

    let mut variant_deserializers = Vec::new();
    variant_def!(variant_default);

    for (index, variant) in syn_enum.variants.iter().enumerate() {
        let variant_wrapper =
            VariantWrapper::new(index as i32, key_type.clone(), enum_ident, variant);
        variant_deserializers.push(variant_wrapper.deserializer());
        if variant_wrapper.is_default {
            variant_default = Some(variant_wrapper.deserializer_raw());
        }
    }

    key_deser!(
        key_value_deserializer,
        passthrough_key,
        key_type,
        enum_ident
    );

    quote::quote! {
        impl mc_serializer::serde::Deserialize for #enum_ident {
            fn deserialize<R: std::io::Read>(
                reader: &mut R,
                protocol_version: mc_serializer::serde::ProtocolVersion,
            ) -> mc_serializer::serde::Result<Self> {
                #key_value_deserializer
                #(#variant_deserializers)*
                #variant_default
            }
        }
    }
}

pub fn expand_serialize_enum(derive_input: &DeriveInput, syn_enum: &DataEnum) -> TokenStream {
    let enum_ident = &derive_input.ident;

    let key_type = parse_key_type(&derive_input.attrs)
        .expect(format!("No key defined for enum {:?}", derive_input.ident).as_str());

    let passthrough_key = key_type.to_string().eq("pass");

    let mut variant_serializers = Vec::new();
    let mut variant_sizers = Vec::new();

    for (index, variant) in syn_enum.variants.iter().enumerate() {
        let variant_wrapper =
            VariantWrapper::new(index as i32, key_type.clone(), enum_ident, variant);
        variant_serializers.push(variant_wrapper.serializer(passthrough_key));
        variant_sizers.push(variant_wrapper.sizer(passthrough_key));
    }

    quote::quote! {
        impl mc_serializer::serde::Serialize for #enum_ident {
            fn serialize<W: std::io::Write>(
                    &self,
                    writer: &mut W,
                    protocol_version: mc_serializer::serde::ProtocolVersion,
                ) -> mc_serializer::serde::Result<()> {
                match self {
                    #(#variant_serializers)*
                }
            }

            fn size(&self, protocol_version: mc_serializer::serde::ProtocolVersion) -> mc_serializer::serde::Result<i32> {
                match self {
                    #(#variant_sizers)*
                }
            }
        }
    }
}

pub fn expand_serial_enum(derive_input: &DeriveInput, syn_enum: &DataEnum) -> TokenStream {
    let enum_ident = &derive_input.ident;

    let key_type = parse_key_type(&derive_input.attrs)
        .expect(format!("No key defined for enum {:?}", derive_input.ident).as_str());

    let passthrough_key = key_type.to_string().eq("pass");

    let mut variant_serializers = Vec::new();
    let mut variant_sizers = Vec::new();
    let mut variant_deserializers = Vec::new();
    variant_def!(variant_default);

    for (index, variant) in syn_enum.variants.iter().enumerate() {
        let variant_wrapper =
            VariantWrapper::new(index as i32, key_type.clone(), enum_ident, variant);
        variant_serializers.push(variant_wrapper.serializer(passthrough_key));
        variant_sizers.push(variant_wrapper.sizer(passthrough_key));
        variant_deserializers.push(variant_wrapper.deserializer());
        if variant_wrapper.is_default {
            variant_default = Some(variant_wrapper.deserializer_raw());
        }
    }

    let key_value_deserializer = if passthrough_key {
        None
    } else {
        Some(quote::quote! {
            let key_value = <#key_type>::deserialize(reader, protocol_version).map_err(|err|
                err.update_context(|ctx| {
                    ctx.current_struct(format!("{}", stringify!(#enum_ident))).current_field(format!("{}", stringify!(key)));
                })
            )?;
        })
    };

    quote::quote! {
        impl mc_serializer::serde::Contextual for #enum_ident {
            fn context() -> String {
                format!("{}", stringify!(#enum_ident))
            }
        }

        impl mc_serializer::serde::Serialize for #enum_ident {
            fn serialize<W: std::io::Write>(
                    &self,
                    writer: &mut W,
                    protocol_version: mc_serializer::serde::ProtocolVersion,
                ) -> mc_serializer::serde::Result<()> {
                match self {
                    #(#variant_serializers)*
                }
            }

            fn size(&self, protocol_version: mc_serializer::serde::ProtocolVersion) -> mc_serializer::serde::Result<i32> {
                match self {
                    #(#variant_sizers)*
                }
            }
        }

        impl mc_serializer::serde::Deserialize for #enum_ident {
            fn deserialize<R: std::io::Read>(
                reader: &mut R,
                protocol_version: mc_serializer::serde::ProtocolVersion,
            ) -> mc_serializer::serde::Result<Self> {
                #key_value_deserializer
                #(#variant_deserializers)*
                #variant_default
            }
        }
    }
}
