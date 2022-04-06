use std::{error::Error, sync::mpsc::Receiver};
use std::sync::mpsc::{self, Sender};
use std::thread;

use lsp_types::request::SemanticTokensFullRequest;
use lsp_types::{DidChangeTextDocumentParams, LogMessageParams, MessageType, DidOpenTextDocumentParams, SemanticTokensOptions, SemanticTokensFullOptions, SemanticToken, SemanticTokens, SemanticTokensParams, SemanticTokensResult, SemanticTokenType, SemanticTokenModifier};
use lsp_types::notification::{LogMessage, DidOpenTextDocument};
use lsp_types::{
    request::{GotoDefinition, Completion, Request}, GotoDefinitionResponse, InitializeParams, ServerCapabilities, CompletionResponse, CompletionItem, CompletionItemKind, Location, GotoDefinitionParams, CompletionParams, TextDocumentSyncCapability, TextDocumentSyncKind, notification::{DidChangeTextDocument, Notification},
};

use lsp_server::{Connection, Message, Response};

fn main() -> Result<(), Box<dyn Error + Sync + Send>> {
    // Note that  we must have our logging only write out to stderr.
    eprintln!("starting generic LSP server");

    // Create the transport. Includes the stdio (stdin and stdout) versions but this could
    // also be implemented to use sockets or HTTP.
    let (connection, io_threads) = Connection::stdio();

    // Run the server and wait for the two threads to end (typically by trigger LSP Exit event).
    let mut server_capabilities = ServerCapabilities::default();

    let text_sync = Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::INCREMENTAL));
    server_capabilities.text_document_sync = text_sync;
    // only support completion for now
    let completion_provider = lsp_types::CompletionOptions { ..Default::default() };
    server_capabilities.definition_provider = Some(lsp_types::OneOf::Left(true));
    server_capabilities.completion_provider = Some(completion_provider);
    let token_provider = lsp_types::SemanticTokensServerCapabilities::from(SemanticTokensOptions{
        full: Some(SemanticTokensFullOptions::Bool(true)),
        legend: lsp_types::SemanticTokensLegend { token_types: vec![
                SemanticTokenType::KEYWORD,
                SemanticTokenType::FUNCTION,
                SemanticTokenType::STRING,
            ],
            token_modifiers: vec![
                SemanticTokenModifier::DECLARATION,
            ] 
        },
        ..Default::default()
    });
    server_capabilities.semantic_tokens_provider = Some(token_provider);
    let serialized_server_capabilities = serde_json::to_value(&server_capabilities).unwrap();
    let initialization_params = connection.initialize(serialized_server_capabilities)?;
    main_loop(connection, initialization_params)?;
    io_threads.join()?;

    // Shut down gracefully.
    eprintln!("shutting down server");
    Ok(())
}

