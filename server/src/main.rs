use std::collections::HashMap;
use unimarkup_core::unimarkup::UnimarkupDocument;
use std::error::Error;
use std::sync::mpsc;
use std::thread;

use lsp_types::request::SemanticTokensFullRequest;
use lsp_types::{DidChangeTextDocumentParams, DidOpenTextDocumentParams, SemanticTokensParams};
use lsp_types::notification::DidOpenTextDocument;
use lsp_types::{
    request::Request, InitializeParams, notification::{DidChangeTextDocument, Notification},
};
use lsp_server::{Connection, Message};
use serde::Serialize;

mod capabilities;
mod doc_sync;
mod semantic_tokens;

#[derive(Debug, Clone, Serialize)]
struct RenderedContent {
    id: String,
    content: String,
}

fn main() -> Result<(), Box<dyn Error + Sync + Send>> {
    let (connection, io_threads) = Connection::stdio();

	let serialized_server_capabilities = serde_json::to_value(&capabilities::get_capabilities()).unwrap();
	let initialization_params = connection.initialize(serialized_server_capabilities)?;
	main_loop(connection, initialization_params)?;
	io_threads.join()?;

    Ok(())
}

fn main_loop(
    connection: Connection,
    params: serde_json::Value,
) -> Result<(), Box<dyn Error + Sync + Send>> {
    let _params: InitializeParams = serde_json::from_value(params).unwrap();

    let (tx_um, rx_um) = mpsc::channel::<UnimarkupDocument>();
    let (tx_doc_open, rx_doc_open) = mpsc::channel::<DidOpenTextDocumentParams>();
    let (tx_doc_change, rx_doc_change) = mpsc::channel::<DidChangeTextDocumentParams>();
    let (tx_shutdown, rx_shutdown) = mpsc::channel::<bool>();

    let doc_change_worker = thread::spawn(move || {
        doc_sync::doc_change_loop(tx_um, rx_doc_open, rx_doc_change, rx_shutdown);
    });

    let mut rendered_documents: HashMap<String, UnimarkupDocument> = HashMap::new();
    let mut update_cnt = 0;

    loop {
        if let Ok(msg) = connection.receiver.try_recv() {
            match msg {
                Message::Request(req) => {
                    if connection.handle_shutdown(&req)? {
                        tx_shutdown.send(true)?;
                        doc_change_worker.join().unwrap();
                        return Ok(());
                    }

                    match req.method.as_str() {
                        SemanticTokensFullRequest::METHOD => {
                            let resp;

                            if let Ok((id, params)) = req.extract::<SemanticTokensParams>(SemanticTokensFullRequest::METHOD) {
                                let file_path = params.text_document.uri.to_file_path().unwrap();
                                if let Some(rendered_um) = rendered_documents.get(&file_path.to_string_lossy().to_string()) {
                                    resp = semantic_tokens::get_semantic_tokens(id, params, Some(rendered_um));
                                } else {
                                    resp = semantic_tokens::get_semantic_tokens(id, params, None);
                                }
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
            update_cnt += 1;

            let file_id = um.config.um_file.to_string_lossy().to_string();
            let rendered_content = RenderedContent {
                id: file_id.clone(),
                content: um.html().body(),
            };

            rendered_documents.insert(file_id, um);

            let resp = lsp_server::Notification{
                method: "extension/renderedContent".to_string(),
                params: serde_json::to_value(rendered_content).unwrap(),
            };
            connection.sender.send(Message::Notification(resp))?;

            connection.sender.send(Message::Request(
                lsp_server::Request{
                    id: format!("doc-update-{}", update_cnt).into(),
                    method: "workspace/semanticTokens/refresh".to_string(),
                    params: serde_json::Value::Null,
                }
            ))?;
        }
    }
}
