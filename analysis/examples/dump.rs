use analysis::Analysis;
use std::{fs, io};

fn main() -> Result<(), io::Error> {
    tracing_subscriber::fmt::init();
    let analysis = Analysis::new("generator-rewrite/Vulkan-Headers");

    let file = std::env::temp_dir().join("analysis.txt");
    println!("dumping analysis output to {}", file.display());
    fs::write(file, format!("{analysis:#?}"))
}
