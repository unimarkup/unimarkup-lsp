use std::collections::HashSet;

use lsp_types::DidChangeTextDocumentParams;
use lsp_types::DidOpenTextDocumentParams;
use tokio::sync::mpsc::Receiver;
use tokio::sync::mpsc::Sender;
use unimarkup_core::commons::config::output::Output;
use unimarkup_core::commons::config::output::OutputFormatKind;
use unimarkup_core::commons::config::Config;
use unimarkup_core::Unimarkup;

pub(crate) struct DocChangeWorker;

impl DocChangeWorker {
    pub(crate) fn init(
        tx_um: Sender<Unimarkup>,
        rx_doc_open: Receiver<DidOpenTextDocumentParams>,
        rx_doc_change: Receiver<DidChangeTextDocumentParams>,
    ) {
        let config = Config {
            output: Output {
                formats: HashSet::from_iter(vec![OutputFormatKind::Html]),
                ..Default::default()
            },
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
        tx_um: Sender<Unimarkup>,
        mut rx_doc_open: Receiver<DidOpenTextDocumentParams>,
        mut config: Config,
    ) {
        loop {
            if let Some(opened_doc) = rx_doc_open.recv().await {
                config.input = opened_doc.text_document.uri.to_file_path().unwrap();

                let rendered_doc = unimarkup_core::Unimarkup::parse(
                    &opened_doc.text_document.text.clone(),
                    config.clone(),
                );
                let _ = tx_um.send(rendered_doc).await;
            }
        }
    }

    async fn doc_change_loop(
        tx_um: Sender<Unimarkup>,
        mut rx_doc_change: Receiver<DidChangeTextDocumentParams>,
        mut config: Config,
    ) {
        loop {
            if let Some(changes) = rx_doc_change.recv().await {
                config.input = changes.text_document.uri.to_file_path().unwrap();

                let rendered_doc = unimarkup_core::Unimarkup::parse(
                    &changes.content_changes[0].text.clone(),
                    config.clone(),
                );
                let _ = tx_um.send(rendered_doc).await;
            }
        }
    }
}
