#![cfg(feature = "cli")]

use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

fn checksmix() -> Command {
    Command::new(env!("CARGO_BIN_EXE_checksmix"))
}

fn mmixasm() -> Command {
    Command::new(env!("CARGO_BIN_EXE_mmixasm"))
}

fn mmixdb() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_mmixdb"));
    // mmixdb assembles before opening its REPL; any invocation that
    // assembles successfully would otherwise block on readline.
    cmd.stdin(Stdio::null());
    cmd
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

fn assert_exit_1_names_input(out: &Output, input: &str) {
    assert_eq!(
        out.status.code(),
        Some(1),
        "expected exit 1; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains(input),
        "stderr should name '{input}'; stderr: {stderr}"
    );
    assert!(
        stderr.contains("contributed no source"),
        "stderr should say 'contributed no source'; stderr: {stderr}"
    );
}

// ── check: clean two-file program ────────────────────────────────────────────

#[test]
fn check_clean_two_file_program() {
    let out = checksmix()
        .args(["check"])
        .arg(fixture("multi_main.mms"))
        .arg(fixture("multi_lib.mms"))
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "check should succeed; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        out.stdout.is_empty(),
        "check should produce no stdout on success"
    );
}

// ── check: undefined symbol reference ────────────────────────────────────────

#[test]
fn check_undefined_symbol_exits_nonzero() {
    let out = checksmix()
        .args(["check"])
        .arg(fixture("undef_ref.mms"))
        .output()
        .unwrap();
    assert!(
        !out.status.success(),
        "check should fail on undefined symbol"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("NoSuchLabel"),
        "error should name the undefined symbol; stderr: {stderr}"
    );
    // Verify file:line:col format appears in the message
    assert!(
        stderr.contains("undef_ref.mms:"),
        "error should contain the source file name; stderr: {stderr}"
    );
}

// ── check: duplicate :Global symbol across two files ─────────────────────────

#[test]
fn check_duplicate_global_names_both_files() {
    let out = checksmix()
        .args(["check"])
        .arg(fixture("dup_global_a.mms"))
        .arg(fixture("dup_global_b.mms"))
        .output()
        .unwrap();
    assert!(
        !out.status.success(),
        "check should fail on duplicate symbol"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("dup_global_a.mms"),
        "error should mention first-definition file; stderr: {stderr}"
    );
    assert!(
        stderr.contains("dup_global_b.mms"),
        "error should mention redefinition file; stderr: {stderr}"
    );
}

// ── build: produces .mmo; round-trip run succeeds ────────────────────────────

#[test]
fn build_produces_mmo_and_run_succeeds() {
    let tmp_mmo = std::env::temp_dir().join("checksmix_test_hello.mmo");

    // build
    let build_out = checksmix()
        .args(["build", "-o"])
        .arg(&tmp_mmo)
        .arg(fixture("hello.mms"))
        .output()
        .unwrap();
    assert!(
        build_out.status.success(),
        "build should succeed; stderr: {}",
        String::from_utf8_lossy(&build_out.stderr)
    );
    let stdout = String::from_utf8_lossy(&build_out.stdout);
    let printed_path = stdout.trim();
    assert!(
        printed_path.ends_with("checksmix_test_hello.mmo"),
        "build should print the output path; got: {printed_path}"
    );
    assert!(tmp_mmo.exists(), "output .mmo file must exist");

    // run the .mmo via the explicit 'run' subcommand
    let run_status = checksmix().args(["run"]).arg(&tmp_mmo).status().unwrap();
    assert!(run_status.success(), "run of built .mmo should succeed");

    let _ = std::fs::remove_file(&tmp_mmo);
}

// ── run file.mmo: regression – MMO decode path unchanged ─────────────────────

#[test]
fn run_mmo_bare_invocation() {
    let tmp_mmo = std::env::temp_dir().join("checksmix_test_bare_run.mmo");

    // build first so we have a known-good .mmo
    let build_status = checksmix()
        .args(["build", "-o"])
        .arg(&tmp_mmo)
        .arg(fixture("hello.mms"))
        .status()
        .unwrap();
    assert!(build_status.success());

    // run without explicit subcommand
    let run_output = checksmix().arg(&tmp_mmo).output().unwrap();
    assert!(
        run_output.status.success(),
        "bare run of .mmo should succeed"
    );

    // The .mmo load path reports the program, never the loader's own probes.
    let stdout = String::from_utf8_lossy(&run_output.stdout);
    assert!(
        !stdout.contains("Debug:"),
        "loading a .mmo emitted debug output:\n{stdout}"
    );

    let _ = std::fs::remove_file(&tmp_mmo);
}

// ── run a.mms b.mms: multi-source assemble + execute ─────────────────────────

#[test]
fn run_multi_source_mms() {
    let status = checksmix()
        .args(["run"])
        .arg(fixture("multi_main.mms"))
        .arg(fixture("multi_lib.mms"))
        .status()
        .unwrap();
    assert!(status.success(), "run of multi-source .mms should succeed");
}

// ── run all_instructions_test.mms: no leftover debug noise on stderr ─────────

#[test]
fn run_all_instructions_test_has_no_debug_write_byte_noise() {
    let example = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join("all_instructions_test.mms");
    let out = checksmix().arg(&example).output().unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("DBG write_byte"),
        "stderr should not contain leftover debug output from write_byte; stderr: {}",
        stderr
    );
}

// ── an input contributing no source: diagnostic, not a panic ─────────────────
//
// resolve_includes trims away a blank segment; an input made entirely of
// such segments collects zero translation units.

