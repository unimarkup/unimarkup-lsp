use lsp_types::DidChangeTextDocumentParams;
use lsp_types::DidOpenTextDocumentParams;
use tokio::sync::mpsc::Receiver;
use tokio::sync::mpsc::Sender;
use unimarkup_core::config::Config;
use unimarkup_core::config::OutputFormat;
use unimarkup_core::unimarkup::UnimarkupDocument;

pub(crate) struct DocChangeWorker {
    tx_um: Sender<UnimarkupDocument>,
    rx_doc_open: Receiver<DidOpenTextDocumentParams>,
    rx_doc_change: Receiver<DidChangeTextDocumentParams>,
    rx_shutdown: Receiver<bool>,
}

impl DocChangeWorker {
    pub(crate) fn new(
        tx_um: Sender<UnimarkupDocument>,
        rx_doc_open: Receiver<DidOpenTextDocumentParams>,
        rx_doc_change: Receiver<DidChangeTextDocumentParams>,
        rx_shutdown: Receiver<bool>,
    ) -> Self {
        Self {
            tx_um,
            rx_doc_open,
            rx_doc_change,
            rx_shutdown,
        }
    }

    pub(crate) async fn make_progress(&mut self) {
        self.doc_change_loop().await
    }

    async fn doc_change_loop(&mut self) {
        let mut config = Config {
            out_formats: Some(vec![OutputFormat::Html]),
            ..Default::default()
        };

        loop {
            tokio::select! {
                _ = self.rx_shutdown.recv() => return,
                Some(changes) = self.rx_doc_change.recv() => {
                    config.um_file = changes.text_document.uri.to_file_path().unwrap();

                    let rendered_doc = unimarkup_core::unimarkup::compile(
                        &changes.content_changes[0].text.clone(),
                        config.clone(),
                    )
                    .unwrap();

                    let _ = self.tx_um.send(rendered_doc).await;
                },
                Some(opened_doc) = self.rx_doc_open.recv() => {
                    config.um_file = opened_doc.text_document.uri.to_file_path().unwrap();

                    let rendered_doc = unimarkup_core::unimarkup::compile(
                        &opened_doc.text_document.text.clone(),
                        config.clone(),
                    )
                    .unwrap();

                    let _ = self.tx_um.send(rendered_doc).await;
                }
            }
        }
    }
}
