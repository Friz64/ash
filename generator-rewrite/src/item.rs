use crate::{Context, output::CodeMap};
use analysis::item::{CommandItem, TypeItem};
use tracing::debug;

mod alias;
mod basetype;
mod bitmask;
mod cmacro;
mod constant;
mod enumeration;
mod function;
mod handle;
mod structure;

pub trait Code {
    fn code(&self, ctx: &Context) -> CodeMap;
}

impl Code for TypeItem {
    fn code(&self, ctx: &Context) -> CodeMap {
        match self {
            TypeItem::Alias(alias) => alias.code(ctx),
            TypeItem::Struct(structure) => structure.code(ctx),
            TypeItem::Union(union) => union.code(ctx),
            TypeItem::Enum(enumeration) => enumeration.code(ctx),
            TypeItem::BitMask(bitmask) => bitmask.code(ctx),
            TypeItem::BitMaskBits { .. } => CodeMap::default(), // covered by `TypeItem::BitMask`
            TypeItem::BaseType(basetype) => basetype.code(ctx),
            TypeItem::Handle(handle) => handle.code(ctx),
        }
    }
}

impl Code for CommandItem {
    fn code(&self, ctx: &Context) -> CodeMap {
        match self {
            CommandItem::Alias(alias) => alias.code(ctx),
            CommandItem::Command(command) => command.code(ctx),
        }
    }
}

impl CodeMap {
    pub fn extend_from_items<'a, C: Code + 'a>(
        &mut self,
        ctx: &Context,
        item_iter: impl IntoIterator<Item = &'a C>,
    ) {
        for item in item_iter {
            self.extend(item.code(ctx));
        }
    }
}

pub fn generate_code(ctx: &Context, codemap: &mut CodeMap) {
    debug!("generating structures code");
    codemap.extend_from_items(ctx, ctx.items.types.values());
    codemap.extend_from_items(ctx, ctx.items.func_pointers.values());
    codemap.extend_from_items(ctx, ctx.items.commands.values());
    codemap.extend_from_items(ctx, ctx.items.constants.values());
    codemap.extend_from_items(ctx, ctx.items.cmacros.values());
}
