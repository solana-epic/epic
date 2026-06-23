use crate::ast::ParameterNode;
use crate::cfg::guards::{
    extract_guards_from_accounts_struct, extract_imperative_checks,
    InstructionAnalysisContext, SymbolId,
};
use crate::cfg::ssa::SSAComputer;
use crate::cfg::CFGBuilder;
use crate::rules::{
    AnalysisContext, OwnerValidationRule, SignerValidationRule, MissingPostCpiReloadRule, PdaSeedCollisionRule, ArbitraryCpiTargetRule, ProgramMetadata, RuleDiagnostic, RuleEngine,
};
use crate::types::{StructDef, TypeDef, TypeRef, TypeRegistry};
use crate::Workspace;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use syn::visit::Visit;
use walkdir::WalkDir;

// Function to extract context generic parameter (accounts struct name)
pub fn extract_context_struct_name(type_ref: &TypeRef) -> Option<String> {
    match type_ref {
        TypeRef::Custom(name) => {
            if name.starts_with("Context<") && name.ends_with('>') {
                let inner = &name["Context<".len()..name.len() - 1];
                let parts: Vec<&str> = inner.split(',').map(|s| s.trim()).collect();
                if let Some(last) = parts.last() {
                    if !last.starts_with('\'') {
                        return Some(last.to_string());
                    }
                }
            }
            if let Some(start_idx) = name.find("Context<") {
                let rest = &name[start_idx + "Context<".len()..];
                if let Some(end_idx) = rest.find('>') {
                    let inner = &rest[..end_idx];
                    let parts: Vec<&str> = inner.split(',').map(|s| s.trim()).collect();
                    if let Some(last) = parts.last() {
                        if !last.starts_with('\'') {
                            return Some(last.to_string());
                        }
                    }
                }
            }
            None
        }
        _ => None,
    }
}

// Struct to represent raw parsed functions for instructions
pub struct RawFunction {
    pub name: String,
    pub file_path: String,
    pub signature: Vec<ParameterNode>,
    pub stmts: Vec<syn::Stmt>,
    pub context_var_name: String,
    pub context_struct_name: String,
    pub program_name: String,
    pub module_path: Vec<String>,
}

// Visitor to extract raw functions from syn AST
pub struct RawFunctionVisitor {
    pub file_path: String,
    pub program_name: String,
    pub module_path: Vec<String>,
    pub functions: Vec<RawFunction>,
}

impl<'ast> Visit<'ast> for RawFunctionVisitor {
    fn visit_item_fn(&mut self, i: &'ast syn::ItemFn) {
        if let Some(raw_fn) = self.process_fn(&i.sig, &i.block.stmts) {
            self.functions.push(raw_fn);
        }
        syn::visit::visit_item_fn(self, i);
    }

    fn visit_impl_item_fn(&mut self, i: &'ast syn::ImplItemFn) {
        if let Some(raw_fn) = self.process_fn(&i.sig, &i.block.stmts) {
            self.functions.push(raw_fn);
        }
        syn::visit::visit_impl_item_fn(self, i);
    }
}

