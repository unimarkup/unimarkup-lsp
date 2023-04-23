use std::error::Error;

fn main() -> Result<(), Box<dyn Error + Sync + Send>> {
    unimarkup_lsp::run()
}
