//! # Stability: Tier 2
//!
//! Target-independent validation and typed lowering for trait functions.
//!
//! Vexil function bodies are code-generation templates. This module accepts
//! the deliberately small portable subset and produces typed statements that
//! target backends can emit without repeating semantic inference.

use std::collections::HashMap;

use smol_str::SmolStr;
use thiserror::Error;

use crate::ast::{PrimitiveType, SemanticType, SubByteType, TypeExpr};
use crate::ir::{
    BinOp, CompiledSchema, Expr, FnBody, ImplDef, ImplFnDef, MessageDef, ResolvedType, Statement,
    TraitDef, TypeDef, TypeId, UnaryOp, POISON_TYPE_ID,
};

/// A checked portable implementation function.
#[derive(Debug, Clone, PartialEq)]
pub struct PortableFunction {
    /// Source function name.
    pub name: SmolStr,
    /// Checked parameters in source order.
    pub params: Vec<PortableParam>,
    /// `None` represents Vexil's no-value return.
    pub return_type: Option<ResolvedType>,
    /// Checked straight-line body.
    pub statements: Vec<PortableStatement>,
}

/// A checked portable function parameter.
#[derive(Debug, Clone, PartialEq)]
pub struct PortableParam {
    /// Source parameter name.
    pub name: SmolStr,
    /// Substituted concrete parameter type.
    pub ty: ResolvedType,
}

/// A checked portable statement.
#[derive(Debug, Clone, PartialEq)]
pub enum PortableStatement {
    /// Immutable local binding.
    Let {
        /// Local name.
        name: SmolStr,
        /// Inferred or declared concrete type.
        ty: ResolvedType,
        /// Initial value.
        value: PortableExpr,
    },
    /// Function return.
    Return(Option<PortableExpr>),
    /// Mutation of a field on the implicit receiver.
    AssignSelfField {
        /// Source field name.
        field: SmolStr,
        /// Assigned value.
        value: PortableExpr,
    },
}

/// A typed portable expression.
#[derive(Debug, Clone, PartialEq)]
pub struct PortableExpr {
    /// Concrete expression type.
    pub ty: ResolvedType,
    /// Target-independent expression form.
    pub kind: PortableExprKind,
}

/// A target-independent portable expression form.
#[derive(Debug, Clone, PartialEq)]
pub enum PortableExprKind {
    Int(i64),
    UInt(u64),
    Float(f64),
    Bool(bool),
    String(String),
    Local(SmolStr),
    SelfRef,
    SelfField(SmolStr),
    Binary(BinOp, Box<PortableExpr>, Box<PortableExpr>),
    Unary(UnaryOp, Box<PortableExpr>),
}

/// Why a trait implementation cannot be projected portably.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum PortableFunctionError {
    #[error("impl references unknown trait '{trait_name}'")]
    UnknownTrait { trait_name: SmolStr },
    #[error("impl target is not a message")]
    InvalidTarget,
    #[error("impl is missing trait function '{function}'")]
    MissingFunction { function: SmolStr },
    #[error("impl function '{function}' is not declared by its trait")]
    ExtraFunction { function: SmolStr },
    #[error("impl function '{function}' has a duplicate implementation")]
    DuplicateFunction { function: SmolStr },
    #[error("impl function '{function}' signature mismatch: {detail}")]
    SignatureMismatch { function: SmolStr, detail: String },
    #[error("external function '{function}' cannot be generated")]
    ExternalFunction { function: SmolStr },
    #[error("function '{function}' uses unsupported free call '{called}'")]
    UnsupportedCall { function: SmolStr, called: SmolStr },
    #[error("function '{function}' uses unsupported method call '{called}'")]
    UnsupportedMethodCall { function: SmolStr, called: SmolStr },
    #[error("function '{function}' contains an unsupported expression statement")]
    UnsupportedExpressionStatement { function: SmolStr },
    #[error("function '{function}' references unknown local '{name}'")]
    UnknownLocal { function: SmolStr, name: SmolStr },
    #[error("function '{function}' references unknown receiver field '{field}'")]
    UnknownField { function: SmolStr, field: SmolStr },
    #[error("function '{function}' redeclares local '{name}'")]
    DuplicateLocal { function: SmolStr, name: SmolStr },
    #[error("function '{function}' has invalid assignment target")]
    InvalidAssignmentTarget { function: SmolStr },
    #[error(
        "function '{function}' type mismatch in {context}: expected {expected}, found {actual}"
    )]
    TypeMismatch {
        function: SmolStr,
        context: String,
        expected: String,
        actual: String,
    },
    #[error("function '{function}' uses operator '{operator}' with unsupported type {actual}")]
    InvalidOperator {
        function: SmolStr,
        operator: String,
        actual: String,
    },
    #[error("function '{function}' has an invalid return: {detail}")]
    ReturnMismatch { function: SmolStr, detail: String },
    #[error("function '{function}' contains statements after return")]
    StatementsAfterReturn { function: SmolStr },
    #[error("function '{function}' contains unresolved type information")]
    UnresolvedType { function: SmolStr },
}

/// Validate and lower all functions in one trait implementation.
pub fn project_impl(
    compiled: &CompiledSchema,
    impl_def: &ImplDef,
) -> Result<Vec<PortableFunction>, PortableFunctionError> {
    let (trait_id, trait_def) = find_trait(compiled, impl_def)?;
    let message = target_message(compiled, impl_def)?;
    let expected = expected_functions(compiled, trait_id, trait_def, impl_def)?;

    let mut actual_by_name: HashMap<&str, &ImplFnDef> = HashMap::new();
    for function in &impl_def.functions {
        if actual_by_name
            .insert(function.name.as_str(), function)
            .is_some()
        {
            return Err(PortableFunctionError::DuplicateFunction {
                function: function.name.clone(),
            });
        }
    }

    for function in &impl_def.functions {
        if !expected
            .iter()
            .any(|expected_function| expected_function.name == function.name)
        {
            return Err(PortableFunctionError::ExtraFunction {
                function: function.name.clone(),
            });
        }
    }

    expected
        .into_iter()
        .map(|expected_function| {
            let Some(actual) = actual_by_name.get(expected_function.name.as_str()) else {
                return Err(PortableFunctionError::MissingFunction {
                    function: expected_function.name,
                });
            };
            check_signature(actual, &expected_function)?;
            project_function(actual, &expected_function, message, impl_def)
        })
        .collect()
}

