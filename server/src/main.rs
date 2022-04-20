use unimarkup_core::unimarkup::UnimarkupDocument;
use std::error::Error;
use std::sync::mpsc;
use std::thread;

use lsp_types::request::SemanticTokensFullRequest;
use lsp_types::{DidChangeTextDocumentParams, LogMessageParams, MessageType, DidOpenTextDocumentParams, SemanticTokensParams};
use lsp_types::notification::{LogMessage, DidOpenTextDocument};
use lsp_types::{
    request::Request, InitializeParams, notification::{DidChangeTextDocument, Notification},
};
use lsp_server::{Connection, Message};

mod capabilities;
mod doc_sync;
mod semantic_tokens;

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
    eprintln!("{:#?}",_params.capabilities.text_document.unwrap().semantic_tokens.unwrap().token_types);

    let (tx_um, rx_um) = mpsc::channel::<UnimarkupDocument>();
    let (tx_doc_open, rx_doc_open) = mpsc::channel::<DidOpenTextDocumentParams>();
    let (tx_doc_change, rx_doc_change) = mpsc::channel::<DidChangeTextDocumentParams>();
    let (tx_shutdown, rx_shutdown) = mpsc::channel::<bool>();

    let doc_change_worker = thread::spawn(move || {
        doc_sync::doc_change_loop(tx_um, rx_doc_open, rx_doc_change, rx_shutdown);
    });

    let mut rendered_um: Option<UnimarkupDocument> = None;
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
                            if let Ok((id, params)) = req.extract::<SemanticTokensParams>(SemanticTokensFullRequest::METHOD) {
                                let resp = semantic_tokens::get_semantic_tokens(id, params, &rendered_um);
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
            let result = Some(LogMessageParams{ typ: MessageType::INFO, message: um.html().body() });
            let result = serde_json::to_value(&result).unwrap();
            let resp = lsp_server::Notification { method: LogMessage::METHOD.to_string(), params: result };
            connection.sender.send(Message::Notification(resp))?;
            //Note: Instead of LogMessage, the actual uri to the renderedFile could be sent and ShowDocument set

            connection.sender.send(Message::Request(
                lsp_server::Request{
                    id: format!("doc-update-{}", update_cnt).into(),
                    method: "workspace/semanticTokens/refresh".to_string(),
                    params: serde_json::Value::Null,
                }
            ))?;

            rendered_um = Some(um);
        }
    }
}
