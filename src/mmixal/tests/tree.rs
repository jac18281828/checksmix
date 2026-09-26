//! Tests for the pest-child accessor: the position `required` reports past
//! the last child, and the text of an `unexpected`/`unexpected_text` error.

use super::*;
use crate::mmixal::tree::Children;
use crate::mmixal::tree::unexpected_text;
use pest::Parser;

fn expr_pair(source: &str) -> pest::iterators::Pair<'_, Rule> {
    MMixalParser::parse(Rule::expr, source)
        .unwrap()
        .next()
        .unwrap()
}

#[test]
fn required_past_the_last_child_names_the_next_position() {
    let mut children = Children::of(expr_pair("1"));
    children.required().unwrap(); // the one term child
    let err = children.required().unwrap_err();
    assert_eq!(err, "internal error: grammar rule expr has no child 2");
}

#[test]
fn required_counts_children_next_already_took() {
    // "1+2+3" is term, weak_op, term, weak_op, term: five children. Four
    // taken by `next`, the fifth by `required`, so the next position past
    // the end is six either way.
    let mut children = Children::of(expr_pair("1+2+3"));
    children.next().unwrap();
    children.next().unwrap();
    children.next().unwrap();
    children.next().unwrap();
    children.required().unwrap();
    let err = children.required().unwrap_err();
    assert_eq!(err, "internal error: grammar rule expr has no child 6");
}

#[test]
fn unexpected_names_the_node_and_the_unexpected_child() {
    let mut children = Children::of(expr_pair("1"));
    let term = children.required().unwrap();
    assert_eq!(
        children.unexpected(&term),
        "internal error: grammar rule expr has unexpected child term"
    );
}

#[test]
fn unexpected_text_names_the_rule_and_the_matched_text() {
    assert_eq!(
        unexpected_text(Rule::unary_op, "!"),
        "internal error: grammar rule unary_op matched unexpected text \"!\""
    );
}