/// Return the concrete signatures required by one implementation.
pub fn expected_signatures(
    compiled: &CompiledSchema,
    impl_def: &ImplDef,
) -> Result<Vec<PortableSignature>, PortableFunctionError> {
    let (trait_id, trait_def) = find_trait(compiled, impl_def)?;
    expected_functions(compiled, trait_id, trait_def, impl_def)
}

/// A trait function signature after generic substitution.
#[derive(Debug, Clone, PartialEq)]
pub struct PortableSignature {
    pub name: SmolStr,
    pub params: Vec<PortableParam>,
    pub return_type: Option<ResolvedType>,
}

/// A source-level trait signature for target declaration emission.
#[derive(Debug, Clone, PartialEq)]
pub struct PortableTraitSignature {
    pub name: SmolStr,
    pub params: Vec<PortableTraitParam>,
    pub return_type: Option<TypeExpr>,
}

/// A source-level trait parameter for target declaration emission.
#[derive(Debug, Clone, PartialEq)]
pub struct PortableTraitParam {
    pub name: SmolStr,
    pub ty: TypeExpr,
}

/// Return source-faithful signatures for a trait declaration.
pub fn trait_signatures(
    compiled: &CompiledSchema,
    trait_id: TypeId,
) -> Result<Vec<PortableTraitSignature>, PortableFunctionError> {
    let Some(TypeDef::Trait(trait_def)) = compiled.registry.get(trait_id) else {
        return Err(PortableFunctionError::InvalidTarget);
    };
    Ok(trait_def
        .functions
        .iter()
        .map(|function| PortableTraitSignature {
            name: function.name.clone(),
            params: function
                .params
                .iter()
                .map(|parameter| PortableTraitParam {
                    name: parameter.name.clone(),
                    ty: expand_alias_type(&parameter.unresolved_ty, compiled),
                })
                .collect(),
            return_type: compiled
                .registry
                .trait_fn_return_type(trait_id, function.name.as_str())
                .cloned()
                .or_else(|| {
                    function
                        .return_type
                        .as_ref()
                        .and_then(resolved_type_to_source)
                })
                .map(|ty| expand_alias_type(&ty, compiled))
                .and_then(|ty| {
                    if ty == TypeExpr::Primitive(PrimitiveType::Void) {
                        None
                    } else {
                        Some(ty)
                    }
                }),
        })
        .collect())
}

fn expand_alias_type(expression: &TypeExpr, compiled: &CompiledSchema) -> TypeExpr {
    match expression {
        TypeExpr::Named(name) => {
            if let Some(primitive) = compiled.registry.lookup_primitive_alias(name) {
                return TypeExpr::Primitive(primitive);
            }
            let Some(id) = compiled.registry.lookup(name) else {
                return expression.clone();
            };
            let Some(definition) = compiled.registry.get(id) else {
                return expression.clone();
            };
            let concrete = crate::remap::type_def_name(definition);
            if concrete.is_empty() || concrete == name {
                expression.clone()
            } else if let Some((namespace, name)) = concrete.rsplit_once('.') {
                TypeExpr::Qualified(namespace.into(), name.into())
            } else {
                TypeExpr::Named(concrete.into())
            }
        }
        TypeExpr::Generic(name, argument) => {
            let expanded_argument = expand_alias_type(&argument.node, compiled);
            let Some(id) = compiled.registry.lookup(name) else {
                return TypeExpr::Generic(
                    name.clone(),
                    Box::new(crate::span::Spanned::new(expanded_argument, argument.span)),
                );
            };
            let Some(TypeDef::GenericAlias(alias)) = compiled.registry.get(id) else {
                return TypeExpr::Generic(
                    name.clone(),
                    Box::new(crate::span::Spanned::new(expanded_argument, argument.span)),
                );
            };
            let Some(parameter) = alias.type_params.first() else {
                return expression.clone();
            };
            let substituted =
                substitute_source_type(&alias.target_type, parameter, &expanded_argument);
            expand_alias_type(&substituted, compiled)
        }
        TypeExpr::Optional(inner) => TypeExpr::Optional(expand_spanned(inner, compiled)),
        TypeExpr::Array(inner) => TypeExpr::Array(expand_spanned(inner, compiled)),
        TypeExpr::FixedArray(inner, length) => {
            TypeExpr::FixedArray(expand_spanned(inner, compiled), *length)
        }
        TypeExpr::Set(inner) => TypeExpr::Set(expand_spanned(inner, compiled)),
        TypeExpr::Map(key, value) => TypeExpr::Map(
            expand_spanned(key, compiled),
            expand_spanned(value, compiled),
        ),
        TypeExpr::Result(ok, error) => TypeExpr::Result(
            expand_spanned(ok, compiled),
            expand_spanned(error, compiled),
        ),
        TypeExpr::Vec2(inner) => TypeExpr::Vec2(expand_spanned(inner, compiled)),
        TypeExpr::Vec3(inner) => TypeExpr::Vec3(expand_spanned(inner, compiled)),
        TypeExpr::Vec4(inner) => TypeExpr::Vec4(expand_spanned(inner, compiled)),
        TypeExpr::Quat(inner) => TypeExpr::Quat(expand_spanned(inner, compiled)),
        TypeExpr::Mat3(inner) => TypeExpr::Mat3(expand_spanned(inner, compiled)),
        TypeExpr::Mat4(inner) => TypeExpr::Mat4(expand_spanned(inner, compiled)),
        _ => expression.clone(),
    }
}

fn expand_spanned(
    expression: &crate::span::Spanned<TypeExpr>,
    compiled: &CompiledSchema,
) -> Box<crate::span::Spanned<TypeExpr>> {
    Box::new(crate::span::Spanned::new(
        expand_alias_type(&expression.node, compiled),
        expression.span,
    ))
}

