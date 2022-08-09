use proc_macro2::{TokenStream, TokenTree};
use proc_macro2::token_stream::IntoIter;

#[derive(Default)]
pub struct AfterDirective {
    pub ser: Option<TokenStream>,
    pub de: Option<TokenStream>,
}

fn parse_instruction(stream: &mut IntoIter, directive: &mut AfterDirective) -> bool {
    let context = stream.next()
        .map(|x| {
            if let TokenTree::Ident(ident) = x {
                ident.to_string()
            } else {
                panic!("Expected ident in first position of after directive.");
            }
        });

    let operator = stream.next()
        .map(|x| {
            if let TokenTree::Group(group) = x {
                group.stream()
            } else {
                panic!("Expected group in second position of after directive.");
            }
        });

    match operator {
        None => return true,
        Some(operator) => {
            match context {
                Some(x) => {
                    match x.as_str() {
                        "ser" => directive.ser = Some(operator),
                        "de" => directive.de = Some(operator),
                        _ => panic!("Unexpected value {}", x)
                    }
                }
                None => return true,
            }
        }
    }
    false
}

pub fn parse_after_directive(directive: &mut AfterDirective, stream: TokenStream) {
    let mut stream = stream.into_iter();
    loop {
        if parse_instruction(&mut stream, directive) {
            break;
        }
    }
}
