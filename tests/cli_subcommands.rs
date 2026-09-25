#![cfg(feature = "cli")]

use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

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

/// Every `--max-steps` check runs with stdin null and `RUST_LOG` unset, so a
/// run's own tracing spans never leak into the assertions.
fn hermetic_output(cmd: &mut Command) -> Output {
    cmd.stdin(Stdio::null())
        .env_remove("RUST_LOG")
        .output()
        .unwrap()
}

/// Runs a child expected to be stopped by its own `--max-steps` budget, but
/// bounds its wall clock too: a budget that misses its path fails the test
/// instead of hanging the test binary on a looping child.
fn hermetic_output_within(cmd: &mut Command, limit: Duration) -> Output {
    cmd.stdin(Stdio::null())
        .env_remove("RUST_LOG")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().unwrap();
    let mut stdout = child.stdout.take().unwrap();
    let mut stderr = child.stderr.take().unwrap();
    let stdout_reader = thread::spawn(move || {
        let mut buf = Vec::new();
        stdout.read_to_end(&mut buf).unwrap();
        buf
    });
    let stderr_reader = thread::spawn(move || {
        let mut buf = Vec::new();
        stderr.read_to_end(&mut buf).unwrap();
        buf
    });
    let deadline = Instant::now() + limit;
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("child exceeded {limit:?} wall clock; --max-steps failed to bound it");
        }
        thread::sleep(Duration::from_millis(20));
    };
    Output {
        status,
        stdout: stdout_reader.join().unwrap(),
        stderr: stderr_reader.join().unwrap(),
    }
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

fn assert_exit_124_names_budget(out: &Output, line: &str) {
    assert_eq!(out.status.code(), Some(124));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.lines().any(|l| l == line), "stderr: {stderr}");
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

// ── run: a diagnostic halt exits 1; a Halt trap keeps its own status ────────

#[test]
fn run_illegal_instruction_exits_1() {
    let status = checksmix()
        .args(["run"])
        .arg(fixture("put_rc_halts.mms"))
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(1), "a diagnostic halt (PUT rC) exits 1");
}

#[test]
fn run_halt_trap_exits_with_its_own_255_value() {
    let status = checksmix()
        .args(["run"])
        .arg(fixture("halt_with_255.mms"))
        .status()
        .unwrap();
    assert_eq!(
        status.code(),
        Some(255),
        "TRAP 0,Halt,0 exits with $255, not 1"
    );
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

// ── run: an unrecognized extension names what runs ──────────────────────────

#[test]
fn run_unknown_extension_exits_1_and_lists_supported_extensions() {
    let out = checksmix().arg("program.txt").output().unwrap();
    assert_eq!(out.status.code(), Some(1), "unknown extension exits 1");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr
            .lines()
            .any(|line| line == "Supported extensions: .mms, .mmo"),
        "stderr should list the supported extensions; stderr: {stderr}"
    );
}

// ── build/mmixasm: a program with a GREG carries it in the postamble ────────

fn find_lop_post(mmo_data: &[u8]) -> usize {
    mmo_data
        .windows(2)
        .position(|w| w[0] == 0x98 && w[1] == 0x0A)
        .expect("built .mmo must have a lop_post record")
}

/// `checksmix build` and `mmixasm` share one object-code path; each must
/// carry a program's `GREG` value through its postamble.
#[test]
fn each_binarys_build_of_a_program_with_a_greg_carries_it_in_the_postamble() {
    let cases: [(Command, &[&str], &str); 2] = [
        (
            checksmix(),
            &["build", "-o"],
            "checksmix_test_greg_build.mmo",
        ),
        (mmixasm(), &["-o"], "checksmix_test_greg_mmixasm.mmo"),
    ];

    for (mut cmd, args, tmp_name) in cases {
        let tmp_mmo = std::env::temp_dir().join(tmp_name);

        let build_out = cmd
            .args(args)
            .arg(&tmp_mmo)
            .arg(fixture("greg_postamble.mms"))
            .output()
            .unwrap();
        assert!(
            build_out.status.success(),
            "build should succeed; stderr: {}",
            String::from_utf8_lossy(&build_out.stderr)
        );

        let mmo_data = std::fs::read(&tmp_mmo).unwrap();
        let post = find_lop_post(&mmo_data);
        assert_eq!(mmo_data[post + 3], 254, "G must be 254 for one GREG");
        let reg254 = u64::from_be_bytes(mmo_data[post + 4..post + 12].try_into().unwrap());
        assert_eq!(reg254, 1000, "$254 must carry the GREG's value");

        let _ = std::fs::remove_file(&tmp_mmo);
    }
}

