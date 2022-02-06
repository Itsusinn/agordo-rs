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
            use yaml_rust::{YamlLoader, YamlEmitter};
            use yaml_rust::Yaml;
            use std::mem::discriminant;
            use linked_hash_map::LinkedHashMap;

            fn merge_hash(former: &LinkedHashMap<Yaml, Yaml>,latter: &LinkedHashMap<Yaml, Yaml>) -> anyhow::Result<LinkedHashMap<Yaml, Yaml>>{
                // if it's empty hash
                if latter.len() == 0 {
                    return Ok(former.clone());
                }
                let mut res = latter.clone();
                for section in latter {
                    match former.contains_key(section.0) {
                        // if former dont's have this key
                        false => {},
                        // if former have this key, then merge
                        true => {
                            let former_value = former.get(section.0).unwrap().clone();
                            let latter_value = section.1.clone();
                            // if they are the same type, then merge
                            if discriminant(&former_value) == discriminant(&latter_value){
                                match latter_value.as_hash() {
                                    // if it's hash type
                                    Some(_) => {
                                        let res_value = merge_hash(&former_value.as_hash().unwrap(), &latter_value.as_hash().unwrap())?;
                                        res.insert(section.0.clone(), Yaml::Hash(res_value));
                                    },
                                    // if it's not a hash type
                                    None => {
                                        res.insert(section.0.clone(), former_value);
                                    },
                                }
                            }
                        },
                    };
                }
                Ok(res)
            }
            if !path.exists() {
               fs::create_dir_all(path.parent().unwrap_or(Path::new("./")))?;
               fs::write(path, Self::default_string())?;
            };
            let data = fs::read(path)?;
            let result: Result<Self, serde_yaml::Error> = serde_yaml::from_slice(&data);
            let result = match result {
               Ok(val) => val,
               Err(_) => {
                  let latter_string = Self::default_string();
                  let reanme_path = format!("{}.old", path.clone().to_string_lossy());
                  log::warn!("无法对配置文件进行反序列化。");
                  log::warn!("这可能是由于版本更新导致的配置文件不兼容造成的。");
                  log::warn!("原文件已被改为{}，正在尝试自动合并配置文件。",reanme_path);
                  let rename_path = Path::new(&reanme_path);
                  let former_str = String::from_utf8_lossy(&data);
                  let former = YamlLoader::load_from_str(&former_str).unwrap()[0].as_hash().unwrap().clone();
                  let latter = YamlLoader::load_from_str(&latter_string).unwrap()[0].as_hash().unwrap().clone();
                  let res = yaml_rust::Yaml::Hash(merge_hash(&former,&latter).unwrap());
                  let mut res_string = String::new();
                  {
                      let mut emitter = YamlEmitter::new(&mut res_string);
                      emitter.dump(&res).unwrap(); // dump the YAML object to a String
                  }
                  fs::rename(path, rename_path)?;
                  fs::write(path, res_string.clone())?;
                  match serde_yaml::from_slice::<Self>(res_string.as_bytes()) {
                      Err(_) => {
                        log::warn!("配置文件合并失败,请手动合并配置文件");
                        Self::default()
                      }
                      Ok(val) => {
                        log::warn!("配置文件合并完成,但仍推荐检查配置文件");
                        val
                      }
                  }
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
pub fn basic_derive(
    _metadata: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let input: TokenStream = input.into();
    let output = quote! {
        #[derive(Debug, Serialize, Deserialize,Educe)]
        #[educe(Default)]
        #input
    };
    output.into()
}
