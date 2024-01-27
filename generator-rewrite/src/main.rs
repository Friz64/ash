use analysis::Analysis;

fn main() {
    tracing_subscriber::fmt::init();
    let _analysis = Analysis::new("generator/Vulkan-Headers");
    // dbg!(_analysis);
    if true {
        _analysis.dump_as_pseudo_rust();
    }
}
