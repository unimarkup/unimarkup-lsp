use insta::assert_snapshot;
use unimarkup_core::commons::config::Config;
use unimarkup_lsp::semantic_tokens::get_semantic_tokens;

#[test]
fn heading_token_type() {
    let input = "# heading";
    let um = unimarkup_core::Unimarkup::parse(input, Config::default());

    let tokens = get_semantic_tokens(&um);

    assert_snapshot!(format!("{:#?}", tokens));
}

#[test]
fn heading_token_type_with_bold_token_modifier() {
    let input = "# **heading**";
    let um = unimarkup_core::Unimarkup::parse(input, Config::default());

    let tokens = get_semantic_tokens(&um);

    assert_snapshot!(format!("{:#?}", tokens));
}
