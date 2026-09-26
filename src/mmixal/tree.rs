//! A pest node's children, taken by position with an internal error in place of a panic.
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable
)]

use super::Rule;
use pest::iterators::Pairs;

/// A pest node's children, taken in order. `next` serves an optional or
/// skipped child; `required` serves one the grammar guarantees, reporting
/// an internal error rather than panicking if that guarantee is ever
/// wrong.
pub(super) struct Children<'i> {
    rule: Rule,
    inner: Pairs<'i, Rule>,
    taken: usize,
}

impl<'i> Children<'i> {
    /// Takes `node`'s children, recording its rule for every error this
    /// accessor reports.
    pub(super) fn of(node: pest::iterators::Pair<'i, Rule>) -> Self {
        Children {
            rule: node.as_rule(),
            inner: node.into_inner(),
            taken: 0,
        }
    }

    /// The next child, or an internal error naming this node's rule and
    /// the 1-based position the grammar failed to guarantee.
    pub(super) fn required(&mut self) -> Result<pest::iterators::Pair<'i, Rule>, String> {
        match self.next() {
            Some(child) => Ok(child),
            None => Err(format!(
                "internal error: grammar rule {:?} has no child {}",
                self.rule,
                self.taken + 1
            )),
        }
    }

    /// An internal error for `child`, found where this node's rule admits
    /// no such child.
    pub(super) fn unexpected(&self, child: &pest::iterators::Pair<'_, Rule>) -> String {
        format!(
            "internal error: grammar rule {:?} has unexpected child {:?}",
            self.rule,
            child.as_rule()
        )
    }
}

impl<'i> Iterator for Children<'i> {
    type Item = pest::iterators::Pair<'i, Rule>;

    fn next(&mut self) -> Option<Self::Item> {
        let child = self.inner.next();
        if child.is_some() {
            self.taken += 1;
        }
        child
    }
}

/// An internal error for `text`, matched where `rule` admits no such text.
pub(super) fn unexpected_text(rule: Rule, text: &str) -> String {
    format!("internal error: grammar rule {rule:?} matched unexpected text {text:?}")
}
