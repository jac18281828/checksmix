//! Integration tests for the MMIX VM's file-I/O TRAP handlers, per the
//! MMIXAL reference ABI: `TRAP 0,Code,Handle`, with `$255` carrying any
//! further argument (an address, for a call that takes two).
//!
//! These necessarily touch the real filesystem (that's what's under test),
//! so per AGENTS.md they belong here rather than in `src/mmix.rs`'s unit
//! test module, which must stay hermetic. Every test uses a process-unique
//! path (never collides with a concurrent `cargo test` invocation from
//! another worktree) and an RAII guard (cleans up even on panic).

use checksmix::MMix;
use std::fs;
use std::path::{Path, PathBuf};

/// Unique per-process temp path so concurrent `cargo test` invocations
/// (e.g. two worktrees) never race the same file.
fn unique_tmp_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("checksmix_test_{}_{}", std::process::id(), name))
}

/// Removes its path on drop — even if the test panics before reaching an
/// explicit cleanup line.
struct TempFileGuard(PathBuf);
impl Drop for TempFileGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

// TRAP codes (MMIXAL reference, plus checksmix's own Fputc extension).
const FOPEN: u8 = 1;
const FCLOSE: u8 = 2;
const FREAD: u8 = 3;
const FGETS: u8 = 4;
const FGETWS: u8 = 5;
const FWRITE: u8 = 6;
const FPUTS: u8 = 7;
const FPUTWS: u8 = 8;
const FSEEK: u8 = 9;
const FTELL: u8 = 10;
const FPUTC: u8 = 0x80;

// Fopen modes (MMIXAL reference). `u64`, matching the mode octa: `fopen`
// checks the full 64 bits, not just its low byte.
const TEXT_READ: u64 = 0;
const TEXT_WRITE: u64 = 1;
const BINARY_READ: u64 = 2;
const BINARY_WRITE: u64 = 3;
const BINARY_READ_WRITE: u64 = 4;

/// `TRAP 0,Y,Z`'s tetra, the immediate form every call in this file uses.
fn trap_tetra(y: u8, z: u8) -> u32 {
    ((y as u32) << 8) | (z as u32)
}

/// Runs `TRAP 0,Y,handle` at PC 0 and returns `$255` as a signed result.
/// Resets PC to 0 first, so callers never have to track it between calls.
fn run_trap(mmix: &mut MMix, y: u8, handle: u8) -> i64 {
    mmix.set_pc(0);
    mmix.write_tetra(0, trap_tetra(y, handle));
    assert!(mmix.execute_instruction(), "TRAP should not halt");
    mmix.get_register(255) as i64
}

/// `Fopen`: builds the two-octa block (name address, mode) at a fixed
/// scratch address, points `$255` at it, and runs the call.
fn fopen(mmix: &mut MMix, handle: u8, path: &Path, mode: u64) -> i64 {
    let mut filename = path.to_string_lossy().into_owned().into_bytes();
    filename.push(0);
    let filename_addr = 50_000u64;
    for (i, &byte) in filename.iter().enumerate() {
        mmix.write_byte(filename_addr + i as u64, byte);
    }
    let param_addr = 40_000u64;
    mmix.write_octa(param_addr, filename_addr);
    mmix.write_octa(param_addr + 8, mode);
    mmix.set_register(255, param_addr);
    run_trap(mmix, FOPEN, handle)
}

fn fclose(mmix: &mut MMix, handle: u8) -> i64 {
    run_trap(mmix, FCLOSE, handle)
}

/// `Fread`/`Fgets`/`Fgetws`/`Fwrite` share the two-octa (buffer, size)
/// block; the block's own address is fixed and distinct from `fopen`'s.
fn two_arg_block(mmix: &mut MMix, first: u64, second: u64) {
    let param_addr = 41_000u64;
    mmix.write_octa(param_addr, first);
    mmix.write_octa(param_addr + 8, second);
    mmix.set_register(255, param_addr);
}

fn fread(mmix: &mut MMix, handle: u8, buffer_addr: u64, size: u64) -> i64 {
    two_arg_block(mmix, buffer_addr, size);
    run_trap(mmix, FREAD, handle)
}

