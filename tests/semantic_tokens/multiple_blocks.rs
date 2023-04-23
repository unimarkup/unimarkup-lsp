use insta::assert_snapshot;
use unimarkup_core::config::Config;
use unimarkup_lsp::semantic_tokens::get_semantic_tokens;

#[test]
fn bold_token_modifier_in_two_blocks() {
    let input = "**bold text**\n\n**other bold text**";
    let document = unimarkup_core::unimarkup::compile(input, Config::default()).unwrap();

    let tokens = get_semantic_tokens(&document);

    assert_snapshot!(format!("{:#?}", tokens));
}
