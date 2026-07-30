use crate::ast::{ExpressionKind, ExpressionNode};
use crate::cfg::guards::{GuardFact, InstructionAnalysisContext, SSAVersionId, SymbolId};
use crate::cfg::ssa::SSANodeState;
use std::collections::HashMap;

pub struct SymbolResolver {
    /// Maps a versioned SSA identity to its canonical parameter SymbolId.
    pub version_to_symbol: HashMap<SSAVersionId, SymbolId>,
    /// Maps context parameter names (like "vault") to their core SymbolId.
    pub name_to_symbol: HashMap<String, SymbolId>,
    /// Union-Find equivalence set to track variable aliasing.
    pub equivalence_relations: HashMap<SymbolId, SymbolId>,
    /// The name of the Context variable parameter (usually "ctx" or "c").
    pub context_var_name: String,
}

impl SymbolResolver {
    pub fn new(context: &InstructionAnalysisContext) -> Self {
        let mut resolver = Self {
            version_to_symbol: HashMap::new(),
            name_to_symbol: context.symbol_table.clone(),
            equivalence_relations: HashMap::new(),
            context_var_name: context.context_var_name.clone(),
        };
        resolver.initialize_parameter_mappings(context);
        resolver
    }

    /// Walk the AST ExpressionNode recursively to resolve it to a canonical SymbolId using SSA state.
    pub fn resolve_expr(
        &self,
        expr: &ExpressionNode,
        ssa_state: &SSANodeState,
    ) -> Option<SymbolId> {
        match &expr.kind {
            ExpressionKind::Identifier(name) => {
                if let Some(ssa_var) = ssa_state.active_variables.get(name) {
                    match ssa_var {
                        crate::cfg::ssa::SSAVariable::Versioned {
                            name: base_name,
                            version,
                        } => {
                            if let Some(base_sym) = self.get_symbol_by_name(base_name) {
                                let ssa_id = SSAVersionId {
                                    symbol_id: base_sym,
                                    version: *version,
                                };
                                if let Some(resolved) = self.resolve_version(ssa_id) {
                                    return Some(resolved);
                                }
                            }
                        }
                        crate::cfg::ssa::SSAVariable::Ambiguous(_) => {}
                    }
                }
                self.get_symbol_by_name(name)
            }
            ExpressionKind::FieldAccess {
                object: _,
                field: _,
            } => {
                let path = self.get_field_access_path(expr);
                if let Some(ref p) = path {
                    let prefix = format!("{}.accounts.", self.context_var_name);
                    if p.starts_with(&prefix) {
                        let field_name = &p[prefix.len()..];
                        if let Some(sym) = self.get_symbol_by_name(field_name) {
                            return Some(sym);
                        }
                    }
                }
                None
            }
            ExpressionKind::Dereference(inner) => self.resolve_expr(inner, ssa_state),
            ExpressionKind::Reference { expression, .. } => {
                self.resolve_expr(expression, ssa_state)
            }
            ExpressionKind::Try(inner) => self.resolve_expr(inner, ssa_state),
            ExpressionKind::MethodCall { object, .. } => self.resolve_expr(object, ssa_state),
            _ => None,
        }
    }

    pub fn resolve_expr_ir(
        &self,
        expr: &epic_ir::IRExpression,
        ssa_state: &SSANodeState,
    ) -> Option<SymbolId> {
        match expr {
            epic_ir::IRExpression::Variable(name) => {
                if let Some(ssa_var) = ssa_state.active_variables.get(name) {
                    match ssa_var {
                        crate::cfg::ssa::SSAVariable::Versioned {
                            name: base_name,
                            version,
                        } => {
                            if let Some(base_sym) = self.get_symbol_by_name(base_name) {
                                let ssa_id = SSAVersionId {
                                    symbol_id: base_sym,
                                    version: *version,
                                };
                                if let Some(resolved) = self.resolve_version(ssa_id) {
                                    return Some(resolved);
                                }
                            }
                        }
                        crate::cfg::ssa::SSAVariable::Ambiguous(_) => {}
                    }
                }
                self.get_symbol_by_name(name)
            }
            epic_ir::IRExpression::FieldAccess { .. } => {
                let path = self.get_field_access_path_ir(expr);
                if let Some(ref p) = path {
                    let prefix = format!("{}.accounts.", self.context_var_name);
                    if p.starts_with(&prefix) {
                        let field_name = &p[prefix.len()..];
                        if let Some(sym) = self.get_symbol_by_name(field_name) {
                            return Some(sym);
                        }
                    }
                }
                None
            }
            epic_ir::IRExpression::Call { target, .. } => self.resolve_expr_ir(target, ssa_state),
            _ => None,
        }
    }

