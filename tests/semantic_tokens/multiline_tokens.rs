use insta::assert_snapshot;
use unimarkup_core::config::Config;
use unimarkup_lsp::semantic_tokens::get_semantic_tokens;

#[test]
fn bold_token_modifier_over_two_lines() {
    let input = "**bold\ntext**";
    let document = unimarkup_core::unimarkup::compile(input, Config::default()).unwrap();

    let tokens = get_semantic_tokens(&document);

    assert_snapshot!(format!("{:#?}", tokens));
}
