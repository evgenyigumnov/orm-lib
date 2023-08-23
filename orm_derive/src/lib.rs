use darling::FromDeriveInput;
use proc_macro::{self, TokenStream};
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[derive(FromDeriveInput, Default)]
#[darling(default, attributes(table), forward_attrs(allow, doc, cfg))]
struct Opts {
    name: Option<String>,
}

#[proc_macro_derive(Table, attributes(table))]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input);
    let opts = Opts::from_derive_input(&input).expect("Wrong options");
    let DeriveInput { ident, .. } = input;

    let answer = match opts.name {
        Some(x) => quote! {
            fn name(&self) -> String {
                #x.to_string()
            }
        },
        None => quote! {
            fn name(&self) -> String {
                let r = format!("{:?}", #ident);
                r
            }
        },
    };

    let output = quote! {
        impl ormlib::Table for #ident {
            #answer
        }
    };
    output.into()
}
