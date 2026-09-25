//! Whole-program soundness suite: each program runs to completion under
//! checksmix and its answer is checked two ways. A known answer, computed
//! independently of checksmix, proves the answer is *correct*; a golden
//! state dump taken from a passing run proves a later run is *the same*.
//! Neither substitutes for the other -- a golden taken from a wrong
//! emulator freezes the wrong answer, and a known answer alone misses a
//! change that leaves the printed answer alone but moves the machine
//! state underneath it.
//!
//! A golden lives at `tests/resources/soundness/golden/<program>.golden`:
//!
//! ```text
//! checksmix <version> <YYYY-MM-DD>
//! sha256 <64 lowercase hex digits>
//! exit <exit code, decimal>
//! instructions <count, decimal>
//! peak-stack-octas <depth, decimal>
//! stdout <byte count, decimal>
//! <the stdout bytes verbatim, then one newline>
//! state
//! <the final machine's Display rendering>
//! memory
//! <one line per octa-aligned address holding a nonzero byte, ascending>
//! ```
//!
//! Line 1 is provenance and plays no part in comparison, so a golden
//! stays valid across releases. Line 2 is the SHA-256 of every byte
//! after it -- the body -- and is checked against that body before the
//! body itself is compared, so a hand-edited golden is caught even if
//! the edit happens to match what the run produces.
//!
//! Set `CHECKSMIX_REWRITE_GOLDEN` to have a program test that has passed
//! its known-answer assertions write its golden instead of comparing
//! against it. A wrong answer never gets a golden.

use checksmix::{
    Host, MMix, MMixAssembler, SpecialReg, TrapCode, entry_point, start_program, write_image,
};
use std::cell::RefCell;
use std::env::var_os;
use std::fs;
use std::io::{Result, Write};
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

const BUDGET: usize = 20_000_000;

const SHA256_EMPTY_DIGEST: &str =
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
const SHA256_ABC_DIGEST: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
const SHA256_TWO_BLOCK_MESSAGE: &str = "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq";
const SHA256_TWO_BLOCK_DIGEST: &str =
    "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1";

const PI_KNOWN_SHA256: &str = "e898fea26734a6d3af5396b9f4c60ae5dcc88fc40944d835911a9ee8a672ea1b";

#[rustfmt::skip]
const PI_DECIMALS: &str = concat!(
    "1415926535897932384626433832795028841971693993751058209749445923078164062862089986280348253421170679",
    "8214808651328230664709384460955058223172535940812848111745028410270193852110555964462294895493038196",
    "4428810975665933446128475648233786783165271201909145648566923460348610454326648213393607260249141273",
    "7245870066063155881748815209209628292540917153643678925903600113305305488204665213841469519415116094",
    "3305727036575959195309218611738193261179310511854807446237996274956735188575272489122793818301194912",
    "9833673362440656643086021394946395224737190702179860943702770539217176293176752384674818467669405132",
    "0005681271452635608277857713427577896091736371787214684409012249534301465495853710507922796892589235",
    "4201995611212902196086403441815981362977477130996051870721134999999837297804995105973173281609631859",
    "5024459455346908302642522308253344685035261931188171010003137838752886587533208381420617177669147303",
    "5982534904287554687311595628638823537875937519577818577805321712268066130019278766111959092164201989",
);

/// The Ackermann-Peter function's closed forms for m = 0..3: A(0,n)=n+1,
/// A(1,n)=n+2, A(2,n)=2n+3, A(3,n)=2^(n+3)-3.
fn ackermann_closed_form(m: u64, n: u64) -> u64 {
    match m {
        0 => n + 1,
        1 => n + 2,
        2 => 2 * n + 3,
        3 => (1u64 << (n + 3)) - 3,
        _ => panic!("ackermann.mms only exercises m = 0..=3, got m={m}"),
    }
}

fn ackermann_expected_stdout() -> String {
    let mut out = String::new();
    for m in 0..=3u64 {
        for n in 0..=5u64 {
            out.push_str(&format!("A({m},{n}) = {}\n", ackermann_closed_form(m, n)));
        }
    }
    out
}

fn sha256_expected_stdout() -> String {
    format!("{SHA256_EMPTY_DIGEST}\n{SHA256_ABC_DIGEST}\n{SHA256_TWO_BLOCK_DIGEST}\n")
}

fn pi_expected_stdout() -> String {
    format!("3.{PI_DECIMALS}\n")
}

// ============================================================
// SHA-256, written here so the goldens' own hash needs no dependency.
// K and H are the first 32 bits of the fractional part of the cube
// roots and square roots of the first 64 and 8 primes (FIPS 180-4
// 4.2.2).
// ============================================================

