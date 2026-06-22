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
    #[serde(default)]
    pub node_id: Option<usize>,
    #[serde(default)]
    pub statement_index: Option<usize>,
}

impl FactProvenance {
    pub fn default_declared() -> Self {
        Self {
            source_file: "lib.rs".to_string(),
            line_number: 0,
            column_number: 0,
            framework: "Anchor".to_string(),
            confidence_level: FactConfidence::Declared,
            node_id: None,
            statement_index: None,
        }
    }
}

/// The analysis context for a single instruction handler, combining GuardFacts with CFG.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstructionAnalysisContext {
    pub name: String,
    pub guard_facts: Vec<(GuardFact, FactProvenance)>,
    pub cfg: crate::cfg::ControlFlowGraph,
    #[serde(default)]
    pub symbol_table: HashMap<String, SymbolId>,
    #[serde(default)]
    pub file_path: String,
    #[serde(default = "default_context_var")]
    pub context_var_name: String,
}

fn default_context_var() -> String {
    "ctx".to_string()
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

fn preprocess_anchor_attribute_string(body: &str) -> String {
    let mut result = String::new();
    let mut depth: usize = 0;
    let mut skipping = false;

    for c in body.chars() {
        match c {
            '(' | '[' | '{' => {
                depth += 1;
                if !skipping {
                    result.push(c);
                }
            }
            ')' | ']' | '}' => {
                depth = depth.saturating_sub(1);
                if !skipping {
                    result.push(c);
                }
            }
            '@' if depth == 0 => {
                skipping = true;
            }
            ',' if depth == 0 => {
                skipping = false;
                result.push(c);
            }
            _ => {
                if !skipping {
                    result.push(c);
                }
            }
        }
    }
    result
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

    let body_preprocessed = preprocess_anchor_attribute_string(body);

    // Preprocess: Replace standalone "mut" keyword with "writable" to avoid syn syntax error
    let parts: Vec<String> = body_preprocessed
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

fn unwrap_account_type(type_ref: &TypeRef) -> Option<String> {
    match type_ref {
        TypeRef::Option(inner) => unwrap_account_type(inner),
        TypeRef::Custom(name) => Some(name.clone()),
        _ => None,
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
        if let Some(custom_type) = unwrap_account_type(&field.type_ref) {
            if custom_type == "Account"
                || custom_type.starts_with("Account<")
                || custom_type == "AccountLoader"
                || custom_type.starts_with("AccountLoader<")
                || custom_type == "InterfaceAccount"
                || custom_type.starts_with("InterfaceAccount<")
            {
                // Implicit owner validation fact (Account<'info, T> owner is the current program ID)
                facts.push((
                    GuardFact::Owner {
                        account: target_acc.clone(),
                        expected_owner: FactExpression::Literal("program_id".to_string()),
                    },
                    FactProvenance::default_declared(),
                ));
            }
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
                            FactProvenance::default_declared(),
                        ));
                    }
                    ParsedAttributeMeta::Signer => {
                        facts.push((
                            GuardFact::Signer(target_acc.clone()),
                            FactProvenance::default_declared(),
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
                            FactProvenance::default_declared(),
                        ));
                    }
                    ParsedAttributeMeta::Owner(expr) => {
                        let owner_expr = convert_syn_expr(expr, symbol_table);
                        facts.push((
                            GuardFact::Owner {
                                account: target_acc.clone(),
                                expected_owner: owner_expr,
                            },
                            FactProvenance::default_declared(),
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
                            FactProvenance::default_declared(),
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
                            FactProvenance::default_declared(),
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
                            FactProvenance::default_declared(),
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
                    FactProvenance::default_declared(),
                ));
            }
        }
    }

    facts
}

use crate::ast::{ExpressionKind, ExpressionNode};
use std::collections::HashSet;

fn is_terminating_branch(cfg: &crate::cfg::ControlFlowGraph, start_node: usize, escape_node: usize) -> bool {
    let mut visited = HashSet::new();
    let mut queue = Vec::new();
    queue.push(start_node);
    visited.insert(start_node);

    while let Some(current) = queue.pop() {
        if current == escape_node {
            return false;
        }
        // Find all outgoing edges from current
        for edge in &cfg.edges {
            if edge.from == current {
                if visited.insert(edge.to) {
                    queue.push(edge.to);
                }
            }
        }
    }
    true
}

