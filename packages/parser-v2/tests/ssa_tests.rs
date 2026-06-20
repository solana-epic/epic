use parser_v2::ast::{InconclusiveReason, InferenceResult, ParameterNode};
use parser_v2::cfg::{CFGBuilder, SSAComputer, SSAVariable};
use parser_v2::types::{FieldDef, StructDef, TypeDef, TypeRef, TypeRegistry};

#[test]
fn test_ssa_reassignment() {
    let source = r#"{
        let mut authority = admin;
        authority = user;
    }"#;

    let block = syn::parse_str::<syn::Block>(source).unwrap();
    let mut builder = CFGBuilder::new();
    let _ = builder.compile_statements(&block.stmts, 0).unwrap();

    let registry = TypeRegistry::new();
    // Setup types for admin and user
    let mut signature = Vec::new();
    signature.push(ParameterNode {
        name: "admin".to_string(),
        type_ref: TypeRef::Pubkey,
    });
    signature.push(ParameterNode {
        name: "user".to_string(),
        type_ref: TypeRef::Primitive("u64".to_string()),
    });

    let mut computer = SSAComputer::new(&registry, &builder.graph);
    let ssa_states = computer.compute(&signature);

    // Node 0 has the statements
    let node_info = ssa_states.get(&0).unwrap();

    // Statements:
    // 0: let mut authority = admin;
    // 1: authority = user;
    assert_eq!(node_info.statement_states.len(), 2);

    // After statement 0: active authority should be authority#1, type Pubkey
    let state_0 = &node_info.statement_states[0];
    assert_eq!(
        state_0.active_variables.get("authority"),
        Some(&SSAVariable::Versioned {
            name: "authority".to_string(),
            version: 1
        })
    );
    assert_eq!(
        state_0.variable_types.get("authority#1"),
        Some(&TypeRef::Pubkey)
    );

    // After statement 1: active authority should be authority#2, type u64
    let state_1 = &node_info.statement_states[1];
    assert_eq!(
        state_1.active_variables.get("authority"),
        Some(&SSAVariable::Versioned {
            name: "authority".to_string(),
            version: 2
        })
    );
    assert_eq!(
        state_1.variable_types.get("authority#2"),
        Some(&TypeRef::Primitive("u64".to_string()))
    );

    // Query helper method
    assert_eq!(
        computer.get_variable_type(0, "authority"),
        InferenceResult::Ok(TypeRef::Primitive("u64".to_string()))
    );
}

#[test]
fn test_ssa_shadowing() {
    let source = r#"{
        let x = admin;
        let x = user;
    }"#;

    let block = syn::parse_str::<syn::Block>(source).unwrap();
    let mut builder = CFGBuilder::new();
    let _ = builder.compile_statements(&block.stmts, 0).unwrap();

    let registry = TypeRegistry::new();
    let signature = vec![
        ParameterNode {
            name: "admin".to_string(),
            type_ref: TypeRef::Pubkey,
        },
        ParameterNode {
            name: "user".to_string(),
            type_ref: TypeRef::Primitive("bool".to_string()),
        },
    ];

    let mut computer = SSAComputer::new(&registry, &builder.graph);
    let ssa_states = computer.compute(&signature);

    let node_info = ssa_states.get(&0).unwrap();

    // After statement 0: x#1 (Pubkey)
    let state_0 = &node_info.statement_states[0];
    assert_eq!(
        state_0.active_variables.get("x"),
        Some(&SSAVariable::Versioned {
            name: "x".to_string(),
            version: 1
        })
    );
    assert_eq!(state_0.variable_types.get("x#1"), Some(&TypeRef::Pubkey));

    // After statement 1: x#2 (bool)
    let state_1 = &node_info.statement_states[1];
    assert_eq!(
        state_1.active_variables.get("x"),
        Some(&SSAVariable::Versioned {
            name: "x".to_string(),
            version: 2
        })
    );
    assert_eq!(
        state_1.variable_types.get("x#2"),
        Some(&TypeRef::Primitive("bool".to_string()))
    );
}