// ── run greg_base_address.mms: the two-operand base-address form ─────────────
//
// The one operand form AGENTS.md's corpus rule excuses from
// examples/all_instructions_test.mms (see its "Intentional coverage
// exceptions" block): it needs a GREG holding a base address, which that
// file's harness cannot admit without a register-numbering rework. The
// fixture halts only through `TRAP 0,Halt,X`, whose exit status is `$255`'s
// own value rather than the 1 every other halt exits with, so checking it
// alongside the printed message confirms the run reached the fixture's own
// Pass or Fail branch rather than some other halt.

#[test]
fn run_greg_base_address_fixture_passes() {
    let out = checksmix()
        .args(["run"])
        .arg(fixture("greg_base_address.mms"))
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("All tests passed!"),
        "stdout should report success; stdout: {stdout}"
    );
    assert_eq!(out.status.code(), Some(0), "the fixture halts with $255=0");
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

// ── run --max-steps: stop a program that has not halted ─────────────────────

#[test]
fn run_max_steps_stops_a_looping_mms_program() {
    let mut cmd = checksmix();
    cmd.args(["run", "--max-steps", "1000"])
        .arg(fixture("loop_forever.mms"));
    let out = hermetic_output_within(&mut cmd, Duration::from_secs(10));

    assert_exit_124_names_budget(
        &out,
        "program did not halt within 1000 instructions; @ = #100",
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Executed 1000 instructions"),
        "stdout: {stdout}"
    );
    assert!(
        !stdout.contains("Execution completed."),
        "an exhausted budget must not print the completion line; stdout: {stdout}"
    );
}

#[test]
fn max_steps_without_a_subcommand_stops_the_default_run() {
    let mut cmd = checksmix();
    cmd.args(["--max-steps", "1000"])
        .arg(fixture("loop_forever.mms"));
    let out = hermetic_output_within(&mut cmd, Duration::from_secs(10));

    assert_exit_124_names_budget(
        &out,
        "program did not halt within 1000 instructions; @ = #100",
    );
}

#[test]
fn run_max_steps_stops_a_looping_mmo_program() {
    let tmp_mmo = std::env::temp_dir().join("checksmix_test_loop_forever.mmo");
    let build_status = checksmix()
        .args(["build", "-o"])
        .arg(&tmp_mmo)
        .arg(fixture("loop_forever.mms"))
        .status()
        .unwrap();
    assert!(build_status.success());

    let mut cmd = checksmix();
    cmd.args(["run", "--max-steps", "1000"]).arg(&tmp_mmo);
    let out = hermetic_output_within(&mut cmd, Duration::from_secs(10));

    let _ = std::fs::remove_file(&tmp_mmo);

    assert_exit_124_names_budget(
        &out,
        "program did not halt within 1000 instructions; @ = #100",
    );
}

#[test]
fn run_max_steps_one_stops_after_the_first_instruction_across_files() {
    let mut cmd = checksmix();
    cmd.args(["run", "--max-steps", "1"])
        .arg(fixture("multi_main.mms"))
        .arg(fixture("multi_lib.mms"));
    let out = hermetic_output(&mut cmd);

    assert_exit_124_names_budget(&out, "program did not halt within 1 instruction; @ = #200");
}

#[test]
fn run_halt_with_255_is_unchanged_by_a_budget_it_does_not_need() {
    let unflagged = hermetic_output(checksmix().args(["run"]).arg(fixture("halt_with_255.mms")));
    assert_eq!(unflagged.status.code(), Some(255));
    let stderr = String::from_utf8_lossy(&unflagged.stderr);
    let lines: Vec<&str> = stderr.lines().collect();
    assert_eq!(
        lines,
        vec![
            "HALT trap at PC=0x0000000000000104, exit code=255",
            "Execution stopped at PC=0x0000000000000108 after 1 instructions",
        ]
    );
    let stdout = String::from_utf8_lossy(&unflagged.stdout);
    assert!(
        stdout.trim_end().ends_with("Execution completed."),
        "stdout: {stdout}"
    );

    let flagged = hermetic_output(
        checksmix()
            .args(["run", "--max-steps", "2"])
            .arg(fixture("halt_with_255.mms")),
    );
    assert_eq!(
        flagged.stdout, unflagged.stdout,
        "stdout must match exactly"
    );
    assert_eq!(
        flagged.stderr, unflagged.stderr,
        "stderr must match exactly"
    );
    assert_eq!(flagged.status.code(), unflagged.status.code());
}