const SHA256_K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

const SHA256_H0: [u32; 8] = [
    0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
];

/// Expands one 64-byte block into the 64-word message schedule (FIPS
/// 180-4 6.2.2 step 1).
fn message_schedule(block: &[u8; 64]) -> [u32; 64] {
    let mut w = [0u32; 64];
    for (i, word) in w.iter_mut().take(16).enumerate() {
        *word = u32::from_be_bytes(block[i * 4..i * 4 + 4].try_into().unwrap());
    }
    for i in 16..64 {
        let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
        let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
        w[i] = w[i - 16]
            .wrapping_add(s0)
            .wrapping_add(w[i - 7])
            .wrapping_add(s1);
    }
    w
}

/// Runs the 64-round compression function over one block's schedule and
/// folds the result into `h` (FIPS 180-4 6.2.2 steps 2-4).
fn compress(h: &mut [u32; 8], w: &[u32; 64]) {
    let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = *h;
    for i in 0..64 {
        let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
        let ch = (e & f) ^ (!e & g);
        let t1 = hh
            .wrapping_add(s1)
            .wrapping_add(ch)
            .wrapping_add(SHA256_K[i])
            .wrapping_add(w[i]);
        let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
        let maj = (a & b) ^ (a & c) ^ (b & c);
        let t2 = s0.wrapping_add(maj);

        hh = g;
        g = f;
        f = e;
        e = d.wrapping_add(t1);
        d = c;
        c = b;
        b = a;
        a = t1.wrapping_add(t2);
    }

    for (word, delta) in h.iter_mut().zip([a, b, c, d, e, f, g, hh]) {
        *word = word.wrapping_add(delta);
    }
}

fn sha256(data: &[u8]) -> [u8; 32] {
    let mut h = SHA256_H0;

    let bit_len = (data.len() as u64) * 8;
    let mut message = data.to_vec();
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bit_len.to_be_bytes());

    let (blocks, _) = message.as_chunks::<64>();
    for block in blocks {
        let w = message_schedule(block);
        compress(&mut h, &w);
    }

    let mut out = [0u8; 32];
    for (i, word) in h.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

fn sha256_hex(data: &[u8]) -> String {
    sha256(data).iter().map(|b| format!("{b:02x}")).collect()
}

// ============================================================
// The run: drives the library the way `checksmix run` does, stepping
// with `execute_instruction` so `rO` can be sampled after every one.
// ============================================================

#[derive(Default)]
struct Capture {
    stdout: Vec<u8>,
    halted: bool,
    disallowed_trap: Option<String>,
    diagnostics: Vec<String>,
}

struct TestHost(Rc<RefCell<Capture>>);

impl Host for TestHost {
    fn write(&mut self, fd: u8, bytes: &[u8]) -> Result<()> {
        if fd == 1 {
            self.0.borrow_mut().stdout.extend_from_slice(bytes);
        }
        Ok(())
    }

    fn now_micros(&mut self) -> u64 {
        0
    }

    fn diagnostic(&mut self, msg: &str) {
        self.0.borrow_mut().diagnostics.push(msg.to_string());
    }

    fn trap(&mut self, code: TrapCode, arg: u8, _arg255: u64, _result255: u64) {
        let mut capture = self.0.borrow_mut();
        match code {
            TrapCode::Halt => {
                capture.halted = true;
                // handle_halt reports a clean shutdown through the same
                // diagnostic hook, just before this call; that notice is
                // not an anomaly, so it never counts against the run.
                capture.diagnostics.pop();
            }
            TrapCode::Fputs if arg == 1 => {}
            other => {
                if capture.disallowed_trap.is_none() {
                    capture.disallowed_trap = Some(format!("{other:?} (handle {arg})"));
                }
            }
        }
    }
}

struct RunOutcome {
    exit_code: u64,
    instructions: usize,
    peak_stack_octas: u64,
    stdout: Vec<u8>,
}

/// Appends every diagnostic collected so far to a failure `reason`, so a
/// harness error names the emulator's own account of what went wrong.
fn fail(program: &str, reason: &str, diagnostics: &[String]) -> String {
    if diagnostics.is_empty() {
        format!("{program}: {reason}")
    } else {
        format!(
            "{program}: {reason} -- diagnostics: {}",
            diagnostics.join("; ")
        )
    }
}

