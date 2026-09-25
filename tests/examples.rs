#![cfg(feature = "cli")]

use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{Read, pipe};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const RUN_DEADLINE: Duration = Duration::from_secs(60);

fn checksmix() -> Command {
    Command::new(env!("CARGO_BIN_EXE_checksmix"))
}

fn examples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples")
}

fn readme_text() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("README.md");
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// Every `.mms` file directly under `examples/`, discovered fresh each run
/// so a new file is covered without editing this list.
fn example_files() -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = fs::read_dir(examples_dir())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "mms"))
        .collect();
    files.sort();
    files
}

fn file_name(path: &Path) -> &str {
    path.file_name().unwrap().to_str().unwrap()
}

/// The `$255` halt value every example under `examples/` is pinned to.
fn expected_halts() -> HashMap<&'static str, u64> {
    HashMap::from([
        ("all_instructions_test.mms", 0),
        ("big_fib.mms", 0),
        ("exit_code.mms", 42),
        ("fibonacci.mms", 0),
        ("hello_halt.mms", 0),
        ("hello_world.mms", 0),
        ("leapyear.mms", 0),
        ("linked_list.mms", 0),
        ("prime.mms", 0),
        ("subroutine.mms", 42),
        ("time.mms", 0),
    ])
}

/// Runs `checksmix run <path>` with stdin null and `RUST_LOG` removed,
/// killing the child and failing the test past `RUN_DEADLINE`: a hang fails
/// the test instead of stalling it.
fn run_example(path: &Path) -> Output {
    let mut child = checksmix()
        .args(["run"])
        .arg(path)
        .stdin(Stdio::null())
        .env_remove("RUST_LOG")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
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
    let deadline = Instant::now() + RUN_DEADLINE;
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("{} exceeded {RUN_DEADLINE:?} wall clock", path.display());
        }
        thread::sleep(Duration::from_millis(20));
    };
    Output {
        status,
        stdout: stdout_reader.join().unwrap(),
        stderr: stderr_reader.join().unwrap(),
    }
}

/// The `N` a `HALT trap at PC=…, exit code=N` line on stderr names.
fn halt_exit_code(out: &Output) -> u64 {
    let stderr = String::from_utf8_lossy(&out.stderr);
    stderr
        .lines()
        .find_map(|line| line.split("exit code=").nth(1))
        .unwrap_or_else(|| panic!("no HALT line in stderr: {stderr}"))
        .trim()
        .parse()
        .unwrap_or_else(|e| panic!("unparseable exit code ({e}); stderr: {stderr}"))
}

// ── halt values ───────────────────────────────────────────────────────────

#[test]
fn every_example_halts_with_its_declared_value() {
    let expected = expected_halts();
    for path in example_files() {
        let name = file_name(&path);
        let Some(&want) = expected.get(name) else {
            // Table completeness is a separate test; an undeclared example
            // is silently skipped here.
            continue;
        };
        let out = run_example(&path);
        let got = halt_exit_code(&out);
        assert_eq!(got, want, "{name}: HALT line's exit code");
        assert_eq!(
            out.status.code(),
            Some((want % 256) as i32),
            "{name}: process exit status"
        );
    }
}

// ── table completeness ───────────────────────────────────────────────────

#[test]
fn halt_table_names_exactly_the_examples_directory() {
    let files: HashSet<String> = example_files()
        .iter()
        .map(|p| file_name(p).to_string())
        .collect();
    let table: HashSet<String> = expected_halts().keys().map(|s| s.to_string()).collect();
    for name in &files {
        assert!(
            table.contains(name),
            "{name} is in examples/ but missing from the halt-value table"
        );
    }
    for name in &table {
        assert!(
            files.contains(name),
            "the halt-value table names {name}, which examples/ does not have"
        );
    }
}

// ── headers ───────────────────────────────────────────────────────────────

#[test]
fn every_example_header_names_itself() {
    for path in example_files() {
        let name = file_name(&path);
        let contents = fs::read_to_string(&path).unwrap();
        let first_line = contents.lines().next().unwrap_or_default();
        let want_prefix = format!("% {name} -- ");
        assert!(
            first_line.starts_with(&want_prefix),
            "{name}: line 1 is {first_line:?}, want it to start with {want_prefix:?}"
        );
    }
}

