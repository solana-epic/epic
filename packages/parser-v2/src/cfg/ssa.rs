use crate::ast::{
    ExpressionKind, InconclusiveReason, InferenceResult, InferenceScope, ParameterNode,
    StatementKind, StatementNode, TypeInferenceEngine,
};
use crate::cfg::nodes::ControlFlowGraph;
use crate::types::{TypeRef, TypeRegistry};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

/// Represents a versioned variable in SSA-lite form.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SSAVariable {
    Versioned { name: String, version: usize },
    Ambiguous(String),
}

impl std::fmt::Display for SSAVariable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SSAVariable::Versioned { name, version } => write!(f, "{}#{}", name, version),
            SSAVariable::Ambiguous(name) => write!(f, "{}#ambiguous", name),
        }
    }
}

impl SSAVariable {
    pub fn name(&self) -> &str {
        match self {
            SSAVariable::Versioned { name, .. } => name,
            SSAVariable::Ambiguous(name) => name,
        }
    }
}

/// The state of all active variables and their types at a specific execution point.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SSANodeState {
    /// Maps original variable name to its active SSA variable version.
    pub active_variables: HashMap<String, SSAVariable>,
    /// Maps each versioned SSA variable string representation to its inferred type.
    pub variable_types: HashMap<String, TypeRef>,
}

/// SSA version tracking information associated with a single CFG node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeSSAInfo {
    /// State immediately before any statement in this node is executed.
    pub start_state: SSANodeState,
    /// State immediately after each statement inside this node.
    pub statement_states: Vec<SSANodeState>,
    /// State after the last statement in this node has executed.
    pub end_state: SSANodeState,
}

/// Computes SSA-lite versioning and type propagation over a Control Flow Graph.
pub struct SSAComputer<'a> {
    pub registry: &'a TypeRegistry,
    pub graph: &'a ControlFlowGraph,
    version_counters: HashMap<String, usize>,
    pub node_states: HashMap<usize, NodeSSAInfo>,
}

impl<'a> SSAComputer<'a> {
    pub fn new(registry: &'a TypeRegistry, graph: &'a ControlFlowGraph) -> Self {
        Self {
            registry,
            graph,
            version_counters: HashMap::new(),
            node_states: HashMap::new(),
        }
    }

