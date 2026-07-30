use crate::ast::generics::unpack_nested_generics;
use crate::ast::nodes::{ExpressionKind, ExpressionNode};
use crate::types::{TypeDef, TypeRef, TypeRegistry};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Enumerates the exact failure causes during expression resolution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InconclusiveReason {
    UnresolvedIdentifier(String),
    AmbiguousType(String),
    UnresolvedField { base_type: String, field: String },
    UnresolvedMethod { base_type: String, method: String },
    UnsupportedConstruct,
}

/// The final terminal state returned by the inference pass.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InferenceResult {
    Ok(TypeRef),
    Inconclusive(InconclusiveReason),
}

/// Scoped variable bindings container supporting shadowed variables.
#[derive(Debug, Clone, Default)]
pub struct InferenceScope {
    pub parent: Option<Box<InferenceScope>>,
    pub bindings: HashMap<String, TypeRef>,
}

impl InferenceScope {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn child(parent: InferenceScope) -> Self {
        Self {
            parent: Some(Box::new(parent)),
            bindings: HashMap::new(),
        }
    }

    pub fn insert(&mut self, name: String, type_ref: TypeRef) {
        self.bindings.insert(name, type_ref);
    }

    pub fn get(&self, name: &str) -> Option<TypeRef> {
        if let Some(type_ref) = self.bindings.get(name) {
            return Some(type_ref.clone());
        }
        if let Some(parent) = &self.parent {
            return parent.get(name);
        }
        None
    }
}

pub enum ResolvedType<'b> {
    Ok(&'b TypeDef),
    Inconclusive(InconclusiveReason),
}

/// Core evaluation engine executing type propagation over AST expressions.
pub struct TypeInferenceEngine<'a> {
    pub registry: &'a TypeRegistry,
    pub scope: &'a InferenceScope,
}

impl<'a> TypeInferenceEngine<'a> {
    pub fn new(registry: &'a TypeRegistry, scope: &'a InferenceScope) -> Self {
        Self { registry, scope }
    }

    pub fn resolve_type_by_name(&self, local_name: &str) -> ResolvedType<'a> {
        if let Some(def) = self.registry.get(local_name) {
            return ResolvedType::Ok(def);
        }

        let suffix = format!("::{}", local_name);
        let matches: Vec<(&String, &TypeDef)> = self
            .registry
            .definitions
            .iter()
            .filter(|(key, _)| key.ends_with(&suffix) || *key == local_name)
            .collect();

