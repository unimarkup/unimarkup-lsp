use unimarkup_inline::Span;
use unimarkup_inline::NestedInline;
use unimarkup_inline::InlineKind;
use unimarkup_inline::Inline;
use unimarkup_core::elements::VerbatimBlock;
use unimarkup_core::elements::ParagraphBlock;
use unimarkup_core::elements::HeadingBlock;
use unimarkup_core::unimarkup::UnimarkupDocument;
use unimarkup_core::unimarkup_block::UnimarkupBlockKind;
use lsp_server::RequestId;
use lsp_types::SemanticToken;
use lsp_types::SemanticTokens;
use lsp_types::SemanticTokensResult;
use lsp_server::Response;
use lsp_types::SemanticTokensParams;

pub fn get_semantic_tokens(id: RequestId, params: SemanticTokensParams, rendered_um: Option<&UnimarkupDocument>) -> Response {
	eprintln!("got semantic token request #{}: {:?}", id, params);
	let mut tokens = SemanticTokens {
		result_id: Some(id.to_string()),
		..Default::default()
	};

	if let Some(um_doc) = rendered_um {
		tokens.data = make_relative(um_doc.tokens());
		eprintln!("Sent tokens: {:?}", tokens.data);
	}

	let result = Some(SemanticTokensResult::Tokens(tokens));

	let result = serde_json::to_value(&result).unwrap();
	Response { id, result: Some(result), error: None }
}

/// Brings all tokens in relative position offsets
fn make_relative(mut tokens: Vec<SemanticToken>) -> Vec<SemanticToken> {
	tokens.sort_by(|b, a| {
		let outer = a.delta_line.partial_cmp(&b.delta_line);
		if let Some(outer_order) = outer {
			if outer_order == std::cmp::Ordering::Equal {
				return a.delta_start.partial_cmp(&b.delta_start).unwrap();
			} else {
				return outer_order;
			}
		}
		std::cmp::Ordering::Equal
	});

	let mut sorted_tokens = tokens.clone();
	eprintln!("Tokens: {:?}", sorted_tokens);
	for (i, token) in sorted_tokens.iter_mut().enumerate() {
		if i < tokens.len() - 1 {
			if let Some(next_token) = tokens.get(i + 1) {
				token.delta_line = token.delta_line - next_token.delta_line;
				if token.delta_line == 0 {
					token.delta_start = token.delta_start - next_token.delta_start;
				}
			}
		}
	}

	sorted_tokens.reverse();
	sorted_tokens
}

trait SemanticBlockTokenizer {
	fn tokens(&self) -> Vec<SemanticToken>;
}

impl SemanticBlockTokenizer for UnimarkupDocument {
	fn tokens(&self) -> Vec<SemanticToken> { 
		let mut tokens = Vec::<SemanticToken>::new();
		for block in &self.elements {
			tokens.append(&mut block.tokens());
		}
		tokens
	}
}

impl SemanticBlockTokenizer for UnimarkupBlockKind {
	fn tokens(&self) -> Vec<SemanticToken> { 
		match self {
			UnimarkupBlockKind::Heading(heading) => {
				return heading.tokens();
			},
			UnimarkupBlockKind::Paragraph(paragraph) => {
				return paragraph.tokens();
			},
			UnimarkupBlockKind::Verbatim(verbatim) => {
				return verbatim.tokens();
			}
		}	
	}
}

impl SemanticBlockTokenizer for HeadingBlock {
	fn tokens(&self) -> Vec<SemanticToken> {
		let mut open_tokens = vec![(3,0)];
		let mut tokens = self.content.tokens(&mut open_tokens);
		let last_inline = self.content.last().unwrap();
		let last_pos;
		match last_inline {
			InlineKind::Bold(nested)
			| InlineKind::Italic(nested)
			| InlineKind::BoldItalic(nested) => { last_pos = nested.span.end },
			InlineKind::Verbatim(flat)
			| InlineKind::Plain(flat)
			| InlineKind::PlainNewLine(flat)
			| InlineKind::EscapedNewLine(flat)
			| InlineKind::EscapedSpace(flat) => { last_pos = flat.span.end },
		};

		tokens.push(
			SemanticToken{
				delta_line: to_lsp_line_nr(last_pos.line),
				delta_start: 0,
				length: last_pos.column as u32,
				token_type: 3,
				token_modifiers_bitset: 0
			}
		);

		tokens
	}
}


impl SemanticBlockTokenizer for ParagraphBlock {
	fn tokens(&self) -> Vec<SemanticToken> {
		let mut open_tokens = vec![].into();
		self.content.tokens(&mut open_tokens)
	}
}

