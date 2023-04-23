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

trait TokenValue {
    fn value(&self) -> u32;
}

pub fn get_semantic_tokens_response(
    id: RequestId,
    _params: SemanticTokensParams,
    document: Option<&Document>,
) -> Response {
    let mut tokens = SemanticTokens {
        result_id: Some(id.to_string()),
        ..Default::default()
    };

    if let Some(um_doc) = document {
        tokens.data = get_semantic_tokens(um_doc);
    }

    let result = Some(SemanticTokensResult::Tokens(tokens));
    let result = serde_json::to_value(&result).unwrap();
    Response {
        id,
        result: Some(result),
        error: None,
    }
}

pub fn get_semantic_tokens(document: &Document) -> Vec<SemanticToken> {
    make_relative(document.tokens())
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

    // Note: unimarkup positions start with line = 1 and column = 1
    // LSP positions start with line = 0 and column = 0
    // => `-1` to correct this
    let mut sorted_tokens = tokens.clone();
    for (i, token) in sorted_tokens.iter_mut().enumerate() {
        if i < tokens.len() - 1 {
            if let Some(next_token) = tokens.get(i + 1) {
                token.delta_line -= next_token.delta_line;
                if token.delta_line == 0 {
                    token.delta_start -= next_token.delta_start;
                } else {
                    token.delta_start -= 1;
                }              
            }
        } else {
            token.delta_start -= 1;
            token.delta_line -= 1;
        }  
    }

    sorted_tokens.reverse();
    sorted_tokens
}
