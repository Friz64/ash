pub enum CExpr<'a> {
    Punct(&'a str),
    Value(&'a str),
    Cast {
        to_type: &'a str,
        expr: Box<CExpr<'a>>,
    },
    MacroCall {
        macro_name: &'a str,
        args: Vec<CExpr<'a>>,
    },
}

impl<'a> CExpr<'a> {
    pub(crate) fn parse_into(s: &'a str, out: &mut impl Extend<CExpr<'a>>) {
        // whatevers
    }
}
