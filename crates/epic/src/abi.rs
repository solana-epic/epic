use crate::layout::LayoutEngine;
use crate::types::{StructDef, TypeDef, TypeRef, TypeRegistry};
use crate::workspace::Workspace;
use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChangeType {
    AccountLayoutChange,
    StructFieldAddition,
    StructFieldRemoval,
    StructFieldReordering,
    TypeChange,
    EnumVariantAddition,
    EnumVariantRemoval,
    EnumVariantReordering,
    InstructionAddition,
    InstructionRemoval,
    PdaAccountDefinitionChange,
    IdlBreakingChange,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Safe,
    Minor,
    Major,
    Critical,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Safe => write!(f, "Safe"),
            Severity::Minor => write!(f, "Minor"),
            Severity::Major => write!(f, "Major"),
            Severity::Critical => write!(f, "Critical"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiffResult {
    pub entity: String,
    pub change_type: ChangeType,
    pub severity: Severity,
    pub description: String,
}

pub struct AbiEngine<'a> {
    pub registry: &'a TypeRegistry,
    pub resolving: Vec<String>,
    pub hash_cache: HashMap<String, String>,
}

impl<'a> AbiEngine<'a> {
    pub fn new(registry: &'a TypeRegistry) -> Self {
        Self {
            registry,
            resolving: Vec::new(),
            hash_cache: HashMap::new(),
        }
    }

    pub fn resolve_absolute_path(&self, current_module: &str, ident: &str) -> Result<String> {
        self.registry.resolve_absolute_path(current_module, ident)
    }

    pub fn hash_of_type_ref(&mut self, current_module: &str, ty: &TypeRef) -> Result<String> {
        let mut hasher = Sha256::new();
        match ty {
            TypeRef::Primitive(p) => {
                hasher.update(b"Primitive");
                hasher.update(p.as_bytes());
            }
            TypeRef::String => hasher.update(b"String"),
            TypeRef::Pubkey => hasher.update(b"Pubkey"),
            TypeRef::Array(inner, len) => {
                hasher.update(b"Array");
                hasher.update(self.hash_of_type_ref(current_module, inner)?.as_bytes());
                hasher.update(len.to_string().as_bytes());
            }
            TypeRef::Vec(inner) => {
                hasher.update(b"Vec");
                hasher.update(self.hash_of_type_ref(current_module, inner)?.as_bytes());
            }
            TypeRef::Option(inner) => {
                hasher.update(b"Option");
                hasher.update(self.hash_of_type_ref(current_module, inner)?.as_bytes());
            }
            TypeRef::Custom(ident) => {
                let abs_path = self.resolve_absolute_path(current_module, ident)?;
                hasher.update(self.hash_of_absolute_path(&abs_path)?.as_bytes());
            }
            TypeRef::Resolved(abs_path) => {
                hasher.update(self.hash_of_absolute_path(abs_path)?.as_bytes());
            }
            _ => bail!("Unsupported type ref for hashing: {:?}", ty),
        }
        Ok(hex::encode(hasher.finalize()))
    }

    pub fn hash_of_absolute_path(&mut self, abs_path: &str) -> Result<String> {
        if let Some(cached) = self.hash_cache.get(abs_path) {
            return Ok(cached.clone());
        }

        if self.resolving.contains(&abs_path.to_string()) {
            bail!("Cyclic dependency detected for {}", abs_path);
        }

        self.resolving.push(abs_path.to_string());

        let def = self
            .registry
            .get(abs_path)
            .ok_or_else(|| anyhow!("Definition missing for {}", abs_path))?
            .clone();

        let current_module = if let Some(idx) = abs_path.rfind("::") {
            &abs_path[..idx]
        } else {
            ""
        };

        let mut hasher = Sha256::new();
        match def {
            TypeDef::Struct(s) => {
                hasher.update(b"Struct");
                if s.is_account {
                    hasher.update(b"Account");
                }
                for field in &s.fields {
                    hasher.update(field.name.as_bytes());
                    hasher.update(
                        self.hash_of_type_ref(current_module, &field.type_ref)?
                            .as_bytes(),
                    );
                }
            }
            TypeDef::Enum(e) => {
                hasher.update(b"Enum");
                for variant in &e.variants {
                    hasher.update(variant.name.as_bytes());
                    for field in &variant.fields {
                        hasher.update(field.name.as_bytes());
                        hasher.update(
                            self.hash_of_type_ref(current_module, &field.type_ref)?
                                .as_bytes(),
                        );
                    }
                }
            }
            TypeDef::Alias(a) => {
                hasher.update(b"Alias");
                hasher.update(self.hash_of_type_ref(current_module, &a.target)?.as_bytes());
            }
            TypeDef::Instruction(inst) => {
                hasher.update(b"Instruction");
                hasher.update(inst.name.as_bytes());
                for arg in &inst.args {
                    hasher.update(arg.name.as_bytes());
                    hasher.update(
                        self.hash_of_type_ref(current_module, &arg.type_ref)?
                            .as_bytes(),
                    );
                }
            }
        };

        let hash_str = hex::encode(hasher.finalize());
        self.resolving.pop();
        self.hash_cache
            .insert(abs_path.to_string(), hash_str.clone());

        Ok(hash_str)
    }
}

fn is_pda_definition(s: &StructDef) -> bool {
    s.attrs
        .iter()
        .any(|a| a.contains("derive(Accounts)") || a.contains("Accounts"))
        || s.fields
            .iter()
            .any(|f| f.attrs.iter().any(|attr| attr.contains("account")))
}

pub fn compare_workspaces(old_ws: &Workspace, new_ws: &Workspace) -> Vec<DiffResult> {
    let mut diffs = Vec::new();
    let old_defs = &old_ws.registry.definitions;
    let new_defs = &new_ws.registry.definitions;

    let all_keys: std::collections::BTreeSet<&String> =
        old_defs.keys().chain(new_defs.keys()).collect();

    for key in all_keys {
        let old_def = old_defs.get(key);
        let new_def = new_defs.get(key);

        match (old_def, new_def) {
            (Some(old), None) => match old {
                TypeDef::Instruction(_) => {
                    diffs.push(DiffResult {
                        entity: key.clone(),
                        change_type: ChangeType::InstructionRemoval,
                        severity: Severity::Critical,
                        description: format!("Instruction '{}' was removed", key),
                    });
                }
                TypeDef::Struct(s) => {
                    let is_pda = is_pda_definition(s);
                    diffs.push(DiffResult {
                        entity: key.clone(),
                        change_type: if is_pda {
                            ChangeType::PdaAccountDefinitionChange
                        } else if s.is_account {
                            ChangeType::AccountLayoutChange
                        } else {
                            ChangeType::StructFieldRemoval
                        },
                        severity: Severity::Critical,
                        description: format!("Struct '{}' was removed", key),
                    });
                }
                TypeDef::Enum(_) => {
                    diffs.push(DiffResult {
                        entity: key.clone(),
                        change_type: ChangeType::EnumVariantRemoval,
                        severity: Severity::Critical,
                        description: format!("Enum '{}' was removed", key),
                    });
                }
                TypeDef::Alias(_) => {
                    diffs.push(DiffResult {
                        entity: key.clone(),
                        change_type: ChangeType::TypeChange,
                        severity: Severity::Critical,
                        description: format!("Alias '{}' was removed", key),
                    });
                }
            },
            (None, Some(new)) => match new {
                TypeDef::Instruction(_) => {
                    diffs.push(DiffResult {
                        entity: key.clone(),
                        change_type: ChangeType::InstructionAddition,
                        severity: Severity::Safe,
                        description: format!("Instruction '{}' was added", key),
                    });
                }
                TypeDef::Struct(s) => {
                    let is_pda = is_pda_definition(s);
                    diffs.push(DiffResult {
                        entity: key.clone(),
                        change_type: if is_pda {
                            ChangeType::PdaAccountDefinitionChange
                        } else if s.is_account {
                            ChangeType::AccountLayoutChange
                        } else {
                            ChangeType::StructFieldAddition
                        },
                        severity: Severity::Safe,
                        description: format!("Struct '{}' was introduced", key),
                    });
                }
                TypeDef::Enum(_) => {
                    diffs.push(DiffResult {
                        entity: key.clone(),
                        change_type: ChangeType::EnumVariantAddition,
                        severity: Severity::Safe,
                        description: format!("Enum '{}' was introduced", key),
                    });
                }
                TypeDef::Alias(_) => {
                    diffs.push(DiffResult {
                        entity: key.clone(),
                        change_type: ChangeType::TypeChange,
                        severity: Severity::Safe,
                        description: format!("Alias '{}' was introduced", key),
                    });
                }
            },
            (Some(old), Some(new)) => {
                diffs.extend(compare_definitions(key, old, new, old_ws, new_ws));
            }
            (None, None) => unreachable!(),
        }
    }

    diffs
}

fn compare_definitions(
    entity: &str,
    old_def: &TypeDef,
    new_def: &TypeDef,
    old_ws: &Workspace,
    new_ws: &Workspace,
) -> Vec<DiffResult> {
    let mut diffs = Vec::new();

    match (old_def, new_def) {
        (TypeDef::Struct(old_struct), TypeDef::Struct(new_struct)) => {
            let is_pda = is_pda_definition(old_struct) || is_pda_definition(new_struct);

            if is_pda {
                let mut changed = false;

                if old_struct.fields.len() != new_struct.fields.len() {
                    changed = true;
                } else {
                    for (i, old_f) in old_struct.fields.iter().enumerate() {
                        let new_f = &new_struct.fields[i];
                        if old_f.name != new_f.name
                            || old_f.type_ref != new_f.type_ref
                            || old_f.attrs != new_f.attrs
                        {
                            changed = true;
                            break;
                        }
                    }
                }

                if old_struct.attrs != new_struct.attrs {
                    changed = true;
                }

                if changed {
                    diffs.push(DiffResult {
                        entity: entity.to_string(),
                        change_type: ChangeType::PdaAccountDefinitionChange,
                        severity: Severity::Critical,
                        description: format!(
                            "PDA or Account validation constraints changed on struct '{}'",
                            entity
                        ),
                    });
                }
            } else {
                let mut old_fields_map = HashMap::new();
                for (idx, f) in old_struct.fields.iter().enumerate() {
                    old_fields_map.insert(&f.name, (idx, f));
                }

                let mut new_fields_map = HashMap::new();
                for (idx, f) in new_struct.fields.iter().enumerate() {
                    new_fields_map.insert(&f.name, (idx, f));
                }

                // Check field reordering or removal
                for (old_idx, old_f) in old_struct.fields.iter().enumerate() {
                    if let Some(&(new_idx, _)) = new_fields_map.get(&old_f.name) {
                        if old_idx != new_idx {
                            diffs.push(DiffResult {
                                entity: entity.to_string(),
                                change_type: ChangeType::StructFieldReordering,
                                severity: Severity::Critical,
                                description: format!(
                                    "field '{}' moved from field #{} → #{}",
                                    old_f.name, old_idx, new_idx
                                ),
                            });
                        }
                    } else {
                        diffs.push(DiffResult {
                            entity: entity.to_string(),
                            change_type: ChangeType::StructFieldRemoval,
                            severity: Severity::Critical,
                            description: format!("Field '{}' was removed", old_f.name),
                        });
                    }
                }

                // Check field additions
                for (new_idx, new_f) in new_struct.fields.iter().enumerate() {
                    if !old_fields_map.contains_key(&new_f.name) {
                        if new_idx < old_struct.fields.len() {
                            diffs.push(DiffResult {
                                entity: entity.to_string(),
                                change_type: ChangeType::StructFieldAddition,
                                severity: Severity::Critical,
                                description: format!(
                                    "Field '{}' was inserted in the middle/front",
                                    new_f.name
                                ),
                            });
                        } else {
                            diffs.push(DiffResult {
                                entity: entity.to_string(),
                                change_type: ChangeType::StructFieldAddition,
                                severity: Severity::Minor,
                                description: format!("Field '{}' appended at end", new_f.name),
                            });
                        }
                    }
                }

                // Check type changes
                for old_f in &old_struct.fields {
                    if let Some(&(_, new_f)) = new_fields_map.get(&old_f.name) {
                        if old_f.type_ref != new_f.type_ref {
                            let mut old_engine = LayoutEngine::new(&old_ws.registry);
                            let mut new_engine = LayoutEngine::new(&new_ws.registry);
                            let old_size = old_engine
                                .size_of_type_ref("", &old_f.type_ref)
                                .map(|l| l.size)
                                .unwrap_or(0);
                            let new_size = new_engine
                                .size_of_type_ref("", &new_f.type_ref)
                                .map(|l| l.size)
                                .unwrap_or(0);

                            if old_size != new_size {
                                diffs.push(DiffResult {
                                    entity: entity.to_string(),
                                    change_type: ChangeType::TypeChange,
                                    severity: Severity::Major,
                                    description: format!(
                                        "Field '{}' type width changed: {} bytes → {} bytes",
                                        old_f.name, old_size, new_size
                                    ),
                                });
                            } else {
                                diffs.push(DiffResult {
                                    entity: entity.to_string(),
                                    change_type: ChangeType::TypeChange,
                                    severity: Severity::Critical,
                                    description: format!(
                                        "Field '{}' type changed without changing size",
                                        old_f.name
                                    ),
                                });
                            }
                        }
                    }
                }

                // Size changed overall check
                let mut old_engine = LayoutEngine::new(&old_ws.registry);
                let mut new_engine = LayoutEngine::new(&new_ws.registry);
                let old_size = old_engine
                    .size_of_absolute_path(entity)
                    .map(|l| l.size)
                    .unwrap_or(0);
                let new_size = new_engine
                    .size_of_absolute_path(entity)
                    .map(|l| l.size)
                    .unwrap_or(0);

                if old_size != new_size {
                    diffs.push(DiffResult {
                        entity: entity.to_string(),
                        change_type: ChangeType::AccountLayoutChange,
                        severity: Severity::Major,
                        description: "account layout changed".to_string(),
                    });
                }
            }
        }
        (TypeDef::Enum(old_enum), TypeDef::Enum(new_enum)) => {
            let mut old_var_map = HashMap::new();
            for (idx, v) in old_enum.variants.iter().enumerate() {
                old_var_map.insert(&v.name, (idx, v));
            }

            let mut new_var_map = HashMap::new();
            for (idx, v) in new_enum.variants.iter().enumerate() {
                new_var_map.insert(&v.name, (idx, v));
            }

            // Check variant reordering or removal
            for (old_idx, old_v) in old_enum.variants.iter().enumerate() {
                if let Some(&(new_idx, _)) = new_var_map.get(&old_v.name) {
                    if old_idx != new_idx {
                        diffs.push(DiffResult {
                            entity: entity.to_string(),
                            change_type: ChangeType::EnumVariantReordering,
                            severity: Severity::Critical,
                            description: format!(
                                "Enum variant '{}' reordered from #{} → #{}",
                                old_v.name, old_idx, new_idx
                            ),
                        });
                    }
                } else {
                    diffs.push(DiffResult {
                        entity: entity.to_string(),
                        change_type: ChangeType::EnumVariantRemoval,
                        severity: Severity::Critical,
                        description: format!("Enum variant '{}' removed", old_v.name),
                    });
                }
            }

            // Check variant additions
            for (new_idx, new_v) in new_enum.variants.iter().enumerate() {
                if !old_var_map.contains_key(&new_v.name) {
                    if new_idx < old_enum.variants.len() {
                        diffs.push(DiffResult {
                            entity: entity.to_string(),
                            change_type: ChangeType::EnumVariantAddition,
                            severity: Severity::Critical,
                            description: format!(
                                "Enum variant '{}' inserted in middle/front",
                                new_v.name
                            ),
                        });
                    } else {
                        diffs.push(DiffResult {
                            entity: entity.to_string(),
                            change_type: ChangeType::EnumVariantAddition,
                            severity: Severity::Minor,
                            description: format!("Enum variant '{}' appended at end", new_v.name),
                        });
                    }
                }
            }
        }
        (TypeDef::Alias(old_alias), TypeDef::Alias(new_alias)) => {
            if old_alias.target != new_alias.target {
                diffs.push(DiffResult {
                    entity: entity.to_string(),
                    change_type: ChangeType::TypeChange,
                    severity: Severity::Major,
                    description: format!(
                        "Type alias target changed from {:?} to {:?}",
                        old_alias.target, new_alias.target
                    ),
                });
            }
        }
        (TypeDef::Instruction(old_inst), TypeDef::Instruction(new_inst)) => {
            let mut changed = false;
            if old_inst.args.len() != new_inst.args.len() {
                changed = true;
            } else {
                for (i, old_arg) in old_inst.args.iter().enumerate() {
                    let new_arg = &new_inst.args[i];
                    if old_arg.name != new_arg.name || old_arg.type_ref != new_arg.type_ref {
                        changed = true;
                        break;
                    }
                }
            }

            if changed {
                diffs.push(DiffResult {
                    entity: entity.to_string(),
                    change_type: ChangeType::IdlBreakingChange,
                    severity: Severity::Critical,
                    description: format!("Instruction '{}' signature changed", entity),
                });
            }
        }
        _ => {
            diffs.push(DiffResult {
                entity: entity.to_string(),
                change_type: ChangeType::IdlBreakingChange,
                severity: Severity::Critical,
                description: format!("Entity '{}' changed its TypeDef type", entity),
            });
        }
    }

    diffs
}

pub fn format_diff_results(diffs: &[DiffResult]) -> String {
    if diffs.is_empty() {
        return "No layout or interface changes detected.".to_string();
    }

    let mut grouped: HashMap<String, Vec<&DiffResult>> = HashMap::new();
    for d in diffs {
        grouped.entry(d.entity.clone()).or_default().push(d);
    }

    let mut output = Vec::new();
    let mut sorted_keys: Vec<String> = grouped.keys().cloned().collect();
    sorted_keys.sort();

    for entity in sorted_keys {
        let entity_diffs = &grouped[&entity];
        let max_severity = entity_diffs
            .iter()
            .map(|d| d.severity)
            .max()
            .unwrap_or(Severity::Safe);

        let entity_name = if let Some(last_seg) = entity.split("::").last() {
            last_seg
        } else {
            &entity
        };

        output.push(format!("⚠ {}", entity_name));
        output.push(format!("Severity: {}", max_severity));
        output.push(String::new());
        output.push("Changes:".to_string());

        let mut sorted_diffs = entity_diffs.clone();
        sorted_diffs.sort_by(|a, b| a.description.cmp(&b.description));
        for d in sorted_diffs {
            output.push(format!("* {}", d.description));
        }

        output.push(String::new());
        output.push("Impact:".to_string());
        match max_severity {
            Severity::Critical => {
                output.push("* Existing accounts incompatible".to_string());
                output.push("* Migration required".to_string());
            }
            Severity::Major => {
                output.push("* Account size changed".to_string());
                output.push("* Reallocation and rent top-up required".to_string());
            }
            Severity::Minor => {
                output.push("* Layout matches or appended at end".to_string());
                output.push("* Safe to upgrade with realloc if needed".to_string());
            }
            Severity::Safe => {
                output.push("* No layout impact".to_string());
                output.push("* No migration required".to_string());
            }
        }
        output.push(String::new());
    }

    output.join("\n").trim().to_string()
}
