use insta::assert_snapshot;
use unimarkup_core::commons::config::Config;
use unimarkup_lsp::semantic_tokens::get_semantic_tokens;

#[test]
fn bold_token_modifier_in_two_blocks() {
    let input = "**bold text**\n\n**other bold text**";
    let um = unimarkup_core::Unimarkup::parse(input, Config::default());

    let tokens = get_semantic_tokens(&um);

    assert_snapshot!(format!("{:#?}", tokens));
}
