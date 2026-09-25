//! Tests for source-level debug info: SourceLoc, source_loc, addr_for_line and source_text.

use super::*;

// ---- Source-level debug info (SourceLoc / source_loc / addr_for_line /
// source_text): pc.1 of the mmixdb effort. ----------------------------

/// Line fidelity across a labeled `debug` directive: the pinning test.
/// A labeled `debug "..."` line, then a labeled instruction a couple of
/// lines down. Reverting the `preprocess_debug` line-count-preserving
/// fix -- i.e. restoring the old two-line label/PUSHJ expansion --
/// shifts every following line by one and makes this assertion fail.
#[test]
fn test_debug_directive_preserves_line_fidelity() {
    let lines = [
        "\tLOC\tData_Segment",
        "\tGREG\t@",
        "Text\tBYTE\t\"Hello world!\",10,0",
        "",
        "\tLOC\t#100",
        "",
        "Main\tdebug \"Version 0.1: Hello World Example\"",
        "Start\tLDA\t$255,Text",
        "\tTRAP\t0,Fputs,StdOut",
        "\tTRAP\t0,Halt,0",
    ];
    let lda_line = lines.iter().position(|l| l.contains("LDA")).unwrap() + 1;
    let source = lines.join("\n");

    let mut asm = MMixAssembler::new(&source, "hello_world.mms");
    asm.parse().unwrap();

    let lda_addr = *asm.labels.get("Start").expect("Start label defined");
    let loc = asm
        .source_loc(lda_addr)
        .expect("LDA's address should have a source location");
    assert_eq!(loc.file, "hello_world.mms");
    assert_eq!(loc.line, lda_line, "LDA must report its ORIGINAL line");
}

/// Inverse round-trip: `addr_for_line` returns the same address
/// `source_loc` mapped back from.
#[test]
fn test_addr_for_line_round_trips_with_source_loc() {
    let source = "Base\tGREG\t1\nMain\tdebug \"hi\"\nStart\tLDA\t$255,Start\n\tTRAP\t0,Halt,0\n";

    let mut asm = MMixAssembler::new(source, "<test>");
    asm.parse().unwrap();

    let lda_addr = *asm.labels.get("Start").unwrap();
    let loc = asm.source_loc(lda_addr).unwrap();
    assert_eq!(
        asm.addr_for_line(&loc.file, loc.line),
        Some(lda_addr),
        "addr_for_line must invert source_loc"
    );
}

/// `source_text` returns the ORIGINAL line (the `debug` directive as the
/// user wrote it), not the preprocessed `PUSHJ` text.
#[test]
fn test_source_text_returns_original_not_preprocessed() {
    let source = "Base\tGREG\t1\nMain\tdebug \"hi\"\nStart\tLDA\t$255,Start\n";
    let mut asm = MMixAssembler::new(source, "<test>");
    asm.parse().unwrap();

    let text = asm.source_text("<test>", 2).expect("line 2 exists");
    assert!(
        text.contains("debug"),
        "expected original text, got {text:?}"
    );
    assert!(
        !text.contains("PUSHJ"),
        "source_text must not leak preprocessed text, got {text:?}"
    );

    let lda_text = asm.source_text("<test>", 3).expect("line 3 exists");
    assert!(lda_text.contains("LDA"));
}

/// A syntax error after a `debug` line must name the ORIGINAL line, not
/// the preprocessed one the landing pad shifts it to.
#[test]
fn test_syntax_error_after_a_debug_line_reports_the_original_line() {
    let source = "\
\tLOC\t#100
Main\tdebug\t\"hi\"
\tSET\t$1,7
\tFLOT\t$1,$2,$3
";
    let mut asm = MMixAssembler::new(source, "<test>");
    let err = asm
        .parse()
        .expect_err("FLOT $1,$2,$3 must still be rejected");
    assert_eq!(
        err, "<test>:4:10: register $2 cannot be used where a pure value is required",
        "must report original line 4, not the preprocessed line the \
             debug expansion's landing pad shifts it to"
    );
}

/// A symbol redefined after two `debug` lines reports both the current
/// and the first-definition site at their ORIGINAL lines.
#[test]
fn test_redefinition_after_two_debug_lines_reports_original_lines() {
    let source = "\
\tLOC\t#100
Main\tdebug\t\"hi\"
\tdebug\t\"ho\"
Foo\tIS\t1
\tSET\t$1,7
Foo\tIS\t2
";
    let mut asm = MMixAssembler::new(source, "<test>");
    let err = asm
        .parse()
        .expect_err("redefining Foo must still be rejected");
    assert_eq!(
        err, "<test>:6:1: symbol 'Foo' redefined (first defined at <test>:4)",
        "both sites must report their ORIGINAL lines, not the \
             preprocessed lines two debug expansions shift them to"
    );
}

