use analysis::Analysis;
use std::io;
use tracing::info;

fn main() -> io::Result<()> {
    tracing_subscriber::fmt::init();
    info!("running analysis");
    let analysis = Analysis::new("generator-rewrite/Vulkan-Headers");
    info!("running generator");
    generator_rewrite::generate(&analysis, "ash-rewrite/src/generated")
}