fn substitute_source_type(
    expression: &TypeExpr,
    parameter: &SmolStr,
    argument: &TypeExpr,
) -> TypeExpr {
    if let TypeExpr::Named(name) = expression {
        if name == parameter {
            return argument.clone();
        }
    }
    match expression {
        TypeExpr::Optional(inner) => {
            TypeExpr::Optional(substitute_spanned(inner, parameter, argument))
        }
        TypeExpr::Array(inner) => TypeExpr::Array(substitute_spanned(inner, parameter, argument)),
        TypeExpr::FixedArray(inner, length) => {
            TypeExpr::FixedArray(substitute_spanned(inner, parameter, argument), *length)
        }
        TypeExpr::Set(inner) => TypeExpr::Set(substitute_spanned(inner, parameter, argument)),
        TypeExpr::Map(key, value) => TypeExpr::Map(
            substitute_spanned(key, parameter, argument),
            substitute_spanned(value, parameter, argument),
        ),
        TypeExpr::Result(ok, error) => TypeExpr::Result(
            substitute_spanned(ok, parameter, argument),
            substitute_spanned(error, parameter, argument),
        ),
        TypeExpr::Generic(name, inner) => {
            TypeExpr::Generic(name.clone(), substitute_spanned(inner, parameter, argument))
        }
        TypeExpr::Vec2(inner) => TypeExpr::Vec2(substitute_spanned(inner, parameter, argument)),
        TypeExpr::Vec3(inner) => TypeExpr::Vec3(substitute_spanned(inner, parameter, argument)),
        TypeExpr::Vec4(inner) => TypeExpr::Vec4(substitute_spanned(inner, parameter, argument)),
        TypeExpr::Quat(inner) => TypeExpr::Quat(substitute_spanned(inner, parameter, argument)),
        TypeExpr::Mat3(inner) => TypeExpr::Mat3(substitute_spanned(inner, parameter, argument)),
        TypeExpr::Mat4(inner) => TypeExpr::Mat4(substitute_spanned(inner, parameter, argument)),
        _ => expression.clone(),
    }
}

fn substitute_spanned(
    expression: &crate::span::Spanned<TypeExpr>,
    parameter: &SmolStr,
    argument: &TypeExpr,
) -> Box<crate::span::Spanned<TypeExpr>> {
    Box::new(crate::span::Spanned::new(
        substitute_source_type(&expression.node, parameter, argument),
        expression.span,
    ))
}

fn find_trait<'a>(
    compiled: &'a CompiledSchema,
    impl_def: &ImplDef,
) -> Result<(TypeId, &'a TraitDef), PortableFunctionError> {
    compiled
        .registry
        .trait_for_impl(impl_def)
        .ok_or_else(|| PortableFunctionError::UnknownTrait {
            trait_name: impl_def.trait_name.clone(),
        })
}

fn target_message<'a>(
    compiled: &'a CompiledSchema,
    impl_def: &ImplDef,
) -> Result<&'a MessageDef, PortableFunctionError> {
    match &impl_def.target_type {
        ResolvedType::Named(id) => match compiled.registry.get(*id) {
            Some(TypeDef::Message(message)) => Ok(message),
            _ => Err(PortableFunctionError::InvalidTarget),
        },
        _ => Err(PortableFunctionError::InvalidTarget),
    }
}

fn expected_functions(
    compiled: &CompiledSchema,
    trait_id: TypeId,
    trait_def: &TraitDef,
    impl_def: &ImplDef,
) -> Result<Vec<PortableSignature>, PortableFunctionError> {
    if trait_def.type_params.len() != impl_def.type_args.len() {
        return Err(PortableFunctionError::SignatureMismatch {
            function: trait_def.name.clone(),
            detail: format!(
                "trait expects {} type arguments but impl supplies {}",
                trait_def.type_params.len(),
                impl_def.type_args.len()
            ),
        });
    }
    let params: Vec<&str> = trait_def
        .type_params
        .iter()
        .map(|parameter| parameter.name.node.as_str())
        .collect();

    trait_def
        .functions
        .iter()
        .map(|function| {
            let return_type = match compiled
                .registry
                .trait_fn_return_type(trait_id, function.name.as_str())
            {
                Some(expression) => Some(substitute_type(
                    expression,
                    &params,
                    &impl_def.type_args,
                    compiled,
                )),
                None => function.return_type.clone(),
            };
            let signature = PortableSignature {
                name: function.name.clone(),
                params: function
                    .params
                    .iter()
                    .map(|parameter| PortableParam {
                        name: parameter.name.clone(),
                        ty: substitute_type(
                            &parameter.unresolved_ty,
                            &params,
                            &impl_def.type_args,
                            compiled,
                        ),
                    })
                    .collect(),
                return_type: normalize_return(return_type),
            };
            if signature
                .params
                .iter()
                .any(|parameter| contains_poison(&parameter.ty))
                || signature.return_type.as_ref().is_some_and(contains_poison)
            {
                Err(PortableFunctionError::UnresolvedType {
                    function: function.name.clone(),
                })
            } else {
                Ok(signature)
            }
        })
        .collect()
}

fn check_signature(
    actual: &ImplFnDef,
    expected: &PortableSignature,
) -> Result<(), PortableFunctionError> {
    if actual.params.len() != expected.params.len() {
        return Err(PortableFunctionError::SignatureMismatch {
            function: expected.name.clone(),
            detail: format!(
                "expected {} parameters, found {}",
                expected.params.len(),
                actual.params.len()
            ),
        });
    }
    for (actual_parameter, expected_parameter) in actual.params.iter().zip(&expected.params) {
        if actual_parameter.name != expected_parameter.name {
            return Err(PortableFunctionError::SignatureMismatch {
                function: expected.name.clone(),
                detail: format!(
                    "expected parameter '{}', found '{}'",
                    expected_parameter.name, actual_parameter.name
                ),
            });
        }
        if actual_parameter.ty != expected_parameter.ty {
            return Err(PortableFunctionError::SignatureMismatch {
                function: expected.name.clone(),
                detail: format!(
                    "parameter '{}' expects {}, found {}",
                    expected_parameter.name,
                    type_name(&expected_parameter.ty),
                    type_name(&actual_parameter.ty)
                ),
            });
        }
    }
    if normalize_return(actual.return_type.clone()) != expected.return_type {
        return Err(PortableFunctionError::SignatureMismatch {
            function: expected.name.clone(),
            detail: format!(
                "expected return {}, found {}",
                return_name(&expected.return_type),
                return_name(&normalize_return(actual.return_type.clone()))
            ),
        });
    }
    Ok(())
}

