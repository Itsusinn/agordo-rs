#![feature(let_chains)]

extern crate proc_macro;
extern crate proc_macro2;

use proc_macro2::Span;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Ident, Lit, Meta, Expr};

#[proc_macro_derive(FigmentWrapper, attributes(location))]
pub fn figment_wrapper_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    let ident = &ast.ident;
    let generics = &ast.generics;
    let where_clause = &ast.generics.where_clause;
    let path_attr = ast
        .attrs
        .iter()
        .filter(|a| a.path().get_ident().unwrap().to_string() == "location")
        .next()
        .unwrap_or_else(|| panic!("缺失 #[location = ?] 的附加属性"));
    let path_value = &path_attr.meta.require_name_value().unwrap().value;
    let path_value = if let syn::Expr::Lit(expr) = path_value
    && let Lit::Str(v) = &expr.lit{
        v.value().to_string()
    } else {
        panic!("location值的类型不支持")
    };

    let uppercase_indent = Ident::new(&ident.to_string().to_ascii_uppercase(), Span::call_site());
    let expanded: proc_macro2::TokenStream = quote! {
      pub static #uppercase_indent: lateinit::LateInit<#ident> = lateinit::LateInit::new();
      impl #ident #generics #where_clause {
         #[allow(unused_variables)]
         pub async fn save(&self) -> Result<()> {
            use std::path::Path;
            use tokio::fs;
            use tracing::info;
            let path:&str= #path_value;
            let path = Path::new(&path);
            let ser = serde_yaml::to_string(self)?;
            info!("配置文件已被保存");
            fs::create_dir_all(path.parent().unwrap_or(Path::new("./"))).await?;
            fs::write(path, ser).await?;
            Ok(())
         }
         #[allow(unused_variables)]
         pub fn default_string() -> String {
            match  serde_yaml::to_string(&Self::default()) {
               Ok(v) => v,
               Err(_) => "serde_yaml error".to_string(),
           }
         }
         #[allow(unused_variables)]
         pub async fn reload() -> Result<()> {
            use std::path::Path;
            let path:&str= #path_value;
            let path = Path::new(&path);
            let value = Self::reload_with_path(&path).await?;
            #uppercase_indent.init(value);
            Ok(())
         }
         #[allow(unused_variables)]
         pub async fn reload_with_path(path: &std::path::Path) -> Result<Self,Error> {
            use tokio::fs;
            use std::path::Path;
            use tracing::warn;
            use figment::{Figment, providers::{Serialized, Format, Yaml}};

            if !path.exists() {
               fs::create_dir_all(path.parent().unwrap_or(Path::new("./"))).await?;
               fs::write(path, Self::default_string()).await?;
            };
            let config: Self = Figment::from(Serialized::defaults(Self::default()))
                .merge(Yaml::file(path))
                .extract()?;
            Ok(config)
        }
      }
   }.into();
    proc_macro::TokenStream::from(expanded)
}

use proc_macro2::TokenStream;

#[proc_macro_attribute]
pub fn figment_derive(
    _metadata: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let input: TokenStream = input.into();
    let output = quote! {
        #[derive(Debug, serde::Serialize, serde::Deserialize,Educe)]
        #[educe(Default)]
        #input
    };
    output.into()
}
