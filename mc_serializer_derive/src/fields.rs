use core::default::Default;
use proc_macro2::{Ident, Span, TokenStream};
use quote::ToTokens;
use syn::{Attribute, Field, Fields, Type};
use crate::directives::after::AfterDirective;

pub enum SerialType {
    Default,
    Nbt,
    Json(TokenStream),
}

impl Default for SerialType {
    fn default() -> Self {
        SerialType::Default
    }
}

#[derive(Default)]
pub(crate) struct SerialConfig {
    serial_type: SerialType,
    conditional: Option<TokenStream>,
    default: Option<TokenStream>,
    after_directive: AfterDirective,
}

impl SerialConfig {
    fn parse_attribute(&mut self, attribute: &Attribute) {
        let segment = attribute.path.segments.last().unwrap();

        match segment.ident.to_string().as_str() {
            "nbt" => self.serial_type = SerialType::Nbt,
            "json" => {
                self.serial_type = SerialType::Json(
                    attribute
                        .parse_args()
                        .expect("Please include a max length for the json string."),
                )
            }
            "serial_if" => {
                self.conditional = Some(
                    attribute
                        .parse_args()
                        .expect("Please provide a conditional for the `serial_if` operator."),
                )
            }
            "default" => {
                self.default = Some(
                    attribute
                        .parse_args()
                        .expect("Please provide an expression for the default value."),
                )
            }
            "after" => {
                crate::directives::after::parse_after_directive(&mut self.after_directive, attribute.parse_args().expect("Please provide arguments for the after directive."))
            }
            _ => (),
        }
    }

    pub(crate) fn new(attributes: &Vec<Attribute>) -> Self {
        let mut marker = Self::default();
        for attribute in attributes {
            marker.parse_attribute(attribute);
        }
        marker
    }
}

struct SerialContext {
    unnamed: (bool, usize),
    struct_context: TokenStream,
    variable_name: Ident,
    field_name: Ident,
    marker: SerialConfig,
    ty: Type,
}

impl SerialContext {
    pub fn named(field: &Field, struct_context: &TokenStream) -> Self {
        let field_ident = field
            .ident
            .as_ref()
            .expect("Named fields should have idents.");
        Self {
            unnamed: (false, 0),
            struct_context: struct_context.to_token_stream(),
            variable_name: Ident::new(
                format!("{}{}", super::PREFIX, field_ident).as_str(),
                Span::call_site(),
            ),
            field_name: Ident::new(field_ident.to_string().as_str(), Span::call_site()),
            marker: SerialConfig::new(&field.attrs),
            ty: field.ty.clone(),
        }
    }

    pub fn unnamed(field: &Field, field_ident: Ident, index: usize, struct_context: &TokenStream) -> Self {
        Self {
            unnamed: (true, index),
            struct_context: struct_context.to_token_stream(),
            variable_name: Ident::new(
                format!("{}{}", super::PREFIX, field_ident).as_str(),
                Span::call_site(),
            ),
            field_name: field_ident,
            marker: SerialConfig::new(&field.attrs),
            ty: field.ty.clone(),
        }
    }

    fn serializer_short(&self, struct_context: &TokenStream, raw: TokenStream) -> TokenStream {
        let real_field_ident = &self.field_name;

        let serializer_base = quote::quote! {
            #raw.map_err(|err| err.update_context(|ctx| {
                ctx.current_struct(format!("{}", #struct_context)).current_field(format!("{}", stringify!(#real_field_ident)));
            }))?;
        };