fn project_function(
    actual: &ImplFnDef,
    expected: &PortableSignature,
    message: &MessageDef,
    impl_def: &ImplDef,
) -> Result<PortableFunction, PortableFunctionError> {
    let FnBody::Block(statements) = &actual.body else {
        return Err(PortableFunctionError::ExternalFunction {
            function: actual.name.clone(),
        });
    };

    let mut context = FunctionContext {
        function: &actual.name,
        locals: expected
            .params
            .iter()
            .map(|parameter| (parameter.name.clone(), parameter.ty.clone()))
            .collect(),
        fields: message
            .fields
            .iter()
            .map(|field| (field.name.clone(), field.resolved_type.clone()))
            .collect(),
        self_type: impl_def.target_type.clone(),
    };
    let mut portable = Vec::with_capacity(statements.len());
    let mut returned = false;

    for statement in statements {
        if returned {
            return Err(PortableFunctionError::StatementsAfterReturn {
                function: actual.name.clone(),
            });
        }
        match statement {
            Statement::Expr(_) => {
                return Err(PortableFunctionError::UnsupportedExpressionStatement {
                    function: actual.name.clone(),
                });
            }
            Statement::Let { name, ty, value } => {
                if context.locals.contains_key(name) {
                    return Err(PortableFunctionError::DuplicateLocal {
                        function: actual.name.clone(),
                        name: name.clone(),
                    });
                }
                let value = infer_expr(value, ty.as_ref(), "local binding", &context)?;
                let local_type = ty.clone().unwrap_or_else(|| value.ty.clone());
                ensure_same_type(&actual.name, "local binding", &local_type, &value.ty)?;
                context.locals.insert(name.clone(), local_type.clone());
                portable.push(PortableStatement::Let {
                    name: name.clone(),
                    ty: local_type,
                    value,
                });
            }
            Statement::Return(value) => {
                let value = match (&expected.return_type, value) {
                    (None, None) => None,
                    (None, Some(_)) => {
                        return Err(PortableFunctionError::ReturnMismatch {
                            function: actual.name.clone(),
                            detail: "void function cannot return a value".to_string(),
                        });
                    }
                    (Some(_), None) => {
                        return Err(PortableFunctionError::ReturnMismatch {
                            function: actual.name.clone(),
                            detail: "value-returning function requires a return value".to_string(),
                        });
                    }
                    (Some(expected_type), Some(expression)) => {
                        let value =
                            infer_expr(expression, Some(expected_type), "return", &context)?;
                        ensure_same_type(&actual.name, "return", expected_type, &value.ty)?;
                        Some(value)
                    }
                };
                portable.push(PortableStatement::Return(value));
                returned = true;
            }
            Statement::Assign { target, value } => {
                let Expr::FieldAccess(receiver, field) = target else {
                    return Err(PortableFunctionError::InvalidAssignmentTarget {
                        function: actual.name.clone(),
                    });
                };
                if !matches!(receiver.as_ref(), Expr::SelfRef) {
                    return Err(PortableFunctionError::InvalidAssignmentTarget {
                        function: actual.name.clone(),
                    });
                }
                let Some(field_type) = context.fields.get(field) else {
                    return Err(PortableFunctionError::UnknownField {
                        function: actual.name.clone(),
                        field: field.clone(),
                    });
                };
                let value = infer_expr(
                    value,
                    Some(field_type),
                    "receiver field assignment",
                    &context,
                )?;
                ensure_same_type(
                    &actual.name,
                    "receiver field assignment",
                    field_type,
                    &value.ty,
                )?;
                portable.push(PortableStatement::AssignSelfField {
                    field: field.clone(),
                    value,
                });
            }
        }
    }

    if expected.return_type.is_some() && !returned {
        return Err(PortableFunctionError::ReturnMismatch {
            function: actual.name.clone(),
            detail: "value-returning body must end with return".to_string(),
        });
    }

    Ok(PortableFunction {
        name: expected.name.clone(),
        params: expected.params.clone(),
        return_type: expected.return_type.clone(),
        statements: portable,
    })
}

struct FunctionContext<'a> {
    function: &'a SmolStr,
    locals: HashMap<SmolStr, ResolvedType>,
    fields: HashMap<SmolStr, ResolvedType>,
    self_type: ResolvedType,
}

