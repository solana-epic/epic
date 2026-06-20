use crate::ast::{ExpressionKind, ExpressionNode, StatementKind, StatementNode};
use crate::cfg::guards::{FactConfidence, GuardFact, InstructionAnalysisContext, SymbolId};
use crate::cfg::ssa::SSANodeState;
use crate::rules::{
    DominanceChecker, FindingLocation, Rule, RuleDiagnostic, RuleSeverity, SymbolResolver,
};
use std::collections::HashMap;

pub struct OwnerValidationRule;

impl Rule for OwnerValidationRule {
    fn id(&self) -> &'static str {
        "EPIC-SEC-001"
    }

    fn name(&self) -> &'static str {
        "Owner Validation Rule"
    }

    fn check(
        &self,
        context: &InstructionAnalysisContext,
        resolver: &SymbolResolver,
        dom_checker: &DominanceChecker,
    ) -> Vec<RuleDiagnostic> {
        let mut diagnostics = Vec::new();

        // Write-Dependency Graph (maps derived local symbols to their parent resource symbols)
        let mut parent_map: HashMap<SymbolId, SymbolId> = HashMap::new();

        for (&node_id, node) in &context.cfg.nodes {
            let node_ssa = match context.cfg.ssa_states.get(&node_id) {
                Some(s) => s,
                None => continue,
            };

            for (stmt_idx, stmt) in node.statements.iter().enumerate() {
                let post_ssa_state = &node_ssa.statement_states[stmt_idx];

                // Track Let-bindings for transitive WDG mutability chains (Defect C-01)
                if let StatementKind::Let {
                    name, initializer, ..
                } = &stmt.kind
                {
                    if let Some(local_var) = post_ssa_state.active_variables.get(name) {
                        let local_sym = match local_var {
                            crate::cfg::ssa::SSAVariable::Versioned { .. } => {
                                resolver.get_symbol_by_name(name)
                            }
                            _ => None,
                        };

                        if let Some(l_sym) = local_sym {
                            if let Some(parent_sym) =
                                self.find_initializer_source(initializer, resolver, post_ssa_state)
                            {
                                parent_map.insert(l_sym, parent_sym);
                            }
                        }
                    }
                }

                // Detect if the statement is a mutable write
                if let Some(write_expr) = self.get_mutable_write_expression(stmt) {
                    if let Some(base_sym) = resolver.resolve_expr(write_expr, post_ssa_state) {
                        // Trace back through the WDG to resolve the root account parameter
                        let root_sym = self.trace_to_root(base_sym, &parent_map);

                        // Enforce dominance checks only on valid account parameters
                        if self.is_account_symbol(root_sym, context) {
                            if !self.has_dominating_owner_check(
                                root_sym,
                                node_id,
                                stmt_idx,
                                context,
                                dom_checker,
                            ) {
                                diagnostics.push(RuleDiagnostic {
                                    rule_id: self.id().to_string(),
                                    severity: RuleSeverity::Critical,
                                    message: format!(
                                        "Mutable write to account symbol {:?} lacks program owner verification.",
                                        root_sym
                                    ),
                                    location: FindingLocation {
                                        file: "lib.rs".to_string(),
                                        line: stmt.line_number,
                                        column: 0,
                                        node_id,
                                        statement_index: Some(stmt_idx),
                                    },
                                    confidence: FactConfidence::Asserted,
                                    target_symbol: root_sym,
                                });
                            }
                        }
                    }
                }
            }
        }

        diagnostics
    }
}

impl OwnerValidationRule {
    fn find_initializer_source(
        &self,
        expr: &ExpressionNode,
        resolver: &SymbolResolver,
        ssa_state: &SSANodeState,
    ) -> Option<SymbolId> {
        match &expr.kind {
            ExpressionKind::MethodCall { object, method, .. } => {
                if method == "try_borrow_mut_data"
                    || method == "borrow_mut"
                    || method == "try_borrow_mut"
                {
                    return resolver.resolve_expr(object, ssa_state);
                }
                self.find_initializer_source(object, resolver, ssa_state)
            }
            ExpressionKind::Reference { expression, .. } => {
                self.find_initializer_source(expression, resolver, ssa_state)
            }
            ExpressionKind::FieldAccess { object, .. } => resolver.resolve_expr(object, ssa_state),
            ExpressionKind::Identifier(_) => resolver.resolve_expr(expr, ssa_state),
            _ => None,
        }
    }

    fn get_mutable_write_expression<'a>(
        &self,
        stmt: &'a StatementNode,
    ) -> Option<&'a ExpressionNode> {
        match &stmt.kind {
            StatementKind::Expr(expr) | StatementKind::Semi(expr) => match &expr.kind {
                ExpressionKind::Assign { left, .. } => Some(left),
                ExpressionKind::MethodCall { object, method, .. } => {
                    if method == "borrow_mut"
                        || method == "try_borrow_mut"
                        || method == "try_borrow_mut_data"
                    {
                        Some(object)
                    } else {
                        None
                    }
                }
                _ => None,
            },
            _ => None,
        }
    }

    fn trace_to_root(
        &self,
        mut sym: SymbolId,
        parent_map: &HashMap<SymbolId, SymbolId>,
    ) -> SymbolId {
        let mut visited = std::collections::HashSet::new();
        while let Some(&parent) = parent_map.get(&sym) {
            if !visited.insert(sym) {
                break;
            }
            sym = parent;
        }
        sym
    }

    fn is_account_symbol(&self, sym: SymbolId, context: &InstructionAnalysisContext) -> bool {
        context.guard_facts.iter().any(|(fact, _)| match fact {
            GuardFact::Owner { account, .. }
            | GuardFact::Signer(account)
            | GuardFact::KeyRelation { account, .. }
            | GuardFact::PDA { account, .. }
            | GuardFact::Initialized { account, .. }
            | GuardFact::Resized { account, .. }
            | GuardFact::Deallocated { account, .. } => account.symbol_id() == Some(sym),
            _ => false,
        })
    }

    fn has_dominating_owner_check(
        &self,
        sym: SymbolId,
        node_id: usize,
        stmt_idx: usize,
        context: &InstructionAnalysisContext,
        dom_checker: &DominanceChecker,
    ) -> bool {
        context.guard_facts.iter().any(|(fact, _)| {
            if let GuardFact::Owner {
                account,
                expected_owner,
            } = fact
            {
                if account.symbol_id() == Some(sym) {
                    if self.is_valid_expected_owner(expected_owner) {
                        return dom_checker.dominates(0, None, node_id, Some(stmt_idx));
                    }
                }
            }
            false
        })
    }

    fn is_valid_expected_owner(&self, expected_owner: &crate::cfg::guards::FactExpression) -> bool {
        match expected_owner {
            crate::cfg::guards::FactExpression::Literal(val) => {
                val == "program_id" || val == "spl_token" || val == "token_program"
            }
            crate::cfg::guards::FactExpression::Unknown => false,
            _ => true,
        }
    }
}