fn get_context_generic_type(ty: &syn::Type) -> Option<String> {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "Context" {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    // Find the last argument that is a Type
                    for arg in args.args.iter().rev() {
                        if let syn::GenericArgument::Type(inner_ty) = arg {
                            if let syn::Type::Path(inner_path) = inner_ty {
                                if let Some(last_seg) = inner_path.path.segments.last() {
                                    return Some(last_seg.ident.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

impl RawFunctionVisitor {
    fn process_fn(&self, sig: &syn::Signature, stmts: &[syn::Stmt]) -> Option<RawFunction> {
        let fn_name = sig.ident.to_string();
        let mut signature = Vec::new();
        let mut context_param = None;

        for input in &sig.inputs {
            if let syn::FnArg::Typed(pat_type) = input {
                if let syn::Pat::Ident(pat_ident) = &*pat_type.pat {
                    let param_name = pat_ident.ident.to_string();
                    let type_ref = crate::workspace::parse_type(&pat_type.ty);
                    signature.push(ParameterNode {
                        name: param_name.clone(),
                        type_ref: type_ref.clone(),
                    });

                    if let Some(struct_name) = get_context_generic_type(&pat_type.ty) {
                        context_param = Some((param_name, struct_name));
                    }
                }
            }
        }

        if let Some((context_var_name, context_struct_name)) = context_param {
            Some(RawFunction {
                name: fn_name,
                file_path: self.file_path.clone(),
                signature,
                stmts: stmts.to_vec(),
                context_var_name,
                context_struct_name,
                program_name: self.program_name.clone(),
                module_path: self.module_path.clone(),
            })
        } else {
            None
        }
    }
}

pub fn find_struct_by_name<'a>(
    registry: &'a TypeRegistry,
    program_name: &str,
    module_path: &[String],
    name: &str,
) -> Option<&'a StructDef> {
    // 1. Try exact match using full module path
    let mut full_path_parts = vec![program_name.to_string()];
    full_path_parts.extend(module_path.iter().cloned());
    full_path_parts.push(name.to_string());
    let full_path = full_path_parts.join("::");

    if let Some(def) = registry.get(&full_path) {
        if let TypeDef::Struct(struct_def) = def {
            return Some(struct_def);
        }
    }

    // 2. Fallback to suffix search
    let suffix = format!("::{}", name);
    for (path, def) in &registry.definitions {
        if path == name || path.ends_with(&suffix) {
            if let TypeDef::Struct(struct_def) = def {
                return Some(struct_def);
            }
        }
    }
    None
}

/// Recursively discovers all programs, compiles CFG & SSA, extracts GuardFacts, and executes rules.
pub fn run_audit(root_path: &str) -> anyhow::Result<Vec<RuleDiagnostic>> {
    let root = Path::new(root_path);
    let mut workspace = Workspace::new();
    let mut raw_functions = Vec::new();

    // 1. Walk directory recursively to find all Rust files and parse them
    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        if entry.path().extension().map_or(false, |ext| ext == "rs") {
            let file_path = entry.path();
            
            // Determine program name based on folder structure
            let mut program_name = "program".to_string();
            let mut module_path = Vec::new();
            
            if let Ok(rel_path) = file_path.strip_prefix(root) {
                let components: Vec<String> = rel_path
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy().into_owned())
                    .collect();
                
                // If path is like programs/vault/src/instructions/deposit.rs
                if components.len() >= 4 && components[0] == "programs" && components[2] == "src" {
                    program_name = components[1].clone();
                    for comp in &components[3..] {
                        let stem = Path::new(comp).file_stem().unwrap().to_string_lossy().into_owned();
                        if stem != "lib" && stem != "mod" {
                            module_path.push(stem);
                        }
                    }
                } else if components.len() >= 2 && components[0] == "src" {
                    // Standard single-program repo
                    for comp in &components[1..] {
                        let stem = Path::new(comp).file_stem().unwrap().to_string_lossy().into_owned();
                        if stem != "lib" && stem != "mod" {
                            module_path.push(stem);
                        }
                    }
                } else {
                    // Fallback
                    let stem = file_path.file_stem().unwrap().to_string_lossy().into_owned();
                    if stem != "lib" && stem != "mod" {
                        module_path.push(stem);
                    }
                }
            }

            let content = match fs::read_to_string(file_path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let mod_path_refs: Vec<&str> = module_path.iter().map(|s| s.as_str()).collect();
            // Add file to Workspace registry so struct layouts are loaded
            if let Err(_) = workspace.add_file(&program_name, &mod_path_refs, &content, Some(&file_path.to_string_lossy())) {
                // Skip files with invalid syntax to keep scanning robust
                continue;
            }

            // Also parse file to extract raw Context-based functions
            if let Ok(file_ast) = syn::parse_str::<syn::File>(&content) {
                let mut function_visitor = RawFunctionVisitor {
                    file_path: file_path.to_string_lossy().into_owned(),
                    program_name: program_name.clone(),
                    module_path: module_path.clone(),
                    functions: Vec::new(),
                };
                function_visitor.visit_file(&file_ast);
                raw_functions.extend(function_visitor.functions);
            }
        }
    }


    let mut diagnostics = Vec::new();
    let mut rule_engine = RuleEngine::new();
    rule_engine.register_rule(Box::new(OwnerValidationRule));
    rule_engine.register_rule(Box::new(SignerValidationRule));
    rule_engine.register_rule(Box::new(MissingPostCpiReloadRule));
    rule_engine.register_rule(Box::new(PdaSeedCollisionRule));
    rule_engine.register_rule(Box::new(ArbitraryCpiTargetRule));

    fn collect_let_variables(stmts: &[syn::Stmt], vars: &mut Vec<String>) {
        for stmt in stmts {
            match stmt {
                syn::Stmt::Local(local) => {
                    if let syn::Pat::Ident(pat_ident) = &local.pat {
                        vars.push(pat_ident.ident.to_string());
                    }
                }
                syn::Stmt::Expr(syn::Expr::Block(expr_block), _) => {
                    collect_let_variables(&expr_block.block.stmts, vars);
                }
                _ => {}
            }
        }
    }

    // 2. Perform semantic analysis and run rule engine on each extracted raw function
    for raw_fn in raw_functions {
        // Find structural accounts definition matching generic Context argument
        if let Some(struct_def) = find_struct_by_name(
            &workspace.registry,
            &raw_fn.program_name,
            &raw_fn.module_path,
            &raw_fn.context_struct_name,
        ) {
            let mut symbol_table = HashMap::new();
            let mut next_symbol_id = 1;

            // Assign SymbolIds for accounts struct fields
            for field in &struct_def.fields {
                symbol_table.insert(field.name.clone(), SymbolId(next_symbol_id));
                next_symbol_id += 1;
            }

            // Assign SymbolIds for let-bound variables
            let mut let_vars = Vec::new();
            collect_let_variables(&raw_fn.stmts, &mut let_vars);
            for var_name in let_vars {
                if !symbol_table.contains_key(&var_name) {
                    symbol_table.insert(var_name, SymbolId(next_symbol_id));
                    next_symbol_id += 1;
                }
            }

            // Extract guard facts
            let mut guard_facts = extract_guards_from_accounts_struct(
                struct_def,
                &mut symbol_table,
                &mut next_symbol_id,
            );

            // Re-map provenance source files in extracted guard facts to point to raw_fn's file
            for (_, prov) in &mut guard_facts {
                prov.source_file = raw_fn.file_path.clone();
            }

            // Build Control Flow Graph from statements
            let mut cfg_builder = CFGBuilder::new();
            let _final_node = match cfg_builder.compile_statements(&raw_fn.stmts, 0) {
                Ok(n) => n,
                Err(_) => continue, // skip function if CFG compilation fails
            };

            let mut cfg = cfg_builder.graph;

            // Extract imperative checks from CFG and add to guard_facts
            let imperative_guards = extract_imperative_checks(
                &cfg,
                &symbol_table,
                &raw_fn.file_path,
            );
            guard_facts.extend(imperative_guards);

            // Compute SSA-lite variables and infer type propagation
            let mut ssa_computer = SSAComputer::new(&workspace.registry, &cfg);
            let ssa_states = ssa_computer.compute(&raw_fn.signature);
            cfg.ssa_states = ssa_states;

            // Construct unified InstructionAnalysisContext
            let instruction_context = InstructionAnalysisContext {
                name: raw_fn.name.clone(),
                guard_facts: guard_facts.clone(),
                cfg: cfg.clone(),
                symbol_table: symbol_table.clone(),
                file_path: raw_fn.file_path.clone(),
                context_var_name: raw_fn.context_var_name.clone(),
            };


            // Build full AnalysisContext
            let analysis_context = AnalysisContext {
                program_metadata: ProgramMetadata {
                    name: raw_fn.program_name.clone(),
                    address: None,
                },
                idl_metadata: None,
                ast_graph: workspace.clone(),
                instruction_context,
                rule_registry: Vec::new(),
            };

            // Run rules using RuleEngine
            let findings = rule_engine.run_all(&analysis_context);
            diagnostics.extend(findings);
        }
    }

    Ok(diagnostics)
}
