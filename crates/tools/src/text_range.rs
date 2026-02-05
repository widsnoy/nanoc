use miette::SourceSpan;
use std::ops::{Deref, Range};

/// 包装 `text_size::TextRange`, 实现 Ord
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct TextRange(pub text_size::TextRange);

impl TextRange {
    pub fn new(start: u32, end: u32) -> Self {
        Self(text_size::TextRange::new(
            text_size::TextSize::new(start),
            text_size::TextSize::new(end),
        ))
    }
}

impl From<TextRange> for Range<usize> {
    fn from(value: TextRange) -> Self {
        value.0.start().into()..value.0.end().into()
    }
}

impl From<Range<usize>> for TextRange {
    fn from(value: Range<usize>) -> Self {
        TextRange::new(value.start as u32, value.end as u32)
    }
}

impl From<TextRange> for SourceSpan {
    fn from(value: TextRange) -> Self {
        SourceSpan::from(value.0.start().into()..value.0.end().into())
    }
}

impl Ord for TextRange {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0
            .start()
            .cmp(&other.0.start())
            .then(self.0.end().cmp(&other.0.end()))
    }
}

impl PartialOrd for TextRange {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Deref for TextRange {
    type Target = text_size::TextRange;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<text_size::TextRange> for TextRange {
    fn from(value: text_size::TextRange) -> Self {
        Self(value)
    }
}

impl From<TextRange> for text_size::TextRange {
    fn from(value: TextRange) -> Self {
        value.0
    }
}
