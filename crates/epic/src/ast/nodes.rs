use crate::types::TypeRef;
use serde::{Deserialize, Serialize};

/// Represents a single analyzed Rust function (e.g. an instruction handler).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionNode {
    pub name: String,
    pub signature: Vec<ParameterNode>,
    pub body: Vec<StatementNode>,
}

/// Represents an input parameter of a function signature.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParameterNode {
    pub name: String,
    pub type_ref: TypeRef,
}

/// Represents a single code statement inside a function body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatementNode {
    pub kind: StatementKind,
    pub line_number: usize,
}

/// Defines the supported categories of statements.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StatementKind {
    /// Variable binding (e.g. `let mut auth = admin;`)
    Let {
        name: String,
        initializer: ExpressionNode,
        type_annotation: Option<TypeRef>,
        is_mutable: bool,
    },
    /// Expression without trailing semicolon
    Expr(ExpressionNode),
    /// Expression with trailing semicolon
    Semi(ExpressionNode),
    /// Macro calls (e.g. `require!`, `msg!`)
    MacroCall { name: String, raw_args: String },
    /// A nested block of statements (e.g. `{ let x = 1; }`)
    Block(Vec<StatementNode>),
}

/// Represents an expression evaluating to a value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExpressionNode {
    pub kind: ExpressionKind,
}

/// Defines the supported categories of expressions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExpressionKind {
    /// Scalar identifiers (e.g. `user`)
    Identifier(String),
    /// Direct literal values (e.g. `100`, `true`)
    Literal(String),
    /// Field accesses (e.g. `ctx.accounts`)
    FieldAccess {
        object: Box<ExpressionNode>,
        field: String,
    },
    /// Method call invocations (e.g. `token.borrow_mut()`)
    MethodCall {
        object: Box<ExpressionNode>,
        method: String,
        arguments: Vec<ExpressionNode>,
    },
    /// Logical or mathematical operations (e.g. `x == y`)
    BinaryOp {
        op: String,
        lhs: Box<ExpressionNode>,
        rhs: Box<ExpressionNode>,
    },
    /// Address-of borrow operations (e.g. `&mut user`)
    Reference {
        expression: Box<ExpressionNode>,
        is_mutable: bool,
    },
    /// De-referencing values (e.g. `*ptr`)
    Dereference(Box<ExpressionNode>),
    /// Implicit error return expression mapping (`?`)
    Try(Box<ExpressionNode>),
    /// Variable assignment (e.g. `x = y`)
    Assign {
        left: Box<ExpressionNode>,
        right: Box<ExpressionNode>,
    },
    /// Fallback for unsupported or complex expression constructs
    Unresolved,
}
