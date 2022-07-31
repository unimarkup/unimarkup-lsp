use lsp_types::SemanticToken;
use unimarkup_core::{
    elements::{HeadingBlock, ParagraphBlock, VerbatimBlock},
    unimarkup::UnimarkupDocument,
    unimarkup_block::UnimarkupBlockKind,
};

use super::{
    delta_conversions::to_lsp_line_nr, inline_tokens::SemanticInlineTokenizer, OpenTokenType,
    TokenType, TokenValue,
};

pub(crate) trait SemanticBlockTokenizer {
    fn tokens(&self, open_types: &mut Vec<OpenTokenType>) -> Vec<SemanticToken>;
}

impl SemanticBlockTokenizer for UnimarkupDocument {
    fn tokens(&self, open_types: &mut Vec<OpenTokenType>) -> Vec<SemanticToken> {
        let mut tokens = Vec::<SemanticToken>::new();
        for block in &self.elements {
            tokens.append(&mut block.tokens(open_types));
        }
        tokens
    }
}

impl SemanticBlockTokenizer for UnimarkupBlockKind {
    fn tokens(&self, open_types: &mut Vec<OpenTokenType>) -> Vec<SemanticToken> {
        match self {
            UnimarkupBlockKind::Heading(heading) => heading.tokens(open_types),
            UnimarkupBlockKind::Paragraph(paragraph) => paragraph.tokens(open_types),
            UnimarkupBlockKind::Verbatim(verbatim) => verbatim.tokens(open_types),
        }
    }
}

impl SemanticBlockTokenizer for HeadingBlock {
    fn tokens(&self, open_types: &mut Vec<OpenTokenType>) -> Vec<SemanticToken> {
        let mut tokens = vec![SemanticToken {
            delta_line: to_lsp_line_nr(self.line_nr),
            delta_start: calculate_column_offset(open_types),
            length: (u8::from(self.level) + 1).into(), // +1 for space
            token_type: TokenType::Heading.value(),
            token_modifiers_bitset: 0,
        }];
        tokens.append(&mut self.content.tokens(&TokenType::Heading, &mut vec![]));
        tokens
    }
}

impl SemanticBlockTokenizer for ParagraphBlock {
    fn tokens(&self, _open_types: &mut Vec<OpenTokenType>) -> Vec<SemanticToken> {
        self.content.tokens(&TokenType::Paragraph, &mut vec![])
    }
}

impl SemanticBlockTokenizer for VerbatimBlock {
    fn tokens(&self, _open_types: &mut Vec<OpenTokenType>) -> Vec<SemanticToken> {
        //TODO: Change length after VerbatimBlock contains needed information
        let mut tokens = vec![SemanticToken {
            delta_line: to_lsp_line_nr(self.line_nr),
            delta_start: 0,
            length: 50,
            token_type: TokenType::VerbatimBlock.value(),
            token_modifiers_bitset: 0,
        }];

        let lines = self.content.lines();
        for (i, line) in lines.enumerate() {
            tokens.push(SemanticToken {
                delta_line: to_lsp_line_nr(self.line_nr + i + 1),
                delta_start: 0,
                length: (line.len() as u32),
                token_type: TokenType::VerbatimBlock.value(),
                token_modifiers_bitset: 0,
            });
        }

        tokens.push(SemanticToken {
            delta_line: to_lsp_line_nr(self.line_nr + self.content.lines().count() + 1),
            delta_start: 0,
            length: 50,
            token_type: TokenType::VerbatimBlock.value(),
            token_modifiers_bitset: 0,
        });

        tokens
    }
}

fn calculate_column_offset(open_types: &mut [OpenTokenType]) -> u32 {
    match open_types.last() {
        Some(last_open) => {
            if let Some(length) = last_open.length {
                length + last_open.start_column_offset
            } else {
                last_open.start_column_offset
            }
        }
        None => 0,
    }
}