fn fgets(mmix: &mut MMix, handle: u8, buffer_addr: u64, size: u64) -> i64 {
    two_arg_block(mmix, buffer_addr, size);
    run_trap(mmix, FGETS, handle)
}

fn fgetws(mmix: &mut MMix, handle: u8, buffer_addr: u64, size: u64) -> i64 {
    two_arg_block(mmix, buffer_addr, size);
    run_trap(mmix, FGETWS, handle)
}

fn fwrite(mmix: &mut MMix, handle: u8, data: &[u8]) -> i64 {
    let buffer_addr = 60_000u64;
    for (i, &b) in data.iter().enumerate() {
        mmix.write_byte(buffer_addr + i as u64, b);
    }
    two_arg_block(mmix, buffer_addr, data.len() as u64);
    run_trap(mmix, FWRITE, handle)
}

fn fputs(mmix: &mut MMix, handle: u8, bytes: &[u8]) -> i64 {
    let str_addr = 61_000u64;
    for (i, &b) in bytes.iter().enumerate() {
        mmix.write_byte(str_addr + i as u64, b);
    }
    mmix.write_byte(str_addr + bytes.len() as u64, 0);
    mmix.set_register(255, str_addr);
    run_trap(mmix, FPUTS, handle)
}

fn fputc(mmix: &mut MMix, handle: u8, byte: u8) -> i64 {
    mmix.set_register(255, byte as u64);
    run_trap(mmix, FPUTC, handle)
}

/// `bytes` is copied verbatim (no NUL terminator): `Fputws` stops at the
/// first zero wyde, so the caller supplies one when it wants to.
fn fputws(mmix: &mut MMix, handle: u8, bytes: &[u8]) -> i64 {
    let str_addr = 62_000u64;
    for (i, &b) in bytes.iter().enumerate() {
        mmix.write_byte(str_addr + i as u64, b);
    }
    mmix.set_register(255, str_addr);
    run_trap(mmix, FPUTWS, handle)
}

fn fseek(mmix: &mut MMix, handle: u8, offset: i64) -> i64 {
    mmix.set_register(255, offset as u64);
    run_trap(mmix, FSEEK, handle)
}

fn ftell(mmix: &mut MMix, handle: u8) -> i64 {
    run_trap(mmix, FTELL, handle)
}

#[test]
fn unique_tmp_path_embeds_process_id_and_is_not_a_legacy_literal() {
    let pid = std::process::id().to_string();
    let path = unique_tmp_path("x");
    assert!(path.to_string_lossy().contains(&pid));
    assert_ne!(
        unique_tmp_path("write"),
        PathBuf::from("/tmp/test_mmix_write.txt")
    );
}

#[test]
fn temp_file_guard_removes_file_on_drop() {
    let path = unique_tmp_path("guard_drop");
    fs::write(&path, b"guard test").unwrap();
    assert!(path.exists());
    let guard = TempFileGuard(path.clone());
    drop(guard);
    assert!(!path.exists());
}

#[test]
fn temp_file_guard_removes_file_on_panic() {
    let path = unique_tmp_path("guard_panic");
    fs::write(&path, b"guard panic test").unwrap();
    let path_for_closure = path.clone();
    let result = std::panic::catch_unwind(move || {
        let _guard = TempFileGuard(path_for_closure);
        panic!("intentional panic to exercise Drop cleanup");
    });
    assert!(result.is_err());
    assert!(!path.exists());
}

#[test]
fn fopen_write_then_close_succeed() {
    let mut mmix = MMix::new();
    let path = unique_tmp_path("fopen_write.txt");
    let guard = TempFileGuard(path.clone());

    assert_eq!(fopen(&mut mmix, 3, &path, TEXT_WRITE), 0);
    assert_eq!(fclose(&mut mmix, 3), 0);

    drop(guard);
}

#[test]
fn fopen_invalid_mode_fails() {
    let mut mmix = MMix::new();
    let path = unique_tmp_path("fopen_bad_mode.txt");
    let guard = TempFileGuard(path.clone());

    assert_eq!(fopen(&mut mmix, 3, &path, 5), -1);

    drop(guard);
    assert!(!path.exists());
}

