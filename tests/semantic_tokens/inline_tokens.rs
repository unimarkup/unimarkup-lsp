use insta::assert_snapshot;
use unimarkup_core::commons::config::Config;
use unimarkup_lsp::semantic_tokens::get_semantic_tokens;

#[test]
fn bold_token_modifier() {
    let input = "**bold text**";
    let um = unimarkup_core::Unimarkup::parse(input, Config::default());

    let tokens = get_semantic_tokens(&um);

    assert_snapshot!(format!("{:#?}", tokens));
}

#[test]
fn bold_token_modifier_before_plain() {
    let input = "**bold text**plain text";
    let um = unimarkup_core::Unimarkup::parse(input, Config::default());

    let tokens = get_semantic_tokens(&um);

    assert_snapshot!(format!("{:#?}", tokens));
}

#[test]
fn bold_token_modifier_after_plain() {
    let input = "plain text**bold text**";
    let um = unimarkup_core::Unimarkup::parse(input, Config::default());

    let tokens = get_semantic_tokens(&um);

    assert_snapshot!(format!("{:#?}", tokens));
}

#[test]
fn italic_token_modifier() {
    let input = "*italic text*";
    let um = unimarkup_core::Unimarkup::parse(input, Config::default());

    let tokens = get_semantic_tokens(&um);

    assert_snapshot!(format!("{:#?}", tokens));
}

#[test]
fn bold_italic_token_modifier() {
    let input = "***bold and italic text***";
    let um = unimarkup_core::Unimarkup::parse(input, Config::default());

    let tokens = get_semantic_tokens(&um);

    assert_snapshot!(format!("{:#?}", tokens));
}