fn extract_owner_check_from_expr(expr: &ExpressionNode) -> Option<(String, String, String)> {
    match &expr.kind {
        ExpressionKind::BinaryOp { op, lhs, rhs } => {
            if op == "==" || op == "!=" {
                if let Some(acc) = get_account_from_owner_field(lhs) {
                    let expected = expr_to_string(rhs);
                    return Some((acc, expected, op.clone()));
                }
                if let Some(acc) = get_account_from_owner_field(rhs) {
                    let expected = expr_to_string(lhs);
                    return Some((acc, expected, op.clone()));
                }
            }
        }
        _ => {}
    }
    None
}

fn get_account_from_owner_field(expr: &ExpressionNode) -> Option<String> {
    match &expr.kind {
        ExpressionKind::FieldAccess { object, field } => {
            if field == "owner" {
                let obj_str = expr_to_string(object);
                if obj_str.starts_with("ctx.accounts.") {
                    return Some(obj_str["ctx.accounts.".len()..].to_string());
                } else {
                    return Some(obj_str);
                }
            }
        }
        _ => {}
    }
    None
}

fn expr_to_string(expr: &ExpressionNode) -> String {
    match &expr.kind {
        ExpressionKind::Identifier(name) => name.clone(),
        ExpressionKind::Literal(val) => val.clone(),
        ExpressionKind::FieldAccess { object, field } => {
            format!("{}.{}", expr_to_string(object), field)
        }
        ExpressionKind::MethodCall { object, method, .. } => {
            format!("{}.{}()", expr_to_string(object), method)
        }
        ExpressionKind::Reference { expression, .. } => expr_to_string(expression),
        ExpressionKind::Dereference(expression) => expr_to_string(expression),
        _ => "unknown".to_string(),
    }
}