impl SemanticBlockTokenizer for VerbatimBlock {
	fn tokens(&self) -> Vec<SemanticToken> {
		let mut tokens = vec![
			SemanticToken{
				delta_line: to_lsp_line_nr(self.line_nr),
				delta_start: 0,
				length: 10,
				token_type: self.token_type(),
				token_modifiers_bitset: 0
			}
		];

		let lines = self.content.lines();
		for (i, line) in lines.enumerate() {
			tokens.push(
				SemanticToken{
					delta_line: to_lsp_line_nr(self.line_nr + i + 1),
					delta_start: 0,
					length: (line.len() as u32),
					token_type: self.token_type(),
					token_modifiers_bitset: 0
				}
			);
		}

		tokens.push(
			SemanticToken{
				delta_line: to_lsp_line_nr(self.line_nr + self.content.lines().count() + 1),
				delta_start: 0,
				length: 10,
				token_type: self.token_type(),
				token_modifiers_bitset: 0
			}
		);

		tokens
	}
}

/// Function converts a Unimarkup line number to LSP by starting at 0
fn to_lsp_line_nr(line_nr: usize) -> u32 {
	let nr = line_nr as u32;
	nr - 1
}

trait SemanticInlineTokenizer {
	fn tokens(&self, open_token_types: &mut Vec<(u32, u32)>) -> Vec<SemanticToken>;
}

impl SemanticInlineTokenizer for Inline {
	fn tokens(&self, open_token_types: &mut Vec<(u32, u32)>) -> Vec<SemanticToken> {
		let mut tokens = Vec::<SemanticToken>::new();
		for inlines in self {
			tokens.append(&mut inlines.tokens(open_token_types))
		}

		tokens
	}
}

impl SemanticInlineTokenizer for InlineKind {
	fn tokens(&self, open_token_types: &mut Vec<(u32, u32)>) -> Vec<SemanticToken> {
		
		match self {
			InlineKind::Bold(nested)
			| InlineKind::Italic(nested) => {
				open_token_types.push((self.token_type(), nested.span.start.column as u32));

				let mut nested_tokens = nested.tokens(open_token_types);
				nested_tokens.push(
					SemanticToken{ 
						delta_line: to_lsp_line_nr(nested.span.end.line),
						delta_start: get_delta_start(nested.span),
						length: get_token_length(nested.span),
						token_type: self.token_type(),
						..Default::default()
					}
				);

				open_token_types.pop();

				return nested_tokens;
			},
			InlineKind::Plain(_) => [].into(),
			InlineKind::PlainNewLine(flat) => {
				let mut tokens = Vec::new();
				for (token_type, start_col) in open_token_types.iter_mut() {
					tokens.push(
						SemanticToken{
							delta_line: to_lsp_line_nr(flat.span.start.line),
							delta_start: *start_col,
							length: flat.span.start.column as u32,
							token_type: *token_type,
							..Default::default()
						}
					);

					if *start_col != 0 {
						*start_col = 0;
					}
				}

				tokens
			},
			_ => {
				[].into()
			}
		}
	}
}

fn get_delta_start(span: Span) -> u32 {
	if span.start.line == span.end.line {
		span.start.column as u32
	} else {
		0
	}
}

fn get_token_length(span: Span) -> u32 {
	if span.start.line == span.end.line {
		(span.end.column - span.start.column) as u32
	} else {
		span.end.column as u32
	}
}

impl SemanticInlineTokenizer for NestedInline {
	fn tokens(&self, open_token_types: &mut Vec<(u32, u32)>) -> Vec<SemanticToken> {
		let mut tokens = Vec::<SemanticToken>::new();
		for nested in &self.content {
			tokens.append(&mut nested.tokens(open_token_types))
		}
		tokens
	}
}



trait SemanticTokenMapType {
	fn token_type(&self) -> u32;
}

const NO_TOKEN_TYPE: u32 = u32::max_value();

impl SemanticTokenMapType for InlineKind {	
	fn token_type(&self) -> u32 {
		match self {
			InlineKind::Bold(_) => 1,
			InlineKind::Italic(_) => 2,
			InlineKind::Verbatim(_) => 18,
			_ => NO_TOKEN_TYPE,
		}
	}
}

impl SemanticTokenMapType for UnimarkupBlockKind {
	fn token_type(&self) -> u32 {
		match self {
			UnimarkupBlockKind::Heading(_) => 16,
			UnimarkupBlockKind::Verbatim(_) => 18,
			_ => NO_TOKEN_TYPE,
		}
	}
}

impl SemanticTokenMapType for VerbatimBlock {
	fn token_type(&self) -> u32 {
		18
	}
}
