use syn::{punctuated::Punctuated, Meta, Token};

use crate::prelude::*;

pub fn system(input: TokenStream, annotated_item: TokenStream) -> TokenStream {
    quote! {
        #annotated_item
    }
}

pub fn init(arguments: TokenStream, item: TokenStream, input: DeriveInput) -> TokenStream {
    input.attrs.into_iter().for_each(|attribute| {
        let Some(ident) = attribute.path().get_ident() else {
            return;
        };

        if ident != "derive" {
            return;
        }

        let Meta::List(derives) = attribute.meta else {
            // Derive should always be a Meta::List so this won't actually happen.
            return;
        };

        attribute.parse_args_with(Punctuated<Ident, Token![,]>);

        let Ok(blah) = attribute.parse_args::<Punctuated<Ident, Token![,]>>() else {
            return;
        };
    });

    quote! {
        #item
    }
}