/// Assembles and runs `source`, stopping at `budget` instructions. Fails
/// on a disallowed trap, on exhausting the budget, on a halt that never
/// went through the `Halt` trap, or on a clean halt that still emitted a
/// diagnostic along the way.
fn run_program(
    program: &str,
    source: &str,
    budget: usize,
) -> std::result::Result<(MMix, RunOutcome), String> {
    let mut asm = MMixAssembler::new(source, program);
    asm.parse()
        .map_err(|err| format!("{program}: assemble error: {err}"))?;

    let capture = Rc::new(RefCell::new(Capture::default()));
    let mut mmix = MMix::with_host(TestHost(capture.clone()));
    write_image(&mut mmix, &asm);
    start_program(&mut mmix, entry_point(&asm));

    let ro0 = mmix.get_special(SpecialReg::RO);
    let mut peak_stack_octas = 0u64;
    let mut instructions = 0usize;

    let halted = loop {
        if instructions >= budget {
            let diagnostics = capture.borrow().diagnostics.clone();
            return Err(fail(
                program,
                &format!("budget {budget} exhausted at PC={:#018x}", mmix.get_pc()),
                &diagnostics,
            ));
        }
        let continued = mmix.execute_instruction();
        if let Some(trap) = capture.borrow().disallowed_trap.clone() {
            let diagnostics = capture.borrow().diagnostics.clone();
            return Err(fail(
                program,
                &format!("disallowed trap {trap}"),
                &diagnostics,
            ));
        }
        let depth = mmix.get_special(SpecialReg::RO).wrapping_sub(ro0) / 8;
        peak_stack_octas = peak_stack_octas.max(depth);
        if !continued {
            break capture.borrow().halted;
        }
        instructions += 1;
    };

    let diagnostics = capture.borrow().diagnostics.clone();
    if !halted {
        return Err(fail(program, "halted without a Halt trap", &diagnostics));
    }
    if !diagnostics.is_empty() {
        return Err(fail(program, "run emitted a diagnostic", &diagnostics));
    }

    let outcome = RunOutcome {
        exit_code: mmix.get_exit_code(),
        instructions,
        peak_stack_octas,
        stdout: capture.borrow().stdout.clone(),
    };
    Ok((mmix, outcome))
}

// ============================================================
// Golden files.
// ============================================================

fn resource_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/resources/soundness")
        .join(name)
}

fn golden_path(program: &str) -> PathBuf {
    resource_path("golden").join(format!("{program}.golden"))
}

/// Every octa-aligned address holding a nonzero byte, ascending.
fn occupied_octas(mmix: &MMix) -> Vec<(u64, u64)> {
    let mut addrs: Vec<u64> = mmix.occupied().map(|(addr, _)| addr & !7).collect();
    addrs.sort_unstable();
    addrs.dedup();
    addrs
        .into_iter()
        .map(|addr| (addr, mmix.read_octa(addr)))
        .collect()
}

fn golden_body(mmix: &MMix, outcome: &RunOutcome) -> Vec<u8> {
    // A Vec<u8> Write impl never fails, so every write! below is infallible.
    let mut body = Vec::new();
    writeln!(body, "exit {}", outcome.exit_code).unwrap();
    writeln!(body, "instructions {}", outcome.instructions).unwrap();
    writeln!(body, "peak-stack-octas {}", outcome.peak_stack_octas).unwrap();
    writeln!(body, "stdout {}", outcome.stdout.len()).unwrap();
    body.extend_from_slice(&outcome.stdout);
    body.push(b'\n');
    writeln!(body, "state").unwrap();
    write!(body, "{mmix}").unwrap();
    writeln!(body, "memory").unwrap();
    for (addr, octa) in occupied_octas(mmix) {
        writeln!(body, "#{addr:016x} #{octa:016x}").unwrap();
    }
    body
}

/// Days-since-epoch to a civil (year, month, day), Howard Hinnant's
/// `civil_from_days` (http://howardhinnant.github.io/date_algorithms.html),
/// so the golden's provenance line needs no date dependency.
fn civil_from_days(days_since_epoch: i64) -> (i64, u32, u32) {
    let z = days_since_epoch + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

fn today_utc() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let (y, m, d) = civil_from_days((secs / 86_400) as i64);
    format!("{y:04}-{m:02}-{d:02}")
}

fn write_golden(program: &str, body: &[u8]) -> std::result::Result<(), String> {
    let mut file = format!(
        "checksmix {} {}\nsha256 {}\n",
        env!("CARGO_PKG_VERSION"),
        today_utc(),
        sha256_hex(body)
    )
    .into_bytes();
    file.extend_from_slice(body);
    fs::write(golden_path(program), file).map_err(|err| format!("{program}: writing golden: {err}"))
}

