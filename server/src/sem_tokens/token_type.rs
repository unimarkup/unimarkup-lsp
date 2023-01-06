pub(crate) enum TokenType {
    Heading = 0,
    Paragraph,
    Verbatim,
    Bold,
    Italic,
    Underline,
    Subscript,
    Superscript,
    Overline,
    Strikethrough,
    Highlight,
    Quote,
    Math,
    Parentheses,
    TextGroup,
    Attributes,
    Substitution,
    Newline,
    Whitespace,
    EndOfLine,
    Plain,
    Multiple,
}

impl From<TokenType> for u32 {
    fn from(token_type: TokenType) -> Self {
        match token_type {
            TokenType::Heading => 3,
            TokenType::Paragraph => 21,
            TokenType::Verbatim => 18,
            TokenType::Bold => 3,
            TokenType::Italic => 18,
            _ => token_type as u32,
        }
    }
}
