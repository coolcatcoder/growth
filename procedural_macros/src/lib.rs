mod save_and_load;

mod prelude {
    pub use proc_macro::TokenStream as StdTokenStream;
    pub use proc_macro2::{Group, Span, TokenStream, TokenTree};
    pub use quote::{quote, ToTokens};
    pub use std::stringify;
    pub use syn::{
        parse::Parse, parse_macro_input, punctuated::Punctuated, Data, DeriveInput, Error, Fields,
        Ident, Meta, Token,
    };
}

use prelude::*;

#[proc_macro]
pub fn erase_idents(input: StdTokenStream) -> StdTokenStream {
    let mut ident_map = IdentMap {
        map: ahash::AHashMap::new(),
        index: 0,
    };

    let output = token_stream_replace_idents(&mut ident_map, TokenStream::from(input));

    quote! {
        const ERASED: &str = stringify!(#output);
    }
    .into()
}

struct IdentMap {
    map: ahash::AHashMap<String, String>,
    index: u32,
}

impl IdentMap {
    const UNCHANGED: &[&str] = &["mut", "let", "fn", "error"];
    fn replace(&mut self, ident: Ident) -> Ident {
        if Self::UNCHANGED.contains(&ident.to_string().as_str()) {
            ident
        } else if let Some(replacement) = self.map.get(&ident.to_string()) {
            Ident::new(replacement, ident.span())
        } else {
            let replacement = format!("ident_{}", self.index);
            let new_ident = Ident::new(&replacement, ident.span());
            assert!(self.map.insert(ident.to_string(), replacement).is_none());
            self.index += 1;
            new_ident
        }
    }
}

fn token_stream_replace_idents(ident_map: &mut IdentMap, token_stream: TokenStream) -> TokenStream {
    let mut output = TokenStream::new();
    token_stream.into_iter().for_each(|token_tree| {
        match token_tree {
            TokenTree::Group(group) => {
                // Recursively replace idents in group.
                output.push(TokenTree::Group(Group::new(
                    group.delimiter(),
                    token_stream_replace_idents(ident_map, group.stream()),
                )));
            }
            TokenTree::Ident(ident) => output.push(TokenTree::Ident(ident_map.replace(ident))),
            _ => output.push(token_tree),
        }
    });
    output
}

trait TokenStreamPush {
    fn push(&mut self, token_tree: TokenTree);
}

impl TokenStreamPush for TokenStream {
    fn push(&mut self, token_tree: TokenTree) {
        self.extend([token_tree].into_iter());
    }
}

#[proc_macro_derive(SaveAndLoad, attributes(location, unload))]
pub fn save_and_load(input: StdTokenStream) -> StdTokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    save_and_load::save_and_load(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[proc_macro]
pub fn save_and_load_external(input: StdTokenStream) -> StdTokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    save_and_load::save_and_load(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