fn main_loop(
    connection: Connection,
    params: serde_json::Value,
) -> Result<(), Box<dyn Error + Sync + Send>> {
    let _params: InitializeParams = serde_json::from_value(params).unwrap();
    eprintln!("{:#?}",_params.capabilities.text_document.unwrap().semantic_tokens.unwrap().token_types);

    let (tx_um, rx_um) = mpsc::channel::<String>();
    let (tx_doc_open, rx_doc_open) = mpsc::channel::<DidOpenTextDocumentParams>();
    let (tx_doc_change, rx_doc_change) = mpsc::channel::<DidChangeTextDocumentParams>();
    let (tx_shutdown, rx_shutdown) = mpsc::channel::<bool>();

    // Start doc-change thread
    let doc_change_worker = thread::spawn(move || {
        doc_change_loop(tx_um, rx_doc_open, rx_doc_change, rx_shutdown);
    });

    eprintln!("starting example main loop");
    loop {
        if let Ok(msg) = connection.receiver.try_recv() {
            eprintln!("got msg: {:?}", msg);
            match msg {
                Message::Request(req) => {
                    if connection.handle_shutdown(&req)? {
                        tx_shutdown.send(true)?;
                        doc_change_worker.join().unwrap();
                        return Ok(());
                    }
                    eprintln!("got request: {:?}", req);

                    match req.method.as_str() {
                        Completion::METHOD => {
                            if let Ok((id, params)) = req.extract::<CompletionParams>(Completion::METHOD) {
                                eprintln!("got completion request #{}: {:?}", id, params);

                                // respond with static completion items for now
                                let completion_items = vec![
                                    CompletionItem{ label: "Item 1".to_string(), kind: Some(CompletionItemKind::TEXT), 
                                        detail: Some("First item".to_string()), data: Some(1.into()),
                                        ..Default::default()
                                    }
                                ];

                                let result = Some(CompletionResponse::Array(completion_items));
                                let result = serde_json::to_value(&result).unwrap();
                                let resp = Response { id, result: Some(result), error: None };
                                connection.sender.send(Message::Response(resp))?;
                            }                        
                        },
                        GotoDefinition::METHOD => {
                            if let Ok((id, params)) = req.extract::<GotoDefinitionParams>(GotoDefinition::METHOD) {
                                eprintln!("got gotoDefinition request #{}: {:?}", id, params);

                                let definitions = vec![
                                    Location{ uri: params.text_document_position_params.text_document.uri, range: lsp_types::Range{ start: lsp_types::Position { line: 1, character: 1 }, end: lsp_types::Position { line: 1, character: 1 } } }
                                ];

                                let result = Some(GotoDefinitionResponse::Array(definitions));
                                let result = serde_json::to_value(&result).unwrap();
                                let resp = Response { id, result: Some(result), error: None };
                                connection.sender.send(Message::Response(resp))?;
                            }
                        },
                        SemanticTokensFullRequest::METHOD => {
                            if let Ok((id, params)) = req.extract::<SemanticTokensParams>(SemanticTokensFullRequest::METHOD) {
                                eprintln!("got semantic token request #{}: {:?}", id, params);

                                let result = Some(SemanticTokensResult::Tokens(
                                    SemanticTokens { result_id: Some(id.to_string()), data: vec![
                                        SemanticToken{ delta_line: 0, delta_start: 0, length: 10, token_type: 0, token_modifiers_bitset: 0 },
                                        SemanticToken{ delta_line: 0, delta_start: 2, length: 5, token_type: 1, token_modifiers_bitset: 0 },
                                        SemanticToken{ delta_line: 1, delta_start: 15, length: 3, token_type: 2, token_modifiers_bitset: 0 },
                                    ] }
                                ));

                                let result = serde_json::to_value(&result).unwrap();
                                let resp = Response { id, result: Some(result), error: None };
                                connection.sender.send(Message::Response(resp))?;
                            }
                        }
                        _ => {
                            eprintln!("Unsupported request: {:?}", req);
                        }
                    }
                }
                Message::Response(resp) => {
                    eprintln!("got response: {:?}", resp);
                }
                Message::Notification(notification) => {
                    eprintln!("got notification: {:?}", notification);
                    match notification.method.as_str() {
                        DidChangeTextDocument::METHOD => {
                            if let Ok(params) = notification.extract::<DidChangeTextDocumentParams>(DidChangeTextDocument::METHOD) {
                                tx_doc_change.send(params)?;
                            }
                        },
                        DidOpenTextDocument::METHOD => {
                            if let Ok(params) = notification.extract::<DidOpenTextDocumentParams>(DidOpenTextDocument::METHOD) {
                                tx_doc_open.send(params)?;
                            }
                        }
                        _ => {
                            eprintln!("Unsupported notification: {:?}", notification);
                        }
                    }
                }
            }
        }
    
        // Check if doc-change thread sent updates
        if let Ok(um) = rx_um.try_recv() {
            let result = Some(LogMessageParams{ typ: MessageType::INFO, message: um });
            let result = serde_json::to_value(&result).unwrap();
            let resp = lsp_server::Notification { method: LogMessage::METHOD.to_string(), params: result };
            connection.sender.send(Message::Notification(resp))?;
            //Note: Instead of LogMessage, the actual uri to the renderedFile could be sent and ShowDocument set
        }
    }
}


fn doc_change_loop(tx_um: Sender<String>, rx_doc_open: Receiver<DidOpenTextDocumentParams>,rx_doc_change: Receiver<DidChangeTextDocumentParams>, rx_shutdown: Receiver<bool>) {
    if let Ok(opened_doc) = rx_doc_open.recv() {
        tx_um.send(format!("Orig content: {:#?}", opened_doc.text_document.text)).unwrap();
    }
    
    while rx_shutdown.try_recv().is_err() {
        if let Ok(changes) = rx_doc_change.recv() {
            tx_um.send(format!("Changes: {:#?}", changes.content_changes)).unwrap();
        }
    }
}
