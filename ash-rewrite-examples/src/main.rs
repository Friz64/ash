use ash_rewrite::Entry;

fn main() {
    unsafe {
        let entry = Entry::load().unwrap();
        if let Ok(properties) = entry.enumerate_instance_extension_properties(None) {
            dbg!(properties.len());
        }
    }
}
