use crate::types::{StructDef, TypeDef, TypeRef, TypeRegistry, EnumDef};
use anyhow::{anyhow, bail, Result};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct LayoutInfo {
    pub size: usize,
    pub dynamic: bool,
    pub dependencies: Vec<String>,
}

pub struct LayoutEngine<'a> {
    pub registry: &'a TypeRegistry,
    pub resolving: Vec<String>,
    pub layout_cache: HashMap<String, LayoutInfo>,
}

impl<'a> LayoutEngine<'a> {
    pub fn new(registry: &'a TypeRegistry) -> Self {
        Self {
            registry,
            resolving: Vec::new(),
            layout_cache: HashMap::new(),
        }
    }

    pub fn resolve_absolute_path(&self, current_module: &str, ident: &str) -> Result<String> {
        let direct_path = format!("{}::{}", current_module, ident);
        if self.registry.definitions.contains_key(&direct_path) {
            return Ok(direct_path);
        }

        let mut matches = Vec::new();
        for key in self.registry.definitions.keys() {
            if key.ends_with(&format!("::{}", ident)) || key == ident {
                matches.push(key.clone());
            }
        }

        if matches.is_empty() {
            bail!("Unknown type: {}", ident);
        } else if matches.len() > 1 {
            bail!("Ambiguous type: {} matches {:?}", ident, matches);
        } else {
            Ok(matches[0].clone())
        }
    }

    pub fn size_of_type_ref(&mut self, current_module: &str, ty: &TypeRef) -> Result<LayoutInfo> {
        match ty {
            TypeRef::Primitive(p) => {
                let size = match p.as_str() {
                    "u8" | "i8" | "bool" => 1,
                    "u16" | "i16" => 2,
                    "u32" | "i32" | "f32" => 4,
                    "u64" | "i64" | "f64" => 8,
                    "u128" | "i128" => 16,
                    _ => bail!("Unknown primitive: {}", p),
                };
                Ok(LayoutInfo { size, dynamic: false, dependencies: vec![] })
            }
            TypeRef::String => Ok(LayoutInfo { size: 4, dynamic: true, dependencies: vec![] }),
            TypeRef::Pubkey => Ok(LayoutInfo { size: 32, dynamic: false, dependencies: vec![] }),
            TypeRef::Array(inner, len) => {
                let inner_layout = self.size_of_type_ref(current_module, inner)?;
                Ok(LayoutInfo {
                    size: inner_layout.size * len,
                    dynamic: inner_layout.dynamic,
                    dependencies: inner_layout.dependencies,
                })
            }
            TypeRef::Vec(inner) => {
                let mut inner_layout = self.size_of_type_ref(current_module, inner)?;
                inner_layout.size = 4; // 4-byte length prefix
                inner_layout.dynamic = true;
                Ok(inner_layout)
            }
            TypeRef::Option(inner) => {
                let mut inner_layout = self.size_of_type_ref(current_module, inner)?;
                inner_layout.size += 1; // 1-byte discriminator
                Ok(inner_layout)
            }
            TypeRef::Custom(ident) => {
                let abs_path = self.resolve_absolute_path(current_module, ident)?;
                self.size_of_absolute_path(&abs_path)
            }
            TypeRef::Resolved(abs_path) => {
                self.size_of_absolute_path(abs_path)
            }
            _ => bail!("Unsupported type ref: {:?}", ty),
        }
    }

    pub fn size_of_absolute_path(&mut self, abs_path: &str) -> Result<LayoutInfo> {
        if let Some(cached) = self.layout_cache.get(abs_path) {
            return Ok(cached.clone());
        }

        if self.resolving.contains(&abs_path.to_string()) {
            bail!("Cyclic dependency detected for {}", abs_path);
        }

        self.resolving.push(abs_path.to_string());

        let def = self.registry.get(abs_path).ok_or_else(|| anyhow!("Definition missing for {}", abs_path))?.clone();

        let current_module = if let Some(idx) = abs_path.rfind("::") {
            &abs_path[..idx]
        } else {
            ""
        };

        let mut layout = match def {
            TypeDef::Struct(s) => self.size_of_struct(current_module, &s)?,
            TypeDef::Enum(e) => self.size_of_enum(current_module, &e)?,
            TypeDef::Alias(a) => self.size_of_type_ref(current_module, &a.target)?,
        };

        layout.dependencies.push(abs_path.to_string());

        self.resolving.pop();
        self.layout_cache.insert(abs_path.to_string(), layout.clone());

        Ok(layout)
    }

    fn size_of_struct(&mut self, current_module: &str, s: &StructDef) -> Result<LayoutInfo> {
        let mut total_size = if s.is_account { 8 } else { 0 }; // 8-byte discriminator
        let mut dynamic = false;
        let mut deps = Vec::new();

        for field in &s.fields {
            let field_layout = self.size_of_type_ref(current_module, &field.type_ref)?;
            total_size += field_layout.size;
            if field_layout.dynamic {
                dynamic = true;
            }
            deps.extend(field_layout.dependencies);
        }

        Ok(LayoutInfo {
            size: total_size,
            dynamic,
            dependencies: deps,
        })
    }

    fn size_of_enum(&mut self, current_module: &str, e: &EnumDef) -> Result<LayoutInfo> {
        let mut max_variant_size = 0;
        let mut dynamic = false;
        let mut deps = Vec::new();

        for variant in &e.variants {
            let mut variant_size = 0;
            for field in &variant.fields {
                let field_layout = self.size_of_type_ref(current_module, &field.type_ref)?;
                variant_size += field_layout.size;
                if field_layout.dynamic {
                    dynamic = true;
                }
                deps.extend(field_layout.dependencies);
            }
            if variant_size > max_variant_size {
                max_variant_size = variant_size;
            }
        }

        Ok(LayoutInfo {
            size: 1 + max_variant_size, // 1 byte discriminator for borsh enums
            dynamic,
            dependencies: deps,
        })
    }
}
