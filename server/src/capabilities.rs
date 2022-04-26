use lsp_types::{SemanticTokensOptions, SemanticTokensFullOptions, SemanticTokenType, SemanticTokenModifier};
use lsp_types::{
    ServerCapabilities, TextDocumentSyncCapability, TextDocumentSyncKind,
};

pub fn get_capabilities() -> ServerCapabilities {
	let mut server_capabilities = ServerCapabilities::default();

    let text_sync = Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL));
    server_capabilities.text_document_sync = text_sync;
    let token_provider = lsp_types::SemanticTokensServerCapabilities::from(SemanticTokensOptions{
        full: Some(SemanticTokensFullOptions::Bool(true)),
        legend: lsp_types::SemanticTokensLegend { token_types: vec![
                SemanticTokenType::NAMESPACE,
                SemanticTokenType::TYPE,
                SemanticTokenType::CLASS,
                SemanticTokenType::ENUM,
                SemanticTokenType::INTERFACE,
                SemanticTokenType::STRUCT,
                SemanticTokenType::TYPE_PARAMETER,
                SemanticTokenType::PARAMETER,
                SemanticTokenType::VARIABLE,
                SemanticTokenType::PROPERTY,
                SemanticTokenType::ENUM_MEMBER,
                SemanticTokenType::EVENT,
                SemanticTokenType::FUNCTION,
                SemanticTokenType::METHOD,
                SemanticTokenType::MACRO,
                SemanticTokenType::KEYWORD,
                SemanticTokenType::MODIFIER,
                SemanticTokenType::COMMENT,
                SemanticTokenType::STRING,
                SemanticTokenType::NUMBER,
                SemanticTokenType::REGEXP,
                SemanticTokenType::OPERATOR,
            ],
            token_modifiers: vec![
                SemanticTokenModifier::DECLARATION,
                SemanticTokenModifier::DEFINITION,
                SemanticTokenModifier::READONLY,
                SemanticTokenModifier::STATIC,
                SemanticTokenModifier::DEPRECATED,
                SemanticTokenModifier::ABSTRACT,
                SemanticTokenModifier::ASYNC,
                SemanticTokenModifier::MODIFICATION,
                SemanticTokenModifier::DOCUMENTATION,
                SemanticTokenModifier::DEFAULT_LIBRARY,
            ] 
        },
        ..Default::default()
    });
    server_capabilities.semantic_tokens_provider = Some(token_provider);
		server_capabilities
}
