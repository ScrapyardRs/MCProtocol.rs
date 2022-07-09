extern crate minecraft_serde;
extern crate proc_macro2;
extern crate quote;
extern crate syn;

use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{Attribute, Data, DataEnum, DataStruct, DeriveInput, Fields};

const PREFIX: &str = "__serde_";

struct SerdePart(TokenStream);

impl ToTokens for SerdePart {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.0.to_tokens(tokens)
    }
}

#[proc_macro_derive(MCSerde, attributes(key))]
pub fn derive_mc_serde(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let derive_input = syn::parse_macro_input!(item as DeriveInput);

    match &derive_input.data {
        Data::Struct(data_struct) => {
            proc_macro::TokenStream::from(expand_mc_serde_struct(&derive_input, data_struct))
        }
        Data::Enum(data_enum) => {
            proc_macro::TokenStream::from(expand_mc_serde_enum(&derive_input, data_enum))
        }
        Data::Union(_) => panic!("Cannot apply mc serde derive on unions."),
    }
}

fn expand_mc_serde_struct(input: &DeriveInput, data_struct: &DataStruct) -> TokenStream {
    let struct_ident = &input.ident;
    let serialize_stream = field_serializer_dynamic(&data_struct.fields);
    let deserialize_stream = field_deserializer_dynamic(&data_struct.fields);
    let make_stream = make_statements(&data_struct.fields, PREFIX);
    let size_stream = field_sizer_dynamic(&data_struct.fields);

    let mut index = 0;
    let self_resolvers = data_struct
        .fields
        .iter()
        .map(|field| {
            let field_ident = mapped_ident(PREFIX, index, &field.ident.as_ref());
            let alt_field_ident = mapped_ident("", index, &field.ident.as_ref());
            index += 1;
            quote!(let #field_ident = &self.#alt_field_ident;)
        })
        .collect::<Vec<TokenStream>>();
    let self_resolvers_clone = self_resolvers.clone();
    let ser = quote! {
        impl minecraft_serde::serde::Serialize for #struct_ident {
            fn serialize<W: std::io::Write>(&self, writer: &mut W) -> minecraft_serde::serde::SerdeResult<()> {
                #(#self_resolvers)*
                #serialize_stream
                Ok(())
            }

            fn size(&self) -> minecraft_serde::serde::SerdeResult<i32> {
                let mut size = 0;
                #(#self_resolvers_clone)*
                #size_stream
                Ok(size)
            }
        }
    };

    quote! {
        #ser

        impl minecraft_serde::serde::Deserialize for #struct_ident {
            fn deserialize<R: std::io::Read>(reader: &mut R) -> minecraft_serde::serde::SerdeResult<Self> {
                #deserialize_stream
                Ok(Self #make_stream)
            }
        }
    }
}

