use crate::xml::name::MacroName;

pub type CExprToks<'a> = Vec<CExprItem<'a>>;

#[derive(Debug)]
pub enum CExprItem<'a> {
    Punct(char),
    IntLit(i128),   // ?
    Value(&'a str), // separate into numbers and variables (and maybe even macro calls?????) quasi du checkst>?
    // unsure about this one
    Cast {
        to_type: &'a str,
        c_expr: CExprToks<'a>,
    },
    MacroCall {
        macro_name: MacroName,
        args: Vec<CExprToks<'a>>,
    },
}

impl<'a> CExprItem<'a> {
    pub(crate) fn parse(input: &'a str) -> CExprToks<'a> {
        let mut c_expr = Vec::new();
        Self::parse_into(input, &mut c_expr);
        c_expr
    }

    pub(crate) fn parse_into(input: &'a str, out: &mut impl Extend<CExprItem<'a>>) {
        assert!(input.is_ascii());
        let mut s = input;
        while let Some(c) = s.chars().next() {
            let is_value = |c: char| c.is_ascii_alphanumeric() || c == '_';
            let item = if is_value(c) {
                let len = s.chars().take_while(|&c| is_value(c)).count();
                let (value, rest) = s.split_at(len);
                s = rest;
                // todo?
                CExprItem::Value(value)
            } else if c.is_ascii_punctuation() {
                s = &s[1..];
                CExprItem::Punct(c)
            } else if c.is_ascii_whitespace() {
                s = s.trim_start();
                continue;
            } else {
                unreachable!("{s:?}");
            };

            out.extend([item]);
        }
    }
}
