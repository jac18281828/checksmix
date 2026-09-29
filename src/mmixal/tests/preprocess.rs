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

// --- leading byte order mark ---

const BOM: &str = "\u{FEFF}";

fn assemble_units(units: &[(String, String)]) -> MMixAssembler {
    let mut asm = MMixAssembler::new(&units[0].1, &units[0].0);
    for (name, src) in &units[1..] {
        asm.add_source(src, name);
    }
    asm.parse().unwrap();
    asm
}

#[test]
fn a_leading_byte_order_mark_leaves_the_first_line_in_force() {
    let plain = "LOC #100\nMain SETL $1,5\n";
    let mut with_mark = MMixAssembler::new(&format!("{BOM}{plain}"), "<test>");
    with_mark.parse().unwrap();
    let mut without_mark = MMixAssembler::new(plain, "<test>");
    without_mark.parse().unwrap();

    assert_eq!(with_mark.instructions, without_mark.instructions);
    assert_eq!(with_mark.instructions[0].0, 0x100);
    assert_eq!(with_mark.labels.get("Main"), Some(&0x100));
}

#[test]
fn a_first_line_error_names_the_same_column_with_or_without_a_byte_order_mark() {
    let plain = "SET $1,-010\n";
    let with_mark = assemble_err(&format!("{BOM}{plain}"));

    assert_eq!(with_mark, assemble_err(plain));
    assert!(with_mark.starts_with("<test>:1:8:"), "err: {with_mark}");
}

#[test]
fn source_text_carries_no_byte_order_mark() {
    let mut asm = MMixAssembler::new(&format!("{BOM}LOC #100\nSWYM\n"), "<test>");
    asm.add_source(&format!("{BOM}SWYM\n"), "second");
    asm.parse().unwrap();

    assert_eq!(asm.source_text("<test>", 1), Some("LOC #100"));
    assert_eq!(asm.source_text("second", 1), Some("SWYM"));
}

#[test]
fn only_the_leading_mark_is_dropped() {
    let mut asm = MMixAssembler::new(&format!("{BOM}SWYM\nBYTE \"{BOM}\"\n"), "<test>");
    asm.parse().unwrap();

    assert_eq!(
        asm.source_text("<test>", 2),
        Some(&format!("BYTE \"{BOM}\"")[..])
    );
}

#[test]
fn a_byte_order_mark_on_an_add_source_unit_is_dropped() {
    let mut with_mark = MMixAssembler::new("SWYM\n", "first");
    with_mark.add_source(&format!("{BOM}LOC #200\nSWYM\n"), "second");
    with_mark.parse().unwrap();

    assert_eq!(with_mark.addr_for_line("second", 2), Some(0x200));
}

fn expand(root: &str, lib: &str) -> Vec<(String, String)> {
    let reader = fixture_reader(vec![("lib.mms", lib), ("inner.mms", "OCTA 2\n")]);
    MMixAssembler::resolve_includes(root, "root.mms", std::path::Path::new(""), &reader).unwrap()
}

#[test]
fn an_included_file_with_a_byte_order_mark_and_an_include_on_line_one_expands() {
    let lib = "INCLUDE inner.mms\nOCTA 1\n";
    let root = "INCLUDE lib.mms\nOCTA 3\n";
    let with_mark = expand(root, &format!("{BOM}{lib}"));
    let without_mark = expand(root, lib);

    assert_eq!(with_mark, without_mark);
    assert_eq!(with_mark.len(), 3);
    assert_eq!(assemble_units(&with_mark).instructions.len(), 3);
}

#[test]
fn a_root_file_with_a_byte_order_mark_and_an_include_on_line_one_expands() {
    let root = "INCLUDE lib.mms\nOCTA 3\n";
    let with_mark = expand(&format!("{BOM}{root}"), "OCTA 1\n");

    assert_eq!(with_mark, expand(root, "OCTA 1\n"));
    assert_eq!(with_mark.len(), 2);
    assert_eq!(with_mark[0].0, "lib.mms");
}

#[test]
fn an_included_unit_reaches_the_assembler_without_a_byte_order_mark() {
    let with_mark = expand("INCLUDE lib.mms\n", &format!("{BOM}LOC #300\nOCTA 1\n"));

    assert_eq!(
        assemble_units(&with_mark).addr_for_line("lib.mms", 2),
        Some(0x300)
    );
}

#[test]
fn a_second_byte_order_mark_is_text_on_every_path() {
    let source = format!("{BOM}{BOM}LOC #100\nMain SETL $1,5\n");
    let mut direct = MMixAssembler::new(&source, "root.mms");
    direct.parse().unwrap();

    let resolved = expand(&source, "");
    let through_includes = assemble_units(&resolved);

    assert_eq!(direct.instructions, through_includes.instructions);
    assert_eq!(direct.instructions[0].0, 0);
    assert_eq!(
        direct.source_text("root.mms", 1),
        Some(&format!("{BOM}LOC #100")[..])
    );
    assert_eq!(
        through_includes.source_text("root.mms", 1),
        direct.source_text("root.mms", 1)
    );
}

#[test]
fn a_file_of_one_byte_order_mark_contributes_no_unit() {
    assert!(expand(BOM, "").is_empty());
    assert!(!expand(&format!("{BOM}{BOM}"), "").is_empty());
}

#[test]
fn a_byte_order_mark_alone_on_a_later_line_is_not_blank() {
    let root = format!("INCLUDE lib.mms\n{BOM}\n");

    assert_eq!(expand(&root, "OCTA 1\n").len(), 2);
}
