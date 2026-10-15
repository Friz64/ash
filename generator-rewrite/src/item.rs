use crate::{Context, output::CodeMap};
use analysis::item::{CommandItem, TypeItem};
use tracing::debug;

mod alias;
mod basetype;
mod bitmask;
mod cmacro;
mod constant;
mod enum_;
mod function;
mod handle;
mod struct_;

pub trait Code {
    fn code(&self, ctx: &Context, codemap: &mut CodeMap);
}

impl Code for TypeItem {
    fn code(&self, ctx: &Context, codemap: &mut CodeMap) {
        match self {
            TypeItem::Alias(alias) => alias.code(ctx, codemap),
            TypeItem::Struct(struct_) => struct_.code(ctx, codemap),
            TypeItem::Union(union) => union.code(ctx, codemap),
            TypeItem::Enum(enum_) => enum_.code(ctx, codemap),
            TypeItem::BitMask(bitmask) => bitmask.code(ctx, codemap),
            TypeItem::BitMaskBits { .. } => (), // covered by `TypeItem::BitMask`
            TypeItem::BaseType(basetype) => basetype.code(ctx, codemap),
            TypeItem::Handle(handle) => handle.code(ctx, codemap),
        }
    }
}

impl Code for CommandItem {
    fn code(&self, ctx: &Context, codemap: &mut CodeMap) {
        match self {
            CommandItem::Alias(alias) => alias.code(ctx, codemap),
            CommandItem::Command(command) => command.code(ctx, codemap),
        }
    }
}

fn extend_from_items<'a, C: Code + 'a>(
    codemap: &mut CodeMap,
    ctx: &Context,
    item_iter: impl IntoIterator<Item = &'a C>,
) {
    for item in item_iter {
        item.code(ctx, codemap);
    }
}

pub fn generate_code(ctx: &Context, codemap: &mut CodeMap) {
    debug!("generating types code");
    extend_from_items(codemap, ctx, ctx.items.types.values());
    debug!("generating function pointers code");
    extend_from_items(codemap, ctx, ctx.items.func_pointers.values());
    debug!("generating commands code");
    extend_from_items(codemap, ctx, ctx.items.commands.values());
    debug!("generating constants code");
    extend_from_items(codemap, ctx, ctx.items.constants.values());
    debug!("generating macros code");
    extend_from_items(codemap, ctx, ctx.items.cmacros.values());
}
