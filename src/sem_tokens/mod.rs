mod token_type;

use lsp_server::{RequestId, Response};
use lsp_types::{SemanticToken, SemanticTokens, SemanticTokensParams, SemanticTokensResult};
use unimarkup_core::{
    document::Document,
    elements::blocks::Block,
    elements::{atomic::Heading, atomic::Paragraph, enclosed::Verbatim},
};
use unimarkup_inline::{Inline, NestedContent, PlainContent};

use self::token_type::TokenType;

pub fn generate_semantic_tokens(
    id: RequestId,
    _params: SemanticTokensParams,
    rendered_um: Option<&Document>,
) -> Response {
    let mut tokens = SemanticTokens {
        result_id: Some(id.to_string()),
        ..Default::default()
    };

    if let Some(um_doc) = rendered_um {
        tokens.data = um_doc.tokens().into_semantic_tokens();
    }

    let result = SemanticTokensResult::Tokens(dbg!(tokens));
    let result = serde_json::to_value(&result).ok();

    Response {
        id,
        result,
        error: None,
    }
}

#[derive(Debug)]
struct AbsoluteSemanticToken {
    line: usize,
    start: usize,
    length: usize,
    token_type: u32,
    token_modifiers_bitset: u32,
}

trait IntoSemanticTokens {
    fn into_semantic_tokens(self) -> Vec<SemanticToken>;
}

impl IntoSemanticTokens for Vec<AbsoluteSemanticToken> {
    fn into_semantic_tokens(self) -> Vec<SemanticToken> {
        if self.is_empty() {
            return vec![];
        }

        let mut prev_line = self[0].line;
        let mut prev_col = self[0].start;

        self.iter()
            .enumerate()
            .map(|(index, abs_token)| {
                let delta_line = if index > 0 {
                    abs_token.line.saturating_sub(prev_line)
                } else {
                    abs_token.line.saturating_sub(1)
                } as u32;

                let delta_start = if delta_line == 0 {
                    abs_token.start.saturating_sub(prev_col)
                } else {
                    abs_token.start.saturating_sub(1)
                } as u32;

                prev_line = abs_token.line;
                prev_col = abs_token.start;

                SemanticToken {
                    delta_line,
                    delta_start,
                    length: abs_token.length as u32,
                    token_type: abs_token.token_type,
                    token_modifiers_bitset: abs_token.token_modifiers_bitset,
                }
            })
            .collect()
    }
}

trait Tokenize {
    fn tokens(&self) -> Vec<AbsoluteSemanticToken>;
}

impl Tokenize for Document {
    fn tokens(&self) -> Vec<AbsoluteSemanticToken> {
        self.blocks
            .iter()
            .flat_map(|block| block.tokens())
            .collect()
    }
}

impl Tokenize for Block {
    fn tokens(&self) -> Vec<AbsoluteSemanticToken> {
        match self {
            Block::Heading(block) => block.tokens(),
            Block::Paragraph(block) => block.tokens(),
            Block::Verbatim(block) => block.tokens(),
            _ => todo!(),
        }
    }
}

impl Tokenize for Heading {
    fn tokens(&self) -> Vec<AbsoluteSemanticToken> {
        let heading_token = AbsoluteSemanticToken {
            line: self.line_nr,
            start: 0,
            length: (u8::from(self.level) + 1) as usize, // + 1 for space
            token_type: TokenType::Heading.into(),
            token_modifiers_bitset: 0,
        };

        let inline_tokens = self.content.iter().flat_map(|inline| inline.tokens());

        [heading_token].into_iter().chain(inline_tokens).collect()
    }
}

impl Tokenize for Paragraph {
    fn tokens(&self) -> Vec<AbsoluteSemanticToken> {
        dbg!(self
            .content
            .iter()
            .flat_map(|inline| inline.tokens())
            .map(|mut abs_token| {
                abs_token.line += self.line_nr - 1;
                abs_token
            })
            .collect())
    }
}

impl Tokenize for Verbatim {
    fn tokens(&self) -> Vec<AbsoluteSemanticToken> {
        let start_line = self.line_nr;

        let first_token = AbsoluteSemanticToken {
            line: start_line,
            start: 0,
            length: 80, // NOTE: hardcoded for now because this information is not available here
            token_type: TokenType::Verbatim.into(),
            token_modifiers_bitset: 0,
        };

        [first_token]
            .into_iter()
            .chain(
                self.content
                    .lines()
                    .chain(["last line of verbatim"].into_iter())
                    .enumerate()
                    .map(|(index, line)| AbsoluteSemanticToken {
                        line: start_line + index + 1,
                        start: 0,
                        length: line.len(),
                        token_type: TokenType::Verbatim.into(),
                        token_modifiers_bitset: 0,
                    }),
            )
            .collect()
    }
}

