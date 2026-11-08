use analysis::Analysis;
use std::io;
use tracing::debug;

fn main() -> io::Result<()> {
    tracing_subscriber::fmt::init();
    debug!("running analysis");
    let analysis = Analysis::new("generator-rewrite/Vulkan-Headers");
    debug!("running generator");
    generator_rewrite::generate(
        &analysis,
        include_str!("../rules/builder.rs"),
        include_str!("../rules/wrapper.rs"),
        "ash-rewrite/src/generated",
    )
}