#[test]
fn fopen_mode_above_four_fails_even_with_the_low_byte_valid() {
    // #104 (260): a truncating check reads this as mode 4
    // (BinaryReadWrite), which creates the file. Check before the guard's
    // drop deletes it regardless of whether `Fopen` did.
    let mut mmix = MMix::new();
    let path = unique_tmp_path("fopen_wide_mode_create.txt");
    let guard = TempFileGuard(path.clone());

    assert_eq!(fopen(&mut mmix, 3, &path, 0x104), -1);
    assert!(!path.exists());

    drop(guard);

    // #100 (256): a truncating check reads this as mode 0 (TextRead),
    // which succeeds against an existing file without touching it.
    let mut mmix = MMix::new();
    let path = unique_tmp_path("fopen_wide_mode_read.txt");
    let guard = TempFileGuard(path.clone());
    fs::write(&path, "unchanged").unwrap();

    assert_eq!(fopen(&mut mmix, 3, &path, 0x100), -1);
    assert_eq!(fs::read_to_string(&path).unwrap(), "unchanged");

    drop(guard);
}

#[test]
fn fopen_and_fclose_on_standard_handles_fail() {
    let mut mmix = MMix::new();
    let path = unique_tmp_path("fopen_standard.txt");
    let guard = TempFileGuard(path.clone());

    for handle in [0u8, 1, 2] {
        assert_eq!(fopen(&mut mmix, handle, &path, TEXT_WRITE), -1);
        assert_eq!(fclose(&mut mmix, handle), -1);
    }

    drop(guard);
    assert!(!path.exists());
}

#[test]
fn fopen_reopening_an_open_handle_closes_it_first() {
    let mut mmix = MMix::new();
    let path_a = unique_tmp_path("fopen_reopen_a.txt");
    let path_b = unique_tmp_path("fopen_reopen_b.txt");
    let guard_a = TempFileGuard(path_a.clone());
    let guard_b = TempFileGuard(path_b.clone());
    fs::write(&path_a, "AAAA").unwrap();
    fs::write(&path_b, "BBBB").unwrap();

    assert_eq!(fopen(&mut mmix, 3, &path_a, TEXT_READ), 0);
    // Reopening handle 3 on a different file, without an explicit Fclose,
    // must still succeed and read from the new file.
    assert_eq!(fopen(&mut mmix, 3, &path_b, TEXT_READ), 0);

    let buffer_addr = 70_000u64;
    assert_eq!(fread(&mut mmix, 3, buffer_addr, 4), 0);
    let read: Vec<u8> = (0..4).map(|i| mmix.read_byte(buffer_addr + i)).collect();
    assert_eq!(read, b"BBBB");

    drop(guard_a);
    drop(guard_b);
}

#[test]
fn fwrite_and_fread_round_trip_exact_size() {
    let mut mmix = MMix::new();
    let path = unique_tmp_path("fwrite_fread.txt");
    let guard = TempFileGuard(path.clone());

    assert_eq!(fopen(&mut mmix, 3, &path, TEXT_WRITE), 0);
    assert_eq!(fwrite(&mut mmix, 3, b"Hello, File!"), 0); // 0: all bytes written
    assert_eq!(fclose(&mut mmix, 3), 0);

    assert_eq!(fopen(&mut mmix, 3, &path, TEXT_READ), 0);
    let buffer_addr = 71_000u64;
    assert_eq!(fread(&mut mmix, 3, buffer_addr, 12), 0);
    let read: Vec<u8> = (0..12).map(|i| mmix.read_byte(buffer_addr + i)).collect();
    assert_eq!(read, b"Hello, File!");
    assert_eq!(fclose(&mut mmix, 3), 0);

    drop(guard);
    assert!(!path.exists());
}

