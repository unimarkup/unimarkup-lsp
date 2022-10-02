use lsp_types::SemanticToken;
use unimarkup_inline::{Inline, NestedContent, TokenDelimiters, TokenKind};

use super::{
    delta_conversions::to_lsp_line_nr, OpenTokenModifier, TokenModifier, TokenType, TokenValue,
    NO_TOKEN_TYPE,
};

pub(crate) trait SemanticInlineTokenizer {
    fn tokens(
        &self,
        token_type: &TokenType,
        open_modifiers: &mut Vec<OpenTokenModifier>,
    ) -> Vec<SemanticToken>;
}

impl SemanticInlineTokenizer for NestedContent {
    fn tokens(
        &self,
        token_type: &TokenType,
        open_modifiers: &mut Vec<OpenTokenModifier>,
    ) -> Vec<SemanticToken> {
        self.iter()
            .flat_map(|inline| inline.tokens(token_type, open_modifiers))
            .collect()
    }
}

impl SemanticInlineTokenizer for Inline {
    fn tokens(
        &self,
        token_type: &TokenType,
        open_modifiers: &mut Vec<OpenTokenModifier>,
    ) -> Vec<SemanticToken> {
        match self {
            Inline::Bold(nested) | Inline::Italic(nested) => {
                open_modifiers.push(self.into());
                let delimiters = self.delimiters();

                let mut tokens = vec![SemanticToken {
                    delta_line: to_lsp_line_nr(self.span().start().line),
                    delta_start: self.span().start().column as u32,
                    length: delimiters.open().as_str().len() as u32,
                    token_type: token_type.value(),
                    token_modifiers_bitset: get_modifier_bitfield(open_modifiers),
                }];

                tokens.append(&mut nested.tokens(token_type, open_modifiers));

                tokens.push(SemanticToken {
                    delta_line: to_lsp_line_nr(self.span().end().line),
                    delta_start: (self.span().end().column
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
                    token_type: token_type.value(),
                    token_modifiers_bitset: get_modifier_bitfield(open_modifiers),
                });

                open_modifiers.pop();
                tokens
            }
            Inline::Plain(plain_content) => {
                if token_type.value() != NO_TOKEN_TYPE || !open_modifiers.is_empty() {
                    vec![SemanticToken {
                        delta_line: to_lsp_line_nr(plain_content.span().start().line),
                        delta_start: plain_content.span().start().column as u32,
                        length: (plain_content.span().end().column
                            - plain_content.span().start().column)
                            as u32,
                        token_type: token_type.value(),
                        token_modifiers_bitset: get_modifier_bitfield(open_modifiers),
                    }]
                } else {
                    vec![]
                }
            }
            Inline::TextGroup(nested) => {
                let delimiters: TokenDelimiters = self.delimiters();

                let mut tokens = vec![SemanticToken {
                    delta_line: to_lsp_line_nr(nested.span().start().line),
                    delta_start: nested.span().start().column as u32,
                    length: delimiters.open().as_str().len() as u32,
                    token_type: token_type.value(),
                    token_modifiers_bitset: get_modifier_bitfield(open_modifiers),
                }];

                tokens.append(&mut nested.tokens(token_type, open_modifiers));

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
                    token_type: token_type.value(),
                    token_modifiers_bitset: get_modifier_bitfield(open_modifiers),
                });

                tokens
            }
            _ => vec![],
        }
    }
}

// impl TokenValue for InlineKind {
//     fn value(&self) -> u32 {
//         match self {
//             InlineKind::Bold(_) => TokenModifier::Bold.value(),
//             InlineKind::Italic(_) => TokenModifier::Italic.value(),
//             InlineKind::BoldItalic(_) => TokenModifier::BoldItalic.value(),
//             InlineKind::Verbatim(_) => TokenModifier::Verbatim.value(),
//             _ => NO_TOKEN_TYPE,
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
