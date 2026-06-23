use parser_v2::ast::{ExpressionKind, ExpressionNode, StatementKind, StatementNode};
use parser_v2::cfg::{
    CFGNode, ControlFlowGraph, FactConfidence, FactExpression, FactProvenance, GuardFact,
    GuardTarget, InstructionAnalysisContext, NodeSSAInfo, SSANodeState, SSAVariable, SSAVersionId,
    SolanaProperty, SymbolId,
};
use parser_v2::rules::{DominanceChecker, ArbitraryCpiTargetRule, RuleEngine, SymbolResolver};
use std::collections::HashMap;

#[test]
fn test_arbitrary_cpi_target_rule() {
    let token_prog_symbol = SymbolId(1);

    // Unsafe CFG: CPI call using token_prog directly without validation
    // Statement 1: invoke(&ix, &[token_prog])
    let invoke_stmt = StatementNode {
        kind: StatementKind::Semi(ExpressionNode {
            kind: ExpressionKind::MethodCall {
                object: Box::new(ExpressionNode {
                    kind: ExpressionKind::Unresolved,
                }),
                method: "invoke".to_string(),
                arguments: vec![
                    ExpressionNode {
                        kind: ExpressionKind::Identifier("ix".to_string()),
                    },
                    ExpressionNode {
                        kind: ExpressionKind::Identifier("token_program".to_string()),
                    },
                ],
            },
        }),
        line_number: 10,
    };

    let mut nodes = HashMap::new();
    nodes.insert(
        0,
        CFGNode {
            id: 0,
            statements: vec![invoke_stmt],
        },
    );

    let mut ssa_states = HashMap::new();
    let mut active_variables = HashMap::new();
    active_variables.insert(
        "token_program".to_string(),
        SSAVariable::Versioned {
            name: "token_program".to_string(),
            version: 1,
        },
    );

    let stmt_state = SSANodeState {
        active_variables,
        variable_types: HashMap::new(),
    };

    ssa_states.insert(
        0,
        NodeSSAInfo {
            start_state: stmt_state.clone(),
            statement_states: vec![stmt_state.clone()],
            end_state: stmt_state.clone(),
        },
    );

    let cfg = ControlFlowGraph {
        nodes,
        edges: Vec::new(),
        entry_node: 0,
        exit_nodes: vec![0],
        boundary_warnings: Vec::new(),
        ssa_states,
    };

    let guard_facts = vec![(
        GuardFact::Owner {
            account: GuardTarget::Account(token_prog_symbol),
            expected_owner: FactExpression::Literal("program_id".to_string()),
        },
        FactProvenance {
            source_file: "lib.rs".to_string(),
            line_number: 1,
            column_number: 1,
            framework: "Anchor".to_string(),
            confidence_level: FactConfidence::Declared,
            node_id: None,
            statement_index: None,
        },
    )];

    let mut symbol_table = HashMap::new();
    symbol_table.insert("token_program".to_string(), token_prog_symbol);

    let context = InstructionAnalysisContext {
        name: "test_instruction".to_string(),
        guard_facts,
        cfg,
        symbol_table,
        file_path: "lib.rs".to_string(),
        context_var_name: "ctx".to_string(),
    };

    let mut engine = RuleEngine::new();
    engine.register_rule(Box::new(ArbitraryCpiTargetRule));

    let analysis_context = parser_v2::rules::AnalysisContext {
        program_metadata: parser_v2::rules::ProgramMetadata {
            name: "test_program".to_string(),
            address: None,
        },
        idl_metadata: None,
        ast_graph: parser_v2::Workspace::new(),
        instruction_context: context,
        rule_registry: Vec::new(),
    };

    let diagnostics = engine.run_all(&analysis_context);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].target_symbol, token_prog_symbol);
    assert_eq!(diagnostics[0].rule_id, "EPIC-SEC-005");
}