fn extract_owner_from_syn_expr(expr: &syn::Expr) -> Option<String> {
    if let syn::Expr::Field(expr_field) = expr {
        if let syn::Member::Named(ident) = &expr_field.member {
            if ident == "owner" {
                let base_str = quote::quote!(#expr_field.base).to_string().replace(" ", "");
                if base_str.starts_with("ctx.accounts.") {
                    return Some(base_str["ctx.accounts.".len()..].to_string());
                } else {
                    return Some(base_str);
                }
            }
        }
    }
    None
}

fn extract_expected_owner_from_syn_expr(expr: &syn::Expr) -> String {
    quote::quote!(#expr).to_string().replace(" ", "")
}

pub fn extract_imperative_checks(
    cfg: &crate::cfg::ControlFlowGraph,
    symbol_table: &HashMap<String, SymbolId>,
    file_path: &str,
) -> Vec<(GuardFact, FactProvenance)> {
    let mut facts = Vec::new();

    // 1. Process If Statement Conditions on CFG Edges
    for edge in &cfg.edges {
        if let Some(cond) = &edge.condition {
            if let Some((acc_name, expected_owner_str, op)) = extract_owner_check_from_expr(cond) {
                if let Some(&symbol_id) = symbol_table.get(&acc_name) {
                    let target_acc = GuardTarget::Account(symbol_id);
                    
                    if op == "!=" {
                        let else_node_opt = cfg.edges.iter()
                            .find(|e| e.from == edge.from && e.to != edge.to)
                            .map(|e| e.to);
                        if let Some(else_node) = else_node_opt {
                            if is_terminating_branch(cfg, edge.to, else_node) {
                                facts.push((
                                    GuardFact::Owner {
                                        account: target_acc.clone(),
                                        expected_owner: FactExpression::Literal(expected_owner_str),
                                    },
                                    FactProvenance {
                                        source_file: file_path.to_string(),
                                        line_number: 0,
                                        column_number: 0,
                                        framework: "Anchor".to_string(),
                                        confidence_level: FactConfidence::Asserted,
                                        node_id: Some(else_node),
                                        statement_index: None,
                                    },
                                ));
                            }
                        }
                    } else if op == "==" {
                        let else_node_opt = cfg.edges.iter()
                            .find(|e| e.from == edge.from && e.to != edge.to)
                            .map(|e| e.to);
                        if let Some(else_node) = else_node_opt {
                            if is_terminating_branch(cfg, else_node, edge.to) {
                                facts.push((
                                    GuardFact::Owner {
                                        account: target_acc.clone(),
                                        expected_owner: FactExpression::Literal(expected_owner_str),
                                    },
                                    FactProvenance {
                                        source_file: file_path.to_string(),
                                        line_number: 0,
                                        column_number: 0,
                                        framework: "Anchor".to_string(),
                                        confidence_level: FactConfidence::Asserted,
                                        node_id: Some(edge.to),
                                        statement_index: None,
                                    },
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    // 2. Process Macro Assertions (require!, assert_eq!, etc.) inside basic blocks
    for (&node_id, node) in &cfg.nodes {
        for (stmt_idx, stmt) in node.statements.iter().enumerate() {
            if let crate::ast::StatementKind::MacroCall { name, raw_args } = &stmt.kind {
                if name == "require" || name == "assert" {
                    let parts: Vec<&str> = raw_args.split(',').collect();
                    if let Some(&first_part) = parts.first() {
                        let cleaned = first_part.replace(" ", "");
                        if let Ok(expr) = syn::parse_str::<syn::Expr>(&cleaned) {
                            if let syn::Expr::Binary(eb) = expr {
                                if matches!(eb.op, syn::BinOp::Eq(_)) {
                                    if let Some(acc) = extract_owner_from_syn_expr(&eb.left) {
                                        let expected = extract_expected_owner_from_syn_expr(&eb.right);
                                        if let Some(&symbol_id) = symbol_table.get(&acc) {
                                            facts.push((
                                                GuardFact::Owner {
                                                    account: GuardTarget::Account(symbol_id),
                                                    expected_owner: FactExpression::Literal(expected),
                                                },
                                                FactProvenance {
                                                    source_file: file_path.to_string(),
                                                    line_number: stmt.line_number,
                                                    column_number: 0,
                                                    framework: "Anchor".to_string(),
                                                    confidence_level: FactConfidence::Asserted,
                                                    node_id: Some(node_id),
                                                    statement_index: Some(stmt_idx),
                                                },
                                            ));
                                        }
                                    } else if let Some(acc) = extract_owner_from_syn_expr(&eb.right) {
                                        let expected = extract_expected_owner_from_syn_expr(&eb.left);
                                        if let Some(&symbol_id) = symbol_table.get(&acc) {
                                            facts.push((
                                                GuardFact::Owner {
                                                    account: GuardTarget::Account(symbol_id),
                                                    expected_owner: FactExpression::Literal(expected),
                                                },
                                                FactProvenance {
                                                    source_file: file_path.to_string(),
                                                    line_number: stmt.line_number,
                                                    column_number: 0,
                                                    framework: "Anchor".to_string(),
                                                    confidence_level: FactConfidence::Asserted,
                                                    node_id: Some(node_id),
                                                    statement_index: Some(stmt_idx),
                                                },
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else if name == "assert_eq" || name == "require_keys_eq" || name == "require_eq" {
                    let parts: Vec<&str> = raw_args.split(',').collect();
                    if parts.len() >= 2 {
                        let arg1 = parts[0].replace(" ", "");
                        let arg2 = parts[1].replace(" ", "");
                        if let (Ok(e1), Ok(e2)) = (syn::parse_str::<syn::Expr>(&arg1), syn::parse_str::<syn::Expr>(&arg2)) {
                            if let Some(acc) = extract_owner_from_syn_expr(&e1) {
                                let expected = extract_expected_owner_from_syn_expr(&e2);
                                if let Some(&symbol_id) = symbol_table.get(&acc) {
                                    facts.push((
                                        GuardFact::Owner {
                                            account: GuardTarget::Account(symbol_id),
                                            expected_owner: FactExpression::Literal(expected),
                                        },
                                        FactProvenance {
                                            source_file: file_path.to_string(),
                                            line_number: stmt.line_number,
                                            column_number: 0,
                                            framework: "Anchor".to_string(),
                                            confidence_level: FactConfidence::Asserted,
                                            node_id: Some(node_id),
                                            statement_index: Some(stmt_idx),
                                        },
                                    ));
                                }
                            } else if let Some(acc) = extract_owner_from_syn_expr(&e2) {
                                let expected = extract_expected_owner_from_syn_expr(&e1);
                                if let Some(&symbol_id) = symbol_table.get(&acc) {
                                    facts.push((
                                        GuardFact::Owner {
                                            account: GuardTarget::Account(symbol_id),
                                            expected_owner: FactExpression::Literal(expected),
                                        },
                                        FactProvenance {
                                            source_file: file_path.to_string(),
                                            line_number: stmt.line_number,
                                            column_number: 0,
                                            framework: "Anchor".to_string(),
                                            confidence_level: FactConfidence::Asserted,
                                            node_id: Some(node_id),
                                            statement_index: Some(stmt_idx),
                                        },
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    facts
}
