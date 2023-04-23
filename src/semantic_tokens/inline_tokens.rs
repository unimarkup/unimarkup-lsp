use lsp_types::SemanticToken;
use unimarkup_inline::{Inline, NestedContent, TokenDelimiters, TokenKind};

use super::{
    block_tokens::TokenType, delta_conversions::to_lsp_line_nr, TokenValue,
};

#[derive(Debug, Default, Clone)]
pub(crate) struct OpenTokenModifier {
    /// The open token modifier
    token_modifier: TokenModifier,
    /// Column start of the modifier
    _start_column: u32,
    /// Column offset the content of this modifier starts.
    /// Needed for nested blocks.
    ///
    /// e.g. verbatim inside list
    _start_column_offset: u32,
}

#[derive(Default, Debug, Clone)]
pub(crate) enum TokenModifier {
    #[default]
    Plain,
    Bold,
    Italic,
    Verbatim,
}

impl TokenValue for TokenModifier {
    fn value(&self) -> u32 {
        // Note: These values must set the correct modifier bit
        match self {
            TokenModifier::Plain => 0,
            TokenModifier::Bold => 1,
            TokenModifier::Italic => 1 << 1,
            TokenModifier::Verbatim => 3,
        }
    }
}

pub(crate) trait SemanticInlineTokenizer {
    fn tokens(
        &self,
        open_modifiers: &mut Vec<OpenTokenModifier>,
    ) -> Vec<SemanticToken>;
}

impl SemanticInlineTokenizer for NestedContent {
    fn tokens(
        &self,
        open_modifiers: &mut Vec<OpenTokenModifier>,
    ) -> Vec<SemanticToken> {
        self.iter()
            .flat_map(|inline| inline.tokens(open_modifiers))
            .collect()
    }
}

impl SemanticInlineTokenizer for Inline {
    fn tokens(
        &self,
        open_modifiers: &mut Vec<OpenTokenModifier>,
    ) -> Vec<SemanticToken> {
        match self {
            Inline::Bold(nested) | Inline::Italic(nested) => {
                open_modifiers.push(self.into());
                let delimiters = self.delimiters();

                let mut tokens = vec![SemanticToken {
                    delta_line: self.span().start().line as u32,
                    delta_start: self.span().start().column as u32,
                    length: delimiters.open().as_str().len() as u32,
                    token_type: TokenType::default().value(),
                    token_modifiers_bitset: get_modifier_bitfield(open_modifiers),
                }];

                tokens.append(&mut nested.tokens(open_modifiers));

                let closing_delim = delimiters
                    .close()
                    .expect("Could not unwrap non-existent closing tag");

                tokens.push(SemanticToken {
                    delta_line: self.span().end().line as u32,
                    delta_start: (self.span().end().column + 1 - closing_delim.as_str().len()) as u32,
                    length: closing_delim.as_str().len() as u32,
                    token_type: TokenType::default().value(),
                    token_modifiers_bitset: get_modifier_bitfield(open_modifiers),
                });

                open_modifiers.pop();
                tokens
            }
            Inline::Verbatim(plain_content) => {
                open_modifiers.push(self.into());

                let tokens = vec![SemanticToken {
                    delta_line: plain_content.span().start().line as u32,
                    delta_start: plain_content.span().start().column as u32,
                    length: plain_content.content_len() as u32,
                    token_type: TokenType::default().value(),
                    token_modifiers_bitset: get_modifier_bitfield(open_modifiers),
                }];

                open_modifiers.pop();

                tokens
            }
            Inline::Plain(plain_content) => {
                dbg!(plain_content.span().start().line);
                if !open_modifiers.is_empty() {
                    vec![SemanticToken {
                        delta_line: plain_content.span().start().line as u32,
                        delta_start: plain_content.span().start().column as u32,
                        length: plain_content.content_len() as u32,
                        token_type: TokenType::default().value(),
                        token_modifiers_bitset: get_modifier_bitfield(open_modifiers),
                    }]
                } else {
                    vec![]
                }
            }
            Inline::TextGroup(nested) => {
                let delimiters: TokenDelimiters = self.delimiters();

                let mut tokens = vec![SemanticToken {
                    delta_line: nested.span().start().line as u32,
                    delta_start: nested.span().start().column as u32,
                    length: delimiters.open().as_str().len() as u32,
                    token_type: TokenType::default().value(),
                    token_modifiers_bitset: get_modifier_bitfield(open_modifiers),
                }];

                tokens.append(&mut nested.tokens(open_modifiers));

                tokens.push(SemanticToken {
                    delta_line: to_lsp_line_nr(nested.span().end().line),
                    delta_start: (nested.span().end().column
                        - delimiters
                            .close()
                            .unwrap_or(TokenKind::Plain)
                            .as_str()
                            .len()) as u32,
                    length: delimiters
                        .close()
                        .unwrap_or(TokenKind::Plain)
                        .as_str()
                        .len() as u32,
                    token_type: TokenType::default().value(),
                    token_modifiers_bitset: get_modifier_bitfield(open_modifiers),
                });

                tokens
            }
            _ => vec![],
        }
    }
}

impl TokenValue for Inline {
    fn value(&self) -> u32 {
        match self {
            Inline::Bold(_) => TokenModifier::Bold.value(),
            Inline::Italic(_) => TokenModifier::Italic.value(),
            Inline::Verbatim(_) => TokenModifier::Verbatim.value(),
            _ => TokenModifier::default().value(),
        }
    }
}

// impl From<&Inline> for TokenType {
//     fn from(inline: &Inline) -> Self {
//         match *inline {
//             Inline::Bold(_) => TokenType::Bold,
//             Inline::Italic(_) => TokenType::Italic,
//             Inline::Verbatim(_) => TokenType::Verbatim,
//             _ => TokenType::Paragraph,
//         }
//     }
// }

impl From<&Inline> for OpenTokenModifier {
    fn from(inline: &Inline) -> Self {
        match inline {
            Inline::Bold(_) => OpenTokenModifier {
                token_modifier: TokenModifier::Bold,
                ..Default::default()
            },
            Inline::Italic(_) => OpenTokenModifier {
                token_modifier: TokenModifier::Italic,
                ..Default::default()
            },
            Inline::Verbatim(_) => OpenTokenModifier {
                token_modifier: TokenModifier::Verbatim,
                ..Default::default()
            },
            _ => OpenTokenModifier {
                ..Default::default()
            },
        }
    }
}

fn get_modifier_bitfield(modifiers: &Vec<OpenTokenModifier>) -> u32 {
    let mut field = 0;

    for modifier in modifiers {
        field |= modifier.token_modifier.value();
    }

    field
}