fn expand_mc_serde_enum(input: &DeriveInput, data_enum: &DataEnum) -> TokenStream {
    fn get_key_type(attributes: &Vec<Attribute>) -> TokenStream {
        for attr in attributes {
            if let Some(segment) = attr.path.segments.first() {
                if segment.ident == "key" {
                    return attr.parse_args().expect("A key should provide a type.");
                }
            }
        }
        panic!("Failed to resolve key for enum auto-MCSerde.")
    }

    let key_type = get_key_type(&input.attrs);

    let input_ident = &input.ident;

    let mut serializer_stream = TokenStream::new();
    let mut deserializer_stream = TokenStream::new();
    let mut sizer_stream = TokenStream::new();

    for variant in &data_enum.variants {
        let variant_ident = &variant.ident;
        let key = get_key_type(&variant.attrs);

        let serialize_stream = field_serializer_dynamic(&variant.fields);
        let deserialize_stream = field_deserializer_dynamic(&variant.fields);
        let size_stream = field_sizer_dynamic(&variant.fields);
        let make_stream = make_statements(&variant.fields, PREFIX);
        let raw_make_stream = make_statements(&variant.fields, "");

        (quote! {
            #input_ident::#variant_ident #raw_make_stream => {
                minecraft_serde::serde::Serialize::serialize(&#key, writer)?;
                #serialize_stream
                Ok(())
            }
        })
        .to_tokens(&mut serializer_stream);
        (quote! {
            if key_value == #key {
                #deserialize_stream
                return Ok(#input_ident::#variant_ident #make_stream)
            }
        })
        .to_tokens(&mut deserializer_stream);
        (quote! {
            #input_ident::#variant_ident #raw_make_stream => {
                let mut size = 0;
                size += minecraft_serde::serde::Serialize::size(&#key)?;
                #size_stream
                Ok(size)
            }
        })
        .to_tokens(&mut sizer_stream);
    }

    quote! {
        impl minecraft_serde::serde::Serialize for #input_ident {
            fn serialize<W: std::io::Write>(&self, writer: &mut W) -> minecraft_serde::serde::SerdeResult<()> {
                match self {
                    #serializer_stream
                }
            }

            fn size(&self) -> minecraft_serde::serde::SerdeResult<i32> {
                match self {
                    #sizer_stream
                }
            }
        }

        impl minecraft_serde::serde::Deserialize for #input_ident {
            fn deserialize<R: std::io::Read>(reader: &mut R) -> minecraft_serde::serde::SerdeResult<Self> {
                let key_value: #key_type = minecraft_serde::serde::Deserialize::deserialize(reader)?;
                #deserializer_stream
                Err(minecraft_serde::serde::Error::Generic(format!("Failed to understand key {:?} for {}", key_value, stringify!(#input_ident))))
            }
        }
    }
}

fn field_serializer_dynamic(fields: &Fields) -> TokenStream {
    let mut serializer_stmts = Vec::new();
    for (index, field) in fields.into_iter().enumerate() {
        let field_ident = mapped_ident(PREFIX, index as i32, &field.ident.as_ref());
        let der = quote!(minecraft_serde::serde::Serialize::serialize(#field_ident, writer)?;);
        serializer_stmts.push(der);
    }

    quote! {
        #(#serializer_stmts)*
    }
}

fn field_deserializer_dynamic(fields: &Fields) -> TokenStream {
    match fields {
        Fields::Named(named_fields) => {
            let mut field_deserializer_stmts = Vec::new();
            for field in &named_fields.named {
                let field_ident = field.ident.as_ref().unwrap();
                let fake_field_ident = Ident::new(
                    format!("{PREFIX}{}", field_ident).as_str(),
                    Span::call_site(),
                );

                let der = quote!(let #fake_field_ident = anyhow::Context::context(minecraft_serde::serde::Deserialize::deserialize(reader), format!("Failure to write property: {}", stringify!(#fake_field_ident)))?;);
                field_deserializer_stmts.push(der);
            }

            quote! {
                #(#field_deserializer_stmts)*
            }
        }
        Fields::Unnamed(unnamed_fields) => {
            let mut field_deserializer_stmts = Vec::new();
            for (index, _) in unnamed_fields.unnamed.iter().enumerate() {
                let field_ident =
                    Ident::new(format!("{PREFIX}{}", index).as_str(), Span::call_site());

                let der = quote!(let #field_ident = minecraft_serde::serde::Deserialize::deserialize(reader)?;);
                field_deserializer_stmts.push(der);
            }

            quote! {
                #(#field_deserializer_stmts)*
            }
        }
        Fields::Unit => TokenStream::new(),
    }
}

fn make_statements<D: std::fmt::Display>(fields: &Fields, prefix: D) -> TokenStream {
    match fields {
        Fields::Named(named_fields) => {
            let mut make_stmts = Vec::new();
            for field in &named_fields.named {
                let field_ident = field.ident.as_ref().unwrap();
                let fake_field_ident = Ident::new(
                    format!("{}{field_ident}", prefix).as_str(),
                    Span::call_site(),
                );

                let make = quote!(#field_ident: #fake_field_ident,);
                make_stmts.push(make);
            }

            quote! {
                { #(#make_stmts)* }
            }
        }
        Fields::Unnamed(unnamed_fields) => {
            let mut make_stmts = Vec::new();
            for (index, _) in unnamed_fields.unnamed.iter().enumerate() {
                let field_ident =
                    Ident::new(format!("{PREFIX}{}", index).as_str(), Span::call_site());
                let make = quote!(#field_ident,);
                make_stmts.push(make);
            }

            quote! {
                (#(#make_stmts)*)
            }
        }
        Fields::Unit => TokenStream::new(),
    }
}

fn field_sizer_dynamic(fields: &Fields) -> TokenStream {
    let mut serializer_stmts = Vec::new();
    for (index, field) in fields.into_iter().enumerate() {
        let field_ident = mapped_ident(PREFIX, index as i32, &field.ident.as_ref());
        let der = quote!(size += minecraft_serde::serde::Serialize::size(#field_ident)?;);
        serializer_stmts.push(der);
    }

    quote! {
        #(#serializer_stmts)*
    }
}

fn mapped_ident<D: std::fmt::Display>(prefix: D, index: i32, ident: &Option<&Ident>) -> Ident {
    ident
        .map(|x| x.clone())
        .unwrap_or_else(|| Ident::new(format!("{}{}", prefix, index).as_str(), Span::call_site()))
}