fn infer_expr(
    expression: &Expr,
    expected: Option<&ResolvedType>,
    expected_context: &str,
    context: &FunctionContext<'_>,
) -> Result<PortableExpr, PortableFunctionError> {
    let expression = match expression {
        Expr::Int(value) => {
            let ty = literal_int_type(*value, expected).ok_or_else(|| {
                type_mismatch(
                    context.function,
                    "integer literal",
                    expected,
                    &ResolvedType::Primitive(PrimitiveType::I64),
                )
            })?;
            PortableExpr {
                ty,
                kind: PortableExprKind::Int(*value),
            }
        }
        Expr::UInt(value) => {
            let ty = literal_uint_type(*value, expected).ok_or_else(|| {
                type_mismatch(
                    context.function,
                    "unsigned integer literal",
                    expected,
                    &ResolvedType::Primitive(PrimitiveType::U64),
                )
            })?;
            PortableExpr {
                ty,
                kind: PortableExprKind::UInt(*value),
            }
        }
        Expr::Float(value) => {
            let ty = match expected {
                Some(ResolvedType::Primitive(PrimitiveType::F32)) => {
                    ResolvedType::Primitive(PrimitiveType::F32)
                }
                Some(ResolvedType::Primitive(PrimitiveType::F64)) | None => {
                    ResolvedType::Primitive(PrimitiveType::F64)
                }
                Some(other) => {
                    return Err(type_mismatch(
                        context.function,
                        "float literal",
                        Some(other),
                        &ResolvedType::Primitive(PrimitiveType::F64),
                    ));
                }
            };
            PortableExpr {
                ty,
                kind: PortableExprKind::Float(*value),
            }
        }
        Expr::Bool(value) => PortableExpr {
            ty: ResolvedType::Primitive(PrimitiveType::Bool),
            kind: PortableExprKind::Bool(*value),
        },
        Expr::String(value) => PortableExpr {
            ty: ResolvedType::Semantic(SemanticType::String),
            kind: PortableExprKind::String(value.clone()),
        },
        Expr::Local(name) => {
            let Some(ty) = context.locals.get(name) else {
                return Err(PortableFunctionError::UnknownLocal {
                    function: context.function.clone(),
                    name: name.clone(),
                });
            };
            PortableExpr {
                ty: ty.clone(),
                kind: PortableExprKind::Local(name.clone()),
            }
        }
        Expr::SelfRef => PortableExpr {
            ty: context.self_type.clone(),
            kind: PortableExprKind::SelfRef,
        },
        Expr::FieldAccess(receiver, field) if matches!(receiver.as_ref(), Expr::SelfRef) => {
            let Some(ty) = context.fields.get(field) else {
                return Err(PortableFunctionError::UnknownField {
                    function: context.function.clone(),
                    field: field.clone(),
                });
            };
            PortableExpr {
                ty: ty.clone(),
                kind: PortableExprKind::SelfField(field.clone()),
            }
        }
        Expr::FieldAccess(_, field) => {
            return Err(PortableFunctionError::UnknownField {
                function: context.function.clone(),
                field: field.clone(),
            });
        }
        Expr::Call(name, _) => {
            return Err(PortableFunctionError::UnsupportedCall {
                function: context.function.clone(),
                called: name.clone(),
            });
        }
        Expr::TraitMethodCall { method_name, .. } => {
            return Err(PortableFunctionError::UnsupportedMethodCall {
                function: context.function.clone(),
                called: method_name.clone(),
            });
        }
        Expr::Binary(operator, left, right) => {
            // Arithmetic produces the operand type, so the surrounding
            // expectation can guide otherwise-untyped literals. Comparisons
            // and equality produce bool; using that result expectation for
            // their operands would incorrectly type `1 < 2` as bool.
            let expected_operand = match operator {
                BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div => expected,
                BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => None,
            };
            let hint = expression_hint(left, context)
                .or_else(|| expression_hint(right, context))
                .or(expected_operand);
            let left = infer_expr(left, hint, "binary expression", context)?;
            let right = infer_expr(right, Some(&left.ty), "binary expression", context)?;
            ensure_same_type(context.function, "binary expression", &left.ty, &right.ty)?;
            let result_type = match operator {
                BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div => {
                    if !is_numeric(&left.ty) {
                        return Err(invalid_operator(context.function, *operator, &left.ty));
                    }
                    left.ty.clone()
                }
                BinOp::Eq | BinOp::Ne => {
                    if !is_equality_type(&left.ty) {
                        return Err(invalid_operator(context.function, *operator, &left.ty));
                    }
                    ResolvedType::Primitive(PrimitiveType::Bool)
                }
                BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                    if !is_numeric(&left.ty) {
                        return Err(invalid_operator(context.function, *operator, &left.ty));
                    }
                    ResolvedType::Primitive(PrimitiveType::Bool)
                }
            };
            PortableExpr {
                ty: result_type,
                kind: PortableExprKind::Binary(*operator, Box::new(left), Box::new(right)),
            }
        }
        Expr::Unary(operator, value) => {
            let value = infer_expr(value, expected, "unary expression", context)?;
            let result_type = match operator {
                UnaryOp::Neg if is_signed_numeric(&value.ty) => value.ty.clone(),
                UnaryOp::Not if value.ty == ResolvedType::Primitive(PrimitiveType::Bool) => {
                    ResolvedType::Primitive(PrimitiveType::Bool)
                }
                _ => {
                    return Err(invalid_unary_operator(
                        context.function,
                        *operator,
                        &value.ty,
                    ))
                }
            };
            PortableExpr {
                ty: result_type,
                kind: PortableExprKind::Unary(*operator, Box::new(value)),
            }
        }
    };

    if let Some(expected) = expected {
        ensure_same_type(context.function, expected_context, expected, &expression.ty)?;
    }
    Ok(expression)
}

fn expression_hint<'a>(
    expression: &Expr,
    context: &'a FunctionContext<'_>,
) -> Option<&'a ResolvedType> {
    match expression {
        Expr::Local(name) => context.locals.get(name),
        Expr::SelfRef => Some(&context.self_type),
        Expr::FieldAccess(receiver, field) if matches!(receiver.as_ref(), Expr::SelfRef) => {
            context.fields.get(field)
        }
        _ => None,
    }
}

fn literal_int_type(value: i64, expected: Option<&ResolvedType>) -> Option<ResolvedType> {
    match expected {
        None => Some(ResolvedType::Primitive(PrimitiveType::I64)),
        Some(ty) if int_fits(value, ty) => Some(ty.clone()),
        _ => None,
    }
}

fn literal_uint_type(value: u64, expected: Option<&ResolvedType>) -> Option<ResolvedType> {
    match expected {
        None => Some(ResolvedType::Primitive(PrimitiveType::U64)),
        Some(ty) if uint_fits(value, ty) => Some(ty.clone()),
        _ => None,
    }
}

fn int_fits(value: i64, ty: &ResolvedType) -> bool {
    match ty {
        ResolvedType::Primitive(PrimitiveType::U8) => u8::try_from(value).is_ok(),
        ResolvedType::Primitive(PrimitiveType::U16) => u16::try_from(value).is_ok(),
        ResolvedType::Primitive(PrimitiveType::U32) => u32::try_from(value).is_ok(),
        ResolvedType::Primitive(PrimitiveType::U64) => u64::try_from(value).is_ok(),
        ResolvedType::Primitive(PrimitiveType::I8) => i8::try_from(value).is_ok(),
        ResolvedType::Primitive(PrimitiveType::I16) => i16::try_from(value).is_ok(),
        ResolvedType::Primitive(PrimitiveType::I32 | PrimitiveType::Fixed32) => {
            i32::try_from(value).is_ok()
        }
        ResolvedType::Primitive(PrimitiveType::I64 | PrimitiveType::Fixed64) => true,
        ResolvedType::Primitive(PrimitiveType::F32 | PrimitiveType::F64) => true,
        ResolvedType::SubByte(SubByteType { signed: true, bits }) => {
            let shift = u32::from(*bits).saturating_sub(1);
            let max = (1_i128 << shift) - 1;
            let min = -(1_i128 << shift);
            i128::from(value) >= min && i128::from(value) <= max
        }
        ResolvedType::SubByte(SubByteType {
            signed: false,
            bits,
        }) => {
            let max = (1_u128 << u32::from(*bits)) - 1;
            u128::try_from(value).is_ok_and(|value| value <= max)
        }
        _ => false,
    }
}