#[test]
fn fread_short_at_eof_returns_n_minus_size() {
    let mut mmix = MMix::new();
    let path = unique_tmp_path("fread_short.txt");
    let guard = TempFileGuard(path.clone());
    fs::write(&path, "Test Content").unwrap(); // 12 bytes

    assert_eq!(fopen(&mut mmix, 3, &path, TEXT_READ), 0);
    let buffer_addr = 72_000u64;
    // Ask for 20; only 12 are there, so the result is 12 - 20 = -8.
    assert_eq!(fread(&mut mmix, 3, buffer_addr, 20), 12i64 - 20);
    let read: Vec<u8> = (0..12).map(|i| mmix.read_byte(buffer_addr + i)).collect();
    assert_eq!(read, b"Test Content");

    drop(guard);
    assert!(!path.exists());
}

#[test]
fn fread_on_a_write_only_handle_fails_without_touching_the_file() {
    let mut mmix = MMix::new();
    let path = unique_tmp_path("fread_capability.txt");
    let guard = TempFileGuard(path.clone());

    assert_eq!(fopen(&mut mmix, 3, &path, TEXT_WRITE), 0);
    let buffer_addr = 73_000u64;
    mmix.write_byte(buffer_addr, 0xAA); // sentinel: must survive untouched
    assert_eq!(fread(&mut mmix, 3, buffer_addr, 5), -1 - 5);
    assert_eq!(mmix.read_byte(buffer_addr), 0xAA);

    drop(guard);
}

#[test]
fn fwrite_on_a_read_only_handle_fails() {
    let mut mmix = MMix::new();
    let path = unique_tmp_path("fwrite_capability.txt");
    let guard = TempFileGuard(path.clone());
    fs::write(&path, "unchanged").unwrap();

    assert_eq!(fopen(&mut mmix, 3, &path, TEXT_READ), 0);
    assert_eq!(fwrite(&mut mmix, 3, b"nope"), -4); // -size
    assert_eq!(fs::read_to_string(&path).unwrap(), "unchanged");

    drop(guard);
}

#[test]
fn fread_and_fwrite_on_a_closed_handle_fail() {
    let mut mmix = MMix::new();
    assert_eq!(fread(&mut mmix, 3, 74_000, 5), -1 - 5);
    assert_eq!(fwrite(&mut mmix, 3, b"nope"), -4);
    assert_eq!(fclose(&mut mmix, 3), -1);
}

#[test]
fn fgets_reads_a_line_and_the_partial_last_line_at_eof() {
    let mut mmix = MMix::new();
    let path = unique_tmp_path("fgets.txt");
    let guard = TempFileGuard(path.clone());
    fs::write(&path, "First Line\nSecond").unwrap(); // no trailing newline

    assert_eq!(fopen(&mut mmix, 3, &path, TEXT_READ), 0);

    let buffer_addr = 75_000u64;
    let n = fgets(&mut mmix, 3, buffer_addr, 50);
    assert_eq!(n, 11); // "First Line\n"
    assert_eq!(mmix.read_byte(buffer_addr + 11), 0);

    // The last line has no newline: still returned, not -1.
    let n2 = fgets(&mut mmix, 3, buffer_addr, 50);
    assert_eq!(n2, 6); // "Second"
    assert_eq!(mmix.read_byte(buffer_addr + 6), 0);

    // Nothing left: end of file before any character.
    let n3 = fgets(&mut mmix, 3, buffer_addr, 50);
    assert_eq!(n3, -1);

    drop(guard);
}

#[test]
fn fgets_with_size_zero_fails() {
    let mut mmix = MMix::new();
    let path = unique_tmp_path("fgets_zero.txt");
    let guard = TempFileGuard(path.clone());
    fs::write(&path, "text").unwrap();

    assert_eq!(fopen(&mut mmix, 3, &path, TEXT_READ), 0);
    assert_eq!(fgets(&mut mmix, 3, 76_000, 0), -1);

    drop(guard);
}

