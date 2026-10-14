use analysis::Analysis;
use std::io;
use tracing::debug;

fn main() -> io::Result<()> {
    tracing_subscriber::fmt::init();
    debug!("running analysis");
    let analysis = Analysis::new("generator-rewrite/Vulkan-Headers");
    debug!("running generator");
    let builder_rules = syn::parse_file(include_str!("../builder_rules.rs")).unwrap();
    generator_rewrite::generate(&analysis, &builder_rules, "ash-rewrite/src/generated")
}
