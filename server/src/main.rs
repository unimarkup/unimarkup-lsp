use std::error::Error;

use lsp_types::Url;
use serde::Serialize;

mod capabilities;
mod doc_sync;
mod semantic_tokens;
mod sync;

#[derive(Debug, Clone, Serialize)]
struct RenderedContent {
    id: Url,
    content: String,
}

fn main() -> Result<(), Box<dyn Error + Sync + Send>> {
    sync::run()
}
