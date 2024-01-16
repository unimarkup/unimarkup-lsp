use lsp_types::SemanticToken;
use unimarkup_inline::{
    element::{
        base::{EscapedPlain, EscapedWhitespace, Plain},
        formatting::{
            Bold, DoubleQuote, Highlight, Italic, Math, Overline, Strikethrough, Subscript,
            Superscript, Underline, Verbatim,
        },
        Inline, InlineElement,
    },
    InlineTokenKind,
};

use super::{block_tokens::TokenType, TokenValue};

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TokenModifier {
    #[default]
    Plain,
    Bold,
    Highlight,
    Italic,
    Overline,
    DoubleQuote,
    Strikethrough,
    Subscript,
    Superscript,
    Underline,
}

impl TokenValue for TokenModifier {
    fn value(&self) -> u32 {
        // Note: These values must set the correct modifier bit
        match self {
            TokenModifier::Plain => 0,
            TokenModifier::Bold => 1,
            TokenModifier::Highlight => 1 << 1,
            TokenModifier::Italic => 1 << 2,
            TokenModifier::Overline => 1 << 3,
            TokenModifier::DoubleQuote => 1 << 4,
            TokenModifier::Strikethrough => 1 << 5,
            TokenModifier::Subscript => 1 << 6,
            TokenModifier::Superscript => 1 << 7,
            TokenModifier::Underline => 1 << 8,
        }
    }
}

pub(crate) trait SemanticInlineTokenizer {
    fn tokens(
        &self,
        token_type: TokenType,
        modifiers: &mut Vec<TokenModifier>,
    ) -> Vec<SemanticToken>;
}

impl SemanticInlineTokenizer for &[Inline] {
    fn tokens(
        &self,
        token_type: TokenType,
        modifiers: &mut Vec<TokenModifier>,
    ) -> Vec<SemanticToken> {
        self.iter()
            .flat_map(|inline| inline.tokens(token_type, modifiers))
            .collect()
    }
}

impl SemanticInlineTokenizer for Inline {
    fn tokens(
        &self,
        token_type: TokenType,
        modifiers: &mut Vec<TokenModifier>,
    ) -> Vec<SemanticToken> {
        match self {
            Inline::Bold(format) => format.tokens(token_type, modifiers),
            Inline::Italic(format) => format.tokens(token_type, modifiers),
            Inline::Underline(format) => format.tokens(token_type, modifiers),
            Inline::Subscript(format) => format.tokens(token_type, modifiers),
            Inline::Superscript(format) => format.tokens(token_type, modifiers),
            Inline::Overline(format) => format.tokens(token_type, modifiers),
            Inline::Strikethrough(format) => format.tokens(token_type, modifiers),
            Inline::Highlight(format) => format.tokens(token_type, modifiers),
            Inline::DoubleQuote(format) => format.tokens(token_type, modifiers),

            Inline::Verbatim(verbatim) => verbatim.tokens(TokenType::Verbatim, modifiers),
            Inline::Math(math) => math.tokens(TokenType::Math, modifiers),

            Inline::Plain(plain) => plain.tokens(token_type, modifiers),
            Inline::EscapedPlain(plain) => plain.tokens(token_type, modifiers),
            Inline::EscapedWhitespace(plain) => plain.tokens(token_type, modifiers),
            Inline::EscapedNewline(escaped_newline) => {
                if modifiers.is_empty() && token_type == TokenType::default() {
                    vec![]
                } else {
                    vec![SemanticToken {
                        delta_line: escaped_newline.start().line as u32,
                        delta_start: escaped_newline.start().col_utf16 as u32,
                        // To highlight the backslash
                        length: 1,
                        token_type: token_type.value(),
                        token_modifiers_bitset: get_modifier_bitfield(modifiers),
                    }]
                }
            }

            Inline::TextBox(textbox) => {
                // TODO: implement proper highlighting for textbox
                textbox.inner().as_slice().tokens(token_type, modifiers)
            }
            Inline::Hyperlink(hyperlink) => {
                // TODO: implement proper highlighting for link and parentheses
                hyperlink.inner().as_slice().tokens(token_type, modifiers)
            }

            Inline::Newline(_) | Inline::ImplicitNewline(_) => vec![],

            Inline::ImplicitSubstitution(_) => todo!(),
            Inline::DirectUri(_) => todo!(),
            Inline::NamedSubstitution(_) => todo!(),
        }
    }
}

fn get_modifier_bitfield(modifiers: &Vec<TokenModifier>) -> u32 {
    let mut field = 0;

    for modifier in modifiers {
        field |= modifier.value();
    }

    field
}

