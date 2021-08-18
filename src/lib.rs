extern crate proc_macro;
extern crate proc_macro2;
use proc_macro2::Span;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Ident};

#[proc_macro_derive(AutoConfig, attributes(location))]
pub fn auto_config_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    let ident = &ast.ident;
    let generics = &ast.generics;
    let where_clause = &ast.generics.where_clause;
    let path_attr = ast
        .attrs
        .iter()
        .filter(|a| a.path.get_ident().unwrap().to_string() == "location")
        .next()
        .unwrap_or_else(|| panic!("缺失 #[location = ?] 的附加属性"));
    let path_value: String = match path_attr
        .parse_meta()
        .unwrap_or_else(|_| panic!("Failed to parse meta"))
    {
        syn::Meta::NameValue(value) => match value.lit {
            syn::Lit::Str(i) => i.value().to_string(),
            _ => panic!("location值的类型不支持"),
        },
        _ => {
            panic!("location值的类型不支持")
        }
    };

    let uppercase_indent = Ident::new(&ident.to_string().to_ascii_uppercase(), Span::call_site());
    let expanded: proc_macro2::TokenStream = quote! {
      pub static #uppercase_indent: once_cell::sync::Lazy<#ident> = once_cell::sync::Lazy::new(|| {
         use std::path::Path;
         let path:&str= #path_value;
         let path = Path::new(path);
         let config = #ident::read_or_create_config(path).unwrap();
         config
     });
      impl #ident #generics #where_clause {
         #[allow(unused_variables)]
         pub fn save(&self) {
            use std::path::Path;
            use std::fs;
            use log::info;
            let path:&str= #path_value;
            let path = Path::new(&path);
            let ser = serde_yaml::to_string(self).unwrap();
            info!("Configuration file was saved");
            info!("配置文件已被保存");
            fs::create_dir_all(path.parent().unwrap_or(Path::new("./"))).unwrap();
            fs::write(path, ser).unwrap();
         }
         #[allow(unused_variables)]
         pub fn default_string() -> String {
            match  serde_yaml::to_string(&Self::default()) {
               Ok(v) => v,
               Err(_) => "serde_yaml error".to_string(),
           }
         }
         #[allow(unused_variables)]
         fn read_or_create_config(path: &Path) -> Result<Self, anyhow::Error> {
            use std::fs;
            use log::error;
            if !path.exists() {
               fs::create_dir_all(path.parent().unwrap_or(Path::new("./")))?;
               fs::write(path, Self::default_string())?;
            };
            let data = fs::read(path)?;
            let result: Result<Self, serde_yaml::Error> = serde_yaml::from_slice(&data);
            let result = match result {
               Ok(val) => val,
               Err(_) => {
                  let default_string = Config::default_string();
                  let reanme_path = format!("{}.old", path.clone().to_string_lossy());
                  error!("Cannot de-serialize the configuration file.");
                  error!("It may be caused by incompatible configuration files due to version updates.");
                  error!("The original file has been changed to {}, please merge the configuration files manually.",reanme_path);
                  error!("无法对配置文件进行反序列化。");
                  error!("这可能是由于版本更新导致的配置文件不兼容造成的。");
                  error!("原文件已被改为{}，请手动合并配置文件。",reanme_path);
                  let rename_path = Path::new(&reanme_path);
                  fs::rename(path, rename_path)?;
                  fs::write(path, default_string)?;
                  Self::default()
               }
            };
            Ok(result)
        }
      }
   }.into();
   proc_macro::TokenStream::from(expanded)
}

use proc_macro2::TokenStream;

#[proc_macro_attribute]
pub fn basic_derive(_metadata: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input: TokenStream = input.into();
    let output = quote! {
        #[derive(Debug, Serialize, Deserialize,Educe)]
        #[educe(Default)]
        #input
    };
    output.into()
}