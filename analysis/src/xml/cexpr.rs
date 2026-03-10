use crate::name::CMacroName;

pub type CExprItems = Vec<CExprItem>;

#[derive(Debug, Clone)]
pub enum CExprItem {
    Punct(char),
    NumericLiteral(&'static str),
    U32ArgVar(&'static str),
    StringLiteral(&'static str),
    MacroCall {
        macro_name: CMacroName,
        args: Vec<CExprItems>,
    },
}

pub(crate) fn parse(input: &'static str) -> CExprItems {
    assert!(input.is_ascii());

    fn eat_char(s: &mut &str, c: char) -> bool {
        if s.starts_with(c) {
            *s = &s[1..];
            return true;
        }

        false
    }

    let mut s = input;
    let mut output = Vec::new();
    let mut arg_expected = false;
    while let Some(c) = s.chars().next() {
        let is_value = |c: char| c.is_ascii_alphanumeric() || ['_', '.'].contains(&c);
        let item = if is_value(c) {
            let len = s.chars().take_while(|&c| is_value(c)).count();
            let (mut value, rest) = s.split_at(len);
            s = rest;

            if matches!(output.last(), Some(CExprItem::Punct('(')))
                && value == "uint32_t"
                && eat_char(&mut s, ')')
            {
                output.pop(); // remove that '('
                arg_expected = true;
                continue;
            } else if arg_expected {
                arg_expected = false;
                CExprItem::U32ArgVar(value)
            } else if value.starts_with(|c: char| c.is_ascii_digit()) {
                let lowercase = value.to_ascii_lowercase();
                if let Some(stripped) = (lowercase.strip_suffix("ull"))
                    .or_else(|| lowercase.strip_suffix("u"))
                    .or_else(|| lowercase.strip_suffix("f"))
                {
                    value = &value[..stripped.len()];
                }

                CExprItem::NumericLiteral(value)
            } else {
                CExprItem::MacroCall {
                    macro_name: CMacroName(value),
                    args: vec![],
                }
            }
        } else if c == '"' {
            let (value, rest) = s[1..].split_once('"').unwrap();
            s = rest;
            CExprItem::StringLiteral(value)
        } else if c.is_ascii_punctuation() {
            s = &s[1..];
            CExprItem::Punct(c)
        } else if c.is_ascii_whitespace() {
            s = s.trim_start();
            continue;
        } else {
            unreachable!("{s:?}");
        };

        output.push(item);
    }

    output
}