#[test]
fn check_empty_file_exits_one() {
    let out = checksmix()
        .args(["check"])
        .arg(fixture("empty.mms"))
        .output()
        .unwrap();
    assert_exit_1_names_input(&out, "empty.mms");
}

#[test]
fn mmixasm_empty_file_exits_one() {
    let out = mmixasm().arg(fixture("empty.mms")).output().unwrap();
    assert_exit_1_names_input(&out, "empty.mms");
}

#[test]
fn mmixdb_empty_file_exits_one() {
    let out = mmixdb().arg(fixture("empty.mms")).output().unwrap();
    assert_exit_1_names_input(&out, "empty.mms");
}

#[test]
fn check_whitespace_only_file_exits_one() {
    let out = checksmix()
        .args(["check"])
        .arg(fixture("whitespace_only.mms"))
        .output()
        .unwrap();
    assert_exit_1_names_input(&out, "whitespace_only.mms");
}

#[test]
fn mmixasm_whitespace_only_file_exits_one() {
    let out = mmixasm()
        .arg(fixture("whitespace_only.mms"))
        .output()
        .unwrap();
    assert_exit_1_names_input(&out, "whitespace_only.mms");
}

#[test]
fn mmixdb_whitespace_only_file_exits_one() {
    let out = mmixdb()
        .arg(fixture("whitespace_only.mms"))
        .output()
        .unwrap();
    assert_exit_1_names_input(&out, "whitespace_only.mms");
}

#[test]
fn check_include_of_empty_file_exits_one() {
    let out = checksmix()
        .args(["check"])
        .arg(fixture("include_empty.mms"))
        .output()
        .unwrap();
    assert_exit_1_names_input(&out, "include_empty.mms");
}

#[test]
fn mmixasm_include_of_empty_file_exits_one() {
    let out = mmixasm()
        .arg(fixture("include_empty.mms"))
        .output()
        .unwrap();
    assert_exit_1_names_input(&out, "include_empty.mms");
}

#[test]
fn mmixdb_include_of_empty_file_exits_one() {
    let out = mmixdb().arg(fixture("include_empty.mms")).output().unwrap();
    assert_exit_1_names_input(&out, "include_empty.mms");
}

// ── checksmix run/build on an empty input ─────────────────────────────────────

#[test]
fn run_empty_file_exits_one_without_doubled_prefix() {
    let out = checksmix()
        .args(["run"])
        .arg(fixture("empty.mms"))
        .output()
        .unwrap();
    assert_exit_1_names_input(&out, "empty.mms");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(
        stderr.matches("Error: ").count(),
        1,
        "run_mms prefixes the error itself; embedding a second prefix in the \
         guard would double it; stderr: {stderr}"
    );
}

#[test]
fn build_empty_file_exits_one() {
    let out = checksmix()
        .args(["build"])
        .arg(fixture("empty.mms"))
        .output()
        .unwrap();
    assert_exit_1_names_input(&out, "empty.mms");
}

// ── one empty input among others: per-path, not aggregate ────────────────────
//
// hello.mms alone is a valid, self-contained program (multi_main.mms is not:
// it references :Lib, defined only in multi_lib.mms, so checking it alone
// fails on an undefined symbol and would mask what this case tests).

#[test]
fn check_empty_file_among_others_still_exits_one() {
    let out = checksmix()
        .args(["check"])
        .arg(fixture("empty.mms"))
        .arg(fixture("hello.mms"))
        .output()
        .unwrap();
    assert_exit_1_names_input(&out, "empty.mms");
}

#[test]
fn mmixasm_empty_file_among_others_exits_one_without_banner() {
    let out = mmixasm()
        .arg(fixture("empty.mms"))
        .arg(fixture("hello.mms"))
        .output()
        .unwrap();
    assert_exit_1_names_input(&out, "empty.mms");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.contains("Assembling 0 inputs:"),
        "the per-path guard fires before the input-count banner; stdout: {stdout}"
    );
}

#[test]
fn mmixdb_empty_file_among_others_still_exits_one() {
    let out = mmixdb()
        .arg(fixture("empty.mms"))
        .arg(fixture("hello.mms"))
        .output()
        .unwrap();
    assert_exit_1_names_input(&out, "empty.mms");
}

// ── regression: legitimate zero-instruction inputs stay unaffected ───────────
//
// The guard keys off resolved units, never instruction count: a comment-only
// file and a definitions-only module both contribute source and must keep
// exiting 0 on `check`.

#[test]
fn check_comment_only_file_exits_zero() {
    let status = checksmix()
        .args(["check"])
        .arg(fixture("comment_only.mms"))
        .status()
        .unwrap();
    assert!(status.success(), "a comment-only file is a valid program");
}

#[test]
fn check_defs_only_file_exits_zero() {
    let status = checksmix()
        .args(["check"])
        .arg(fixture("defs_only.mms"))
        .status()
        .unwrap();
    assert!(
        status.success(),
        "a definitions-only module has zero instructions and is still valid"
    );
}

#[test]
fn build_defs_only_file_fails_on_instruction_count_not_the_source_guard() {
    let out = checksmix()
        .args(["build"])
        .arg(fixture("defs_only.mms"))
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("no instructions"),
        "build's existing zero-instruction guard must still fire; stderr: {stderr}"
    );
    assert!(
        !stderr.contains("contributed no source"),
        "defs_only.mms resolves to a unit; the new guard must not fire; stderr: {stderr}"
    );
}
