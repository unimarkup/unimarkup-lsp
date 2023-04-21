use lsp_types::DidChangeTextDocumentParams;
use lsp_types::DidOpenTextDocumentParams;
use tokio::sync::mpsc::Receiver;
use tokio::sync::mpsc::Sender;
use unimarkup_core::config::Config;
use unimarkup_core::config::OutputFormat;
use unimarkup_core::document::Document;

pub(crate) struct DocChangeWorker;

impl DocChangeWorker {
    pub(crate) fn init(
        tx_um: Sender<Document>,
        rx_doc_open: Receiver<DidOpenTextDocumentParams>,
        rx_doc_change: Receiver<DidChangeTextDocumentParams>,
    ) {
        let config = Config {
            out_formats: Some(vec![OutputFormat::Html]),
            ..Default::default()
        };

        let tx_um_clone = tx_um.clone();
        let config_clone = config.clone();
        tokio::spawn(
            async move { Self::doc_open_loop(tx_um_clone, rx_doc_open, config_clone).await },
        );
        tokio::spawn(async move { Self::doc_change_loop(tx_um, rx_doc_change, config).await });
    }

    async fn doc_open_loop(
        tx_um: Sender<Document>,
        mut rx_doc_open: Receiver<DidOpenTextDocumentParams>,
        mut config: Config,
    ) {
        loop {
            if let Some(opened_doc) = rx_doc_open.recv().await {
                config.um_file = opened_doc.text_document.uri.to_file_path().unwrap();

                if let Ok(rendered_doc) = unimarkup_core::unimarkup::compile(
                    &opened_doc.text_document.text.clone(),
                    config.clone(),
                ) {
                    let _ = tx_um.send(rendered_doc).await;
                }
            }
        }
    }

    async fn doc_change_loop(
        tx_um: Sender<Document>,
        mut rx_doc_change: Receiver<DidChangeTextDocumentParams>,
        mut config: Config,
    ) {
        loop {
            if let Some(changes) = rx_doc_change.recv().await {
                config.um_file = changes.text_document.uri.to_file_path().unwrap();

                if let Ok(rendered_doc) = unimarkup_core::unimarkup::compile(
                    &changes.content_changes[0].text.clone(),
                    config.clone(),
                ) {
                    let _ = tx_um.send(rendered_doc).await;
                }
            }
        }
    }
}