/// Checks `actual_body` against a golden file's raw `contents`: line 2
/// must be `sha256 ` followed by the SHA-256 of everything after it, and
/// that everything must equal `actual_body` byte for byte. `program` and
/// the rewrite variable name every failure.
fn check_golden_bytes(
    program: &str,
    contents: &[u8],
    actual_body: &[u8],
) -> std::result::Result<(), String> {
    let mut parts = contents.splitn(3, |&b| b == b'\n');
    let _provenance = parts.next().unwrap_or_default();
    let line2 = parts.next().unwrap_or_default();
    let golden_body = parts.next().unwrap_or_default();

    let expected_line2 = format!("sha256 {}", sha256_hex(golden_body));
    let recorded_line2 = String::from_utf8_lossy(line2);
    if recorded_line2 != expected_line2 {
        return Err(format!(
            "{program}: golden's line 2 ({recorded_line2}) is stale -- its body hashes to \
             {expected_line2}; set CHECKSMIX_REWRITE_GOLDEN to rewrite it"
        ));
    }

    if golden_body != actual_body {
        let expected_lines: Vec<&[u8]> = golden_body.split(|&b| b == b'\n').collect();
        let actual_lines: Vec<&[u8]> = actual_body.split(|&b| b == b'\n').collect();
        let mismatch = expected_lines
            .iter()
            .zip(actual_lines.iter())
            .enumerate()
            .find(|(_, (expected, actual))| expected != actual);
        let (line, expected, actual) = match mismatch {
            Some((i, (expected, actual))) => (
                i + 1,
                String::from_utf8_lossy(expected).into_owned(),
                String::from_utf8_lossy(actual).into_owned(),
            ),
            None => (
                expected_lines.len().min(actual_lines.len()) + 1,
                "<end of body>".to_string(),
                "<more lines than the golden>".to_string(),
            ),
        };
        return Err(format!(
            "{program}: golden differs at body line {line}\n  expected: {expected}\n  \
             actual:   {actual}\nset CHECKSMIX_REWRITE_GOLDEN to rewrite it"
        ));
    }

    Ok(())
}

fn compare_golden(program: &str, actual_body: &[u8]) -> std::result::Result<(), String> {
    let path = golden_path(program);
    let contents = fs::read(&path).map_err(|_| {
        format!(
            "{program}: no golden at {}; set CHECKSMIX_REWRITE_GOLDEN to write one",
            path.display()
        )
    })?;
    check_golden_bytes(program, &contents, actual_body)
}

/// Writes the golden if `CHECKSMIX_REWRITE_GOLDEN` is set, else compares
/// against it. Called only after a program's known-answer assertions
/// have already passed, so a wrong answer never gets a golden.
fn check_or_rewrite_golden(program: &str, mmix: &MMix, outcome: &RunOutcome) {
    let body = golden_body(mmix, outcome);
    if var_os("CHECKSMIX_REWRITE_GOLDEN").is_some() {
        write_golden(program, &body).unwrap_or_else(|err| panic!("{err}"));
    } else {
        compare_golden(program, &body).unwrap_or_else(|err| panic!("{err}"));
    }
}

// ============================================================
// Harness tests: hermetic, no program under tests/resources involved.
// ============================================================

#[test]
fn harness_sha256_matches_published_digests() {
    assert_eq!(sha256_hex(b""), SHA256_EMPTY_DIGEST);
    assert_eq!(sha256_hex(b"abc"), SHA256_ABC_DIGEST);
    assert_eq!(
        sha256_hex(SHA256_TWO_BLOCK_MESSAGE.as_bytes()),
        SHA256_TWO_BLOCK_DIGEST
    );
}

#[test]
fn golden_comparison_skips_provenance_but_checks_the_rest() {
    let body = b"alpha\nbeta\ngamma\n";
    let line2 = format!("sha256 {}", sha256_hex(body));

    let mut original = format!("checksmix 0.0.0 2000-01-01\n{line2}\n").into_bytes();
    original.extend_from_slice(body);
    assert!(check_golden_bytes("t", &original, body).is_ok());

    // A different line 1 (provenance) still passes.
    let mut different_provenance = format!("checksmix 9.9.9 2099-12-31\n{line2}\n").into_bytes();
    different_provenance.extend_from_slice(body);
    assert!(check_golden_bytes("t", &different_provenance, body).is_ok());

    // A body changed by one byte fails, naming the body line -- wording
    // the stale-line-2 message below does not share, so hashing
    // actual_body in place of golden_body at the compare turns this red.
    let changed_body: &[u8] = b"alpha\nBETA\ngamma\n";
    let err = check_golden_bytes("t", &original, changed_body).unwrap_err();
    assert!(err.contains("body line 2"), "{err}");

    // A stale line 2 fails.
    let mut stale = format!("checksmix 0.0.0 2000-01-01\nsha256 {}\n", "0".repeat(64)).into_bytes();
    stale.extend_from_slice(body);
    let err = check_golden_bytes("t", &stale, body).unwrap_err();
    assert!(err.contains("stale"), "{err}");
}

