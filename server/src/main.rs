use std::error::Error;

use lsp_types::Url;
use serde::Serialize;

mod asynchron;
mod capabilities;
mod semantic_tokens;
mod sync;

#[derive(Debug, Clone, Serialize)]
struct RenderedContent {
    id: Url,
    content: String,
}

struct Config {
    run_async: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self { run_async: true }
    }
}

fn main() -> Result<(), Box<dyn Error + Sync + Send>> {
    let config = Config::default();

    if config.run_async {
        asynchron::run()
    } else {
        sync::run()
    }
}