        if matches.len() == 1 {
            ResolvedType::Ok(matches[0].1)
        } else if matches.len() > 1 {
            ResolvedType::Inconclusive(InconclusiveReason::AmbiguousType(local_name.to_string()))
        } else {
            ResolvedType::Inconclusive(InconclusiveReason::UnresolvedIdentifier(
                local_name.to_string(),
            ))
        }
    }

    pub fn infer_ir(&self, expr: &epic_ir::IRExpression) -> InferenceResult {
        match expr {
            epic_ir::IRExpression::Variable(name) => {
                if let Some(type_ref) = self.scope.get(name) {
                    InferenceResult::Ok(type_ref)
                } else {
                    InferenceResult::Inconclusive(InconclusiveReason::UnresolvedIdentifier(
                        name.clone(),
                    ))
                }
            }
            epic_ir::IRExpression::Literal(_) => {
                InferenceResult::Inconclusive(InconclusiveReason::UnsupportedConstruct)
            }
            epic_ir::IRExpression::FieldAccess { object, field } => match self.infer_ir(object) {
                InferenceResult::Ok(base_type) => self.resolve_field_of_type(&base_type, field),
                inconclusive => inconclusive,
            },
            _ => InferenceResult::Inconclusive(InconclusiveReason::UnsupportedConstruct),
        }
    }

    pub fn infer(&self, expr: &ExpressionNode) -> InferenceResult {
        match &expr.kind {
            ExpressionKind::Identifier(name) => {
                if let Some(type_ref) = self.scope.get(name) {
                    InferenceResult::Ok(type_ref)
                } else {
                    InferenceResult::Inconclusive(InconclusiveReason::UnresolvedIdentifier(
                        name.clone(),
                    ))
                }
            }
            ExpressionKind::Literal(val) => {
                if val == "true" || val == "false" {
                    InferenceResult::Ok(TypeRef::Primitive("bool".to_string()))
                } else if val.chars().all(|c| c.is_ascii_digit()) {
                    InferenceResult::Ok(TypeRef::Primitive("u64".to_string()))
                } else {
                    InferenceResult::Inconclusive(InconclusiveReason::UnsupportedConstruct)
                }
            }
            ExpressionKind::FieldAccess { object, field } => {
                self.resolve_field_access(object, field)
            }
            ExpressionKind::MethodCall { object, method, .. } => {
                self.resolve_method_call(object, method)
            }
            ExpressionKind::BinaryOp { op, .. } => {
                if op == "==" || op == "!=" || op == "<" || op == ">" {
                    InferenceResult::Ok(TypeRef::Primitive("bool".to_string()))
                } else {
                    InferenceResult::Inconclusive(InconclusiveReason::UnsupportedConstruct)
                }
            }
            ExpressionKind::Reference { expression, .. } => self.infer(expression),
            ExpressionKind::Dereference(expression) => self.infer(expression),
            ExpressionKind::Try(expression) => match self.infer(expression) {
                InferenceResult::Ok(TypeRef::Result(ok_type, _)) => InferenceResult::Ok(*ok_type),
                InferenceResult::Ok(other) => InferenceResult::Ok(other),
                inconclusive => inconclusive,
            },
            ExpressionKind::Assign { left: _, right: _ } => {
                InferenceResult::Ok(TypeRef::Primitive("()".to_string()))
            }
            ExpressionKind::Unresolved => {
                InferenceResult::Inconclusive(InconclusiveReason::UnsupportedConstruct)
            }
        }
    }

    fn resolve_field_access(&self, object: &ExpressionNode, field: &str) -> InferenceResult {
        match self.infer(object) {
            InferenceResult::Ok(base_type) => self.resolve_field_of_type(&base_type, field),
            inconclusive => inconclusive,
        }
    }

    fn resolve_field_of_type(&self, ty: &TypeRef, field: &str) -> InferenceResult {
        match ty {
            TypeRef::Custom(struct_name) => {
                let concrete_name = unpack_nested_generics(struct_name);

                match self.resolve_type_by_name(&concrete_name) {
                    ResolvedType::Ok(TypeDef::Struct(struct_def)) => {
                        for f in &struct_def.fields {
                            if f.name == field {
                                return InferenceResult::Ok(f.type_ref.clone());
                            }
                        }
                        InferenceResult::Inconclusive(InconclusiveReason::UnresolvedField {
                            base_type: struct_name.clone(),
                            field: field.to_string(),
                        })
                    }
                    ResolvedType::Inconclusive(reason) => InferenceResult::Inconclusive(reason),
                    _ => InferenceResult::Inconclusive(InconclusiveReason::UnresolvedField {
                        base_type: struct_name.clone(),
                        field: field.to_string(),
                    }),
                }
            }
            _ => InferenceResult::Inconclusive(InconclusiveReason::UnresolvedField {
                base_type: format!("{:?}", ty),
                field: field.to_string(),
            }),
        }
    }

    fn resolve_method_call(&self, object: &ExpressionNode, method: &str) -> InferenceResult {
        match method {
            "key" | "key_ref" => InferenceResult::Ok(TypeRef::Pubkey),
            "load" => match self.infer(object) {
                InferenceResult::Ok(TypeRef::Custom(struct_name)) => {
                    let concrete_name = unpack_nested_generics(&struct_name);
                    match self.resolve_type_by_name(&concrete_name) {
                        ResolvedType::Ok(_) => {
                            if struct_name.starts_with("AccountLoader") {
                                InferenceResult::Ok(TypeRef::Result(
                                    Box::new(TypeRef::Custom(concrete_name)),
                                    Box::new(TypeRef::Custom("ProgramError".to_string())),
                                ))
                            } else {
                                InferenceResult::Inconclusive(
                                    InconclusiveReason::UnresolvedMethod {
                                        base_type: struct_name,
                                        method: method.to_string(),
                                    },
                                )
                            }
                        }
                        ResolvedType::Inconclusive(reason) => InferenceResult::Inconclusive(reason),
                    }
                }
                _ => InferenceResult::Inconclusive(InconclusiveReason::UnresolvedMethod {
                    base_type: "Unknown".to_string(),
                    method: method.to_string(),
                }),
            },
            _ => {
                let base_name = match self.infer(object) {
                    InferenceResult::Ok(type_ref) => format!("{:?}", type_ref),
                    _ => "Unknown".to_string(),
                };
                InferenceResult::Inconclusive(InconclusiveReason::UnresolvedMethod {
                    base_type: base_name,
                    method: method.to_string(),
                })
            }
        }
    }
}
