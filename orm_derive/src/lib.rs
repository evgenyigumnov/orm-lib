use darling::FromDeriveInput;
use proc_macro::{self, TokenStream};
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[derive(FromDeriveInput, Default)]
#[darling(default, attributes(table), forward_attrs(allow, doc, cfg))]
struct Opts {
    name: Option<String>,
}

#[proc_macro_derive(TableSerialize, attributes(table))]
pub fn derive(input: TokenStream) -> TokenStream {
    // println!("!!!!!!!!!!!!!");

    let input = parse_macro_input!(input);
    let opts = Opts::from_derive_input(&input).expect("Wrong options");
    let DeriveInput { ident, .. } = input;
    let answer = match opts.name {
        Some(x) => quote! {
            fn name(&self) -> String {
                #x.to_string()
            }
            fn get_id(&self) -> String {
                self.id.to_string()
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
        impl ormlib::TableSerialize for #ident {
            #answer
        }
    };
    // println!("++++++++++++++++");
    // println!("{}", output);
    output.into()
}

#[proc_macro_derive(TableDeserialize, attributes(table))]
pub fn derive_de(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input);
    let opts = Opts::from_derive_input(&input).expect("Wrong options");
    let DeriveInput { ident, .. } = input;

    let syn::Data::Struct(data) = input.data else {
        unimplemented!()
    };

    let mut fields: Vec<String> = Vec::new();
    for f in data.fields.iter() {
        fields.push(f.ident.as_ref().unwrap().to_string());

    }
    let code1: String = r#"
    fn fields() -> Vec<String> {

        let mut fields: Vec<String> = Vec::new();

    "#.to_string();

    let mut code2: String = String::new();

    for f in fields.iter() {
        code2.push_str(&format!("fields.push(\"{}\".to_string());\n", f));
    }

    let code3: String = r#"

        fields
    }

    "#.to_string();

    let code_all = format!("{}{}{}", code1, code2, code3);
    let code = code_all.as_str();

    let code_token: proc_macro2::TokenStream = code.parse().unwrap(); // Преобразование строки в TokenStream

    let  answer = match opts.name {
        Some(x) => quote! {
            fn same_name() -> String {
                #x.to_string()
            }
        },
        None => quote! {
        },
    };

    let output = quote! {
        impl ormlib::TableDeserialize for #ident {
            #answer

            #code_token
        }
    };

    output.into()
}
