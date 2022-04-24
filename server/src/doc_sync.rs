use unimarkup_core::config::Config;
use unimarkup_core::unimarkup::UnimarkupDocument;
use lsp_types::DidChangeTextDocumentParams;
use lsp_types::DidOpenTextDocumentParams;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use unimarkup_core::config::OutputFormat;


pub(crate) fn doc_change_loop(tx_um: Sender<UnimarkupDocument>, rx_doc_open: Receiver<DidOpenTextDocumentParams>,rx_doc_change: Receiver<DidChangeTextDocumentParams>, rx_shutdown: Receiver<bool>) {
	let mut config = Config::default();
	config.out_formats = Some(vec![OutputFormat::Html]);
	
	if let Ok(opened_doc) = rx_doc_open.recv() {
		config.um_file = opened_doc.text_document.uri.to_file_path().unwrap();
		let rendered_doc = unimarkup_core::unimarkup::compile(&opened_doc.text_document.text, config.clone()).unwrap();
		tx_um.send(rendered_doc).unwrap();
	}
	
	while rx_shutdown.try_recv().is_err() {
		if let Ok(changes) = rx_doc_change.recv() {
			config.um_file = changes.text_document.uri.to_file_path().unwrap();
			let rendered_doc = unimarkup_core::unimarkup::compile(&changes.content_changes[0].text, config.clone()).unwrap();
			tx_um.send(rendered_doc).unwrap();
		} else if let Ok(opened_doc) = rx_doc_open.recv() {
			config.um_file = opened_doc.text_document.uri.to_file_path().unwrap();
			let rendered_doc = unimarkup_core::unimarkup::compile(&opened_doc.text_document.text, config.clone()).unwrap();
			tx_um.send(rendered_doc).unwrap();
		}
	}
}