trait InlineFormat: InlineElement {
    fn keyword_len(&self) -> u32;
    fn implicit_end(&self) -> bool;
    fn inner(&self) -> &[Inline];
    fn modifier(&self) -> TokenModifier;
}

impl<T> SemanticInlineTokenizer for T
where
    T: InlineFormat,
{
    fn tokens(
        &self,
        token_type: TokenType,
        modifiers: &mut Vec<TokenModifier>,
    ) -> Vec<SemanticToken> {
        modifiers.push(self.modifier());

        let mut tokens = vec![SemanticToken {
            delta_line: self.start().line as u32,
            delta_start: self.start().col_utf16 as u32,
            length: self.keyword_len(),
            token_type: token_type.value(),
            token_modifiers_bitset: get_modifier_bitfield(modifiers),
        }];

        tokens.append(&mut self.inner().tokens(token_type, modifiers));

        if !self.implicit_end() {
            tokens.push(SemanticToken {
                delta_line: self.end().line as u32,
                delta_start: (self.end().col_utf16 as u32 - self.keyword_len()),
                length: self.keyword_len(),
                token_type: token_type.value(),
                token_modifiers_bitset: get_modifier_bitfield(modifiers),
            });
        }

        modifiers.pop();
        tokens
    }
}

impl InlineFormat for DoubleQuote {
    fn keyword_len(&self) -> u32 {
        // Because quote formatting has two double quotes
        (unimarkup_inline::InlineTokenKind::DoubleQuote.len() * 2) as u32
    }

    fn implicit_end(&self) -> bool {
        self.implicit_end()
    }

    fn inner(&self) -> &[Inline] {
        self.inner()
    }

    fn modifier(&self) -> TokenModifier {
        TokenModifier::DoubleQuote
    }
}

macro_rules! impl_inline_format {
    ($($format:ident),+) => {
        $(
            impl InlineFormat for $format {
                fn keyword_len(&self) -> u32 {
                    unimarkup_inline::InlineTokenKind::$format.len() as u32
                }

                fn implicit_end(&self) -> bool {
                    self.implicit_end()
                }

                fn inner(&self) -> &[Inline] {
                    self.inner()
                }

                fn modifier(&self) -> TokenModifier {
                    TokenModifier::$format
                }
            }
        )+
    };
}

impl_inline_format!(
    Bold,
    Highlight,
    Italic,
    Overline,
    Strikethrough,
    Subscript,
    Superscript,
    Underline
);

macro_rules! scoped_format_tokens {
    ($($scoped:ident),+) => {
        $(
            impl SemanticInlineTokenizer for $scoped {
                fn tokens(&self, token_type: TokenType, modifiers: &mut Vec<TokenModifier>) -> Vec<SemanticToken> {
                    let mut tokens = vec![SemanticToken {
                        delta_line: self.start().line as u32,
                        delta_start: self.start().col_utf16 as u32,
                        length: InlineTokenKind::$scoped.len() as u32,
                        token_type: token_type.value(),
                        token_modifiers_bitset: get_modifier_bitfield(modifiers),
                    }];

                    tokens.append(&mut self.inner().as_slice().tokens(token_type, modifiers));

                    if !self.implicit_end() {
                        tokens.push(SemanticToken {
                            delta_line: self.end().line as u32,
                            delta_start: (self.end().col_utf16 - InlineTokenKind::$scoped.len()) as u32,
                            length: InlineTokenKind::$scoped.len() as u32,
                            token_type: token_type.value(),
                            token_modifiers_bitset: get_modifier_bitfield(modifiers),
                        });
                    }

                    tokens
                }
            }
        )+
    }
}

scoped_format_tokens!(Verbatim, Math);

macro_rules! plain_tokens {
    ($($plain:ident),+) => {
        $(
            impl SemanticInlineTokenizer for $plain {
                fn tokens(&self, token_type: TokenType, modifiers: &mut Vec<TokenModifier>) -> Vec<SemanticToken> {
                    if modifiers.is_empty() && token_type == TokenType::default() {
                        vec![]
                    } else {
                        vec![SemanticToken {
                            delta_line: self.start().line as u32,
                            delta_start: self.start().col_utf16 as u32,
                            // Works, because plain elements never span across multiple lines
                            length: (self.end().col_utf16 - self.start().col_utf16) as u32,
                            token_type: token_type.value(),
                            token_modifiers_bitset: get_modifier_bitfield(modifiers),
                        }]
                    }
                }
            }
        )+
    }
}

plain_tokens!(Plain, EscapedPlain, EscapedWhitespace);