fn uint_fits(value: u64, ty: &ResolvedType) -> bool {
    match ty {
        ResolvedType::Primitive(PrimitiveType::U8) => u8::try_from(value).is_ok(),
        ResolvedType::Primitive(PrimitiveType::U16) => u16::try_from(value).is_ok(),
        ResolvedType::Primitive(PrimitiveType::U32) => u32::try_from(value).is_ok(),
        ResolvedType::Primitive(PrimitiveType::U64) => true,
        ResolvedType::Primitive(PrimitiveType::I8) => i8::try_from(value).is_ok(),
        ResolvedType::Primitive(PrimitiveType::I16) => i16::try_from(value).is_ok(),
        ResolvedType::Primitive(PrimitiveType::I32 | PrimitiveType::Fixed32) => {
            i32::try_from(value).is_ok()
        }
        ResolvedType::Primitive(PrimitiveType::I64 | PrimitiveType::Fixed64) => {
            i64::try_from(value).is_ok()
        }
        ResolvedType::Primitive(PrimitiveType::F32 | PrimitiveType::F64) => true,
        ResolvedType::SubByte(SubByteType {
            signed: false,
            bits,
        }) => {
            let max = if *bits == 64 {
                u64::MAX
            } else {
                (1_u64 << bits) - 1
            };
            value <= max
        }
        ResolvedType::SubByte(SubByteType { signed: true, bits }) => {
            let shift = u32::from(*bits).saturating_sub(1);
            value <= ((1_u64 << shift) - 1)
        }
        _ => false,
    }
}

fn ensure_same_type(
    function: &SmolStr,
    context: &str,
    expected: &ResolvedType,
    actual: &ResolvedType,
) -> Result<(), PortableFunctionError> {
    if expected == actual {
        Ok(())
    } else {
        Err(type_mismatch(function, context, Some(expected), actual))
    }
}

fn type_mismatch(
    function: &SmolStr,
    context: &str,
    expected: Option<&ResolvedType>,
    actual: &ResolvedType,
) -> PortableFunctionError {
    PortableFunctionError::TypeMismatch {
        function: function.clone(),
        context: context.to_string(),
        expected: expected
            .map(type_name)
            .unwrap_or_else(|| "<inferred>".to_string()),
        actual: type_name(actual),
    }
}

fn invalid_operator(
    function: &SmolStr,
    operator: BinOp,
    actual: &ResolvedType,
) -> PortableFunctionError {
    PortableFunctionError::InvalidOperator {
        function: function.clone(),
        operator: binary_operator(operator).to_string(),
        actual: type_name(actual),
    }
}

fn invalid_unary_operator(
    function: &SmolStr,
    operator: UnaryOp,
    actual: &ResolvedType,
) -> PortableFunctionError {
    PortableFunctionError::InvalidOperator {
        function: function.clone(),
        operator: match operator {
            UnaryOp::Neg => "-",
            UnaryOp::Not => "!",
        }
        .to_string(),
        actual: type_name(actual),
    }
}

fn binary_operator(operator: BinOp) -> &'static str {
    match operator {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Eq => "==",
        BinOp::Ne => "!=",
        BinOp::Lt => "<",
        BinOp::Le => "<=",
        BinOp::Gt => ">",
        BinOp::Ge => ">=",
    }
}

fn is_numeric(ty: &ResolvedType) -> bool {
    matches!(
        ty,
        ResolvedType::Primitive(
            PrimitiveType::U8
                | PrimitiveType::U16
                | PrimitiveType::U32
                | PrimitiveType::U64
                | PrimitiveType::I8
                | PrimitiveType::I16
                | PrimitiveType::I32
                | PrimitiveType::I64
                | PrimitiveType::F32
                | PrimitiveType::F64
                | PrimitiveType::Fixed32
                | PrimitiveType::Fixed64
        ) | ResolvedType::SubByte(_)
    )
}

fn is_signed_numeric(ty: &ResolvedType) -> bool {
    matches!(
        ty,
        ResolvedType::Primitive(
            PrimitiveType::I8
                | PrimitiveType::I16
                | PrimitiveType::I32
                | PrimitiveType::I64
                | PrimitiveType::F32
                | PrimitiveType::F64
                | PrimitiveType::Fixed32
                | PrimitiveType::Fixed64
        ) | ResolvedType::SubByte(SubByteType { signed: true, .. })
    )
}

fn is_equality_type(ty: &ResolvedType) -> bool {
    is_numeric(ty)
        || matches!(
            ty,
            ResolvedType::Primitive(PrimitiveType::Bool)
                | ResolvedType::Semantic(SemanticType::String)
        )
}

fn normalize_return(return_type: Option<ResolvedType>) -> Option<ResolvedType> {
    match return_type {
        Some(ResolvedType::Primitive(PrimitiveType::Void)) | None => None,
        other => other,
    }
}

fn return_name(return_type: &Option<ResolvedType>) -> String {
    return_type
        .as_ref()
        .map(type_name)
        .unwrap_or_else(|| "void".to_string())
}

fn contains_poison(ty: &ResolvedType) -> bool {
    match ty {
        ResolvedType::Named(id) => *id == POISON_TYPE_ID,
        ResolvedType::Optional(inner)
        | ResolvedType::Array(inner)
        | ResolvedType::FixedArray(inner, _)
        | ResolvedType::Set(inner)
        | ResolvedType::Vec2(inner)
        | ResolvedType::Vec3(inner)
        | ResolvedType::Vec4(inner)
        | ResolvedType::Quat(inner)
        | ResolvedType::Mat3(inner)
        | ResolvedType::Mat4(inner) => contains_poison(inner),
        ResolvedType::Map(key, value) | ResolvedType::Result(key, value) => {
            contains_poison(key) || contains_poison(value)
        }
        _ => false,
    }
}