#[test]
fn a_missing_golden_fails_naming_the_rewrite_variable() {
    let err = compare_golden("no-such-program-in-this-suite", b"exit 0\n").unwrap_err();
    assert!(err.contains("no golden"), "{err}");
    assert!(err.contains("CHECKSMIX_REWRITE_GOLDEN"), "{err}");
}

#[test]
fn a_trap_outside_the_allowed_set_fails_the_run() {
    let source = "\tLOC\t#100\nMain\tTRAP\t0,Time,0\n\tTRAP\t0,Halt,0\n";
    let err = match run_program("time-trap", source, 1_000) {
        Ok(_) => panic!("expected a program calling Time to fail"),
        Err(err) => err,
    };
    assert!(err.contains("Time"), "{err}");
}

#[test]
fn a_program_that_never_halts_fails_on_budget() {
    let source = "\tLOC\t#100\nMain\tJMP\tMain\n";
    let err = match run_program("infinite-loop", source, 1_000) {
        Ok(_) => panic!("expected a program that never halts to fail"),
        Err(err) => err,
    };
    assert!(err.contains("budget"), "{err}");
}

#[test]
fn a_run_that_never_reaches_a_halt_trap_fails_naming_it() {
    // Register-form TRAP halts the machine directly, without going
    // through the Halt trap.
    let source = "\tLOC\t#100\nMain\tTRAP\t1,2,3\n";
    let err = match run_program("register-trap", source, 1_000) {
        Ok(_) => panic!("expected a run without a Halt trap to fail"),
        Err(err) => err,
    };
    assert!(err.contains("halted without a Halt trap"), "{err}");
}

#[test]
fn a_diagnostic_before_a_clean_halt_still_fails_the_run() {
    let source = "\tLOC\t#100\nMain\tTRAP\t0,99,0\n\tTRAP\t0,Halt,0\n";
    let err = match run_program("unhandled-trap-code", source, 1_000) {
        Ok(_) => panic!("expected a run that emitted a diagnostic to fail"),
        Err(err) => err,
    };
    assert!(err.contains("Unhandled TRAP code 99"), "{err}");
}

// ============================================================
// The three programs.
// ============================================================

/// Reads `program`'s source, runs it, checks its known answer (a clean
/// exit and stdout equal to `expected_stdout`), gives `extra` a chance to
/// pin a further known-answer invariant, then checks or rewrites the
/// golden. Shared by every program test so none can check its golden
/// before its known answer has passed.
fn check_program(program: &str, expected_stdout: &str, extra: impl FnOnce(&RunOutcome)) {
    let path = resource_path(&format!("{program}.mms"));
    let source =
        fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {}: {err}", path.display()));
    let (mmix, outcome) =
        run_program(program, &source, BUDGET).unwrap_or_else(|err| panic!("{err}"));

    assert_eq!(outcome.exit_code, 0, "{program} exit code");
    assert_eq!(
        String::from_utf8(outcome.stdout.clone())
            .unwrap_or_else(|_| panic!("{program} stdout is not ASCII")),
        expected_stdout,
        "{program} stdout"
    );

    extra(&outcome);

    check_or_rewrite_golden(program, &mmix, &outcome);
}

#[test]
fn ackermann_matches_closed_forms() {
    check_program("ackermann", &ackermann_expected_stdout(), |outcome| {
        assert!(
            outcome.peak_stack_octas > 256,
            "ackermann peak stack depth {} is not above 256",
            outcome.peak_stack_octas
        );
    });
}

#[test]
fn sha256_matches_published_digests() {
    check_program("sha256", &sha256_expected_stdout(), |_| {});
}

#[test]
fn pi_matches_known_digits() {
    // PI_DECIMALS is transcribed by hand; confirm it against its own
    // published SHA-256 before trusting it as the expected answer below.
    let expected_stdout = pi_expected_stdout();
    assert_eq!(expected_stdout.len(), 1003, "pi expected stdout length");
    assert_eq!(
        sha256_hex(expected_stdout.as_bytes()),
        PI_KNOWN_SHA256,
        "PI_DECIMALS constant does not match its published SHA-256"
    );

    check_program("pi", &expected_stdout, |_| {});
}
