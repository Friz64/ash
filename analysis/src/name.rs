use derive_more::Deref;

#[derive(Deref)]
pub struct TypeName {
    #[deref]
    original: &'static str,
    prefix_trimmed: &'static str,
    prefix_tag_trimmed: &'static str,
}

impl TypeName {
    pub(crate) fn parse(original: &'static str) -> TypeName {
        let prefix_trimmed = original.trim_start_matches("Vk");
        let tag_boundary = prefix_trimmed
            .rfind(|c: char| c.is_lowercase() || c.is_ascii_digit())
            .unwrap();
        let prefix_tag_trimmed = prefix_trimmed.split_at(tag_boundary).0;

        TypeName {
            original,
            prefix_trimmed,
            prefix_tag_trimmed,
        }
    }

    pub fn original(&self) -> &'static str {
        self.original
    }

    pub fn prefix_trimmed(&self) -> &'static str {
        self.prefix_trimmed
    }

    pub fn prefix_tag_trimmed(&self) -> &'static str {
        self.prefix_tag_trimmed
    }
}
