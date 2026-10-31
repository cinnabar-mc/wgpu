use alloc::vec::Vec;

use super::{ast::StructLayout, context::Context, offset, Result};
use crate::{Expression, Handle, Span, Type, TypeInner, UniqueArena};

/// Reports whether a type has a concrete value layout.
fn has_value_layout(ty: Handle<Type>, types: &UniqueArena<Type>) -> bool {
    match types[ty].inner {
        TypeInner::Scalar(_) | TypeInner::Vector { .. } | TypeInner::Matrix { .. } => true,
        TypeInner::Array { base, .. } => has_value_layout(base, types),
        TypeInner::Struct { ref members, .. } => {
            members.iter().all(|m| has_value_layout(m.ty, types))
        }
        _ => false,
    }
}

/// Gives whole aggregate rvalues their ordinary layout without changing buffer types.
pub(super) fn normalize_value_layout(
    ctx: &mut Context,
    expr: &mut Handle<Expression>,
    meta: Span,
) -> Result<()> {
    if !matches!(
        ctx.resolve_type(*expr, meta)?,
        TypeInner::Array { .. } | TypeInner::Struct { .. }
    ) {
        return Ok(());
    }
    let source = ctx.resolve_type_handle(*expr, meta)?;
    if !has_value_layout(source, &ctx.module.types) {
        return Ok(());
    }
    let target = ordinary_type(ctx, source, meta)?;
    convert_value_layout(ctx, expr, target, meta)
}

/// Returns the ordinary type used to compare aggregate values across buffer layouts.
fn ordinary_type(ctx: &mut Context, source: Handle<Type>, meta: Span) -> Result<Handle<Type>> {
    let mut errors = Vec::new();
    let target = offset::calculate_offset(
        source,
        meta,
        StructLayout::Std430,
        &mut ctx.module.types,
        &mut errors,
    )
    .ty;
    if let Some(error) = errors.into_iter().next() {
        return Err(error);
    }
    Ok(target)
}

/// Rebuilds an array or a layout variant of the same declared struct for its destination.
pub(super) fn convert_value_layout(
    ctx: &mut Context,
    expr: &mut Handle<Expression>,
    target: Handle<Type>,
    meta: Span,
) -> Result<()> {
    let target_inner = ctx.module.types[target].inner.clone();
    if !matches!(
        target_inner,
        TypeInner::Array { .. } | TypeInner::Struct { .. }
    ) {
        return Ok(());
    }
    let source = ctx.resolve_type_handle(*expr, meta)?;
    if source == target {
        return Ok(());
    }
    let count = match target_inner {
        TypeInner::Array {
            size: crate::ArraySize::Constant(target_size),
            ..
        } => match ctx.module.types[source].inner {
            TypeInner::Array {
                size: crate::ArraySize::Constant(source_size),
                ..
            } if target_size == source_size => target_size.get() as usize,
            _ => return Ok(()),
        },
        TypeInner::Struct { ref members, .. } => {
            if !matches!(ctx.module.types[source].inner, TypeInner::Struct { .. })
                || !has_value_layout(source, &ctx.module.types)
                || !has_value_layout(target, &ctx.module.types)
            {
                return Ok(());
            }
            if ordinary_type(ctx, source, meta)? != ordinary_type(ctx, target, meta)? {
                return Ok(());
            }
            members.len()
        }
        _ => return Ok(()),
    };
    let mut components = Vec::with_capacity(count);
    for index in 0..count {
        let element_type = match target_inner {
            TypeInner::Array { base, .. } => base,
            TypeInner::Struct { ref members, .. } => members[index].ty,
            _ => unreachable!(),
        };
        let mut element = ctx.add_expression(
            Expression::AccessIndex {
                base: *expr,
                index: index as u32,
            },
            meta,
        )?;
        convert_value_layout(ctx, &mut element, element_type, meta)?;
        components.push(element);
    }
    *expr = ctx.add_expression(
        Expression::Compose {
            ty: target,
            components,
        },
        meta,
    )?;
    Ok(())
}
