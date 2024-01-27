use analysis::Analysis;

fn main() {
    tracing_subscriber::fmt::init();
    let _analysis = Analysis::new("generator/Vulkan-Headers");
    if false {
        _analysis.dump_as_pseudo_rust();
    }
}
