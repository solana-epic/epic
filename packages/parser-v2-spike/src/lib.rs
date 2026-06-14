use serde::{Serialize, Deserialize};
use syn::{parse_str, visit::{self, Visit}};
use std::collections::HashMap;
use quote::ToTokens;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AccountField {
    pub name: String,
    pub r#type: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AccountStruct {
    pub name: String,
    pub fields: Vec<AccountField>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct WorkspaceAnalysis {
    pub accounts: Vec<AccountStruct>,
    pub structs: HashMap<String, Vec<AccountField>>,
    pub aliases: HashMap<String, String>,
}

struct ParserVisitor {
    analysis: WorkspaceAnalysis,
}

impl<'ast> Visit<'ast> for ParserVisitor {
    fn visit_item_struct(&mut self, i: &'ast syn::ItemStruct) {
        let is_account = i.attrs.iter().any(|attr| {
            attr.path().is_ident("account")
        });

        let fields = self.extract_fields(&i.fields);
        
        if is_account {
            self.analysis.accounts.push(AccountStruct {
                name: i.ident.to_string(),
                fields: fields.clone(),
            });
        }
        
        self.analysis.structs.insert(i.ident.to_string(), fields);
        
        visit::visit_item_struct(self, i);
    }

    fn visit_item_type(&mut self, i: &'ast syn::ItemType) {
        let type_name = i.ident.to_string();
        let type_def = i.ty.to_token_stream().to_string().replace(" ", "");
        self.analysis.aliases.insert(type_name, type_def);
        visit::visit_item_type(self, i);
    }
}

impl ParserVisitor {
    fn extract_fields(&self, fields: &syn::Fields) -> Vec<AccountField> {
        let mut extracted = Vec::new();
        if let syn::Fields::Named(named) = fields {
            for field in &named.named {
                if let Some(ident) = &field.ident {
                    let type_str = field.ty.to_token_stream().to_string().replace(" ", "");
                    extracted.push(AccountField {
                        name: ident.to_string(),
                        r#type: type_str,
                    });
                }
            }
        }
        extracted
    }
}

pub fn parse_source(source: &str) -> anyhow::Result<WorkspaceAnalysis> {
    let file = parse_str::<syn::File>(source)?;
    let mut visitor = ParserVisitor {
        analysis: WorkspaceAnalysis::default(),
    };
    visitor.visit_file(&file);
    Ok(visitor.analysis)
}