    /// Run the SSA version tracking analysis and return the computed states.
    pub fn compute(&mut self, signature: &[ParameterNode]) -> HashMap<usize, NodeSSAInfo> {
        // Initialize entry node starting state with function parameters as version 1
        let mut entry_start = SSANodeState::default();
        for param in signature {
            let var = SSAVariable::Versioned {
                name: param.name.clone(),
                version: 1,
            };
            self.version_counters.insert(param.name.clone(), 1);
            entry_start
                .active_variables
                .insert(param.name.clone(), var.clone());
            entry_start
                .variable_types
                .insert(var.to_string(), param.type_ref.clone());
        }

        // Kahn's topological sort to process each node exactly once
        let mut in_degree = HashMap::new();
        for &node_id in self.graph.nodes.keys() {
            in_degree.insert(node_id, 0);
        }
        for edge in &self.graph.edges {
            *in_degree.entry(edge.to).or_insert(0) += 1;
        }

        let mut queue = VecDeque::new();
        for (&node_id, &deg) in &in_degree {
            if deg == 0 {
                queue.push_back(node_id);
            }
        }

        let mut order = Vec::new();
        while let Some(u) = queue.pop_front() {
            order.push(u);
            for edge in &self.graph.edges {
                if edge.from == u {
                    let v = edge.to;
                    if let Some(deg) = in_degree.get_mut(&v) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push_back(v);
                        }
                    }
                }
            }
        }

        // Process each node in topological order
        for u in order {
            let start_state = if u == 0 {
                entry_start.clone()
            } else {
                self.merge_predecessors(u)
            };

            let (statement_states, end_state) = self.propagate_node(u, &start_state);

            let info = NodeSSAInfo {
                start_state,
                statement_states,
                end_state,
            };
            self.node_states.insert(u, info);
        }

        self.node_states.clone()
    }

    fn merge_predecessors(&self, u: usize) -> SSANodeState {
        let mut preds = Vec::new();
        for edge in &self.graph.edges {
            if edge.to == u {
                if let Some(pred_info) = self.node_states.get(&edge.from) {
                    preds.push(&pred_info.end_state);
                }
            }
        }

        if preds.is_empty() {
            return SSANodeState::default();
        }
        if preds.len() == 1 {
            return preds[0].clone();
        }

        let mut merged = SSANodeState::default();
        let mut all_vars = HashSet::new();
        for pred in &preds {
            for name in pred.active_variables.keys() {
                all_vars.insert(name.clone());
            }
        }

        for var_name in all_vars {
            let mut versions = Vec::new();
            for pred in &preds {
                if let Some(ver) = pred.active_variables.get(&var_name) {
                    versions.push(ver.clone());
                }
            }

            versions.sort_by_key(|v| v.to_string());
            versions.dedup();

            if versions.len() == 1 {
                let ver = versions[0].clone();
                merged
                    .active_variables
                    .insert(var_name.clone(), ver.clone());
                for pred in &preds {
                    if let Some(ty) = pred.variable_types.get(&ver.to_string()) {
                        merged.variable_types.insert(ver.to_string(), ty.clone());
                        break;
                    }
                }
            } else {
                let amb = SSAVariable::Ambiguous(var_name.clone());
                merged.active_variables.insert(var_name.clone(), amb);
            }
        }

        merged
    }

    fn propagate_node(
        &mut self,
        u: usize,
        start_state: &SSANodeState,
    ) -> (Vec<SSANodeState>, SSANodeState) {
        let mut current_state = start_state.clone();
        let mut statement_states = Vec::new();

        if let Some(node) = self.graph.nodes.get(&u) {
            for stmt in &node.statements {
                let mut declared_in_block = Vec::new();
                self.propagate_statement(stmt, &mut current_state, &mut declared_in_block);
                statement_states.push(current_state.clone());
            }
        }

        (statement_states, current_state)
    }

    fn propagate_statement(
        &mut self,
        stmt: &StatementNode,
        current_state: &mut SSANodeState,
        declared_in_block: &mut Vec<String>,
    ) {
        match &stmt.kind {
            StatementKind::Let {
                name, initializer, ..
            } => {
                let next_ver = self.version_counters.entry(name.clone()).or_insert(0);
                *next_ver += 1;
                let ver_num = *next_ver;
                let ssa_var = SSAVariable::Versioned {
                    name: name.clone(),
                    version: ver_num,
                };

                let mut inference_scope = InferenceScope::new();
                for (var_name, active_var) in &current_state.active_variables {
                    if let Some(ty) = current_state.variable_types.get(&active_var.to_string()) {
                        inference_scope.insert(var_name.clone(), ty.clone());
                    }
                }

                let engine = TypeInferenceEngine::new(self.registry, &inference_scope);
                if let InferenceResult::Ok(type_ref) = engine.infer(initializer) {
                    current_state
                        .variable_types
                        .insert(ssa_var.to_string(), type_ref);
                }

                current_state.active_variables.insert(name.clone(), ssa_var);
                declared_in_block.push(name.clone());
            }
            StatementKind::Expr(expr) | StatementKind::Semi(expr) => {
                if let ExpressionKind::Assign { left, right } = &expr.kind {
                    if let ExpressionKind::Identifier(name) = &left.kind {
                        if current_state.active_variables.contains_key(name) {
                            let next_ver = self.version_counters.entry(name.clone()).or_insert(0);
                            *next_ver += 1;
                            let ver_num = *next_ver;
                            let ssa_var = SSAVariable::Versioned {
                                name: name.clone(),
                                version: ver_num,
                            };

                            let mut inference_scope = InferenceScope::new();
                            for (var_name, active_var) in &current_state.active_variables {
                                if let Some(ty) =
                                    current_state.variable_types.get(&active_var.to_string())
                                {
                                    inference_scope.insert(var_name.clone(), ty.clone());
                                }
                            }

                            let engine = TypeInferenceEngine::new(self.registry, &inference_scope);
                            if let InferenceResult::Ok(type_ref) = engine.infer(right) {
                                current_state
                                    .variable_types
                                    .insert(ssa_var.to_string(), type_ref);
                            }

                            current_state.active_variables.insert(name.clone(), ssa_var);
                        }
                    }
                }
            }
            StatementKind::Block(inner_stmts) => {
                let mut inner_declared = Vec::new();
                let mut before_block_versions = HashMap::new();
                for (name, val) in &current_state.active_variables {
                    before_block_versions.insert(name.clone(), val.clone());
                }

                for inner_stmt in inner_stmts {
                    self.propagate_statement(inner_stmt, current_state, &mut inner_declared);
                }

                for name in inner_declared {
                    if let Some(old_val) = before_block_versions.get(&name) {
                        current_state.active_variables.insert(name, old_val.clone());
                    } else {
                        current_state.active_variables.remove(&name);
                    }
                }
            }
            _ => {}
        }
    }

    /// Query the inferred type of a variable at the end of a CFG node.
    pub fn get_variable_type(&self, node_id: usize, name: &str) -> InferenceResult {
        if let Some(node_info) = self.node_states.get(&node_id) {
            if let Some(ssa_var) = node_info.end_state.active_variables.get(name) {
                match ssa_var {
                    SSAVariable::Versioned { .. } => {
                        if let Some(ty) =
                            node_info.end_state.variable_types.get(&ssa_var.to_string())
                        {
                            InferenceResult::Ok(ty.clone())
                        } else {
                            InferenceResult::Inconclusive(InconclusiveReason::UnresolvedIdentifier(
                                name.to_string(),
                            ))
                        }
                    }
                    SSAVariable::Ambiguous(_) => InferenceResult::Inconclusive(
                        InconclusiveReason::AmbiguousType(name.to_string()),
                    ),
                }
            } else {
                InferenceResult::Inconclusive(InconclusiveReason::UnresolvedIdentifier(
                    name.to_string(),
                ))
            }
        } else {
            InferenceResult::Inconclusive(InconclusiveReason::UnresolvedIdentifier(
                name.to_string(),
            ))
        }
    }
}
