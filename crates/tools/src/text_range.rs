use std::ops::Deref;

/// 包装 `rowan::TextRange`, 实现 Ord
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct TextRange(pub text_size::TextRange);

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
