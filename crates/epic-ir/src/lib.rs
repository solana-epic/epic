use serde::{Deserialize, Serialize};

/// The root of an intermediate representation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IRModule {
    pub name: String,
    pub functions: Vec<IRFunction>,
    pub state_objects: Vec<IRStateObject>,
}

/// A parsed, chain-agnostic function representation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IRFunction {
    pub name: String,
    pub signature: Vec<IRVariable>,
    pub blocks: Vec<IRBlock>,
    pub body: Vec<IRInstruction>,
    pub location: Option<IRDiagnosticLocation>,
}

/// A variable definition, used in signatures or local bindings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IRVariable {
    pub name: String,
    pub ir_type: IRType,
    pub is_mutable: bool,
}

/// Abstract types supported by the IR.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IRType {
    /// E.g., u8, bool
    Primitive(String),
    /// Account, address, or pubkey representation
    Address,
    /// UTF-8 string
    String,
    /// An array/vector of items
    Collection(Box<IRType>),
    /// Domain-specific structured type
    Struct(String),
    /// Unresolved or unknown
    Unknown,
}

/// Basic blocks used by CFG generation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IRBlock {
    pub id: usize,
    pub instructions: Vec<IRInstruction>,
}

/// A single operation within a block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IRInstruction {
    Assignment(IRAssignment),
    Call(IRCall),
    Branch(IRBranch),
    Loop(IRLoop),
    Return(IRReturn),
    Mutation(IRMutation),
    Guard(IRGuard),
    Expr(IRExpression),
}

/// Evaluates to a value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IRExpression {
    Variable(String),
    Literal(String),
    FieldAccess {
        object: Box<IRExpression>,
        field: String,
    },
    BinaryOp {
        op: String,
        lhs: Box<IRExpression>,
        rhs: Box<IRExpression>,
    },
    Call {
        target: Box<IRExpression>,
        method: Option<String>,
        arguments: Vec<IRExpression>,
    },
    Assign {
        left: Box<IRExpression>,
        right: Box<IRExpression>,
    },
    Reference {
        expression: Box<IRExpression>,
        is_mutable: bool,
    },
    Dereference(Box<IRExpression>),
    Try(Box<IRExpression>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IRAssignment {
    pub target: String,
    pub value: IRExpression,
    pub is_declaration: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IRCall {
    pub target: String,
    pub arguments: Vec<IRExpression>,
    pub returns: Option<String>, // Variable to bind the result to
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IRBranch {
    pub condition: IRExpression,
    pub then_block_id: usize,
    pub else_block_id: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IRLoop {
    pub condition: Option<IRExpression>,
    pub body_block_id: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IRReturn {
    pub value: Option<IRExpression>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IRMutation {
    pub target: String,
    pub value: IRExpression,
}

/// Represents structural state definitions, e.g. Anchor accounts or Solidity structs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IRStateObject {
    pub name: String,
    pub fields: Vec<IRVariable>,
    pub constraints: Vec<IRConstraint>,
}

/// High-level constraints evaluated by the rules engine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IRConstraint {
    Authentication(IRAuthentication),
    Ownership {
        target: String,
        expected_owner: String,
    },
    Custom {
        name: String,
        value: String,
    },
}

/// Represents a signature check or caller authorization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IRAuthentication {
    pub actor: String,
}

/// Imperative runtime checks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IRGuard {
    pub condition: IRExpression,
}

/// Represents the precise source location for diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IRDiagnosticLocation {
    pub file: String,
    pub line: usize,
    pub column: usize,
}
