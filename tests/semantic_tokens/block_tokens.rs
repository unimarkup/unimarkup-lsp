use insta::assert_snapshot;
use unimarkup_core::config::Config;
use unimarkup_lsp::semantic_tokens::get_semantic_tokens;



#[test]
fn heading_token_type() {
  let input = "# heading";
  let document = unimarkup_core::unimarkup::compile(input, Config::default()).unwrap();

  let tokens = get_semantic_tokens(&document);

  assert_snapshot!(format!("{:#?}", tokens));
}

#[test]
fn heading_token_type_with_bold_token_modifier() {
  let input = "# **heading**";
  let document = unimarkup_core::unimarkup::compile(input, Config::default()).unwrap();

  let tokens = get_semantic_tokens(&document);

  assert_snapshot!(format!("{:#?}", tokens));
}