#[test]
fn run_max_steps_one_stops_before_the_halt_trap_executes() {
    let out = hermetic_output(
        checksmix()
            .args(["run", "--max-steps", "1"])
            .arg(fixture("halt_with_255.mms")),
    );

    assert_exit_124_names_budget(&out, "program did not halt within 1 instruction; @ = #104");
}

#[test]
fn run_max_steps_rejects_zero_and_non_numeric_values() {
    for bad in ["0", "ten"] {
        let out = hermetic_output(
            checksmix()
                .args(["run", "--max-steps", bad])
                .arg(fixture("hello.mms")),
        );

        assert_eq!(out.status.code(), Some(2), "bad value: {bad}");
        assert!(out.stdout.is_empty(), "bad value: {bad}");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.contains(&format!("invalid value '{bad}'")) && stderr.contains("--max-steps"),
            "stderr should name the rejected value and --max-steps; bad value: {bad}; \
             stderr: {stderr}"
        );
        assert!(
            !stderr.contains("unexpected argument"),
            "bad value: {bad}; stderr: {stderr}"
        );
    }
}

// ── An overflowing data value warns to stderr and still exits 0 ──────────────

fn overflow_warning() -> String {
    format!(
        "{}:5:17: warning: value 300 does not fit in a byte; its low byte assembles",
        fixture("overflowing_byte.mms").display()
    )
}

#[test]
fn check_overflowing_byte_warns_and_exits_zero() {
    let out = checksmix()
        .args(["check"])
        .arg(fixture("overflowing_byte.mms"))
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "an overflowing data value warns, it does not fail; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains(&overflow_warning()),
        "check should print the overflow warning; stderr: {stderr}"
    );
}

#[test]
fn mmixasm_overflowing_byte_warns_and_exits_zero() {
    let tmp_mmo = std::env::temp_dir().join("checksmix_test_overflowing_byte.mmo");
    let out = mmixasm()
        .args(["-o"])
        .arg(&tmp_mmo)
        .arg(fixture("overflowing_byte.mms"))
        .output()
        .unwrap();
    let _ = std::fs::remove_file(&tmp_mmo);
    assert!(
        out.status.success(),
        "an overflowing data value warns, it does not fail; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains(&overflow_warning()),
        "mmixasm should print the overflow warning; stderr: {stderr}"
    );
}

#[test]
fn mmixdb_overflowing_byte_warns_and_exits_zero() {
    let out = mmixdb()
        .arg(fixture("overflowing_byte.mms"))
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "an overflowing data value warns, it does not fail; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains(&overflow_warning()),
        "mmixdb should print the overflow warning; stderr: {stderr}"
    );
}

#[test]
fn run_overflowing_byte_warns_and_halts_cleanly() {
    let out = checksmix()
        .args(["run"])
        .arg(fixture("overflowing_byte.mms"))
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "an overflowing data value warns, it does not stop the run; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains(&overflow_warning()),
        "run should print the overflow warning; stderr: {stderr}"
    );
}

// ── mmixasm: a clean build writes nothing to stderr ──────────────────────

#[test]
fn mmixasm_clean_build_writes_nothing_to_stderr() {
    let tmp_mmo = std::env::temp_dir().join("checksmix_test_mmixasm_clean_build.mmo");
    let mut cmd = mmixasm();
    cmd.args(["-o"]).arg(&tmp_mmo).arg(fixture("hello.mms"));
    let out = hermetic_output(&mut cmd);
    let _ = std::fs::remove_file(&tmp_mmo);
    assert!(
        out.status.success(),
        "a clean build should succeed; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        out.stderr.is_empty(),
        "mmixasm should write nothing to stderr on a clean build; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}
