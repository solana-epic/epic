use parser_v2::ast::{ExpressionKind, ExpressionNode, StatementKind, StatementNode};
use parser_v2::cfg::{
    CFGNode, ControlFlowGraph, FactConfidence, FactExpression, FactProvenance, GuardFact,
    GuardTarget, InstructionAnalysisContext, NodeSSAInfo, SSANodeState, SSAVariable, SSAVersionId,
    SolanaProperty, SymbolId,
};
use parser_v2::rules::{DominanceChecker, OwnerValidationRule, RuleEngine, SymbolResolver};
use parser_v2::Rule;
use std::collections::HashMap;

#[test]
fn test_owner_validation_safe_vs_unsafe() {
    // 1. Setup variables
    let vault_symbol = SymbolId(1);
    let unchecked_symbol = SymbolId(2);

    // 2. Setup CFG with a single block containing statement writes
    // Statement 1: vault = 10;
    // Statement 2: unchecked = 20;
    let write_vault = StatementNode {
        kind: StatementKind::Expr(ExpressionNode {
            kind: ExpressionKind::Assign {
                left: Box::new(ExpressionNode {
                    kind: ExpressionKind::Identifier("vault".to_string()),
                }),
                right: Box::new(ExpressionNode {
                    kind: ExpressionKind::Literal("10".to_string()),
                }),
            },
        }),
        line_number: 10,
    };

    let write_unchecked = StatementNode {
        kind: StatementKind::Expr(ExpressionNode {
            kind: ExpressionKind::Assign {
                left: Box::new(ExpressionNode {
                    kind: ExpressionKind::Identifier("unchecked".to_string()),
                }),
                right: Box::new(ExpressionNode {
                    kind: ExpressionKind::Literal("20".to_string()),
                }),
            },
        }),
        line_number: 11,
    };

    let mut nodes = HashMap::new();
    nodes.insert(
        0,
        CFGNode {
            id: 0,
            statements: vec![write_vault, write_unchecked],
        },
    );

    // Setup active SSA state for statements
    let mut ssa_states = HashMap::new();
    let mut active_variables = HashMap::new();
    active_variables.insert(
        "vault".to_string(),
        SSAVariable::Versioned {
            name: "vault".to_string(),
            version: 1,
        },
    );
    active_variables.insert(
        "unchecked".to_string(),
        SSAVariable::Versioned {
            name: "unchecked".to_string(),
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
            statement_states: vec![stmt_state.clone(), stmt_state.clone()],
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

    // 3. Setup guard facts (vault is checked, unchecked is not)
    let guard_facts = vec![
        (
            GuardFact::Owner {
                account: GuardTarget::Account(vault_symbol),
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
        ),
        (
            GuardFact::KeyRelation {
                account: GuardTarget::Account(unchecked_symbol),
                field: SolanaProperty::IsWritable,
                target: GuardTarget::Literal("true".to_string()),
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
        ),
    ];

    let mut symbol_table = HashMap::new();
    symbol_table.insert("vault".to_string(), vault_symbol);
    symbol_table.insert("unchecked".to_string(), unchecked_symbol);

    let context = InstructionAnalysisContext {
        name: "test_instruction".to_string(),
        guard_facts,
        cfg,
        symbol_table,
        file_path: "lib.rs".to_string(),
        context_var_name: "ctx".to_string(),
    };

    // 4. Register parameters inside resolver
    let mut engine = RuleEngine::new();
    engine.register_rule(Box::new(OwnerValidationRule));

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

    println!("DIAGNOSTICS: {:#?}", diagnostics);

    // Verify results:
    // - Write to `vault` is SAFE (0 findings for symbol 1)
    // - Write to `unchecked` is UNSAFE (1 finding for symbol 2)
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].target_symbol, unchecked_symbol);
    assert_eq!(diagnostics[0].rule_id, "EPIC-SEC-001");
}

#[test]
fn test_owner_validation_wdg_transitive() {
    // Test transitive mutability tracking:
    // let data = unchecked.try_borrow_mut_data();
    // let state = try_from_slice(&data);
    // state = 100; (write to state)
    let unchecked_symbol = SymbolId(1);
    let data_symbol = SymbolId(2);
    let state_symbol = SymbolId(3);

    // Statement 1: let data = unchecked.try_borrow_mut_data();
    let let_data = StatementNode {
        kind: StatementKind::Let {
            name: "data".to_string(),
            initializer: ExpressionNode {
                kind: ExpressionKind::MethodCall {
                    object: Box::new(ExpressionNode {
                        kind: ExpressionKind::Identifier("unchecked".to_string()),
                    }),
                    method: "try_borrow_mut_data".to_string(),
                    arguments: Vec::new(),
                },
            },
            type_annotation: None,
            is_mutable: true,
        },
        line_number: 10,
    };

    // Statement 2: let state = State::try_from_slice(&data);
    let let_state = StatementNode {
        kind: StatementKind::Let {
            name: "state".to_string(),
            initializer: ExpressionNode {
                kind: ExpressionKind::Identifier("data".to_string()),
            },
            type_annotation: None,
            is_mutable: true,
        },
        line_number: 11,
    };

    // Statement 3: state = 100;
    let write_state = StatementNode {
        kind: StatementKind::Expr(ExpressionNode {
            kind: ExpressionKind::Assign {
                left: Box::new(ExpressionNode {
                    kind: ExpressionKind::Identifier("state".to_string()),
                }),
                right: Box::new(ExpressionNode {
                    kind: ExpressionKind::Literal("100".to_string()),
                }),
            },
        }),
        line_number: 12,
    };

    let mut nodes = HashMap::new();
    nodes.insert(
        0,
        CFGNode {
            id: 0,
            statements: vec![let_data, let_state, write_state],
        },
    );

    let mut ssa_states = HashMap::new();
    let mut active_variables = HashMap::new();
    active_variables.insert(
        "unchecked".to_string(),
        SSAVariable::Versioned {
            name: "unchecked".to_string(),
            version: 1,
        },
    );
    active_variables.insert(
        "data".to_string(),
        SSAVariable::Versioned {
            name: "data".to_string(),
            version: 1,
        },
    );
    active_variables.insert(
        "state".to_string(),
        SSAVariable::Versioned {
            name: "state".to_string(),
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
            statement_states: vec![stmt_state.clone(), stmt_state.clone(), stmt_state.clone()],
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

    // unchecked account is raw AccountInfo (no default owner check)
    let guard_facts = vec![(
        // Struct has some other fact (e.g. Signer) but no owner check
        GuardFact::Signer(GuardTarget::Account(unchecked_symbol)),
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
    symbol_table.insert("unchecked".to_string(), unchecked_symbol);
    symbol_table.insert("data".to_string(), data_symbol);
    symbol_table.insert("state".to_string(), state_symbol);

    let context = InstructionAnalysisContext {
        name: "test_instruction".to_string(),
        guard_facts,
        cfg,
        symbol_table,
        file_path: "lib.rs".to_string(),
        context_var_name: "ctx".to_string(),
    };

    let mut engine = RuleEngine::new();
    engine.register_rule(Box::new(OwnerValidationRule));

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

    println!("WDG DIAGNOSTICS: {:#?}", diagnostics);

    // Verify: Write on `state` propagated through WDG back to `unchecked`.
    // Since `unchecked` lacks an owner check, it generates a critical finding.
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].target_symbol, unchecked_symbol);
}

#[test]
fn test_post_cpi_reload_rule() {
    use parser_v2::rules::MissingPostCpiReloadRule;

    let vault_symbol = SymbolId(1);

    // Unsafe CFG: CPI call then immediate write
    // Statement 1: CPI (represented as a method call named transfer)
    let cpi_stmt = StatementNode {
        kind: StatementKind::Semi(ExpressionNode {
            kind: ExpressionKind::MethodCall {
                object: Box::new(ExpressionNode {
                    kind: ExpressionKind::Unresolved,
                }),
                method: "token::transfer".to_string(),
                arguments: Vec::new(),
            },
        }),
        line_number: 10,
    };

    // Statement 2: vault.amount = 100; (Access)
    let access_stmt = StatementNode {
        kind: StatementKind::Expr(ExpressionNode {
            kind: ExpressionKind::Assign {
                left: Box::new(ExpressionNode {
                    kind: ExpressionKind::FieldAccess {
                        object: Box::new(ExpressionNode {
                            kind: ExpressionKind::Identifier("vault".to_string()),
                        }),
                        field: "amount".to_string(),
                    },
                }),
                right: Box::new(ExpressionNode {
                    kind: ExpressionKind::Literal("100".to_string()),
                }),
            },
        }),
        line_number: 11,
    };

    let mut nodes = HashMap::new();
    nodes.insert(
        0,
        CFGNode {
            id: 0,
            statements: vec![cpi_stmt.clone(), access_stmt.clone()],
        },
    );

    let mut ssa_states = HashMap::new();
    let mut active_variables = HashMap::new();
    active_variables.insert(
        "vault".to_string(),
        SSAVariable::Versioned {
            name: "vault".to_string(),
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
            statement_states: vec![stmt_state.clone(), stmt_state.clone()],
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
            account: GuardTarget::Account(vault_symbol),
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
    symbol_table.insert("vault".to_string(), vault_symbol);

    let context = InstructionAnalysisContext {
        name: "test_instruction".to_string(),
        guard_facts,
        cfg,
        symbol_table,
        file_path: "lib.rs".to_string(),
        context_var_name: "ctx".to_string(),
    };

    let mut engine = RuleEngine::new();
    engine.register_rule(Box::new(MissingPostCpiReloadRule));

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
    assert_eq!(diagnostics[0].target_symbol, vault_symbol);
    assert_eq!(diagnostics[0].rule_id, "EPIC-SEC-003");
}