#[test]
fn fputws_and_fgetws_round_trip_wydes_byte_exact() {
    let mut mmix = MMix::new();
    let path = unique_tmp_path("fputws_fgetws.txt");
    let guard = TempFileGuard(path.clone());

    assert_eq!(fopen(&mut mmix, 3, &path, TEXT_WRITE), 0);
    // Two wydes ("Hi") then a terminating zero wyde Fputws must not write.
    assert_eq!(fputws(&mut mmix, 3, &[b'H', b'i', 0x00, 0x00]), 1);
    assert_eq!(fclose(&mut mmix, 3), 0);
    assert_eq!(fs::read(&path).unwrap(), b"Hi");

    assert_eq!(fopen(&mut mmix, 3, &path, TEXT_READ), 0);
    let buffer_addr = 77_000u64;
    let n = fgetws(&mut mmix, 3, buffer_addr, 50);
    assert_eq!(n, 1);
    assert_eq!(mmix.read_byte(buffer_addr), b'H');
    assert_eq!(mmix.read_byte(buffer_addr + 1), b'i');
    assert_eq!(mmix.read_byte(buffer_addr + 2), 0);
    assert_eq!(mmix.read_byte(buffer_addr + 3), 0);

    drop(guard);
}

#[test]
fn fgetws_rounds_an_odd_buffer_address_down() {
    let mut mmix = MMix::new();
    let path = unique_tmp_path("fgetws_odd.txt");
    let guard = TempFileGuard(path.clone());
    fs::write(&path, [b'H', b'i']).unwrap();

    assert_eq!(fopen(&mut mmix, 3, &path, TEXT_READ), 0);
    // 77_001 is odd; Fgetws must round down to 77_000 before writing.
    let n = fgetws(&mut mmix, 3, 77_001, 50);
    assert_eq!(n, 1);
    assert_eq!(mmix.read_byte(77_000), b'H');
    assert_eq!(mmix.read_byte(77_001), b'i');

    drop(guard);
}

#[test]
fn fseek_negative_one_lands_at_the_end_and_ftell_agrees() {
    let mut mmix = MMix::new();
    let path = unique_tmp_path("fseek_end.txt");
    let guard = TempFileGuard(path.clone());
    fs::write(&path, "0123456789ABCDEF").unwrap(); // 16 bytes

    assert_eq!(fopen(&mut mmix, 3, &path, BINARY_READ), 0);
    assert_eq!(fseek(&mut mmix, 3, -1), 0);
    assert_eq!(ftell(&mut mmix, 3), 16);

    assert_eq!(fseek(&mut mmix, 3, 5), 0);
    assert_eq!(ftell(&mut mmix, 3), 5);

    drop(guard);
}

#[test]
fn fseek_to_a_nonzero_offset_returns_zero_not_the_position() {
    let mut mmix = MMix::new();
    let path = unique_tmp_path("fseek_nonzero.txt");
    let guard = TempFileGuard(path.clone());
    fs::write(&path, "0123456789ABCDEF").unwrap(); // 16 bytes

    assert_eq!(fopen(&mut mmix, 3, &path, BINARY_READ), 0);
    assert_eq!(fseek(&mut mmix, 3, 9), 0);
    assert_eq!(ftell(&mut mmix, 3), 9);

    drop(guard);
}

#[test]
fn fseek_and_ftell_need_seek_capability() {
    let mut mmix = MMix::new();
    let path = unique_tmp_path("fseek_capability.txt");
    let guard = TempFileGuard(path.clone());
    fs::write(&path, "text").unwrap();

    // TextRead/TextWrite grant read or write alone, never seek.
    assert_eq!(fopen(&mut mmix, 3, &path, TEXT_READ), 0);
    assert_eq!(fseek(&mut mmix, 3, 0), -1);
    assert_eq!(ftell(&mut mmix, 3), -1);

    drop(guard);
}

#[test]
fn binary_read_write_switches_capability_and_fseek_restores_it() {
    let mut mmix = MMix::new();
    let path = unique_tmp_path("read_write_switch.txt");
    let guard = TempFileGuard(path.clone());

    assert_eq!(fopen(&mut mmix, 3, &path, BINARY_READ_WRITE), 0);
    assert_eq!(fwrite(&mut mmix, 3, b"0123456789"), 0);
    assert_eq!(fseek(&mut mmix, 3, 0), 0);

    // A read clears write; writing again fails until Fseek restores it.
    let buffer_addr = 78_000u64;
    assert_eq!(fread(&mut mmix, 3, buffer_addr, 4), 0);
    assert_eq!(fwrite(&mut mmix, 3, b"XX"), -2); // -size: write cleared

    assert_eq!(fseek(&mut mmix, 3, 0), 0); // restores both
    assert_eq!(fwrite(&mut mmix, 3, b"XX"), 0);

    // A write clears read; reading again fails until Fseek restores it.
    assert_eq!(fread(&mut mmix, 3, buffer_addr, 4), -1 - 4); // read cleared

    drop(guard);
}

