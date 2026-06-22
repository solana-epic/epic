use crate::types::{
    AliasDef, EnumDef, FieldDef, InstructionDef, StructDef, TypeDef, TypeRef, TypeRegistry,
    VariantDef,
};
use serde::{Deserialize, Serialize};
use syn::{visit::Visit, ItemEnum, ItemStruct, ItemType};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Workspace {
    pub registry: TypeRegistry,
}

struct FileVisitor<'a> {
    current_module_path: Vec<String>,
    registry: &'a mut TypeRegistry,
    file_path: Option<String>,
}

impl<'a> FileVisitor<'a> {
    fn insert_def(&mut self, name: String, def: TypeDef) {
        let mut path = self.current_module_path.clone();
        path.push(name.clone());
        let abs_path = path.join("::");
        self.registry.insert(abs_path.clone(), def);
        self.registry.module_paths.insert(abs_path.clone(), self.current_module_path.clone());
        self.registry.symbol_names.insert(abs_path.clone(), name);
        if let Some(ref fp) = self.file_path {
            self.registry.file_paths.insert(abs_path, fp.clone());
        }
    }

    fn insert_instruction_def(&mut self, mod_name: &str, name: String, def: TypeDef) {
        let mut path = self.current_module_path.clone();
        path.push(mod_name.to_string());
        path.push(name.clone());
        let abs_path = path.join("::");
        self.registry.insert(abs_path.clone(), def);
        let mut mod_path = self.current_module_path.clone();
        mod_path.push(mod_name.to_string());
        self.registry.module_paths.insert(abs_path.clone(), mod_path);
        self.registry.symbol_names.insert(abs_path.clone(), name);
        if let Some(ref fp) = self.file_path {
            self.registry.file_paths.insert(abs_path, fp.clone());
        }
    }
}