    pub fn register_name(&mut self, name: &str, symbol_id: SymbolId) {
        self.name_to_symbol.insert(name.to_string(), symbol_id);
    }

    pub fn get_symbol_by_name(&self, name: &str) -> Option<SymbolId> {
        self.name_to_symbol
            .get(name)
            .map(|&sym| self.find_canonical(sym))
    }

    pub fn resolve_version(&self, ssa_id: SSAVersionId) -> Option<SymbolId> {
        self.version_to_symbol
            .get(&ssa_id)
            .map(|&sym| self.find_canonical(sym))
    }

    pub fn register_alias(&mut self, ssa_id: SSAVersionId, parent: SymbolId) {
        self.version_to_symbol
            .insert(ssa_id, self.find_canonical(parent));
    }

    pub fn register_equivalence(&mut self, sym_a: SymbolId, sym_b: SymbolId) {
        let root_a = self.find_canonical(sym_a);
        let root_b = self.find_canonical(sym_b);
        if root_a != root_b {
            self.equivalence_relations.insert(root_a, root_b);
        }
    }

    pub fn find_canonical(&self, mut sym_id: SymbolId) -> SymbolId {
        while let Some(&parent) = self.equivalence_relations.get(&sym_id) {
            sym_id = parent;
        }
        sym_id
    }

    fn get_field_access_path(&self, expr: &ExpressionNode) -> Option<String> {
        match &expr.kind {
            ExpressionKind::Identifier(name) => Some(name.clone()),
            ExpressionKind::FieldAccess { object, field } => {
                let obj_path = self.get_field_access_path(object)?;
                Some(format!("{}.{}", obj_path, field))
            }
            _ => None,
        }
    }

    fn get_field_access_path_ir(&self, expr: &epic_ir::IRExpression) -> Option<String> {
        match expr {
            epic_ir::IRExpression::Variable(name) => Some(name.clone()),
            epic_ir::IRExpression::FieldAccess { object, field } => {
                let obj_path = self.get_field_access_path_ir(object)?;
                Some(format!("{}.{}", obj_path, field))
            }
            _ => None,
        }
    }

    fn initialize_parameter_mappings(&mut self, context: &InstructionAnalysisContext) {
        // Register version 1 mappings for all symbols in name_to_symbol
        for &sym_id in context.symbol_table.values() {
            let ssa_id = SSAVersionId {
                symbol_id: sym_id,
                version: 1,
            };
            self.version_to_symbol.insert(ssa_id, sym_id);
        }

        // Extract SymbolIds from accounts structurally defined in guard facts
        for (fact, _) in &context.guard_facts {
            match fact {
                GuardFact::Owner { account, .. }
                | GuardFact::Signer(account)
                | GuardFact::KeyRelation { account, .. }
                | GuardFact::PDA { account, .. }
                | GuardFact::Initialized { account, .. }
                | GuardFact::Resized { account, .. }
                | GuardFact::Deallocated { account, .. } => {
                    if let Some(sym_id) = account.symbol_id() {
                        let ssa_id = SSAVersionId {
                            symbol_id: sym_id,
                            version: 1,
                        };
                        self.version_to_symbol.entry(ssa_id).or_insert(sym_id);
                    }
                }
                _ => {}
            }
        }
    }
}
