use lsp_types::SemanticToken;
use unimarkup_inline::{InlineKind, Inline, NestedInline, TokenIdentifier, InlineIdentifiers};

use super::{delta_conversions::to_lsp_line_nr, OpenTokenModifier, TokenValue, TokenType, TokenModifier, NO_TOKEN_TYPE};


pub(crate) trait SemanticInlineTokenizer {
	fn tokens(&self, token_type: &TokenType, open_modifiers: &mut Vec<OpenTokenModifier>) -> Vec<SemanticToken>;
}

impl SemanticInlineTokenizer for Inline {
	fn tokens(&self, token_type: &TokenType, open_modifiers: &mut Vec<OpenTokenModifier>) -> Vec<SemanticToken> {
		let mut tokens = Vec::<SemanticToken>::new();
		for inlines in self {
			tokens.append(&mut inlines.tokens(token_type, open_modifiers))
		}

		tokens
	}
}

impl SemanticInlineTokenizer for InlineKind {
	fn tokens(&self, token_type: &TokenType, open_modifiers: &mut Vec<OpenTokenModifier>) -> Vec<SemanticToken> {
		match self {
			InlineKind::Bold(nested)
			| InlineKind::Italic(nested)
      | InlineKind::BoldItalic(nested)
      | InlineKind::Verbatim(nested) => {
        
        open_modifiers.push(self.into());
        let identifier: TokenIdentifier = self.get_identifier();

        let mut tokens = vec![
          SemanticToken{
            delta_line: to_lsp_line_nr(nested.span.start.line),
            delta_start: nested.span.start.column as u32,
            length: identifier.start.len() as u32,
            token_type: token_type.value(),
            token_modifiers_bitset: get_modifier_bitfield(open_modifiers),
          }
        ];

				tokens.append(&mut nested.tokens(token_type, open_modifiers));

				tokens.push(
					SemanticToken{
            delta_line: to_lsp_line_nr(nested.span.end.line),
            delta_start: (nested.span.end.column - identifier.end.len()) as u32,
            length: identifier.end.len() as u32,
            token_type: token_type.value(),
            token_modifiers_bitset: get_modifier_bitfield(open_modifiers),
          }
				);

				open_modifiers.pop();
				tokens
			},
			InlineKind::Plain(flat)
      | InlineKind::EscapedNewLine(flat)
      | InlineKind::EscapedSpace(flat) => {

        if token_type.value() != NO_TOKEN_TYPE || !open_modifiers.is_empty() {
          vec![
            SemanticToken{
              delta_line: to_lsp_line_nr(flat.span.start.line),
              delta_start: flat.span.start.column as u32,
              length: (flat.span.end.column - flat.span.start.column) as u32,
              token_type: token_type.value(),
              token_modifiers_bitset: get_modifier_bitfield(open_modifiers),
            }
          ]
        } else {
          vec![] 
        }
      },
      InlineKind::TextGroup(nested, _attributes) => {
        let identifier: TokenIdentifier = self.get_identifier();

        let mut tokens = vec![
          SemanticToken{
            delta_line: to_lsp_line_nr(nested.span.start.line),
            delta_start: nested.span.start.column as u32,
            length: identifier.start.len() as u32,
            token_type: token_type.value(),
            token_modifiers_bitset: get_modifier_bitfield(open_modifiers),
          }
        ];

				tokens.append(&mut nested.tokens(token_type, open_modifiers));

				tokens.push(
					SemanticToken{
            delta_line: to_lsp_line_nr(nested.span.end.line),
            delta_start: (nested.span.end.column - identifier.end.len()) as u32,
            length: identifier.end.len() as u32,
            token_type: token_type.value(),
            token_modifiers_bitset: get_modifier_bitfield(open_modifiers),
          }
				);

				tokens
      }
			_ => {
				[].into()
			}
		}
	}
}

impl TokenValue for InlineKind {
  fn value(&self) -> u32 {
    match self {
      InlineKind::Bold(_) => TokenModifier::Bold.value(),
      InlineKind::Italic(_) => TokenModifier::Italic.value(),
      InlineKind::BoldItalic(_) => TokenModifier::BoldItalic.value(),
      InlineKind::Verbatim(_) => TokenModifier::Verbatim.value(),
      _ => NO_TOKEN_TYPE,
    }
  }
}

impl From<&InlineKind> for OpenTokenModifier {
  fn from(inline: &InlineKind) -> Self {
    match inline {
      InlineKind::Bold(_) => OpenTokenModifier{ token_modifier: TokenModifier::Bold, ..Default::default() },
      InlineKind::Italic(_) => OpenTokenModifier{ token_modifier: TokenModifier::Italic, ..Default::default() },
      InlineKind::BoldItalic(_) => OpenTokenModifier{ token_modifier: TokenModifier::BoldItalic, ..Default::default() },
      InlineKind::Verbatim(_) => OpenTokenModifier{ token_modifier: TokenModifier::Verbatim, ..Default::default() },
      _ => OpenTokenModifier{ ..Default::default() }
    }
  }
}

impl SemanticInlineTokenizer for NestedInline {
	fn tokens(&self, token_type: &TokenType, open_modifiers: &mut Vec<OpenTokenModifier>) -> Vec<SemanticToken> {
		let mut tokens = Vec::<SemanticToken>::new();
		for nested in &self.content {
			tokens.append(&mut nested.tokens(token_type, open_modifiers))
		}
		tokens
	}
}

fn get_modifier_bitfield(modifiers: &Vec<OpenTokenModifier>) -> u32 {
  let mut field = 0;

  for modifier in modifiers {
    field |= modifier.token_modifier.value();
  }

  field
}
