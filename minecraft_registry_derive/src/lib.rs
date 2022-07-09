use syn::parse_macro_input;

#[proc_macro_attribute]
pub fn packet_handler(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let input = parse_macro_input!(item as syn::ItemFn);
    let block = &input.block;

    let attribute_iter = attr.into_iter();

    let context_tokens = proc_macro2::TokenStream::from(
        attribute_iter
            .clone()
            .take_while(|item| match item {
                proc_macro::TokenTree::Punct(punc) => punc.as_char() != ',',
                _ => true,
            })
            .collect::<proc_macro::TokenStream>(),
    );
    let mapping_tokens = proc_macro2::TokenStream::from(
        attribute_iter
            .clone()
            .skip_while(|item| match item {
                proc_macro::TokenTree::Punct(punc) => punc.as_char() != ',',
                _ => true,
            })
            .skip(1)
            .collect::<proc_macro::TokenStream>(),
    );

    let fn_name = &input.sig.ident;

    proc_macro::TokenStream::from(quote::quote! {
        fn #fn_name(
            __context: minecraft_registry::registry::LockedContext<#context_tokens>,
            __registry: minecraft_registry::registry::LockedStateRegistry<#context_tokens>,
            __protocol_version: minecraft_serde::serde::ProtocolVersion,
            __buffer: std::io::Cursor<Vec<u8>>,
        ) -> minecraft_registry::registry::BoxedFuture {
            Box::pin(async move {
                let packet = minecraft_registry::mappings::create_packet::<#mapping_tokens>(__protocol_version, __buffer)?;
                let registry = __registry;
                let context = __context;
                #block
                Ok(())
            })
        }
    })

    // let generics_iter = &mut input.sig.generics.params.iter();
    // let context_tokens = generics_iter.next().expect("Context generic required.");
    // let mapping_tokens = generics_iter.next().expect("Mapping generic required.");
    //
    // let locked_context_tokens = quote::quote!(minecraft_registry::registry::LockedContext<#context_tokens>);
    // let locked_registry_tokens = quote::quote!(minecraft_registry::registry::LockedStateRegistry<#context_tokens>);
    // let boxed_future_tokens = quote::quote!(minecraft_registry::registry::BoxedFuture);
    //
    // proc_macro::TokenStream::from(quote::quote! {
    //     fn #fn_name(
    //         __context #locked_context_tokens,
    //         __registry: #locked_registry_tokens,
    //         __protocol_version: minecraft_serde::serde::ProtocolVersion,
    //         __buffer: std::io::Cursor<Vec<u8>>
    //     ) -> #boxed_future_tokens {
    //         Box::pin(async move {
    //             let packet = minecraft_registry::mappings::create_packet::<#mapping_tokens>(__protocol_version, __buffer)?;
    //             let context = __context;
    //             let registry = __registry;
    //             #block
    //             Ok(())
    //         })
    //     }
    // })
}