#[test]
fn fopen_on_binary_write_and_binary_read_never_switches() {
    // A single-capability binary handle carries `seek` but never toggles:
    // there is nothing to switch to.
    let mut mmix = MMix::new();
    let path = unique_tmp_path("binary_single_capability.txt");
    let guard = TempFileGuard(path.clone());

    assert_eq!(fopen(&mut mmix, 3, &path, BINARY_WRITE), 0);
    assert_eq!(fwrite(&mut mmix, 3, b"data"), 0);
    assert_eq!(fseek(&mut mmix, 3, 0), 0);
    assert_eq!(fwrite(&mut mmix, 3, b"more"), 0); // still writable
    assert_eq!(fclose(&mut mmix, 3), 0);

    assert_eq!(fopen(&mut mmix, 3, &path, BINARY_READ), 0);
    let buffer_addr = 79_000u64;
    assert_eq!(fread(&mut mmix, 3, buffer_addr, 4), 0);
    assert_eq!(fseek(&mut mmix, 3, 0), 0);
    assert_eq!(fread(&mut mmix, 3, buffer_addr, 4), 0); // still readable

    drop(guard);
}

#[test]
fn fputs_to_a_file_descriptor() {
    // Fputs targeted at an Fopen'd handle must write the bytes through to
    // that file.
    let mut mmix = MMix::new();
    let path = unique_tmp_path("fputs_to_fd.txt");
    let guard = TempFileGuard(path.clone());

    assert_eq!(fopen(&mut mmix, 3, &path, TEXT_WRITE), 0);
    assert_eq!(fputs(&mut mmix, 3, b"Hello, file fd!"), 15);
    assert_eq!(fclose(&mut mmix, 3), 0);

    assert_eq!(fs::read(&path).unwrap(), b"Hello, file fd!");

    drop(guard);
    assert!(!path.exists());
}

#[test]
fn fputs_high_bytes_to_file_are_raw() {
    // Bytes 0x80..=0xFF must be written verbatim, not widened via UTF-8.
    let mut mmix = MMix::new();
    let path = unique_tmp_path("fputs_raw_bytes.bin");
    let guard = TempFileGuard(path.clone());

    assert_eq!(fopen(&mut mmix, 3, &path, BINARY_WRITE), 0);
    assert_eq!(fputs(&mut mmix, 3, &[0xFF, 0x80, 0x41]), 3);
    assert_eq!(fclose(&mut mmix, 3), 0);

    assert_eq!(fs::read(&path).unwrap(), vec![0xFFu8, 0x80, 0x41]);

    drop(guard);
}

#[test]
fn fputc_high_byte_to_file_is_raw() {
    let mut mmix = MMix::new();
    let path = unique_tmp_path("fputc_raw.bin");
    let guard = TempFileGuard(path.clone());

    assert_eq!(fopen(&mut mmix, 3, &path, BINARY_WRITE), 0);
    assert_eq!(fputc(&mut mmix, 3, 0xFF), 0);
    assert_eq!(fclose(&mut mmix, 3), 0);

    assert_eq!(fs::read(&path).unwrap(), vec![0xFFu8]);

    drop(guard);
}

#[test]
fn fputc_on_an_unknown_handle_returns_error() {
    let mut mmix = MMix::new();
    assert_eq!(fputc(&mut mmix, 3, b'A'), -1);
}

