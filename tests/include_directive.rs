use checksmix::MMixAssembler;
use std::fs;
use std::path::{Path, PathBuf};

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

fn real_reader(p: &Path) -> std::io::Result<String> {
    fs::read_to_string(p)
}

/// End-to-end: `INCLUDE`-ing a real fixture file makes a label defined in
/// the included file resolve in the final assembly. Non-vacuous: remove
/// resolution and the raw `INCLUDE include_lib.mms` line reaches the
/// grammar, which does not understand it, so `parse()` fails and this test
/// fails.
#[test]
fn include_directive_resolves_label_from_included_file() {
    let main_path = fixture_path("include_main.mms");
    let main_src = fs::read_to_string(&main_path).expect("read include_main.mms");

    let units = MMixAssembler::resolve_includes(
        &main_src,
        main_path.to_str().unwrap(),
        &fixtures_dir(),
        &real_reader,
    )
    .expect("resolve_includes over real fixtures");

    assert!(
        units.len() >= 2,
        "expected the host file split around INCLUDE plus the included unit, got {}",
        units.len()
    );
    assert!(
        units
            .iter()
            .any(|(name, _)| name.contains("include_lib.mms")),
        "included unit should carry include_lib.mms's own filename"
    );

    let (first_name, first_src) = &units[0];
    let mut asm = MMixAssembler::new(first_src, first_name);
    for (name, src) in units.iter().skip(1) {
        asm.add_source(src, name);
    }
    asm.parse()
        .expect("assemble include_main.mms + include_lib.mms");

    assert_eq!(
        asm.labels.get(":LibHalt").copied(),
        Some(0x100),
        ":LibHalt, defined in the included file, should resolve at LOC #100"
    );
    let (addr, _) = asm
        .instructions
        .iter()
        .find(|(_, inst)| format!("{:?}", inst).contains("PUSHJ"))
        .expect("PUSHJ instruction present");
    assert_eq!(*addr, 0x104, "Main's PUSHJ follows :LibHalt's TRAP");
}

/// Per-file diagnostics: a parse error inside an INCLUDE-d file reports
/// that included file's OWN name, not the including file's -- the property
/// the translation-unit approach (vs. text-splicing) is supposed to give.
/// Non-vacuous: if INCLUDE were resolved by splicing text into one unit
/// tagged with the root's filename, this error would report the root's
/// name instead, and this test would fail.
#[test]
fn include_directive_parse_error_names_the_included_file() {
    let root_source = "; header, valid on its own\nINCLUDE include_bad_lib.mms\n";
    let units = MMixAssembler::resolve_includes(
        root_source,
        "include_diag_root.mms",
        &fixtures_dir(),
        &real_reader,
    )
    .expect("resolve_includes over real fixtures");

    let (first_name, first_src) = &units[0];
    let mut asm = MMixAssembler::new(first_src, first_name);
    for (name, src) in units.iter().skip(1) {
        asm.add_source(src, name);
    }
    let err = asm
        .parse()
        .expect_err("malformed included file must fail to parse");

    assert!(
        err.contains("include_bad_lib.mms"),
        "parse error should name the included file, got: {}",
        err
    );
    assert!(
        !err.contains("include_diag_root.mms"),
        "parse error should NOT name the (unrelated) including root, got: {}",
        err
    );
}