impl Tokenize for Inline {
    fn tokens(&self) -> Vec<AbsoluteSemanticToken> {
        match self {
            Inline::Bold(nested)
            | Inline::Italic(nested)
            | Inline::Underline(nested)
            | Inline::Subscript(nested)
            | Inline::Superscript(nested)
            | Inline::Overline(nested)
            | Inline::Strikethrough(nested)
            | Inline::Highlight(nested)
            | Inline::Quote(nested)
            | Inline::Math(nested)
            | Inline::Multiple(nested)
            | Inline::TextGroup(nested)
            | Inline::Attributes(nested)
            | Inline::Substitution(nested) => {
                let delims = self.delimiters();

                let start_delim = delims.open();

                let start_token = AbsoluteSemanticToken {
                    line: self.span().start().line,
                    start: self.span().start().column,
                    length: start_delim.as_str().len(),
                    token_type: TokenType::from(self).into(),
                    token_modifiers_bitset: 0,
                };

                let end_delim = delims.close().expect("Can't extract closing delimiter");
                let end_token = AbsoluteSemanticToken {
                    line: self.span().end().line,
                    start: self
                        .span()
                        .end()
                        .column
                        .saturating_sub(end_delim.as_str().len().saturating_sub(1)),
                    length: end_delim.as_str().len(),
                    token_type: TokenType::from(self).into(),
                    token_modifiers_bitset: 0,
                };

                [start_token]
                    .into_iter()
                    .chain(tokenize_nested_inline(nested).drain(..))
                    .chain([end_token].into_iter())
                    .collect()
            }
            Inline::Verbatim(plain)
            | Inline::Newline(plain)
            | Inline::Parentheses(plain)
            | Inline::Whitespace(plain)
            | Inline::EndOfLine(plain)
            | Inline::Plain(plain) => {
                let delims = self.delimiters();

                let open_delim = delims.open();

                let mut tokens = Vec::new();

                if open_delim.len() > 0 {
                    let token = AbsoluteSemanticToken {
                        line: self.span().start().line,
                        start: self.span().start().column,
                        length: open_delim.len(),
                        token_type: TokenType::from(self).into(),
                        token_modifiers_bitset: 0,
                    };

                    tokens.push(token);
                }

                tokens.append(&mut tokenize_plain_inline(plain));

                match delims.close() {
                    Some(close_delim) if close_delim.len() > 0 => {
                        let token = AbsoluteSemanticToken {
                            line: self.span().end().line,
                            start: self.span().end().column.saturating_sub(close_delim.len()) + 1,
                            length: open_delim.len(),
                            token_type: TokenType::from(self).into(),
                            token_modifiers_bitset: 0,
                        };

                        tokens.push(token);
                    }
                    _ => { /* Do nothing */ }
                }

                tokens
            }
        }
    }
}

impl From<&Inline> for TokenType {
    fn from(inline: &Inline) -> Self {
        match inline {
            Inline::Bold(_) => TokenType::Bold,
            Inline::Italic(_) => TokenType::Italic,
            Inline::Underline(_) => TokenType::Underline,
            Inline::Subscript(_) => TokenType::Subscript,
            Inline::Superscript(_) => TokenType::Superscript,
            Inline::Overline(_) => TokenType::Overline,
            Inline::Strikethrough(_) => TokenType::Strikethrough,
            Inline::Highlight(_) => TokenType::Highlight,
            Inline::Verbatim(_) => TokenType::Verbatim,
            Inline::Quote(_) => TokenType::Quote,
            Inline::Math(_) => TokenType::Math,
            Inline::Parentheses(_) => TokenType::Parentheses,
            Inline::TextGroup(_) => TokenType::TextGroup,
            Inline::Attributes(_) => TokenType::Attributes,
            Inline::Substitution(_) => TokenType::Substitution,
            Inline::Newline(_) => TokenType::Newline,
            Inline::Whitespace(_) => TokenType::Whitespace,
            Inline::EndOfLine(_) => TokenType::EndOfLine,
            Inline::Plain(_) => TokenType::Plain,
            Inline::Multiple(_) => TokenType::Multiple,
        }
    }
}

fn tokenize_nested_inline(content: &NestedContent) -> Vec<AbsoluteSemanticToken> {
    content.iter().flat_map(|inline| inline.tokens()).collect()
}

fn tokenize_plain_inline(content: &PlainContent) -> Vec<AbsoluteSemanticToken> {
    vec![AbsoluteSemanticToken {
        line: content.span().start().line,
        start: content.span().start().column,
        length: content.as_str().len(),
        token_type: TokenType::Paragraph.into(),
        token_modifiers_bitset: 0,
    }]
}
