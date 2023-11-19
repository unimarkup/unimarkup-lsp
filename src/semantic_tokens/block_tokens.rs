use lsp_types::SemanticToken;
use unimarkup_core::{
    parser::{
        document::Document,
        elements::{
            atomic::{Heading, Paragraph},
            blocks::Block,
            enclosed::VerbatimBlock,
            indents::{BulletList, BulletListEntry},
            BlockElement,
        },
    },
    Unimarkup,
};

use super::{inline_tokens::SemanticInlineTokenizer, TokenValue};

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TokenType {
    #[default]
    Paragraph,
    Heading,
    BulletListEntry,
    Verbatim,
    Math,
}

impl TokenValue for TokenType {
    fn value(&self) -> u32 {
        match self {
            TokenType::Paragraph => 2, //21,
            TokenType::Heading => 3,
            TokenType::BulletListEntry => 3,
            TokenType::Verbatim => 18,
            TokenType::Math => 3,
        }
    }
}

pub(crate) trait SemanticBlockTokenizer {
    fn tokens(&self) -> Vec<SemanticToken>;
}

impl SemanticBlockTokenizer for Unimarkup {
    fn tokens(&self) -> Vec<SemanticToken> {
        self.get_document().tokens()
    }
}

impl SemanticBlockTokenizer for Document {
    fn tokens(&self) -> Vec<SemanticToken> {
        let mut tokens = Vec::<SemanticToken>::new();
        for block in &self.blocks {
            tokens.append(&mut block.tokens());
        }
        tokens
    }
}

impl SemanticBlockTokenizer for Block {
    fn tokens(&self) -> Vec<SemanticToken> {
        match self {
            Block::Heading(heading) => heading.tokens(),
            Block::Paragraph(paragraph) => paragraph.tokens(),
            Block::VerbatimBlock(verbatim) => verbatim.tokens(),
            Block::Blankline(_) => vec![],
            Block::BulletList(bullet_list) => bullet_list.tokens(),
            Block::BulletListEntry(_) => {
                debug_assert!(
                    false,
                    "Bullet list entries must only be handled inside a bullet list element."
                );
                vec![]
            }
        }
    }
}

impl SemanticBlockTokenizer for Heading {
    fn tokens(&self) -> Vec<SemanticToken> {
        let mut tokens = vec![SemanticToken {
            delta_line: self.start().line as u32,
            delta_start: self.start().col_utf16 as u32,
            length: (u8::from(self.level)).into(),
            token_type: TokenType::Heading.value(),
            token_modifiers_bitset: 0,
        }];

        // Multiline heading => add tokens for prefix
        // Tokens are ordered at the end anyways, so it is ok that these are added out of order
        let mut lines = self.end.line - self.start.line;
        while lines > 0 {
            tokens.push(SemanticToken {
                delta_line: (self.start().line + lines) as u32,
                delta_start: self.start().col_utf16 as u32,
                length: (u8::from(self.level)).into(),
                token_type: TokenType::Heading.value(),
                token_modifiers_bitset: 0,
            });

            lines -= 1;
        }

        tokens.append(
            &mut self
                .content
                .iter()
                .flat_map(|inline| inline.tokens(TokenType::Paragraph, &mut vec![]))
                .collect(),
        );

        tokens
    }
}

impl SemanticBlockTokenizer for Paragraph {
    fn tokens(&self) -> Vec<SemanticToken> {
        self.content
            .iter()
            .flat_map(|inline| inline.tokens(TokenType::Paragraph, &mut vec![]))
            .collect()
    }
}

impl SemanticBlockTokenizer for VerbatimBlock {
    fn tokens(&self) -> Vec<SemanticToken> {
        // NOTE: Only keywords are highlighted, but not the inner content
        let mut tokens = vec![SemanticToken {
            delta_line: self.start().line as u32,
            delta_start: self.start().col_utf16 as u32,
            length: self.tick_len as u32,
            token_type: TokenType::Verbatim.value(),
            token_modifiers_bitset: 0,
        }];

        if !self.implicit_closed {
            tokens.push(SemanticToken {
                delta_line: self.end().line as u32,
                delta_start: self.start().col_utf16 as u32, // Start, because start & end ticks have same column offset
                length: self.tick_len as u32,
                token_type: TokenType::Verbatim.value(),
                token_modifiers_bitset: 0,
            });
        }

        tokens
    }
}

impl SemanticBlockTokenizer for BulletList {
    fn tokens(&self) -> Vec<SemanticToken> {
        let mut tokens = Vec::new();

        for entry in &self.entries {
            tokens.append(&mut entry.tokens());
        }

        tokens
    }
}

impl SemanticBlockTokenizer for BulletListEntry {
    fn tokens(&self) -> Vec<SemanticToken> {
        let mut tokens = vec![SemanticToken {
            delta_line: self.start().line as u32,
            delta_start: self.start().col_utf16 as u32,
            length: 1,
            token_type: TokenType::BulletListEntry.value(),
            token_modifiers_bitset: 0,
        }];

        tokens.append(&mut self.body.iter().flat_map(|block| block.tokens()).collect());

        tokens
    }
}