/// A data directive that emits multiple words maps every emitted address
/// to the same source line, and `addr_for_line` returns the lowest one.
/// Checked for `BYTE` (1-byte units) and `OCTA` (8-byte units), so the
/// invariant is shown to hold independent of unit width.
#[test]
fn test_multi_word_data_directive_maps_to_one_line() {
    let source = "Data\tBYTE\t1,2,3\nWide\tOCTA\t1,2,3\n";
    let mut asm = MMixAssembler::new(source, "<test>");
    asm.parse().unwrap();

    let base_addr = *asm.labels.get("Data").unwrap();
    for offset in 0..3 {
        let loc = asm
            .source_loc(base_addr + offset)
            .unwrap_or_else(|| panic!("no source_loc at offset {offset}"));
        assert_eq!(loc.line, 1);
    }
    assert_eq!(asm.addr_for_line("<test>", 1), Some(base_addr));

    let wide_addr = *asm.labels.get("Wide").unwrap();
    for offset in 0..3 {
        let addr = wide_addr + offset * 8;
        let loc = asm
            .source_loc(addr)
            .unwrap_or_else(|| panic!("no source_loc at unit {offset}"));
        assert_eq!(loc.line, 2);
    }
    assert_eq!(asm.addr_for_line("<test>", 2), Some(wide_addr));
}

/// A stack program with both a data and a text region, so `source_loc`
/// is asked about an address in the gap between them.
const TWO_REGION_PROGRAM: &str = "\
        LOC     Data_Segment
Cells   OCTA    0
        OCTA    0
        OCTA    0
Sp      GREG    Cells

        LOC     #100
Main    SETI    $1,7
        STOI    $1,Sp,0
        ADDUI   Sp,Sp,8
        SETI    $1,35
        STOI    $1,Sp,0
        LDOI    $2,Sp,0
        SUBUI   Sp,Sp,8
        LDOI    $3,Sp,0
        ADDU    $255,$2,$3
        TRAP    0,Halt,0
";

/// `SETI $X,imm` expands to four tetras. Every address in the expansion
/// belongs to the statement that emitted it, and the address just past
/// the last text entry belongs to nothing -- the data region lies far
/// above it, and a lookup that ran to the end of the image would hand
/// that whole gap to the last text line.
#[test]
fn test_source_loc_covers_an_expansion_and_stops_at_its_end() {
    let mut asm = MMixAssembler::new(TWO_REGION_PROGRAM, "stack.mms");
    asm.parse().unwrap();

    let main = *asm.labels.get("Main").unwrap();
    assert_eq!(main, 0x100);
    for addr in [0x100, 0x104, 0x108, 0x10c] {
        let loc = asm
            .source_loc(addr)
            .unwrap_or_else(|| panic!("no source_loc at 0x{addr:x}"));
        assert_eq!(loc.line, 8, "0x{addr:x} is inside line 8's SETI");
    }
    assert_eq!(asm.source_loc(0x110).map(|loc| loc.line), Some(9));

    // One tetra past the TRAP that ends the text region, and far below
    // the data region.
    assert_eq!(asm.source_loc(0x140), None);
}

/// The data region keeps its own lines; the text region above it does
/// not bleed into the gap, nor the data region below.
#[test]
fn test_source_loc_maps_the_data_region_independently() {
    let mut asm = MMixAssembler::new(TWO_REGION_PROGRAM, "stack.mms");
    asm.parse().unwrap();

    let cells = *asm.labels.get("Cells").unwrap();
    assert!(cells >= 0x2000_0000_0000_0000);
    assert_eq!(asm.source_loc(cells).map(|loc| loc.line), Some(2));
    assert_eq!(asm.source_loc(cells + 7).map(|loc| loc.line), Some(2));
    assert_eq!(asm.source_loc(cells + 8).map(|loc| loc.line), Some(3));
    assert_eq!(asm.source_loc(cells + 24), None);
}

/// Duplicate-name translation units: reverse lookups (`addr_for_line`,
/// `source_text`) resolve to the FIRST unit with that filename in
/// command-line order, while `source_loc` (keyed by address) stays
/// unambiguous regardless.
#[test]
fn test_source_text_resolves_duplicate_filename_to_first_unit() {
    let mut asm = MMixAssembler::new("First\tHALT\n", "dup.mms");
    asm.add_source("Second\tHALT\n", "dup.mms");
    asm.parse().unwrap();

    let text = asm.source_text("dup.mms", 1).unwrap();
    assert!(text.contains("First"));
}
