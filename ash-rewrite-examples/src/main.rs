fn main() {
    unsafe {
        let entry = ash_rewrite::Entry::load().unwrap();
        if let Ok(properties) = entry.enumerate_instance_extension_properties(None) {
            // [ash-rewrite-examples/src/main.rs:6:13] properties.len() = 25
            dbg!(properties.len());
        }
    }
}
