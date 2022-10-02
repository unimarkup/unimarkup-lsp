use std::collections::HashMap;
use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;
use unimarkup_core::unimarkup::UnimarkupDocument;

use lsp_server::{Connection, Message, RequestId};
use lsp_types::notification::DidOpenTextDocument;
use lsp_types::request::SemanticTokensFullRequest;
use lsp_types::{
    notification::{DidChangeTextDocument, Notification},
    request::Request,
    InitializeParams,
};
use lsp_types::{
    DidChangeTextDocumentParams, DidOpenTextDocumentParams, SemanticTokensParams, Url,
};

use crate::{capabilities, semantic_tokens, RenderedContent};

use self::doc_sync::DocChangeWorker;

mod doc_sync;

pub(crate) fn run() -> Result<(), Box<dyn Error + Sync + Send>> {
    let (connection, io_threads) = Connection::stdio();

    let serialized_server_capabilities =
        serde_json::to_value(&capabilities::get_capabilities()).unwrap();

    let initialization_params = connection.initialize(serialized_server_capabilities)?;

    let runtime = tokio::runtime::Runtime::new()?;

    runtime.block_on(async {
        let _ = main_loop(connection, initialization_params).await;
        io_threads.join()
    })?;

    Ok(())
}

async fn main_loop(
    connection: Connection,
    params: serde_json::Value,
) -> Result<(), Box<dyn Error + Sync + Send>> {
    let params: InitializeParams = serde_json::from_value(params).unwrap();

    let mut semantic_tokens_supported = false;

    if let Some(workspace_capabilities) = params.capabilities.workspace {
        semantic_tokens_supported = workspace_capabilities.semantic_tokens.is_some();
    }

    let (tx_um, mut rx_um) = mpsc::channel::<UnimarkupDocument>(10);
    let (tx_doc_open, rx_doc_open) = mpsc::channel::<DidOpenTextDocumentParams>(10);
    let (tx_doc_change, rx_doc_change) = mpsc::channel::<DidChangeTextDocumentParams>(10);
    let (tx_shutdown, rx_shutdown) = mpsc::channel::<bool>(10);

    let mut doc_change_worker =
        DocChangeWorker::new(tx_um, rx_doc_open, rx_doc_change, rx_shutdown);

    let mut rendered_documents: HashMap<Url, UnimarkupDocument> = HashMap::new();
    let mut update_cnt = 0;

    let conn = Arc::new(connection);

    loop {
        tokio::select! {
            Ok(msg) = {
                let conn = Arc::clone(&conn);
                tokio::task::spawn_blocking(move || conn.receiver.recv().unwrap())
            } => {
                let connection = Arc::clone(&conn);

                match handle_msg(msg, &connection)? {
                    LspAction::SendSemanticTokens { id, params, file_path } => {
                        let rendered_um = rendered_documents
                            .get(&Url::from_file_path(file_path).unwrap())
                            .map(|rendered_um| (*rendered_um).clone());

                        let resp = semantic_tokens::get_semantic_tokens(id, params, rendered_um);
                        connection.sender.send(Message::Response(resp))?;
                    },
                    LspAction::UpdateDoc(params) => tx_doc_change.send(params).await?,
                    LspAction::OpenDoc(params) => tx_doc_open.send(params).await?,
                    LspAction::Shutdown => {
                        doc_change_worker.make_progress().await;
                        tx_shutdown.send(true).await?;

                        return Ok(());
                    },
                    LspAction::Continue => {},
                }

            },

            Some(um) = rx_um.recv() => {
                update_cnt += 1;

                let file_id = Url::from_file_path(um.config.um_file.clone()).unwrap();
                let rendered_content = RenderedContent {
                    id: file_id.clone(),
                    content: um.html().body(),
                };

                rendered_documents.insert(file_id, um);

                let resp = lsp_server::Notification {
                    method: "extension/renderedContent".to_string(),
                    params: serde_json::to_value(rendered_content).unwrap(),
                };
                conn.sender.send(Message::Notification(resp))?;

                if semantic_tokens_supported {
                    conn
                        .sender
                        .send(Message::Request(lsp_server::Request {
                            id: format!("doc-update-{}", update_cnt).into(),
                            method: "workspace/semanticTokens/refresh".to_string(),
                            params: serde_json::Value::Null,
                        }))?;
                }
            }

            _ = doc_change_worker.make_progress() => {
                // render document!
            }
        }
    }
}

enum LspAction {
    SendSemanticTokens {
        id: RequestId,
        params: SemanticTokensParams,
        file_path: PathBuf,
    },
    UpdateDoc(DidChangeTextDocumentParams),
    OpenDoc(DidOpenTextDocumentParams),
    Shutdown,
    Continue,
}

fn handle_msg(
    msg: Message,
    connection: &Connection,
) -> Result<LspAction, Box<dyn Error + Sync + Send>> {
    match msg {
        Message::Request(req) => {
            if connection.handle_shutdown(&req).unwrap() {
                Ok(LspAction::Shutdown)
            } else if let SemanticTokensFullRequest::METHOD = req.method.as_str() {
                if let Ok((id, params)) =
                    req.extract::<SemanticTokensParams>(SemanticTokensFullRequest::METHOD)
                {
                    let file_path = params.text_document.uri.to_file_path().unwrap();

                    Ok(LspAction::SendSemanticTokens {
                        id,
                        params,
                        file_path,
                    })
                } else {
                    Ok(LspAction::Continue)
                }
            } else {
                eprintln!("Unsupported request: {:?}", req);
                Ok(LspAction::Continue)
            }
        }
        Message::Response(resp) => {
            eprintln!("got response: {:?}", resp);
            Ok(LspAction::Continue)
        }
        Message::Notification(notification) => match notification.method.as_str() {
            DidChangeTextDocument::METHOD => {
                if let Ok(params) = notification
                    .extract::<DidChangeTextDocumentParams>(DidChangeTextDocument::METHOD)
                {
                    Ok(LspAction::UpdateDoc(params))
                } else {
                    Ok(LspAction::Continue)
                }
            }
            DidOpenTextDocument::METHOD => {
                if let Ok(params) =
                    notification.extract::<DidOpenTextDocumentParams>(DidOpenTextDocument::METHOD)
                {
                    Ok(LspAction::OpenDoc(params))
                } else {
                    Ok(LspAction::Continue)
                }
            }
            _ => {
                eprintln!("Unsupported notification: {:?}", notification);
                Ok(LspAction::Continue)
            }
        },
    }
}
