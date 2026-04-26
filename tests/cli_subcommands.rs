use std::path::PathBuf;
use std::process::Command;

fn checksmix() -> Command {
    Command::new(env!("CARGO_BIN_EXE_checksmix"))
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
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
    let run_status = checksmix().arg(&tmp_mmo).status().unwrap();
    assert!(run_status.success(), "bare run of .mmo should succeed");

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