#[test]
fn every_handle_taking_call_keeps_its_own_failure_value() {
    // Table-driven: each handle-taking call, run on (a) a closed handle and
    // (b) an open handle that lacks the needed capability (where one
    // exists — Fclose has none), must report exactly the failure value the
    // ABI table names for it. The guards now share one shape; this pins
    // that the failure value stays per call rather than collapsing to -1.
    const SIZE: u64 = 5;
    const CLOSED: u8 = 9; // never opened in this test

    struct Case {
        name: &'static str,
        closed: i64,
        wrong_mode: Option<u64>,
        wrong: i64,
        run: fn(&mut MMix, u8) -> i64,
    }

    fn buf() -> u64 {
        90_000
    }

    let cases: Vec<Case> = vec![
        Case {
            name: "Fclose",
            closed: -1,
            wrong_mode: None,
            wrong: 0,
            run: fclose,
        },
        Case {
            name: "Fread",
            closed: -1 - SIZE as i64,
            wrong_mode: Some(TEXT_WRITE),
            wrong: -1 - SIZE as i64,
            run: |m, h| fread(m, h, buf(), SIZE),
        },
        Case {
            name: "Fgets",
            closed: -1,
            wrong_mode: Some(TEXT_WRITE),
            wrong: -1,
            run: |m, h| fgets(m, h, buf(), 50),
        },
        Case {
            name: "Fgetws",
            closed: -1,
            wrong_mode: Some(TEXT_WRITE),
            wrong: -1,
            run: |m, h| fgetws(m, h, buf(), 50),
        },
        Case {
            name: "Fwrite",
            closed: -(SIZE as i64),
            wrong_mode: Some(TEXT_READ),
            wrong: -(SIZE as i64),
            run: |m, h| fwrite(m, h, &[0u8; SIZE as usize]),
        },
        Case {
            name: "Fputs",
            closed: -1,
            wrong_mode: Some(TEXT_READ),
            wrong: -1,
            run: |m, h| fputs(m, h, b"x"),
        },
        Case {
            name: "Fputws",
            closed: -1,
            wrong_mode: Some(TEXT_READ),
            wrong: -1,
            run: |m, h| fputws(m, h, &[b'x', 0, 0, 0]),
        },
        Case {
            name: "Fseek",
            closed: -1,
            wrong_mode: Some(TEXT_READ),
            wrong: -1,
            run: |m, h| fseek(m, h, 0),
        },
        Case {
            name: "Ftell",
            closed: -1,
            wrong_mode: Some(TEXT_READ),
            wrong: -1,
            run: ftell,
        },
        Case {
            name: "Fputc",
            closed: -1,
            wrong_mode: Some(TEXT_READ),
            wrong: -1,
            run: |m, h| fputc(m, h, b'x'),
        },
    ];

    for case in cases {
        let mut mmix = MMix::new();
        assert_eq!(
            (case.run)(&mut mmix, CLOSED),
            case.closed,
            "{}: closed handle",
            case.name
        );

        if let Some(mode) = case.wrong_mode {
            let path = unique_tmp_path(&format!("wrong_capability_{}.txt", case.name));
            let guard = TempFileGuard(path.clone());
            fs::write(&path, "content").unwrap(); // TEXT_READ needs the file to exist
            assert_eq!(fopen(&mut mmix, 3, &path, mode), 0, "{}: fopen", case.name);
            assert_eq!(
                (case.run)(&mut mmix, 3),
                case.wrong,
                "{}: wrong capability",
                case.name
            );
            drop(guard);
        }
    }
}

#[test]
fn binary_read_write_switches_capability_for_fputc_too() {
    let mut mmix = MMix::new();
    let path = unique_tmp_path("fputc_read_write_switch.txt");
    let guard = TempFileGuard(path.clone());

    assert_eq!(fopen(&mut mmix, 3, &path, BINARY_READ_WRITE), 0);
    assert_eq!(fwrite(&mut mmix, 3, b"0123456789"), 0);
    assert_eq!(fseek(&mut mmix, 3, 0), 0);

    // A read clears write; Fputc fails until Fseek restores it.
    let buffer_addr = 78_600u64;
    assert_eq!(fread(&mut mmix, 3, buffer_addr, 4), 0);
    assert_eq!(fputc(&mut mmix, 3, b'X'), -1);

    assert_eq!(fseek(&mut mmix, 3, 0), 0); // restores both
    assert_eq!(fputc(&mut mmix, 3, b'X'), 0);

    // Fputc clears read; reading again fails until Fseek restores it.
    assert_eq!(fread(&mut mmix, 3, buffer_addr, 4), -1 - 4);

    drop(guard);
}
