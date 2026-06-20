use crate::types::{StructDef, TypeRef};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use syn::parse::Parser;

/// A globally unique symbol identifier within an instruction handler context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SymbolId(pub usize);

/// Identifies a specific version of a symbol in the SSA-lite tracking system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SSAVersionId {
    pub symbol_id: SymbolId,
    pub version: usize,
}

/// A canonical target of a guard constraint.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GuardTarget {
    Variable(SSAVersionId),
    Account(SymbolId),
    Literal(String),
}

impl GuardTarget {
    pub fn symbol_id(&self) -> Option<SymbolId> {
        match self {
            GuardTarget::Variable(v) => Some(v.symbol_id),
            GuardTarget::Account(s) => Some(*s),
            GuardTarget::Literal(_) => None,
        }
    }
}

/// A framework-neutral semantic expression layer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FactExpression {
    Target(GuardTarget),
    Literal(String),
    PropertyOf {
        target: GuardTarget,
        property: SolanaProperty,
    },
    BinaryOp {
        op: String,
        lhs: Box<FactExpression>,
        rhs: Box<FactExpression>,
    },
    Unknown,
}

/// Standard Solana Virtual Machine account properties.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SolanaProperty {
    Address,
    Owner,
    Lamports,
    DataLength,
    IsSigner,
    IsWritable,
    Executable,
}

