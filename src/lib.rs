use std::collections::HashMap;
use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use unimarkup_core::document::Document;

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
use serde::Serialize;

use self::doc_sync::DocChangeWorker;
use self::semantic_tokens::get_semantic_tokens_response;

mod capabilities;
mod doc_sync;
pub mod semantic_tokens;

#[derive(Debug, Clone, Serialize)]
struct RenderedContent {
    id: Url,
    content: String,
}

pub fn run() -> Result<(), Box<dyn Error + Sync + Send>> {
    let (connection, io_threads) = Connection::stdio();

    let serialized_server_capabilities =
        serde_json::to_value(capabilities::get_capabilities()).unwrap();

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

    let (tx_um, mut rx_um) = mpsc::channel::<Document>(10);
    let (tx_doc_open, rx_doc_open) = mpsc::channel::<DidOpenTextDocumentParams>(10);
    let (tx_doc_change, rx_doc_change) = mpsc::channel::<DidChangeTextDocumentParams>(10);
    let (tx_shutdown, _rx_shutdown) = mpsc::channel::<bool>(10);

    let parsed_documents: Arc<RwLock<HashMap<Url, Document>>> =
        Arc::new(RwLock::new(HashMap::new()));
    let mut update_cnt = 0;

    let conn = Arc::new(connection);

    DocChangeWorker::init(tx_um, rx_doc_open, rx_doc_change);

    let conn2 = Arc::clone(&conn);
    let mut ren_docs = Arc::clone(&parsed_documents);
    tokio::spawn(async move {
        loop {
            if let Some(um) = rx_um.recv().await {
                update_cnt += 1;
                let _ = update_um_file(
                    um,
                    &conn2,
                    &mut ren_docs,
                    semantic_tokens_supported,
                    update_cnt,
                )
                .await;
            }
        }
    });

    loop {
        if let Ok(msg) = {
            let conn = Arc::clone(&conn);
            tokio::task::spawn_blocking(move || conn.receiver.recv().unwrap())
        }
        .await
        {
            let connection = Arc::clone(&conn);

            match handle_msg(msg, &connection)? {
                LspAction::SendSemanticTokens {
                    id,
                    params,
                    file_path,
                } => {
                    let documents = parsed_documents.read().await;
                    let document = documents.get(&Url::from_file_path(file_path).unwrap());

                    let resp = get_semantic_tokens_response(id, params, document);
                    connection.sender.send(Message::Response(resp))?;
                }
                LspAction::UpdateDoc(params) => {
                    tx_doc_change.send(params).await?;
                    continue;
                }
                LspAction::OpenDoc(params) => {
                    tx_doc_open.send(params).await?;
                    continue;
                }
                LspAction::Shutdown => {
                    tx_shutdown.send(true).await?;

                    return Ok(());
                }
                LspAction::Continue => {}
            }
        }
    }
}

async fn update_um_file(
    um: Document,
    conn: &Connection,
    rendered_documents: &mut Arc<RwLock<HashMap<Url, Document>>>,
    semantic_tokens_supported: bool,
    update_cnt: usize,
) -> Result<(), Box<dyn Error + Sync + Send>> {
    let file_id = Url::from_file_path(um.config.um_file.clone()).unwrap();
    let rendered_content = RenderedContent {
        id: file_id.clone(),
        content: um.html().body,
    };

    rendered_documents.write().await.insert(file_id, um);

    let resp = lsp_server::Notification {
        method: "extension/renderedContent".to_string(),
        params: serde_json::to_value(rendered_content).unwrap(),
    };

    conn.sender.send(Message::Notification(resp))?;

    if semantic_tokens_supported {
        conn.sender.send(Message::Request(lsp_server::Request {
            id: format!("doc-update-{}", update_cnt).into(),
            method: "workspace/semanticTokens/refresh".to_string(),
            params: serde_json::Value::Null,
        }))?;
    }

    Ok(())
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
