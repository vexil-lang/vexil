use std::collections::HashSet;
use vexil_lang::ir::{CompiledSchema, ResolvedType, TypeDef, TypeId};

/// Returns set of (type_id, field_index) pairs that need `Box<T>` wrapping.
pub fn detect_boxing(compiled: &CompiledSchema) -> HashSet<(TypeId, usize)> {
    let mut needs_box = HashSet::new();
    for &id in &compiled.declarations {
        let mut path = Vec::new();
        path.push(id);
        match compiled.registry.get(id) {
            Some(TypeDef::Message(msg)) => {
                for (fi, field) in msg.fields.iter().enumerate() {
                    walk_for_boxing(
                        &field.resolved_type,
                        id,
                        fi,
                        &path,
                        compiled,
                        &mut needs_box,
                    );
                }
            }
            Some(TypeDef::Union(un)) => {
                for variant in &un.variants {
                    for (fi, field) in variant.fields.iter().enumerate() {
                        walk_for_boxing(
                            &field.resolved_type,
                            id,
                            fi,
                            &path,
                            compiled,
                            &mut needs_box,
                        );
                    }
                }
            }
            _ => {}
        }
    }
    needs_box
}

fn walk_for_boxing(
    ty: &ResolvedType,
    parent_id: TypeId,
    field_index: usize,
    path: &[TypeId],
    compiled: &CompiledSchema,
    needs_box: &mut HashSet<(TypeId, usize)>,
) {
    match ty {
        ResolvedType::Optional(inner) => {
            check_inner_for_cycle(inner, parent_id, field_index, path, compiled, needs_box);
        }
        ResolvedType::Result(ok, err) => {
            check_inner_for_cycle(ok, parent_id, field_index, path, compiled, needs_box);
            check_inner_for_cycle(err, parent_id, field_index, path, compiled, needs_box);
        }
        ResolvedType::Named(id) => {
            if path.contains(id) {
                // Direct cycle through a Named type — this field must be boxed
                // to break infinite struct size (e.g. ExprKind::Binary { left: Expr })
                needs_box.insert((parent_id, field_index));
                return;
            }
            let mut new_path = path.to_vec();
            new_path.push(*id);
            match compiled.registry.get(*id) {
                Some(TypeDef::Message(msg)) => {
                    for (fi, field) in msg.fields.iter().enumerate() {
                        walk_for_boxing(
                            &field.resolved_type,
                            *id,
                            fi,
                            &new_path,
                            compiled,
                            needs_box,
                        );
                    }
                }
                Some(TypeDef::Union(un)) => {
                    for variant in &un.variants {
                        for (fi, field) in variant.fields.iter().enumerate() {
                            walk_for_boxing(
                                &field.resolved_type,
                                *id,
                                fi,
                                &new_path,
                                compiled,
                                needs_box,
                            );
                        }
                    }
                }
                _ => {}
            }
        }
        ResolvedType::Array(_) | ResolvedType::Set(_) | ResolvedType::Map(_, _) => {
            // Heap-allocated containers — no boxing needed through them
        }
        _ => {} // Primitive, SubByte, Semantic — terminal
    }
}

fn check_inner_for_cycle(
    ty: &ResolvedType,
    parent_id: TypeId,
    field_index: usize,
    path: &[TypeId],
    compiled: &CompiledSchema,
    needs_box: &mut HashSet<(TypeId, usize)>,
) {
    if let ResolvedType::Named(id) = ty {
        if path.contains(id) {
            needs_box.insert((parent_id, field_index));
            return;
        }
    }
    walk_for_boxing(ty, parent_id, field_index, path, compiled, needs_box);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn analyze(src: &str) -> HashSet<(TypeId, usize)> {
        let result = vexil_lang::compile(src);
        let compiled = result.compiled.unwrap();
        detect_boxing(&compiled)
    }

    #[test]
    fn no_recursion_no_boxing() {
        let needs = analyze(
            r#"
            namespace test.box
            message Simple { name @0 : string }
        "#,
        );
        assert!(needs.is_empty());
    }

    #[test]
    fn optional_self_reference_needs_box() {
        let needs = analyze(
            r#"
            namespace test.box
            message Node {
                value @0 : i32
                next  @1 : optional<Node>
            }
        "#,
        );
        assert!(!needs.is_empty());
    }

    #[test]
    fn mutual_recursion_needs_box() {
        let needs = analyze(
            r#"
            namespace test.box
            message Expr {
                kind @0 : ExprKind
            }
            union ExprKind {
                Literal @0 { value @0 : i64 }
                Binary  @1 { left @0 : Expr  op @1 : u8  right @2 : Expr }
            }
        "#,
        );
        assert!(!needs.is_empty());
    }

    #[test]
    fn array_self_reference_no_box() {
        let needs = analyze(
            r#"
            namespace test.box
            message Tree {
                value    @0 : i32
                children @1 : array<Tree>
            }
        "#,
        );
        assert!(needs.is_empty());
    }
}
