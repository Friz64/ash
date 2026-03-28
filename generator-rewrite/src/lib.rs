mod item;
mod output;

use crate::output::CodeMap;
use analysis::Analysis;
use quote::format_ident;
use std::{fmt::Display, io, path::Path};
use syn::Ident;
use tracing::debug;

pub fn generate(analysis: &Analysis, output_path: impl AsRef<Path>) -> io::Result<()> {
    debug!("building codemap");
    let mut codemap = CodeMap::default();

    item::generate_code(analysis.items(), &mut codemap);

    codemap.write(output_path)
}

pub(crate) fn refpage_doc(target: &str, description: impl Display) -> String {
    let valid = matches!(
        target.get(0..2).map(|ab| ab.eq_ignore_ascii_case("vk")),
        Some(true)
    );

    let refpage = if valid {
        format!(
            "[Vulkan Manual Page]\
            (https://docs.vulkan.org/refpages/latest/refpages/source/{target}.html)"
        )
    } else {
        "<s>Vulkan Manual Page</s>".into()
    };

    format!("{} · {}", refpage, description)
}

/// Tries to prepend an underscore in case the name is not a valid identifier
pub(crate) fn escape_ident(name: &str) -> Ident {
    syn::parse_str(name).unwrap_or_else(|_| format_ident!("_{name}"))
}