#[test]
fn test_ssa_nested_scopes() {
    let source = r#"{
        let mut x = admin;
        {
            let mut x = user;
            x = flag;
        }
        x = val;
    }"#;

    let block = syn::parse_str::<syn::Block>(source).unwrap();
    let mut builder = CFGBuilder::new();
    let _ = builder.compile_statements(&block.stmts, 0).unwrap();

    let registry = TypeRegistry::new();
    let signature = vec![
        ParameterNode {
            name: "admin".to_string(),
            type_ref: TypeRef::Pubkey,
        },
        ParameterNode {
            name: "user".to_string(),
            type_ref: TypeRef::Primitive("u64".to_string()),
        },
        ParameterNode {
            name: "flag".to_string(),
            type_ref: TypeRef::Primitive("bool".to_string()),
        },
        ParameterNode {
            name: "val".to_string(),
            type_ref: TypeRef::Primitive("i32".to_string()),
        },
    ];

    let mut computer = SSAComputer::new(&registry, &builder.graph);
    let ssa_states = computer.compute(&signature);

    let node_info = ssa_states.get(&0).unwrap();

    // Statements inside node 0:
    // 0: let mut x = admin;
    // 1: Block({ let mut x = user; x = flag; })
    // 2: x = val;
    assert_eq!(node_info.statement_states.len(), 3);

    // After Statement 0: x#1 (Pubkey)
    assert_eq!(
        node_info.statement_states[0].active_variables.get("x"),
        Some(&SSAVariable::Versioned {
            name: "x".to_string(),
            version: 1
        })
    );
    assert_eq!(
        node_info.statement_states[0].variable_types.get("x#1"),
        Some(&TypeRef::Pubkey)
    );

    // After Statement 1 (nested block exits): x shadows pop, active x goes back to x#1.
    // However, since it is x#1, we didn't reassign it inside the block (we reassigned inner x#2 to x#3).
    // So the active version should be x#1.
    assert_eq!(
        node_info.statement_states[1].active_variables.get("x"),
        Some(&SSAVariable::Versioned {
            name: "x".to_string(),
            version: 1
        })
    );

    // After Statement 2: x is reassigned to val, version becomes x#4 (since x#2 and x#3 were used inside).
    assert_eq!(
        node_info.statement_states[2].active_variables.get("x"),
        Some(&SSAVariable::Versioned {
            name: "x".to_string(),
            version: 4
        })
    );
    assert_eq!(
        node_info.statement_states[2].variable_types.get("x#4"),
        Some(&TypeRef::Primitive("i32".to_string()))
    );
}

#[test]
fn test_ssa_type_preservation() {
    let mut registry = TypeRegistry::new();
    registry.insert(
        "VaultState".to_string(),
        TypeDef::Struct(StructDef {
            name: "VaultState".to_string(),
            is_account: true,
            fields: vec![FieldDef {
                name: "owner".to_string(),
                type_ref: TypeRef::Pubkey,
                attrs: vec![],
            }],
            attrs: vec![],
        }),
    );

    let source = r#"{
        let vault = my_vault;
    }"#;

    let block = syn::parse_str::<syn::Block>(source).unwrap();
    let mut builder = CFGBuilder::new();
    let _ = builder.compile_statements(&block.stmts, 0).unwrap();

    let signature = vec![ParameterNode {
        name: "my_vault".to_string(),
        type_ref: TypeRef::Custom("VaultState".to_string()),
    }];

    let mut computer = SSAComputer::new(&registry, &builder.graph);
    let ssa_states = computer.compute(&signature);

    let node_info = ssa_states.get(&0).unwrap();
    let active_vault = node_info.statement_states[0]
        .active_variables
        .get("vault")
        .unwrap();
    assert_eq!(active_vault.to_string(), "vault#1");
    assert_eq!(
        node_info.statement_states[0].variable_types.get("vault#1"),
        Some(&TypeRef::Custom("VaultState".to_string()))
    );
}

#[test]
fn test_ssa_ambiguous_alias_detection() {
    let source = r#"{
        let mut x = admin;
        if cond {
            x = user;
        } else {
            x = flag;
        }
        let y = x;
    }"#;

    let block = syn::parse_str::<syn::Block>(source).unwrap();
    let mut builder = CFGBuilder::new();
    let end_node = builder.compile_statements(&block.stmts, 0).unwrap();

    // 0 has first assignment and split.
    // 0 -> 1 (then, x = user)
    // 0 -> 2 (else, x = flag)
    // 1 -> 3 (merge)
    // 2 -> 3 (merge, has let y = x)
    assert_eq!(end_node, 3);

    let registry = TypeRegistry::new();
    let signature = vec![
        ParameterNode {
            name: "admin".to_string(),
            type_ref: TypeRef::Pubkey,
        },
        ParameterNode {
            name: "user".to_string(),
            type_ref: TypeRef::Primitive("u64".to_string()),
        },
        ParameterNode {
            name: "flag".to_string(),
            type_ref: TypeRef::Primitive("bool".to_string()),
        },
    ];

    let mut computer = SSAComputer::new(&registry, &builder.graph);
    let ssa_states = computer.compute(&signature);

    // Merge node 3
    let merge_info = ssa_states.get(&3).unwrap();

    // Before any statements in merge node:
    // x should be SSAVariable::Ambiguous because it was reassigned differently in branches
    assert_eq!(
        merge_info.start_state.active_variables.get("x"),
        Some(&SSAVariable::Ambiguous("x".to_string()))
    );

    // The statement is `let y = x;`.
    // Since x is ambiguous, y's inferred type should be inconclusive.
    let after_y = &merge_info.statement_states[0];
    assert_eq!(
        after_y.active_variables.get("y"),
        Some(&SSAVariable::Versioned {
            name: "y".to_string(),
            version: 1
        })
    );
    // y#1 should NOT have a type in variable_types (because x was ambiguous)
    assert!(after_y.variable_types.get("y#1").is_none());

    // Query helper should return Inconclusive(AmbiguousType)
    assert_eq!(
        computer.get_variable_type(3, "x"),
        InferenceResult::Inconclusive(InconclusiveReason::AmbiguousType("x".to_string()))
    );
}
