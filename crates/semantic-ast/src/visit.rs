//! AST Visitor 接口

use crate::{Block, Document, TextRun};

/// 只读访问者。
pub trait Visitor {
    fn visit_document(&mut self, doc: &Document) {
        for b in &doc.blocks {
            self.visit_block(b);
        }
    }
    fn visit_block(&mut self, b: &Block) {
        match b {
            Block::Heading { .. } => {}
            Block::Paragraph { runs, .. } => {
                for r in runs {
                    self.visit_run(r);
                }
            }
            Block::List { items, .. } => {
                for item in items {
                    for child in item {
                        self.visit_block(child);
                    }
                }
            }
            Block::Table { rows, .. } => {
                for row in rows {
                    for cell in &row.cells {
                        for r in &cell.runs {
                            self.visit_run(r);
                        }
                    }
                }
            }
            Block::Figure { .. }
            | Block::Equation { .. }
            | Block::TheoremLike { .. }
            | Block::Bibliography { .. }
            | Block::Algorithm { .. }
            | Block::CodeBlock { .. }
            | Block::RawFallback { .. } => {}
        }
    }
    fn visit_run(&mut self, _r: &TextRun) {}
}

/// 统计块数量。
pub struct BlockCounter(pub usize);

impl Visitor for BlockCounter {
    fn visit_block(&mut self, _b: &Block) {
        self.0 += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Span;

    #[test]
    fn counts_blocks() {
        let mut doc = Document::new();
        doc.push(Block::Heading {
            level: 1,
            text: "x".into(),
            number: None,
            span: Span::default(),
        });
        doc.push(Block::Paragraph {
            runs: vec![],
            span: Span::default(),
        });
        let mut c = BlockCounter(0);
        c.visit_document(&doc);
        assert_eq!(c.0, 2);
    }
}