/// Structured security metadata derived from declarations or explicit checks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GuardFact {
    Signer(GuardTarget),
    Owner {
        account: GuardTarget,
        expected_owner: FactExpression,
    },
    KeyRelation {
        account: GuardTarget,
        field: SolanaProperty,
        target: GuardTarget,
    },
    PDA {
        account: GuardTarget,
        seeds: Vec<FactExpression>,
        bump: Option<FactExpression>,
    },
    Initialized {
        account: GuardTarget,
        payer: GuardTarget,
        space: Option<FactExpression>,
    },
    Resized {
        account: GuardTarget,
        new_size: FactExpression,
        payer: GuardTarget,
    },
    Deallocated {
        account: GuardTarget,
        destination: GuardTarget,
    },
    Custom {
        namespace: String,
        kind: String,
        payload: Vec<FactExpression>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DominanceInterval {
    pub dfs_entry: usize,
    pub dfs_exit: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuardFactLocation {
    pub node_id: usize,
    pub statement_index: Option<usize>,
    pub dominance_interval: DominanceInterval,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FactConfidence {
    Declared,
    Asserted,
    Inconclusive,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FactProvenance {
    pub source_file: String,
    pub line_number: usize,
    pub column_number: usize,
    pub framework: String,
    pub confidence_level: FactConfidence,
}

/// The analysis context for a single instruction handler, combining GuardFacts with CFG.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstructionAnalysisContext {
    pub name: String,
    pub guard_facts: Vec<(GuardFact, FactProvenance)>,
    pub cfg: crate::cfg::ControlFlowGraph,
}

// === Parser & Translator Implementation ===

#[allow(dead_code)]
#[derive(Debug, Clone)]
enum ParsedAttributeMeta {
    Mut,
    Signer,
    Owner(syn::Expr),
    HasOne(syn::Expr),
    Constraint(syn::Expr),
    Close(syn::Expr),
    Seeds(syn::Expr),
    Bump(Option<syn::Expr>),
    Init,
    Realloc(syn::Expr),
    Space(syn::Expr),
    Payer(syn::Expr),
}

fn parse_anchor_attribute_string(attr_str: &str) -> Vec<ParsedAttributeMeta> {
    let mut results = Vec::new();

    // Normalize string: Remove starting #[account( and ending )
    let body = if attr_str.starts_with("#[account(") && attr_str.ends_with(")]") {
        &attr_str["#[account(".len()..attr_str.len() - ")]".len()]
    } else if attr_str.starts_with("account(") && attr_str.ends_with(")") {
        &attr_str["account(".len()..attr_str.len() - ")".len()]
    } else {
        attr_str
    };

    // Preprocess: Replace standalone "mut" keyword with "writable" to avoid syn syntax error
    let parts: Vec<String> = body
        .split(',')
        .map(|s| {
            let trimmed = s.trim();
            if trimmed == "mut" {
                "writable".to_string()
            } else {
                s.to_string()
            }
        })
        .collect();
    let body_normalized = parts.join(",");

    // Use syn to parse the tokens inside the attribute parameters list
    let tokens: proc_macro2::TokenStream = match body_normalized.parse() {
        Ok(t) => t,
        Err(_) => return results,
    };

    let parser = syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated;
    let meta_list = match parser.parse2(tokens) {
        Ok(parsed) => parsed,
        Err(_) => {
            return results;
        }
    };

    for meta in meta_list {
        match meta {
            syn::Meta::Path(path) => {
                let ident = path.get_ident().map(|id| id.to_string());
                match ident.as_deref() {
                    Some("writable") => results.push(ParsedAttributeMeta::Mut),
                    Some("signer") => results.push(ParsedAttributeMeta::Signer),
                    Some("init") => results.push(ParsedAttributeMeta::Init),
                    Some("bump") => results.push(ParsedAttributeMeta::Bump(None)),
                    _ => {}
                }
            }
            syn::Meta::NameValue(nv) => {
                let ident = nv.path.get_ident().map(|id| id.to_string());
                match ident.as_deref() {
                    Some("owner") => results.push(ParsedAttributeMeta::Owner(nv.value)),
                    Some("has_one") => results.push(ParsedAttributeMeta::HasOne(nv.value)),
                    Some("constraint") => results.push(ParsedAttributeMeta::Constraint(nv.value)),
                    Some("close") => results.push(ParsedAttributeMeta::Close(nv.value)),
                    Some("seeds") => results.push(ParsedAttributeMeta::Seeds(nv.value)),
                    Some("bump") => results.push(ParsedAttributeMeta::Bump(Some(nv.value))),
                    Some("realloc") => results.push(ParsedAttributeMeta::Realloc(nv.value)),
                    Some("space") => results.push(ParsedAttributeMeta::Space(nv.value)),
                    Some("payer") => results.push(ParsedAttributeMeta::Payer(nv.value)),
                    _ => {}
                }
            }
            _ => {}
        }
    }

    results
}

pub fn convert_syn_expr(
    expr: &syn::Expr,
    symbol_table: &HashMap<String, SymbolId>,
) -> FactExpression {
    match expr {
        syn::Expr::Path(expr_path) => {
            let name = quote::quote!(#expr_path).to_string().replace(" ", "");
            if let Some(&symbol_id) = symbol_table.get(&name) {
                FactExpression::Target(GuardTarget::Variable(SSAVersionId {
                    symbol_id,
                    version: 1,
                }))
            } else {
                FactExpression::Literal(name)
            }
        }
        syn::Expr::Lit(expr_lit) => {
            let val = quote::quote!(#expr_lit).to_string().replace(" ", "");
            FactExpression::Literal(val)
        }
        syn::Expr::Field(expr_field) => {
            let base = convert_syn_expr(&expr_field.base, symbol_table);
            let field = match &expr_field.member {
                syn::Member::Named(ident) => ident.to_string(),
                syn::Member::Unnamed(idx) => idx.index.to_string(),
            };
            let target = match base {
                FactExpression::Target(t) => t,
                _ => GuardTarget::Literal(
                    quote::quote!(#expr_field.base).to_string().replace(" ", ""),
                ),
            };
            let property = match field.as_str() {
                "owner" => SolanaProperty::Owner,
                "lamports" => SolanaProperty::Lamports,
                "key" => SolanaProperty::Address,
                _ => SolanaProperty::Address,
            };
            FactExpression::PropertyOf { target, property }
        }
        syn::Expr::Binary(expr_binary) => {
            let op_token = &expr_binary.op;
            let op = match op_token {
                syn::BinOp::Eq(_) => "==".to_string(),
                syn::BinOp::Ne(_) => "!=".to_string(),
                syn::BinOp::Lt(_) => "<".to_string(),
                syn::BinOp::Gt(_) => ">".to_string(),
                _ => quote::quote!(#op_token).to_string().replace(" ", ""),
            };
            let lhs = convert_syn_expr(&expr_binary.left, symbol_table);
            let rhs = convert_syn_expr(&expr_binary.right, symbol_table);
            FactExpression::BinaryOp {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            }
        }
        _ => {
            let val = quote::quote!(#expr).to_string().replace(" ", "");
            FactExpression::Literal(val)
        }
    }
}

/// Ingest and translate Anchor field declarations of an Accounts struct into canonical GuardFacts.
pub fn extract_guards_from_accounts_struct(
    struct_def: &StructDef,
    symbol_table: &mut HashMap<String, SymbolId>,
    next_symbol_id: &mut usize,
) -> Vec<(GuardFact, FactProvenance)> {
    let mut facts = Vec::new();

    // First, populate the symbol table for all account fields in this struct
    for field in &struct_def.fields {
        if !symbol_table.contains_key(&field.name) {
            symbol_table.insert(field.name.clone(), SymbolId(*next_symbol_id));
            *next_symbol_id += 1;
        }
    }

    for field in &struct_def.fields {
        let account_symbol = symbol_table.get(&field.name).cloned().unwrap();
        let target_acc = GuardTarget::Account(account_symbol);

        // Map implicit ownership validation checks based on TypeRef
        match &field.type_ref {
            TypeRef::Custom(custom_type) => {
                if custom_type.starts_with("Account") || custom_type.starts_with("AccountLoader") {
                    // Implicit owner validation fact (Account<'info, T> owner is the current program ID)
                    facts.push((
                        GuardFact::Owner {
                            account: target_acc.clone(),
                            expected_owner: FactExpression::Literal("program_id".to_string()),
                        },
                        FactProvenance {
                            source_file: "lib.rs".to_string(),
                            line_number: 0,
                            column_number: 0,
                            framework: "Anchor".to_string(),
                            confidence_level: FactConfidence::Declared,
                        },
                    ));
                }
            }
            _ => {}
        }

        // Parse explicit constraints
        for attr_str in &field.attrs {
            let parsed_metas = parse_anchor_attribute_string(attr_str);

            let mut is_init = false;
            let mut payer_target = None;
            let mut space_expr = None;

            for meta in &parsed_metas {
                match meta {
                    ParsedAttributeMeta::Mut => {
                        // SVM-level is_writable constraint
                        facts.push((
                            GuardFact::KeyRelation {
                                account: target_acc.clone(),
                                field: SolanaProperty::IsWritable,
                                target: GuardTarget::Literal("true".to_string()),
                            },
                            FactProvenance {
                                source_file: "lib.rs".to_string(),
                                line_number: 0,
                                column_number: 0,
                                framework: "Anchor".to_string(),
                                confidence_level: FactConfidence::Declared,
                            },
                        ));
                    }
                    ParsedAttributeMeta::Signer => {
                        facts.push((
                            GuardFact::Signer(target_acc.clone()),
                            FactProvenance {
                                source_file: "lib.rs".to_string(),
                                line_number: 0,
                                column_number: 0,
                                framework: "Anchor".to_string(),
                                confidence_level: FactConfidence::Declared,
                            },
                        ));
                    }
                    ParsedAttributeMeta::HasOne(expr) => {
                        let target_expr = convert_syn_expr(expr, symbol_table);
                        let target_var = match target_expr {
                            FactExpression::Target(t) => t,
                            _ => GuardTarget::Literal(
                                quote::quote!(#expr).to_string().replace(" ", ""),
                            ),
                        };
                        facts.push((
                            GuardFact::KeyRelation {
                                account: target_acc.clone(),
                                field: SolanaProperty::Address,
                                target: target_var,
                            },
                            FactProvenance {
                                source_file: "lib.rs".to_string(),
                                line_number: 0,
                                column_number: 0,
                                framework: "Anchor".to_string(),
                                confidence_level: FactConfidence::Declared,
                            },
                        ));
                    }
                    ParsedAttributeMeta::Owner(expr) => {
                        let owner_expr = convert_syn_expr(expr, symbol_table);
                        facts.push((
                            GuardFact::Owner {
                                account: target_acc.clone(),
                                expected_owner: owner_expr,
                            },
                            FactProvenance {
                                source_file: "lib.rs".to_string(),
                                line_number: 0,
                                column_number: 0,
                                framework: "Anchor".to_string(),
                                confidence_level: FactConfidence::Declared,
                            },
                        ));
                    }
                    ParsedAttributeMeta::Close(expr) => {
                        let dest_expr = convert_syn_expr(expr, symbol_table);
                        let dest_target = match dest_expr {
                            FactExpression::Target(t) => t,
                            _ => GuardTarget::Literal(
                                quote::quote!(#expr).to_string().replace(" ", ""),
                            ),
                        };
                        facts.push((
                            GuardFact::Deallocated {
                                account: target_acc.clone(),
                                destination: dest_target,
                            },
                            FactProvenance {
                                source_file: "lib.rs".to_string(),
                                line_number: 0,
                                column_number: 0,
                                framework: "Anchor".to_string(),
                                confidence_level: FactConfidence::Declared,
                            },
                        ));
                    }
                    ParsedAttributeMeta::Seeds(expr) => {
                        let seed_expr = convert_syn_expr(expr, symbol_table);
                        facts.push((
                            GuardFact::PDA {
                                account: target_acc.clone(),
                                seeds: vec![seed_expr],
                                bump: None,
                            },
                            FactProvenance {
                                source_file: "lib.rs".to_string(),
                                line_number: 0,
                                column_number: 0,
                                framework: "Anchor".to_string(),
                                confidence_level: FactConfidence::Declared,
                            },
                        ));
                    }
                    ParsedAttributeMeta::Bump(opt_expr) => {
                        let bump_expr =
                            opt_expr.as_ref().map(|e| convert_syn_expr(e, symbol_table));
                        // Update existing PDA fact with bump if present
                        if let Some(pos) = facts
                            .iter()
                            .position(|f| matches!(f.0, GuardFact::PDA { .. }))
                        {
                            if let GuardFact::PDA { account, seeds, .. } = &facts[pos].0 {
                                facts[pos].0 = GuardFact::PDA {
                                    account: account.clone(),
                                    seeds: seeds.clone(),
                                    bump: bump_expr,
                                };
                            }
                        }
                    }
                    ParsedAttributeMeta::Init => {
                        is_init = true;
                    }
                    ParsedAttributeMeta::Payer(expr) => {
                        let payer_val = convert_syn_expr(expr, symbol_table);
                        payer_target = match payer_val {
                            FactExpression::Target(t) => Some(t),
                            _ => Some(GuardTarget::Literal(
                                quote::quote!(#expr).to_string().replace(" ", ""),
                            )),
                        };
                    }
                    ParsedAttributeMeta::Space(expr) => {
                        space_expr = Some(convert_syn_expr(expr, symbol_table));
                    }
                    ParsedAttributeMeta::Realloc(expr) => {
                        let size_expr = convert_syn_expr(expr, symbol_table);
                        facts.push((
                            GuardFact::Resized {
                                account: target_acc.clone(),
                                new_size: size_expr,
                                payer: GuardTarget::Literal("payer".to_string()),
                            },
                            FactProvenance {
                                source_file: "lib.rs".to_string(),
                                line_number: 0,
                                column_number: 0,
                                framework: "Anchor".to_string(),
                                confidence_level: FactConfidence::Declared,
                            },
                        ));
                    }
                    _ => {}
                }
            }

            if is_init {
                let payer = payer_target.unwrap_or(GuardTarget::Literal("payer".to_string()));
                facts.push((
                    GuardFact::Initialized {
                        account: target_acc.clone(),
                        payer,
                        space: space_expr,
                    },
                    FactProvenance {
                        source_file: "lib.rs".to_string(),
                        line_number: 0,
                        column_number: 0,
                        framework: "Anchor".to_string(),
                        confidence_level: FactConfidence::Declared,
                    },
                ));
            }
        }
    }

    facts
}
