use epic::ast::{
    ExpressionKind, ExpressionNode, FunctionNode, ParameterNode, StatementKind, StatementNode,
};
use epic::types::TypeRef;

#[test]
fn test_ast_node_structural_equivalence() {
    let param = ParameterNode {
        name: "ctx".to_string(),
        type_ref: TypeRef::Custom("Context".to_string()),
    };

    let initializer = ExpressionNode {
        kind: ExpressionKind::FieldAccess {
            object: Box::new(ExpressionNode {
                kind: ExpressionKind::Identifier("ctx".to_string()),
            }),
            field: "accounts".to_string(),
        },
    };

    let stmt = StatementNode {
        kind: StatementKind::Let {
            name: "auth".to_string(),
            initializer,
            type_annotation: None,
            is_mutable: false,
        },
        line_number: 12,
    };

    let func = FunctionNode {
        name: "update_state".to_string(),
        signature: vec![param],
        body: vec![stmt],
    };

    assert_eq!(func.name, "update_state");
    assert_eq!(func.signature.len(), 1);
    assert_eq!(func.body.len(), 1);
}
