//! Shared test fixtures for every area under `tests/`.

use super::*;
use crate::mmix::{MMix, SpecialReg};
use crate::mmo::MmoDecoder;

mod auto_immediate;
mod diagnostics;
mod directives;
mod expressions;
mod grammar;
mod instructions;
mod location;
mod operands;
mod passes;
mod preprocess;
mod source_loc;
mod symbols;

/// Assemble a snippet whose first instruction is the one under test
/// and assert it produced the expected enum variant. Snippets are
/// kept to one logical instruction so failures point at the case.
fn assert_first_instruction(src: &str, expected: MMixInstruction) {
    let mut asm = MMixAssembler::new(src, "<test>");
    asm.parse()
        .unwrap_or_else(|e| panic!("failed to parse {src:?}: {e}"));
    assert!(
        !asm.instructions.is_empty(),
        "no instructions produced for {src:?}"
    );
    assert_eq!(
        asm.instructions[0].1, expected,
        "wrong instruction for {src:?}"
    );
}

fn assert_first_instruction_matches(src: &str, predicate: impl Fn(&MMixInstruction) -> bool) {
    let mut asm = MMixAssembler::new(src, "<test>");
    asm.parse()
        .unwrap_or_else(|e| panic!("failed to parse {src:?}: {e}"));
    assert!(
        !asm.instructions.is_empty(),
        "no instructions produced for {src:?}"
    );
    assert!(
        predicate(&asm.instructions[0].1),
        "wrong instruction variant for {src:?}: got {:?}",
        asm.instructions[0].1
    );
}

/// Build an in-memory `read` closure keyed by exact `PathBuf`s, so tests
/// stay hermetic (no real filesystem access).
fn fixture_reader(
    files: Vec<(&str, &str)>,
) -> impl Fn(&std::path::Path) -> std::io::Result<String> {
    let map: HashMap<std::path::PathBuf, String> = files
        .into_iter()
        .map(|(p, s)| (std::path::PathBuf::from(p), s.to_string()))
        .collect();
    move |p: &std::path::Path| {
        map.get(p).cloned().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "no such fixture file")
        })
    }
}

fn assemble_err(source: &str) -> String {
    let mut asm = MMixAssembler::new(source, "<test>");
    asm.parse()
        .expect_err(&format!("{source:?} must be rejected"))
}