fn substitute_type(
    expression: &TypeExpr,
    params: &[&str],
    args: &[ResolvedType],
    compiled: &CompiledSchema,
) -> ResolvedType {
    match expression {
        TypeExpr::Named(name) => params
            .iter()
            .position(|parameter| *parameter == name.as_str())
            .and_then(|index| args.get(index))
            .cloned()
            .or_else(|| {
                compiled
                    .registry
                    .lookup_primitive_alias(name)
                    .map(ResolvedType::Primitive)
            })
            .or_else(|| compiled.registry.lookup(name).map(ResolvedType::Named))
            .unwrap_or(ResolvedType::Named(POISON_TYPE_ID)),
        TypeExpr::Qualified(namespace, name) => compiled
            .registry
            .lookup(&format!("{namespace}.{name}"))
            .map(ResolvedType::Named)
            .unwrap_or(ResolvedType::Named(POISON_TYPE_ID)),
        TypeExpr::Primitive(primitive) => ResolvedType::Primitive(*primitive),
        TypeExpr::SubByte(sub_byte) => ResolvedType::SubByte(*sub_byte),
        TypeExpr::Semantic(semantic) => ResolvedType::Semantic(*semantic),
        TypeExpr::Generic(name, argument) => {
            let Some(alias_id) = compiled.registry.lookup(name) else {
                return ResolvedType::Named(POISON_TYPE_ID);
            };
            let Some(TypeDef::GenericAlias(alias)) = compiled.registry.get(alias_id) else {
                return ResolvedType::Named(POISON_TYPE_ID);
            };
            if alias.type_params.len() != 1 {
                return ResolvedType::Named(POISON_TYPE_ID);
            }
            let argument = substitute_type(&argument.node, params, args, compiled);
            let alias_params: Vec<&str> = alias.type_params.iter().map(SmolStr::as_str).collect();
            substitute_type(
                &alias.target_type,
                &alias_params,
                std::slice::from_ref(&argument),
                compiled,
            )
        }
        TypeExpr::Optional(inner) => ResolvedType::Optional(Box::new(substitute_type(
            &inner.node,
            params,
            args,
            compiled,
        ))),
        TypeExpr::Array(inner) => ResolvedType::Array(Box::new(substitute_type(
            &inner.node,
            params,
            args,
            compiled,
        ))),
        TypeExpr::FixedArray(inner, length) => ResolvedType::FixedArray(
            Box::new(substitute_type(&inner.node, params, args, compiled)),
            *length,
        ),
        TypeExpr::Set(inner) => ResolvedType::Set(Box::new(substitute_type(
            &inner.node,
            params,
            args,
            compiled,
        ))),
        TypeExpr::Map(key, value) => ResolvedType::Map(
            Box::new(substitute_type(&key.node, params, args, compiled)),
            Box::new(substitute_type(&value.node, params, args, compiled)),
        ),
        TypeExpr::Result(ok, error) => ResolvedType::Result(
            Box::new(substitute_type(&ok.node, params, args, compiled)),
            Box::new(substitute_type(&error.node, params, args, compiled)),
        ),
        TypeExpr::Vec2(inner) => ResolvedType::Vec2(Box::new(substitute_type(
            &inner.node,
            params,
            args,
            compiled,
        ))),
        TypeExpr::Vec3(inner) => ResolvedType::Vec3(Box::new(substitute_type(
            &inner.node,
            params,
            args,
            compiled,
        ))),
        TypeExpr::Vec4(inner) => ResolvedType::Vec4(Box::new(substitute_type(
            &inner.node,
            params,
            args,
            compiled,
        ))),
        TypeExpr::Quat(inner) => ResolvedType::Quat(Box::new(substitute_type(
            &inner.node,
            params,
            args,
            compiled,
        ))),
        TypeExpr::Mat3(inner) => ResolvedType::Mat3(Box::new(substitute_type(
            &inner.node,
            params,
            args,
            compiled,
        ))),
        TypeExpr::Mat4(inner) => ResolvedType::Mat4(Box::new(substitute_type(
            &inner.node,
            params,
            args,
            compiled,
        ))),
        TypeExpr::BitsInline(names) => ResolvedType::BitsInline(names.clone()),
    }
}

fn type_name(ty: &ResolvedType) -> String {
    match ty {
        ResolvedType::Primitive(primitive) => format!("{primitive:?}").to_lowercase(),
        ResolvedType::SubByte(sub_byte) => {
            format!(
                "{}{}",
                if sub_byte.signed { "i" } else { "u" },
                sub_byte.bits
            )
        }
        ResolvedType::Semantic(semantic) => format!("{semantic:?}").to_lowercase(),
        ResolvedType::Named(id) => format!("type#{}", id.index()),
        ResolvedType::Optional(inner) => format!("optional<{}>", type_name(inner)),
        ResolvedType::Array(inner) => format!("array<{}>", type_name(inner)),
        ResolvedType::FixedArray(inner, length) => {
            format!("array<{}, {length}>", type_name(inner))
        }
        ResolvedType::Set(inner) => format!("set<{}>", type_name(inner)),
        ResolvedType::Map(key, value) => {
            format!("map<{}, {}>", type_name(key), type_name(value))
        }
        ResolvedType::Result(ok, error) => {
            format!("result<{}, {}>", type_name(ok), type_name(error))
        }
        ResolvedType::Vec2(inner) => format!("vec2<{}>", type_name(inner)),
        ResolvedType::Vec3(inner) => format!("vec3<{}>", type_name(inner)),
        ResolvedType::Vec4(inner) => format!("vec4<{}>", type_name(inner)),
        ResolvedType::Quat(inner) => format!("quat<{}>", type_name(inner)),
        ResolvedType::Mat3(inner) => format!("mat3<{}>", type_name(inner)),
        ResolvedType::Mat4(inner) => format!("mat4<{}>", type_name(inner)),
        ResolvedType::BitsInline(names) => format!("bits{{{}}}", names.join(",")),
    }
}

