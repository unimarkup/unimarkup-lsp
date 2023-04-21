use lsp_server::RequestId;
use lsp_server::Response;
use lsp_types::SemanticToken;
use lsp_types::SemanticTokens;
use lsp_types::SemanticTokensParams;
use lsp_types::SemanticTokensResult;
use unimarkup_core::document::Document;

use self::block_tokens::SemanticBlockTokenizer;

mod block_tokens;
mod delta_conversions;
mod inline_tokens;

pub fn get_semantic_tokens(
    id: RequestId,
    _params: SemanticTokensParams,
    rendered_um: Option<Document>,
) -> Response {
    let mut tokens = SemanticTokens {
        result_id: Some(id.to_string()),
        ..Default::default()
    };

    if let Some(um_doc) = rendered_um {
        tokens.data = make_relative(um_doc.tokens(&mut vec![]));
    }

    let result = Some(SemanticTokensResult::Tokens(tokens));

    let result = serde_json::to_value(&result).unwrap();
    Response {
        id,
        result: Some(result),
        error: None,
    }
}

#[derive(Debug, Default, Clone)]
pub(crate) struct OpenTokenType {
    /// The open token type
    _token_type: TokenType,
    /// Column offset the content of this type starts.
    /// Needed for nested blocks.
    ///
    /// e.g. verbatim inside list
    start_column_offset: u32,
    /// The type length, or None, if the type goes to end of line
    length: Option<u32>,
}

#[derive(Default, Debug, Clone)]
pub(crate) enum TokenType {
    Heading,
    #[default]
    Paragraph,
    Verbatim,
    Bold,
    Italic,
}

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
    Bold,
    Italic,
    // BoldItalic,
    Verbatim,
}

const NO_TOKEN_TYPE: u32 = u32::max_value();

trait TokenValue {
    fn value(&self) -> u32;
}

impl TokenValue for TokenType {
    fn value(&self) -> u32 {
        match self {
            TokenType::Paragraph => 21,
            TokenType::Heading => 3,
            TokenType::Verbatim => 18,
            TokenType::Bold => 3,
            TokenType::Italic => 18,
        }
    }
}

impl TokenValue for TokenModifier {
    fn value(&self) -> u32 {
        // Note: These values must set the correct modifier bit
        match self {
            TokenModifier::Bold => 1,
            TokenModifier::Italic => 1 << 1,
            TokenModifier::Verbatim => 3,
        }
    }
}

/// Brings all tokens in relative position offsets
fn make_relative(mut tokens: Vec<SemanticToken>) -> Vec<SemanticToken> {
    // Bring tokens in descending order (highest line/column at first)
    tokens.sort_by(|a, b| {
        let outer = b.delta_line.partial_cmp(&a.delta_line);
        if let Some(outer_order) = outer {
            if outer_order == std::cmp::Ordering::Equal {
                return b.delta_start.partial_cmp(&a.delta_start).unwrap();
            } else {
                return outer_order;
            }
        }
        std::cmp::Ordering::Equal
    });

    let mut sorted_tokens = tokens.clone();
    for (i, token) in sorted_tokens.iter_mut().enumerate() {
        if i < tokens.len() - 1 {
            if let Some(next_token) = tokens.get(i + 1) {
                token.delta_line -= next_token.delta_line;
                if token.delta_line == 0 {
                    token.delta_start -= next_token.delta_start;
                }
            }
        }
    }

    sorted_tokens.reverse();
    sorted_tokens
}
