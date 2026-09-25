//! Tests for INCLUDE resolution.

use super::*;

// --- resolve_includes (INCLUDE directive) ---

#[test]
fn resolve_includes_single_include_inserts_a_unit() {
    let reader = fixture_reader(vec![("lib.mms", "OCTA 1\n")]);
    let units = MMixAssembler::resolve_includes(
        "INCLUDE lib.mms\nOCTA 2\n",
        "root.mms",
        std::path::Path::new(""),
        &reader,
    )
    .unwrap();

    assert_eq!(units.len(), 2);
    assert_eq!(units[0].0, "lib.mms");
    assert!(units[0].1.contains("OCTA 1"));
    assert_eq!(units[1].0, "root.mms");
    assert!(units[1].1.contains("OCTA 2"));
}

#[test]
fn resolve_includes_filename_fidelity() {
    // The included unit's filename must be the included file's own path,
    // NOT the including (root) file's name -- the property text-splicing
    // could never give, since a spliced file has no filename of its own.
    let reader = fixture_reader(vec![("lib.mms", "OCTA 1\n")]);
    let units = MMixAssembler::resolve_includes(
        "INCLUDE lib.mms\nOCTA 2\n",
        "root.mms",
        std::path::Path::new(""),
        &reader,
    )
    .unwrap();

    assert_eq!(units[0].0, "lib.mms");
    assert_ne!(units[0].0, "root.mms");
}

#[test]
fn resolve_includes_pads_line_numbers_after_an_include() {
    let reader = fixture_reader(vec![("lib.mms", "OCTA 1\n")]);
    let units = MMixAssembler::resolve_includes(
        "% a\nINCLUDE lib.mms\nBYTE 0\n",
        "root.mms",
        std::path::Path::new(""),
        &reader,
    )
    .unwrap();

    // Trailing host segment: `BYTE 0` was on absolute line 3, so it must
    // be preceded by exactly 2 padding newlines.
    let host_segment = &units.last().unwrap().1;
    assert_eq!(host_segment.matches('\n').count() - 1, 2);
    assert!(host_segment.starts_with("\n\nBYTE 0"));

    // Parse it through the real assembler and confirm the reported line
    // number is the ABSOLUTE line 3, not the padded segment's line 1.
    let mut asm = MMixAssembler::new(&units[0].1, &units[0].0);
    for (name, src) in units.iter().skip(1) {
        asm.add_source(src, name);
    }
    asm.parse().unwrap();
    // `lib.mms`'s `OCTA 1` occupies address 0..8, so `BYTE 0` -- at
    // absolute line 3 thanks to padding -- lands at address 8.
    assert_eq!(asm.addr_for_line("root.mms", 3), Some(8));
}

#[test]
fn resolve_includes_nested_relative_resolution() {
    let reader = fixture_reader(vec![
        ("a/sub/b.mms", "INCLUDE c.mms\nOCTA 2\n"),
        ("a/sub/c.mms", "OCTA 3\n"),
    ]);
    let units = MMixAssembler::resolve_includes(
        "INCLUDE sub/b.mms\nOCTA 1\n",
        "a/root.mms",
        std::path::Path::new("a"),
        &reader,
    )
    .unwrap();

    let c_unit = units
        .iter()
        .find(|(name, _)| name.contains("c.mms"))
        .expect("c.mms unit present");
    assert_eq!(c_unit.0, "a/sub/c.mms");
    assert!(c_unit.1.contains("OCTA 3"));
}

#[test]
fn resolve_includes_cycle_is_an_error() {
    let reader = fixture_reader(vec![
        ("a.mms", "INCLUDE b.mms\n"),
        ("b.mms", "INCLUDE a.mms\n"),
    ]);
    let err = MMixAssembler::resolve_includes(
        "INCLUDE a.mms\n",
        "driver.mms",
        std::path::Path::new(""),
        &reader,
    )
    .unwrap_err();

    assert!(err.contains("cycle"));
    assert!(err.contains("a.mms"));
    assert!(err.contains("b.mms"));
}

#[test]
fn resolve_includes_missing_file_is_an_error_not_a_panic() {
    let reader = fixture_reader(vec![]);
    let err = MMixAssembler::resolve_includes(
        "INCLUDE missing.mms\n",
        "root.mms",
        std::path::Path::new(""),
        &reader,
    )
    .unwrap_err();

    assert!(err.contains("missing.mms"));
}

#[test]
fn resolve_includes_passthrough_preserves_content_with_no_include() {
    let reader = fixture_reader(vec![]);
    let source = "OCTA 1\nOCTA 2";
    let units =
        MMixAssembler::resolve_includes(source, "root.mms", std::path::Path::new(""), &reader)
            .unwrap();

    assert_eq!(units.len(), 1);
    assert_eq!(units[0].0, "root.mms");
    assert_eq!(units[0].1, source);
}

#[test]
fn resolve_includes_recognizes_comment_case() {
    // INCLUDE matches in upper case only; a lower-case
    // `include` is ordinary source text, never expanded.
    let reader = fixture_reader(vec![("lib.mms", "OCTA 1\n")]);

    let lower_with_comment = MMixAssembler::resolve_includes(
        "include lib.mms  % pull it in\n",
        "root.mms",
        std::path::Path::new(""),
        &reader,
    )
    .unwrap();
    let quoted = MMixAssembler::resolve_includes(
        "INCLUDE \"lib.mms\"\n",
        "root.mms",
        std::path::Path::new(""),
        &reader,
    )
    .unwrap();

    assert_eq!(lower_with_comment.len(), 1);
    assert_eq!(lower_with_comment[0].1, "include lib.mms  % pull it in\n");
    assert_eq!(quoted.len(), 1);
    assert_eq!(quoted[0].1, "OCTA 1\n");
    assert!(quoted[0].1.contains("OCTA 1"));
}
