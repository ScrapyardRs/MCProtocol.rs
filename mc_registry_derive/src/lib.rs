use proc_macro2::{TokenStream, TokenTree};
use quote::ToTokens;
use syn::parse_macro_input;

#[proc_macro_attribute]
pub fn packet_handler(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let input = parse_macro_input!(item as syn::ItemFn);
    let block = &input.block;

    let attribute_iter = attr.into_iter();

    let fn_name = &input.sig.ident;

    let mut extra_setters: Vec<TokenStream> = Vec::new();
    let mut context_tokens_guard: Option<TokenStream> = None;
    let mut full_context_tokens_guard: Option<TokenStream> = None;
    let mut mappings_tokens_guard: Option<TokenStream> = None;
    for input in &input.sig.inputs {
        let mut stream = input.to_token_stream().into_iter();
        let next_ident = stream.next().unwrap();

        if let TokenTree::Ident(ident) = next_ident {
            let mut ident_str = ident.to_string();
            let mut true_ident = ident;
            let mut extra = TokenStream::new();

            if ident_str.as_str() == "mut" {
                extra = true_ident.to_token_stream();
                match stream.next().unwrap() {
                    TokenTree::Ident(ident) => {
                        ident_str = ident.to_string();
                        true_ident = ident;
                    }
                    _ => panic!("Unrecognized identifier."),
                };
            }

            match ident_str.as_str() {
                "context" | "_context" => {
                    extra_setters.push(quote::quote!(let #extra #true_ident = __context;));
                    context_tokens_guard = Some(stream.clone().skip_while(|next| {
                        if let TokenTree::Punct(punct) = next {
                            punct.as_char() != '<'
                        } else {
                            true
                        }
                    }).skip(1).take_while(|next| {
                        if let TokenTree::Punct(punct) = next {
                            punct.as_char() != '>'
                        } else {
                            true
                        }
                    }).collect());
                    full_context_tokens_guard = Some(stream.skip(1).collect())
                }
                "packet" | "_packet" => {
                    extra_setters.push(quote::quote!(let #extra #true_ident = __packet;));
                    mappings_tokens_guard = Some(stream.skip(1).collect());
                }
                "registry" | "_registry" => {
                    extra_setters.push(quote::quote!(let #extra #true_ident = __registry;));
                }
                _ => panic!("Unknown ident {}, please pick one of (mut, context, _context, packet, _packet, registry, _registry)", true_ident),
            }
        } else {
            panic!("Expecting literal, found otherwise.");
        }
    }

    let (context_tokens, mappings_tokens) = match (context_tokens_guard, mappings_tokens_guard) {
        (Some(context_tokens), Some(mappings_tokens)) => (context_tokens, mappings_tokens),
        (Some(context_tokens), None) => (
            context_tokens,
            attribute_iter.collect::<proc_macro::TokenStream>().into(),
        ),
        (None, Some(mappings_tokens)) => (
            attribute_iter.collect::<proc_macro::TokenStream>().into(),
            mappings_tokens,
        ),
        (None, None) => {
            let context_tokens = attribute_iter
                .clone()
                .take_while(|item| match item {
                    proc_macro::TokenTree::Punct(punc) => punc.as_char() != ',',
                    _ => true,
                })
                .collect::<proc_macro::TokenStream>()
                .into();
            let mapping_tokens = attribute_iter
                .skip_while(|item| match item {
                    proc_macro::TokenTree::Punct(punc) => punc.as_char() != ',',
                    _ => true,
                })
                .skip(1)
                .collect::<proc_macro::TokenStream>()
                .into();
            (context_tokens, mapping_tokens)
        }
    };

    let full_context_tokens = full_context_tokens_guard
        .unwrap_or(quote::quote!(mc_registry::registry::LockedContext<#context_tokens>));

    proc_macro::TokenStream::from(quote::quote! {
        fn #fn_name(
            __context: #full_context_tokens,
            __registry: mc_registry::registry::LockedStateRegistry<'_, #context_tokens>,
            __protocol_version: mc_serializer::serde::ProtocolVersion,
            __buffer: std::io::Cursor<Vec<u8>>,
        ) -> mc_registry::registry::BoxedFuture<'_> {
            Box::pin(async move {
                let __packet = mc_registry::mappings::create_packet::<#mappings_tokens>(__protocol_version, __buffer)?;
                #(#extra_setters)*
                #block
                Ok(())
            })
        }
    })
}