impl<'a, 'ast> Visit<'ast> for FileVisitor<'a> {
    fn visit_item_struct(&mut self, i: &'ast ItemStruct) {
        let name = i.ident.to_string();
        let is_account = i.attrs.iter().any(|attr| attr.path().is_ident("account"));
        let mut fields = Vec::new();

        if let syn::Fields::Named(named) = &i.fields {
            for field in &named.named {
                if let Some(ident) = &field.ident {
                    let type_ref = parse_type(&field.ty);
                    let mut attrs = Vec::new();
                    for attr in &field.attrs {
                        if !attr.path().is_ident("doc") {
                            attrs.push(quote::quote!(#attr).to_string().replace(" ", ""));
                        }
                    }
                    fields.push(FieldDef {
                        name: ident.to_string(),
                        type_ref,
                        attrs,
                    });
                }
            }
        }

        let mut struct_attrs = Vec::new();
        for attr in &i.attrs {
            if !attr.path().is_ident("doc") {
                struct_attrs.push(quote::quote!(#attr).to_string().replace(" ", ""));
            }
        }

        let def = TypeDef::Struct(StructDef {
            name: name.clone(),
            is_account,
            fields,
            attrs: struct_attrs,
        });

        self.insert_def(name, def);
    }

    fn visit_item_enum(&mut self, i: &'ast ItemEnum) {
        let name = i.ident.to_string();
        let mut variants = Vec::new();

        for variant in &i.variants {
            let mut fields = Vec::new();
            if let syn::Fields::Named(named) = &variant.fields {
                for field in &named.named {
                    if let Some(ident) = &field.ident {
                        let type_ref = parse_type(&field.ty);
                        let mut attrs = Vec::new();
                        for attr in &field.attrs {
                            if !attr.path().is_ident("doc") {
                                attrs.push(quote::quote!(#attr).to_string().replace(" ", ""));
                            }
                        }
                        fields.push(FieldDef {
                            name: ident.to_string(),
                            type_ref,
                            attrs,
                        });
                    }
                }
            }
            variants.push(VariantDef {
                name: variant.ident.to_string(),
                fields,
            });
        }

        let def = TypeDef::Enum(EnumDef {
            name: name.clone(),
            variants,
        });

        self.insert_def(name, def);
    }

    fn visit_item_type(&mut self, i: &'ast ItemType) {
        let name = i.ident.to_string();
        let target = parse_type(&i.ty);

        let def = TypeDef::Alias(AliasDef {
            name: name.clone(),
            target,
        });

        self.insert_def(name, def);
    }

    fn visit_item_mod(&mut self, i: &'ast syn::ItemMod) {
        let is_program = i.attrs.iter().any(|attr| attr.path().is_ident("program"));

        if is_program {
            if let Some((_, items)) = &i.content {
                for item in items {
                    if let syn::Item::Fn(item_fn) = item {
                        let name = item_fn.sig.ident.to_string();
                        let mut args = Vec::new();

                        for input in &item_fn.sig.inputs {
                            if let syn::FnArg::Typed(pat_type) = input {
                                if let syn::Pat::Ident(pat_ident) = &*pat_type.pat {
                                    let arg_name = pat_ident.ident.to_string();
                                    let type_ref = parse_type(&pat_type.ty);
                                    args.push(FieldDef {
                                        name: arg_name,
                                        type_ref,
                                        attrs: vec![],
                                    });
                                }
                            }
                        }

                        let def = TypeDef::Instruction(InstructionDef {
                            name: name.clone(),
                            args,
                        });

                        self.insert_instruction_def(&i.ident.to_string(), name, def);
                    }
                }
            }
        }

        self.current_module_path.push(i.ident.to_string());
        syn::visit::visit_item_mod(self, i);
        self.current_module_path.pop();
    }
}

pub fn parse_type(ty: &syn::Type) -> TypeRef {
    match ty {
        syn::Type::Path(type_path) => {
            let segment = type_path.path.segments.last().unwrap();
            let ident = segment.ident.to_string();

            match ident.as_str() {
                "u8" | "u16" | "u32" | "u64" | "u128" | "i8" | "i16" | "i32" | "i64" | "i128"
                | "f32" | "f64" | "bool" => TypeRef::Primitive(ident),
                "String" => TypeRef::String,
                "Pubkey" => TypeRef::Pubkey,
                "Vec" => {
                    if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                        if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                            return TypeRef::Vec(Box::new(parse_type(inner_ty)));
                        }
                    }
                    let full_ty = quote::quote!(#ty).to_string().replace(" ", "");
                    TypeRef::Custom(full_ty)
                }
                "Option" => {
                    if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                        if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                            return TypeRef::Option(Box::new(parse_type(inner_ty)));
                        }
                    }
                    let full_ty = quote::quote!(#ty).to_string().replace(" ", "");
                    TypeRef::Custom(full_ty)
                }
                "Box" => {
                    if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                        if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                            return parse_type(inner_ty);
                        }
                    }
                    let full_ty = quote::quote!(#ty).to_string().replace(" ", "");
                    TypeRef::Custom(full_ty)
                }
                _ => {
                    let full_ty = quote::quote!(#ty).to_string().replace(" ", "");
                    TypeRef::Custom(full_ty)
                }
            }
        }
        syn::Type::Array(type_array) => {
            let inner = parse_type(&type_array.elem);
            let len = match &type_array.len {
                syn::Expr::Lit(expr_lit) => {
                    if let syn::Lit::Int(lit_int) = &expr_lit.lit {
                        lit_int.base10_parse::<usize>().unwrap_or(0)
                    } else {
                        0
                    }
                }
                _ => 0, // In a real parser we need to resolve constants
            };
            TypeRef::Array(Box::new(inner), len)
        }
        _ => TypeRef::Custom(quote::quote!(#ty).to_string().replace(" ", "")),
    }
}

impl Workspace {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_file(
        &mut self,
        program_name: &str,
        module_path: &[&str],
        source: &str,
        file_path: Option<&str>,
    ) -> anyhow::Result<()> {
        let file = syn::parse_str::<syn::File>(source)?;
        let mut full_path = vec![program_name.to_string()];
        for m in module_path {
            full_path.push(m.to_string());
        }

        let mut visitor = FileVisitor {
            current_module_path: full_path,
            registry: &mut self.registry,
            file_path: file_path.map(|s| s.to_string()),
        };

        visitor.visit_file(&file);
        Ok(())
    }
}