// ── fibonacci and time: printed output ──────────────────────────────────

#[test]
fn fibonacci_prints_its_result() {
    let out = run_example(&examples_dir().join("fibonacci.mms"));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("fib(20) = 6765"), "stdout: {stdout}");
}

#[test]
fn time_prints_the_host_clock() {
    let before = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let out = run_example(&examples_dir().join("time.mms"));
    let stdout = String::from_utf8_lossy(&out.stdout);
    let line = stdout
        .lines()
        .find(|line| line.ends_with("seconds since the Unix epoch"))
        .unwrap_or_else(|| panic!("no clock line in stdout: {stdout}"));
    let n: u64 = line
        .split_whitespace()
        .next()
        .unwrap_or_else(|| panic!("empty clock line: {line:?}"))
        .parse()
        .unwrap_or_else(|e| panic!("unparseable seconds ({e}) in {line:?}"));
    let after = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    assert!(
        n + 60 >= before && n <= after + 60,
        "n={n} not within 60s of before={before}, after={after}"
    );
}

// ── the README's leapyear listing and transcript ─────────────────────────

/// The text between an opening ` ```<lang> ` fence and its closing ` ``` `.
fn fenced_block(markdown: &str, lang: &str) -> String {
    let open = format!("```{lang}\n");
    let start = markdown
        .find(&open)
        .unwrap_or_else(|| panic!("no ```{lang} fence in README.md"))
        + open.len();
    let rest = &markdown[start..];
    let end = rest
        .find("```")
        .unwrap_or_else(|| panic!("unterminated ```{lang} fence in README.md"));
    rest[..end].to_string()
}

#[test]
fn readme_listing_matches_leapyear_source() {
    let block = fenced_block(&readme_text(), "mmix");
    let source = fs::read_to_string(examples_dir().join("leapyear.mms")).unwrap();
    assert_eq!(
        block, source,
        "README's mmix listing block vs examples/leapyear.mms"
    );
}

/// Runs `checksmix leapyear.mms` from `examples/`, stdout and stderr
/// interleaved on one pipe the way a terminal would show them, bounded by
/// `RUN_DEADLINE` so a hang fails the test instead of stalling it.
fn run_leapyear_combined() -> Vec<u8> {
    let (mut reader, writer) = pipe().unwrap();
    let writer2 = writer.try_clone().unwrap();
    let child = checksmix()
        .arg("leapyear.mms")
        .current_dir(examples_dir())
        .stdin(Stdio::null())
        .env_remove("RUST_LOG")
        .stdout(Stdio::from(writer))
        .stderr(Stdio::from(writer2))
        .spawn()
        .unwrap();
    let child = Arc::new(Mutex::new(child));
    let done = Arc::new(AtomicBool::new(false));
    let killed = Arc::new(AtomicBool::new(false));
    // Detached: it sleeps for the full deadline regardless of `done`, so
    // joining it would defeat the point of an early, successful read.
    {
        let child = child.clone();
        let done = done.clone();
        let killed = killed.clone();
        thread::spawn(move || {
            thread::sleep(RUN_DEADLINE);
            if !done.load(Ordering::SeqCst) {
                killed.store(true, Ordering::SeqCst);
                let _ = child.lock().unwrap().kill();
            }
        });
    }
    let mut buf = Vec::new();
    reader.read_to_end(&mut buf).unwrap();
    done.store(true, Ordering::SeqCst);
    let status = child.lock().unwrap().wait().unwrap();
    assert!(
        !killed.load(Ordering::SeqCst),
        "checksmix leapyear.mms exceeded {RUN_DEADLINE:?} wall clock"
    );
    assert!(
        status.success(),
        "checksmix leapyear.mms should exit 0; combined output: {}",
        String::from_utf8_lossy(&buf)
    );
    buf
}

#[test]
fn readme_transcript_matches_a_live_run() {
    let console = fenced_block(&readme_text(), "console");
    let prompt_line = "$ checksmix leapyear.mms\n";
    assert!(
        console.starts_with(prompt_line),
        "README's Run block should start with {prompt_line:?}"
    );
    let expected = &console[prompt_line.len()..];
    let got = run_leapyear_combined();
    assert_eq!(
        String::from_utf8_lossy(&got),
        expected,
        "README's Run block vs a live `checksmix leapyear.mms` run"
    );
}