        self.marker
            .conditional
            .as_ref()
            .map(|conditional| {
                let serializer_base = serializer_base.to_token_stream();
                quote::quote! {
                    if #conditional {
                        #serializer_base
                    };
                }
            })
            .unwrap_or(serializer_base)
    }

    pub fn serializer(&self) -> TokenStream {
        let struct_context = &self.struct_context;
        let field_ident = &self.variable_name;

        let raw_serializer = match &self.marker.serial_type {
            SerialType::Default => {
                quote::quote!(mc_serializer::serde::Serialize::serialize(#field_ident, writer, protocol_version))
            }
            SerialType::Nbt => {
                quote::quote!(mc_serializer::ext::write_nbt(#field_ident, writer, protocol_version))
            }
            SerialType::Json(max_length_tokens) => {
                quote::quote!(mc_serializer::ext::write_json(#max_length_tokens, #field_ident, writer, protocol_version))
            }
        };

        let short = self.serializer_short(struct_context, raw_serializer);
        match self.marker.after_directive.ser.as_ref() {
            None => short,
            Some(operator) => quote::quote! {
                #short
                #operator
            }
        }
    }

    pub fn sizer(&self) -> TokenStream {
        let struct_context = &self.struct_context;
        let field_ident = &self.variable_name;

        let raw_serializer = match &self.marker.serial_type {
            SerialType::Default => {
                quote::quote!(size += mc_serializer::serde::Serialize::size(#field_ident, protocol_version))
            }
            SerialType::Nbt => {
                quote::quote!(size += mc_serializer::ext::size_nbt(#field_ident, protocol_version))
            }
            SerialType::Json(_) => {
                quote::quote!(size += mc_serializer::ext::size_json(#field_ident, protocol_version))
            }
        };

        self.serializer_short(struct_context, raw_serializer)
    }

    pub fn deserializer(&self) -> TokenStream {
        let struct_context = &self.struct_context;
        let field_ident = &self.variable_name;
        let real_field_ident = &self.field_name;

        let raw_deserializer = match &self.marker.serial_type {
            SerialType::Default => quote::quote!(mc_serializer::serde::Deserialize::deserialize(
                reader,
                protocol_version
            )),
            SerialType::Nbt => {
                quote::quote!(mc_serializer::ext::read_nbt(reader, protocol_version))
            }
            SerialType::Json(max_length_tokens) => {
                quote::quote!(mc_serializer::ext::read_json(#max_length_tokens, reader, protocol_version))
            }
        };

        let serializer_base = quote::quote! {
            #raw_deserializer.map_err(|err| err.update_context(|ctx| {
                ctx.current_struct(format!("{}", #struct_context)).current_field(format!("{}", stringify!(#real_field_ident)));
            }))?
        };

        let ty = &self.ty;

        let tokens = self.marker
            .conditional
            .as_ref()
            .map(|conditional| {
                let serializer_base = serializer_base.to_token_stream();
                let otherwise = self
                    .marker
                    .default
                    .as_ref()
                    .map(|ts| ts.to_token_stream())
                    .unwrap_or(quote::quote!(None));
                quote::quote! {
                    let #field_ident: #ty = if #conditional {
                        #serializer_base
                    } else {
                        #otherwise
                    };
                }
            })
            .unwrap_or(quote::quote!(let #field_ident: #ty = #serializer_base;));
        match self.marker.after_directive.de.as_ref() {
            None => tokens,
            Some(operator) => quote::quote! {
                #tokens
                #operator
            }
        }
    }

    pub fn enum_variant_def(&self) -> TokenStream {
        let real_field_name = &self.field_name;
        quote::quote!(#real_field_name,)
    }

    pub fn creation_def(&self) -> TokenStream {
        let real_field_name = &self.field_name;
        let fake_field_name = &self.variable_name;
        if self.unnamed.0 {
            quote::quote!(#fake_field_name,)
        } else {
            quote::quote!(#real_field_name: #fake_field_name,)
        }
    }

    pub fn simple_let_map(&self) -> TokenStream {
        let real_field_name = &self.field_name;
        let fake_field_name = &self.variable_name;
        if self.unnamed.0 {
            let index = syn::Index::from(self.unnamed.1);
            quote::quote!(let #fake_field_name = &self.#index;)
        } else {
            quote::quote!(let #fake_field_name = &self.#real_field_name;)
        }
    }

    pub fn variant_let_map(&self) -> TokenStream {
        let real_field_name = &self.field_name;
        let fake_field_name = &self.variable_name;
        quote::quote!(let #fake_field_name = #real_field_name;)
    }
}

pub struct FieldsWrapper {
    unnamed: bool,
    fields: Vec<SerialContext>,
}

impl FieldsWrapper {
    pub fn new(fields: &Fields, struct_context: TokenStream) -> Self {
        let mut serial_fields = Vec::with_capacity(fields.len());
        let mut unnamed = true;
        match fields {
            Fields::Named(named) => {
                unnamed = false;
                for field in named.named.iter() {
                    serial_fields.push(SerialContext::named(field, &struct_context))
                }
            }
            Fields::Unnamed(unnamed) => {
                for (index, field) in unnamed.unnamed.iter().enumerate() {
                    let field_ident =
                        Ident::new(format!("tuple_v{}", index).as_str(), Span::call_site());
                    serial_fields.push(SerialContext::unnamed(field, field_ident, index, &struct_context))
                }
            }
            Fields::Unit => {
                return Self {
                    unnamed,
                    fields: vec![],
                };
            }
        }
        Self {
            unnamed,
            fields: serial_fields,
        }
    }

    pub fn serializer(&self) -> TokenStream {
        let tokens = self
            .fields
            .iter()
            .map(|item| item.serializer())
            .collect::<Vec<TokenStream>>();
        quote::quote!(#(#tokens)*)
    }

    pub fn sizer(&self) -> TokenStream {
        let tokens = self
            .fields
            .iter()
            .map(|item| item.sizer())
            .collect::<Vec<TokenStream>>();
        quote::quote!(#(#tokens)*)
    }

    pub fn deserializer(&self) -> TokenStream {
        let tokens = self
            .fields
            .iter()
            .map(|item| item.deserializer())
            .collect::<Vec<TokenStream>>();
        quote::quote!(#(#tokens)*)
    }

    pub fn enum_variant_def(&self) -> TokenStream {
        if self.fields.is_empty() {
            return quote::quote!();
        }
        let tokens = self
            .fields
            .iter()
            .map(|item| item.enum_variant_def())
            .collect::<Vec<TokenStream>>();
        if self.unnamed {
            quote::quote!((#(#tokens)*))
        } else {
            quote::quote!({#(#tokens)*})
        }
    }

    pub fn creation_def(&self) -> TokenStream {
        if self.fields.is_empty() {
            return quote::quote!();
        }
        let tokens = self
            .fields
            .iter()
            .map(|item| item.creation_def())
            .collect::<Vec<TokenStream>>();
        if self.unnamed {
            quote::quote!((#(#tokens)*))
        } else {
            quote::quote!({#(#tokens)*})
        }
    }

    pub fn simple_let_map(&self) -> TokenStream {
        let tokens = self
            .fields
            .iter()
            .map(|item| item.simple_let_map())
            .collect::<Vec<TokenStream>>();
        quote::quote!(#(#tokens)*)
    }

    pub fn variant_let_map(&self) -> TokenStream {
        let tokens = self
            .fields
            .iter()
            .map(|item| item.variant_let_map())
            .collect::<Vec<TokenStream>>();
        quote::quote!(#(#tokens)*)
    }
}
