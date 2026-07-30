use crate::ast::nodes::{ExpressionKind, ExpressionNode, StatementKind, StatementNode};
use epic_ir::{IRAssignment, IRCall, IRExpression, IRInstruction};
use syn;

pub fn convert_stmt_to_ir(_stmt: &syn::Stmt) -> Vec<IRInstruction> {
    // A simplified conversion for now to get the ball rolling
    vec![]
}

pub fn convert_statement_node_to_ir(stmt: &StatementNode) -> Vec<IRInstruction> {
    // Incremental step: convert from the existing AST intermediate to the new IR
    let mut ir_instructions = Vec::new();

    match &stmt.kind {
        StatementKind::Let {
            name,
            initializer,
            type_annotation: _,
            is_mutable: _,
        } => {
            ir_instructions.push(IRInstruction::Assignment(IRAssignment {
                target: name.clone(),
                value: convert_expr_node_to_ir(initializer),
                is_declaration: true,
            }));
        }
        StatementKind::Expr(expr) | StatementKind::Semi(expr) => {
            ir_instructions.push(IRInstruction::Expr(convert_expr_node_to_ir(expr)));
        }
        StatementKind::MacroCall { name, raw_args } => {
            ir_instructions.push(IRInstruction::Call(IRCall {
                target: name.clone(),
                arguments: vec![IRExpression::Literal(raw_args.clone())],
                returns: None,
            }));
        }
        StatementKind::Block(stmts) => {
            for s in stmts {
                ir_instructions.extend(convert_statement_node_to_ir(s));
            }
        }
    }

    ir_instructions
}

pub fn convert_expr_node_to_ir(expr: &ExpressionNode) -> IRExpression {
    match &expr.kind {
        ExpressionKind::Identifier(name) => IRExpression::Variable(name.clone()),
        ExpressionKind::Literal(val) => IRExpression::Literal(val.clone()),
        ExpressionKind::FieldAccess { object, field } => IRExpression::FieldAccess {
            object: Box::new(convert_expr_node_to_ir(object)),
            field: field.clone(),
        },
        ExpressionKind::MethodCall {
            object,
            method,
            arguments,
        } => {
            let ir_args = arguments.iter().map(convert_expr_node_to_ir).collect();
            IRExpression::Call {
                target: Box::new(convert_expr_node_to_ir(object)),
                method: Some(method.clone()),
                arguments: ir_args,
            }
        }
        ExpressionKind::BinaryOp { op, lhs, rhs } => IRExpression::BinaryOp {
            op: op.clone(),
            lhs: Box::new(convert_expr_node_to_ir(lhs)),
            rhs: Box::new(convert_expr_node_to_ir(rhs)),
        },
        ExpressionKind::Assign { left, right } => IRExpression::Assign {
            left: Box::new(convert_expr_node_to_ir(left)),
            right: Box::new(convert_expr_node_to_ir(right)),
        },
        ExpressionKind::Reference {
            expression,
            is_mutable,
        } => IRExpression::Reference {
            expression: Box::new(convert_expr_node_to_ir(expression)),
            is_mutable: *is_mutable,
        },
        ExpressionKind::Dereference(inner) => {
            IRExpression::Dereference(Box::new(convert_expr_node_to_ir(inner)))
        }
        ExpressionKind::Try(inner) => IRExpression::Try(Box::new(convert_expr_node_to_ir(inner))),
        _ => IRExpression::Literal("unresolved".to_string()),
    }
}
