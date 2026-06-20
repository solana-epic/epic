use parser_v2::cfg::{CFGBoundaryWarning, CFGBuilder};

#[test]
fn test_cfg_sequential_statements() {
    let source = r#"{
        let x = 5;
        let y = 10;
        let z = x + y;
    }"#;

    let block = syn::parse_str::<syn::Block>(source).unwrap();
    let mut builder = CFGBuilder::new();
    let end_node = builder.compile_statements(&block.stmts, 0).unwrap();

    assert_eq!(end_node, 0); // Sequential block stays in current node
    let node = builder.graph.nodes.get(&0).unwrap();
    assert_eq!(node.statements.len(), 3);
    assert_eq!(builder.graph.edges.len(), 0);
}

#[test]
fn test_cfg_if_else_branches() {
    let source = r#"{
        if cond {
            let x = 1;
        } else {
            let y = 2;
        }
    }"#;

    let block = syn::parse_str::<syn::Block>(source).unwrap();
    let mut builder = CFGBuilder::new();
    let end_node = builder.compile_statements(&block.stmts, 0).unwrap();

    // 0 is entry/guard. If split creates: then_node(1), else_node(2), merge_node(3)
    assert_eq!(end_node, 3);
    assert_eq!(builder.graph.nodes.len(), 4); // 0, 1, 2, 3
    assert_eq!(builder.graph.edges.len(), 4); // 0->1, 0->2, 1->3, 2->3
}

#[test]
fn test_cfg_try_operator_split() {
    let source = r#"{
        let vault = unpack()?;
        let x = 5;
    }"#;

    let block = syn::parse_str::<syn::Block>(source).unwrap();
    let mut builder = CFGBuilder::new();
    let end_node = builder.compile_statements(&block.stmts, 0).unwrap();

    // 0 has Try, splits to early_return(1) and sequential(2)
    assert_eq!(end_node, 2);
    assert_eq!(builder.graph.nodes.len(), 3); // 0, 1, 2
    assert_eq!(builder.graph.edges.len(), 2); // 0->1 (early exit), 0->2 (success)
    assert!(builder.graph.exit_nodes.contains(&1));
    assert!(builder.graph.exit_nodes.contains(&2));
}

#[test]
fn test_cfg_boundary_warnings_loops_matches() {
    let source = r#"{
        for i in 0..10 {
            let x = i;
        }
        match val {
            1 => {},
            _ => {},
        }
    }"#;

    let block = syn::parse_str::<syn::Block>(source).unwrap();
    let mut builder = CFGBuilder::new();
    let _ = builder.compile_statements(&block.stmts, 0).unwrap();

    assert!(builder
        .graph
        .boundary_warnings
        .contains(&CFGBoundaryWarning::LoopDetected));
    assert!(builder
        .graph
        .boundary_warnings
        .contains(&CFGBoundaryWarning::MatchExpression));
}

#[test]
fn test_cfg_return_termination() {
    let source = r#"{
        if cond {
            return Err(e);
        }
        let y = 10;
    }"#;

    let block = syn::parse_str::<syn::Block>(source).unwrap();
    let mut builder = CFGBuilder::new();
    let end_node = builder.compile_statements(&block.stmts, 0).unwrap();

    // 0 is entry, if split creates then_node(1), else_node(2), merge_node(3).
    // But since then_node (1) contains return, it terminates.
    // So only else_node (2) merges to merge_node (3).
    assert_eq!(end_node, 3);
    assert!(builder.graph.exit_nodes.contains(&1)); // then_node is terminal exit

    // Check edges: no edge from 1 (then_node) to 3 (merge_node)
    for edge in &builder.graph.edges {
        assert_ne!((edge.from, edge.to), (1, 3));
    }
}

#[test]
fn test_cfg_multiple_try_operators() {
    let source = r#"{
        let res = f()? + g()?;
    }"#;

    let block = syn::parse_str::<syn::Block>(source).unwrap();
    let mut builder = CFGBuilder::new();
    let end_node = builder.compile_statements(&block.stmts, 0).unwrap();

    // 2 try operators should trigger 2 separate splits.
    // 0 has first try -> exit(1) and seq(2)
    // 2 has second try -> exit(3) and seq(4)
    assert_eq!(end_node, 4);
    assert_eq!(builder.graph.nodes.len(), 5); // 0, 1, 2, 3, 4
    assert!(builder.graph.exit_nodes.contains(&1));
    assert!(builder.graph.exit_nodes.contains(&3));
    assert!(builder.graph.exit_nodes.contains(&4));
}