fn resolved_type_to_source(ty: &ResolvedType) -> Option<TypeExpr> {
    match ty {
        ResolvedType::Primitive(value) => Some(TypeExpr::Primitive(*value)),
        ResolvedType::SubByte(value) => Some(TypeExpr::SubByte(*value)),
        ResolvedType::Semantic(value) => Some(TypeExpr::Semantic(*value)),
        ResolvedType::Optional(inner) => Some(TypeExpr::Optional(Box::new(
            crate::span::Spanned::new(resolved_type_to_source(inner)?, crate::span::Span::empty(0)),
        ))),
        ResolvedType::Array(inner) => Some(TypeExpr::Array(Box::new(crate::span::Spanned::new(
            resolved_type_to_source(inner)?,
            crate::span::Span::empty(0),
        )))),
        ResolvedType::FixedArray(inner, length) => Some(TypeExpr::FixedArray(
            Box::new(crate::span::Spanned::new(
                resolved_type_to_source(inner)?,
                crate::span::Span::empty(0),
            )),
            *length,
        )),
        ResolvedType::Set(inner) => Some(TypeExpr::Set(Box::new(crate::span::Spanned::new(
            resolved_type_to_source(inner)?,
            crate::span::Span::empty(0),
        )))),
        ResolvedType::Map(key, value) => Some(TypeExpr::Map(
            Box::new(crate::span::Spanned::new(
                resolved_type_to_source(key)?,
                crate::span::Span::empty(0),
            )),
            Box::new(crate::span::Spanned::new(
                resolved_type_to_source(value)?,
                crate::span::Span::empty(0),
            )),
        )),
        ResolvedType::Result(ok, error) => Some(TypeExpr::Result(
            Box::new(crate::span::Spanned::new(
                resolved_type_to_source(ok)?,
                crate::span::Span::empty(0),
            )),
            Box::new(crate::span::Spanned::new(
                resolved_type_to_source(error)?,
                crate::span::Span::empty(0),
            )),
        )),
        ResolvedType::BitsInline(names) => Some(TypeExpr::BitsInline(names.clone())),
        ResolvedType::Named(_)
        | ResolvedType::Vec2(_)
        | ResolvedType::Vec3(_)
        | ResolvedType::Vec4(_)
        | ResolvedType::Quat(_)
        | ResolvedType::Mat3(_)
        | ResolvedType::Mat4(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projects_generic_signature_and_mutating_body() {
        let source = r#"
namespace test.portable
trait Adjustable<T> {
    value @0 : T
    fn adjust(delta: T) -> T
}
message Counter { value @0 : i32 }
impl Adjustable<i32> for Counter {
    fn adjust(delta: i32) -> i32 {
        let previous: i32 = self.value
        self.value = self.value + delta
        return previous
    }
}
"#;
        let result = crate::compile(source);
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        let compiled = result.compiled.expect("compiled schema");
        let (_, implementation) = compiled.impls().next().expect("implementation");
        let functions = project_impl(&compiled, implementation).expect("portable projection");
        assert_eq!(functions.len(), 1);
        assert_eq!(
            functions[0].return_type,
            Some(ResolvedType::Primitive(PrimitiveType::I32))
        );
        assert_eq!(functions[0].statements.len(), 3);
    }

    #[test]
    fn rejects_calls_during_codegen_projection() {
        let source = r#"
namespace test.portable_call
trait Validatable { fn validate() -> bool }
message Event { value @0 : bool }
impl Validatable for Event {
    fn validate() -> bool { return helper() }
}
"#;
        let result = crate::compile(source);
        assert!(
            result.diagnostics.is_empty(),
            "calls remain valid until code generation: {:?}",
            result.diagnostics
        );
        let compiled = result.compiled.expect("compiled schema");
        let (_, implementation) = compiled.impls().next().expect("implementation");
        assert!(matches!(
            project_impl(&compiled, implementation),
            Err(PortableFunctionError::UnsupportedCall { .. })
        ));
    }

    #[test]
    fn types_decimal_literals_from_unsigned_context() {
        let source = r#"
namespace test.portable_unsigned
trait Resettable { fn reset() }
message Counter { value @0 : u32 }
impl Resettable for Counter {
    fn reset() { self.value = 1 }
}
"#;
        let compiled = crate::compile(source).compiled.expect("compiled schema");
        let (_, implementation) = compiled.impls().next().expect("implementation");
        let functions = project_impl(&compiled, implementation).expect("portable projection");
        let PortableStatement::AssignSelfField { value, .. } = &functions[0].statements[0] else {
            panic!("expected assignment");
        };
        assert_eq!(value.ty, ResolvedType::Primitive(PrimitiveType::U32));
    }

    #[test]
    fn infers_comparison_operands_independently_from_bool_results() {
        let source = r#"
namespace test.portable_comparisons
trait Comparable {
    fn ordered() -> bool
    fn same() -> bool
}
message Value { marker @0 : bool }
impl Comparable for Value {
    fn ordered() -> bool { return 1 < 2 }
    fn same() -> bool { return "vexil" == "vexil" }
}
"#;
        let result = crate::compile(source);
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        let compiled = result.compiled.expect("compiled schema");
        let (_, implementation) = compiled.impls().next().expect("implementation");
        let functions = project_impl(&compiled, implementation).expect("portable projection");

        let PortableStatement::Return(Some(ordered)) = &functions[0].statements[0] else {
            panic!("expected ordered return");
        };
        let PortableExprKind::Binary(BinOp::Lt, left, right) = &ordered.kind else {
            panic!("expected ordering expression");
        };
        assert_eq!(left.ty, ResolvedType::Primitive(PrimitiveType::I64));
        assert_eq!(right.ty, ResolvedType::Primitive(PrimitiveType::I64));

        let PortableStatement::Return(Some(same)) = &functions[1].statements[0] else {
            panic!("expected equality return");
        };
        let PortableExprKind::Binary(BinOp::Eq, left, right) = &same.kind else {
            panic!("expected equality expression");
        };
        assert_eq!(left.ty, ResolvedType::Semantic(SemanticType::String));
        assert_eq!(right.ty, ResolvedType::Semantic(SemanticType::String));
    }

    #[test]
    fn substitutes_primitive_and_generic_aliases_structurally() {
        let source = r#"
namespace test.portable_aliases
trait AliasOps<T> {
    fn normalize(candidate: Maybe<T>) -> T
    fn score() -> Score
}
message Counter {
    current @0 : optional<i32>
}
type Score = i32
type Maybe<T> = optional<T>
impl AliasOps<i32> for Counter {
    fn normalize(candidate: Maybe<i32>) -> i32 {
        let answer: i32 = 1
        return answer
    }
    fn score() -> i32 { return 1 }
}
"#;
        let result = crate::compile(source);
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        let compiled = result.compiled.expect("compiled schema");
        let (trait_id, _) = compiled.find_type("AliasOps").expect("trait");
        let (_, implementation) = compiled.impls().next().expect("implementation");
        let trait_signatures = trait_signatures(&compiled, trait_id).expect("trait signatures");
        let functions = project_impl(&compiled, implementation).expect("portable projection");
        let expected_optional =
            ResolvedType::Optional(Box::new(ResolvedType::Primitive(PrimitiveType::I32)));

        assert!(
            matches!(
                &trait_signatures[0].params[0].ty,
                TypeExpr::Optional(inner)
                    if inner.node == TypeExpr::Named("T".into())
            ),
            "generic alias should expand to optional<T>"
        );
        assert_eq!(
            trait_signatures[1].return_type,
            Some(TypeExpr::Primitive(PrimitiveType::I32)),
            "primitive alias should expand to i32"
        );
        assert_eq!(functions[0].params[0].ty, expected_optional);
        assert_eq!(
            functions[0].return_type,
            Some(ResolvedType::Primitive(PrimitiveType::I32))
        );
        assert!(matches!(
            compiled.find_type("Counter"),
            Some((_, TypeDef::Message(_)))
        ));
    }
}
