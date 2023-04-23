use lsp_types::SemanticToken;
use unimarkup_core::{
    document::Document,
    elements::blocks::Block,
    elements::{atomic::Heading, atomic::Paragraph, enclosed::Verbatim},
};

use super::{
    inline_tokens::SemanticInlineTokenizer, TokenValue,
};

// #[derive(Debug, Default, Clone)]
// pub(crate) struct OpenTokenType {
//     /// The open token type
//     _token_type: TokenType,
//     /// Column offset the content of this type starts.
//     /// Needed for nested blocks.
//     ///
//     /// e.g. verbatim inside list
//     start_column_offset: u32,
//     /// The type length, or `None` if the type goes to end of line
//     length: Option<u32>,
// }

#[derive(Default, Debug, Clone)]
pub(crate) enum TokenType {
    #[default]
    Paragraph,
    Heading,
    Verbatim,
}

impl TokenValue for TokenType {
    fn value(&self) -> u32 {
        match self {
            TokenType::Paragraph => 2,//21,
            TokenType::Heading => 3,
            TokenType::Verbatim => 18,
        }
    }
}

pub(crate) trait SemanticBlockTokenizer {
    fn tokens(&self) -> Vec<SemanticToken>;
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
            Block::Verbatim(verbatim) => verbatim.tokens(),
            _ => todo!(),
        }
    }
}

impl SemanticBlockTokenizer for Heading {
    fn tokens(&self) -> Vec<SemanticToken> {
        let mut tokens = vec![SemanticToken {
            delta_line: self.line_nr as u32,
            delta_start: 1,
            length: (u8::from(self.level)).into(),
            token_type: TokenType::Heading.value(),
            token_modifiers_bitset: 0,
        }];

        tokens.append(
            &mut self
                .content
                .iter()
                .flat_map(|inline| inline.tokens(&mut vec![]))
                .collect(),
        );

        dbg!(tokens)
    }
}

impl SemanticBlockTokenizer for Paragraph {
    fn tokens(&self) -> Vec<SemanticToken> {
        self.content
            .iter()
            .flat_map(|inline| inline.tokens(&mut vec![]))
            .collect()
    }
}

impl SemanticBlockTokenizer for Verbatim {
    fn tokens(&self) -> Vec<SemanticToken> {
        //TODO: Change length after Verbatim contains needed information
        let mut tokens = vec![SemanticToken {
            delta_line: self.line_nr as u32,
            delta_start: 1,
            length: 50,
            token_type: TokenType::Verbatim.value(),
            token_modifiers_bitset: 0,
        }];

        let lines = self.content.lines();
        for (i, line) in lines.enumerate() {
            tokens.push(SemanticToken {
                delta_line: (self.line_nr + i + 1) as u32,
                delta_start: 1,
                length: (line.len() as u32),
                token_type: TokenType::Verbatim.value(),
                token_modifiers_bitset: 0,
            });
        }

        tokens.push(SemanticToken {
            delta_line: (self.line_nr + self.content.lines().count() + 1) as u32,
            delta_start: 1,
            length: 50,
            token_type: TokenType::Verbatim.value(),
            token_modifiers_bitset: 0,
        });

        tokens
    }
}

// fn calculate_column_offset(open_types: &mut [OpenTokenType]) -> u32 {
//     match open_types.last() {
//         Some(last_open) => {
//             if let Some(length) = last_open.length {
//                 length + last_open.start_column_offset
//             } else {
//                 last_open.start_column_offset
//             }
//         }
//         None => 0,
//     }
// }
