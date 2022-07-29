use std::collections::HashSet;
use std::fmt::Display;
use proc_macro2::{Ident, Span, TokenStream};
use quote::ToTokens;
use syn::{Attribute, Field, Fields};
use syn::ReturnType::Default;

pub struct FieldMarker {
    is_nbt: bool,
    conditional: Option<TokenStream>,
}

impl FieldMarker {
    fn parse_attribute(&mut self, attribute: &Attribute) {
        let segment = attribute.path.segments.last().unwrap();
        match segment.ident.to_string() {
            &"nbt" => self.is_nbt = true,
            &"serial_if" => self.conditional = Some(attribute.tokens.to_token_stream()),
            _ => None,
        }
    }

    pub fn new(attributes: &Vec<Attribute>) -> Self {
        let mut marker = Self {
            is_nbt: false,
            conditional: None,
        };
        for attribute in attributes {
            marker.parse_attribute(attribute);
        }
        marker
    }
}

struct FieldContext {
    wrapping_struct: TokenStream,
    hidden_ident: Ident,
    true_ident: Ident,
    marker: FieldMarker,
}

impl FieldContext {
    fn serializer(&self) -> TokenStream {
        let struct_context = &self.wrapping_struct;
        let field_ident = &self.hidden_ident;
        let serializer_base = if self.marker.is_nbt {
            quote::quote!(nbt::ser::to_writer)
        } else {
            quote::quote!(mc_serializer::serde::Serialize::serialize_with_protocol)
        };

        let serializer_base = quote::quote! {
            anyhow::Context::context(
                #serializer_base(
                    #field_ident,
                    writer
                ),
                format!("Failure in serialization struct_context:{}; field:{}", #struct_context, stringify!(#field_ident))
            )?;
        };

        self.marker.conditional.map(|conditional| {
            let serializer_base = serializer_base.to_token_stream();
            quote::quote! {
                if #conditional {
                    #serializer_base
                }
            }
        }).unwrap_or(serializer_base)
    }

    fn size(&self) -> TokenStream {
        
    }

    fn deserializer(&self) -> TokenStream {}

    fn make(&self) -> TokenStream {}

    fn raw_make(&self) -> TokenStream {}
}

struct FieldsContext {
    wrapping_struct: TokenStream,
    fields: Vec<FieldContext>,
}

impl FieldsContext {
    fn create_field_context<D: Display>(struct_context: TokenStream, field_ident: D, field: &Field) -> FieldContext {
        FieldContext {
            wrapping_struct: struct_context,
            hidden_ident: Ident::new(format!("{}{}", super::PREFIX, field_ident).as_str(), Span::call_site()),
            true_ident: Ident::new(format!("{}", field_ident).as_str(), Span::call_site()),
            marker: FieldMarker::new(&field.attrs),
        }
    }

    pub fn new(struct_context: TokenStream, fields: &Fields) -> Self {
        let mut field_contexts = Vec::with_capacity(fields.len());

        match fields {
            Fields::Named(named_fields) => {
                for field in named_fields.named.iter() {
                    let field_ident = field.ident.as_ref().unwrap();
                    field_contexts.push(Self::create_field_context(
                        struct_context.to_token_stream(), field_ident, field));
                }
            }
            Fields::Unnamed(unnamed_fields) => {
                for (index, field) in unnamed_fields.unnamed.iter().enumerate() {
                    let field_ident = format!("tuple_v{}", index);
                    field_contexts.push(Self::create_field_context(
                        struct_context.to_token_stream(), field_ident, field));
                }
            }
            Fields::Unit => panic!("Unsupported unit fields."),
        }

        Self {
            wrapping_struct: struct_context,
            fields: field_contexts,
        }
    }
}

fn parse_attribute(attribute: &Attribute, marker: &mut FieldMarker) {
    let segment = attribute.path.segments.last().unwrap();
    match segment.ident.to_string() {
        &"nbt" => marker.is_nbt = true,
        &"serial_if" => marker.conditional = Some(attribute.tokens.to_token_stream()),
        _ => None,
    }
}

fn parse_field_attributes(field: &Field) -> FieldMarker {
    let mut marker = FieldMarker::default();
    for attribute in field.attrs.iter() {
        parse_attribute(attribute, &mut marker);
    }
    marker
}

pub fn make_field_serializer(struct_context: &TokenStream, field_ident: &Ident, field: &Field) -> TokenStream {
    let marker = parse_field_attributes(field);

    let serializer_base = if marker.is_nbt {
        quote::quote!(
            anyhow::Context::context(
                nbt::ser::to_writer(
                    #field_ident,
                    writer
                ),
                format!("Failure in serialization struct_context:{}; field:{}", #struct_context, stringify!(#field_ident))
            )?;
        )
    } else {
        quote::quote!(
            anyhow::Context::context(
                mc_serializer::serde::Serialize::serialize_with_protocol(
                    #field_ident,
                    writer
                ),
                format!("Failure in serialization struct_context:{}; field:{}", #struct_context, stringify!(#field_ident))
            )?;
        )
    };

    marker.conditional.map(|conditional| {
        let serializer_base = serializer_base.to_token_stream();
        quote::quote! {
            if #conditional {
                #serializer_base
            }
        }
    }).unwrap_or(serializer_base)
}

pub fn make_field_deserializer(struct_context: &TokenStream, field_ident: &Ident, field: &Field) -> TokenStream {
    let marker = parse_field_attributes(field);

    let deserializer_base = if marker.is_nbt {
        quote::quote!(
            anyhow::Context::context(
                nbt::de::from_reader(reader),
                format!("Failure in deserialization struct_context:{}; field:{}", #struct_context, stringify!(#field_ident))
            )?;
        )
    } else {
        quote::quote!(
            anyhow::Context::context(
                mc_serializer::serde::Deserialize::deserialize_with_protocol(reader),
                format!("Failure in deserialization struct_context:{}; field:{}", #struct_context, stringify!(#field_ident))
            )?;
        )
    };

    marker.conditional.map(|conditional| {
        let deserializer_base = deserializer_base.to_token_stream();
        quote::quote! {
            let #field_ident = if #conditional {
                #deserializer_base
            } else {
                None
            };
        }
    }).unwrap_or(quote::quote!(let #field_ident = #deserializer_base;))
}
