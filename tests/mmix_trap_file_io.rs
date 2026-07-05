//! Integration tests for the MMIX VM's file-I/O TRAP handlers
//! (Fopen/Fread/Fwrite/Fgets/Fclose/Fseek/Ftell/Fgetws/Fputs/Fputc/Fputws).
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

/// Opens `path` via a real `TRAP 0,Fopen,mode` instruction (mode 0=read,
/// 1=write/create/truncate, 2=append/create) at PC 0, and resets PC back to
/// 0 afterward so the caller can write its next instruction there. Returns
/// the fd `Fopen` allocated (via the public API only — `MMix::file_handles`
/// is a private field, so this is the only way an integration test can
/// obtain an open fd).
fn open_via_trap(mmix: &mut MMix, path: &Path, mode: u8) -> u8 {
    let mut filename = path.to_string_lossy().into_owned().into_bytes();
    filename.push(0);
    let filename_addr = 100u64;
    for (i, &byte) in filename.iter().enumerate() {
        mmix.write_byte(filename_addr + i as u64, byte);
    }
    mmix.set_register(255, filename_addr);
    mmix.write_tetra(0, 0x00000200 | mode as u32); // TRAP 0, Fopen(2), mode
    assert!(mmix.execute_instruction(), "Fopen TRAP should not halt");
    let fd = mmix.get_register(255) as u8;
    assert!(
        fd > 2 && fd < 255,
        "Fopen should return a valid fd, got {fd}"
    );
    mmix.set_pc(0);
    fd
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
fn trap_fopen_write() {
    let mut mmix = MMix::new();
    let path = unique_tmp_path("fopen.txt");
    let guard = TempFileGuard(path.clone());

    let fd = open_via_trap(&mut mmix, &path, 1);
    assert!(fd > 2 && fd < 255);

    drop(guard);
    assert!(!path.exists());
}

#[test]
fn trap_fwrite_basic() {
    let mut mmix = MMix::new();
    let test_file = unique_tmp_path("write.txt");
    let guard = TempFileGuard(test_file.clone());

    let fd = open_via_trap(&mut mmix, &test_file, 1);

    // Prepare data in memory
    let data = b"Hello, File!\0";
    let data_addr = 2000u64;
    for (i, &byte) in data.iter().enumerate() {
        mmix.write_byte(data_addr + i as u64, byte);
    }

    // Write 12 bytes (excluding null terminator)
    // Set up parameter block for Fwrite at address 5000
    let param_addr = 5000u64;
    mmix.write_octa(param_addr, fd as u64); // OCTA 0: file descriptor
    mmix.write_octa(param_addr + 8, data_addr); // OCTA 1: buffer address
    mmix.write_octa(param_addr + 16, 12); // OCTA 2: number of bytes
    mmix.set_register(255, param_addr); // $255 points to parameter block
    mmix.write_tetra(0, 0x00000700); // TRAP 0, 7, 0
    let should_continue = mmix.execute_instruction();
    assert!(should_continue);
    assert_eq!(mmix.get_register(255), 12); // Bytes written returned in $255

    // Verify file contents
    let contents = fs::read_to_string(&test_file).unwrap();
    assert_eq!(contents, "Hello, File!");

    drop(guard);
    assert!(!test_file.exists());
}

#[test]
fn trap_fread_basic() {
    let mut mmix = MMix::new();
    let test_file = unique_tmp_path("read.txt");
    let guard = TempFileGuard(test_file.clone());
    fs::write(&test_file, "Test Content").unwrap();

    let fd = open_via_trap(&mut mmix, &test_file, 0);

    // Read into buffer at address 3000
    let buffer_addr = 3000u64;
    // Set up parameter block for Fread at address 4000
    let param_addr = 4000u64;
    mmix.write_octa(param_addr, fd as u64); // OCTA 0: file descriptor
    mmix.write_octa(param_addr + 8, buffer_addr); // OCTA 1: buffer address
    mmix.write_octa(param_addr + 16, 20); // OCTA 2: max bytes to read
    mmix.set_register(255, param_addr); // $255 points to parameter block
    mmix.write_tetra(0, 0x00000400); // TRAP 0, 4, 0
    let should_continue = mmix.execute_instruction();
    assert!(should_continue);

    let bytes_read = mmix.get_register(255);
    assert_eq!(bytes_read, 12); // "Test Content" is 12 bytes

    // Verify read data
    let mut result = String::new();
    for i in 0..bytes_read {
        result.push(mmix.read_byte(buffer_addr + i) as char);
    }
    assert_eq!(result, "Test Content");

    drop(guard);
    assert!(!test_file.exists());
}

#[test]
fn trap_fgets_basic() {
    let mut mmix = MMix::new();
    let test_file = unique_tmp_path("gets.txt");
    let guard = TempFileGuard(test_file.clone());
    fs::write(&test_file, "First Line\nSecond Line\n").unwrap();

    let fd = open_via_trap(&mut mmix, &test_file, 0);
    assert_eq!(fd, 3, "first fd allocated by a fresh MMix is always 3");

    // Read line into buffer
    let buffer_addr = 4000u64;
    // Set up parameter block for Fgets at address 5000
    let param_addr = 5000u64;
    mmix.write_octa(param_addr, buffer_addr); // OCTA 0: buffer address
    mmix.write_octa(param_addr + 8, 50); // OCTA 1: max size
    mmix.set_register(255, param_addr); // $255 points to parameter block
    mmix.write_tetra(0, 0x00000503); // TRAP 0, 5, 3 (Fgets from fd 3)
    let should_continue = mmix.execute_instruction();
    assert!(should_continue);

    let bytes_read = mmix.get_register(255);
    // Read line includes the newline character, so "First Line\n" = 11 bytes
    assert_eq!(bytes_read, 11);

    drop(guard);
    assert!(!test_file.exists());
}

#[test]
fn trap_fclose_success() {
    let mut mmix = MMix::new();
    let test_file = unique_tmp_path("close.txt");
    let guard = TempFileGuard(test_file.clone());

    let fd = open_via_trap(&mut mmix, &test_file, 1);

    mmix.set_register(255, fd as u64); // $255 contains file descriptor
    mmix.write_tetra(0, 0x00000300); // TRAP 0, 3, 0
    let should_continue = mmix.execute_instruction();
    assert!(should_continue);
    assert_eq!(mmix.get_register(255), 0); // Success returned in $255

    drop(guard);
    assert!(!test_file.exists());
}

#[test]
fn trap_fseek_basic() {
    let mut mmix = MMix::new();
    let test_file = unique_tmp_path("seek.txt");
    let guard = TempFileGuard(test_file.clone());
    fs::write(&test_file, "0123456789ABCDEF").unwrap();

    let fd = open_via_trap(&mut mmix, &test_file, 0);

    // Seek to position 5
    let param_addr = 5000u64;
    mmix.write_octa(param_addr, fd as u64); // OCTA 0: file descriptor
    mmix.write_octa(param_addr + 8, 5i64 as u64); // OCTA 1: offset
    mmix.write_octa(param_addr + 16, 0); // OCTA 2: whence = start
    mmix.set_register(255, param_addr); // $255 points to parameter block
    mmix.write_tetra(0, 0x00000B00); // TRAP 0, 11, 0
    let should_continue = mmix.execute_instruction();
    assert!(should_continue);
    assert_eq!(mmix.get_register(255), 5); // New position returned in $255

    drop(guard);
    assert!(!test_file.exists());
}

#[test]
fn trap_ftell_basic() {
    let mut mmix = MMix::new();
    let test_file = unique_tmp_path("tell.txt");
    let guard = TempFileGuard(test_file.clone());
    fs::write(&test_file, "Test Data").unwrap();

    let fd = open_via_trap(&mut mmix, &test_file, 0);

    // Get current position (should be 0)
    mmix.set_register(255, fd as u64); // $255 contains file descriptor
    mmix.write_tetra(0, 0x00000C00); // TRAP 0, 12, 0
    let should_continue = mmix.execute_instruction();
    assert!(should_continue);
    assert_eq!(mmix.get_register(255), 0); // Position 0 returned in $255

    drop(guard);
    assert!(!test_file.exists());
}

#[test]
fn trap_fgetws_basic() {
    let mut mmix = MMix::new();
    let test_file = unique_tmp_path("getws.txt");
    let guard = TempFileGuard(test_file.clone());
    fs::write(&test_file, "Wide line\nAnother line\n").unwrap();

    let fd = open_via_trap(&mut mmix, &test_file, 0);
    assert_eq!(fd, 3, "first fd allocated by a fresh MMix is always 3");

    let buffer_addr = 5000u64;
    // Set up parameter block for Fgetws at address 6000
    let param_addr = 6000u64;
    mmix.write_octa(param_addr, buffer_addr); // OCTA 0: buffer address
    mmix.write_octa(param_addr + 8, 50); // OCTA 1: max size
    mmix.set_register(255, param_addr); // $255 points to parameter block
    mmix.write_tetra(0, 0x00000603); // TRAP 0, 6, 3 (Fgetws from fd 3)
    let should_continue = mmix.execute_instruction();
    assert!(should_continue);

    let bytes_read = mmix.get_register(255); // Return value in $255
    assert!(bytes_read > 0);

    drop(guard);
    assert!(!test_file.exists());
}

#[test]
fn trap_fputs_to_file_descriptor() {
    // Fputs targeted at an Fopen'd fd must write the bytes through to that
    // file, not silently no-op as it did when fd>2 was unsupported.
    let mut mmix = MMix::new();
    let path = unique_tmp_path("fputs_to_fd.txt");
    let guard = TempFileGuard(path.clone());

    let fd = open_via_trap(&mut mmix, &path, 1);
    assert_eq!(fd, 3, "first fd allocated by a fresh MMix is always 3");

    let test_string = b"Hello, file fd!\0";
    let str_addr = 1500u64;
    for (i, &byte) in test_string.iter().enumerate() {
        mmix.write_byte(str_addr + i as u64, byte);
    }
    mmix.set_register(255, str_addr);
    // TRAP 0, Fputs (8), 3
    mmix.write_tetra(0, 0x00000803);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(255), 15); // bytes written, not -1

    // Close via TRAP so the OS flushes contents before reading (no direct
    // access to file_handles from an integration test).
    mmix.set_pc(4);
    mmix.set_register(255, fd as u64);
    mmix.write_tetra(4, 0x00000300); // TRAP 0, Fclose(3), 0
    assert!(mmix.execute_instruction());

    let contents = fs::read(&path).unwrap();
    assert_eq!(contents, b"Hello, file fd!");

    drop(guard);
    assert!(!path.exists());
}

#[test]
fn trap_fputs_high_bytes_to_file_are_raw() {
    // Bytes 0x80..=0xFF must be written verbatim, not widened via UTF-8.
    // Pre-fix code did `output.push(byte as char)`, so 0xFF on stdout came
    // out as 0xC3 0xBF.
    let mut mmix = MMix::new();
    let path = unique_tmp_path("fputs_raw_bytes.bin");
    let guard = TempFileGuard(path.clone());

    let fd = open_via_trap(&mut mmix, &path, 1);
    assert_eq!(fd, 3, "first fd allocated by a fresh MMix is always 3");

    let bytes = [0xFFu8, 0x80, 0x41, 0x00];
    let str_addr = 700u64;
    for (i, &b) in bytes.iter().enumerate() {
        mmix.write_byte(str_addr + i as u64, b);
    }
    mmix.set_register(255, str_addr);
    mmix.write_tetra(0, 0x00000803); // TRAP 0, Fputs (8), 3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(255), 3);

    mmix.set_pc(4);
    mmix.set_register(255, fd as u64);
    mmix.write_tetra(4, 0x00000300); // TRAP 0, Fclose(3), 0
    assert!(mmix.execute_instruction());

    let contents = fs::read(&path).unwrap();
    assert_eq!(contents, vec![0xFFu8, 0x80, 0x41]);

    drop(guard);
    assert!(!path.exists());
}

#[test]
fn trap_fputc_high_byte_to_file_is_raw() {
    // Same regression as Fputs: a byte ≥ 0x80 must be written as one raw
    // byte, not UTF-8-expanded.
    let mut mmix = MMix::new();
    let path = unique_tmp_path("fputc_raw.bin");
    let guard = TempFileGuard(path.clone());

    let fd = open_via_trap(&mut mmix, &path, 1);
    assert_eq!(fd, 3, "first fd allocated by a fresh MMix is always 3");

    mmix.set_register(255, 0xFF);
    mmix.write_tetra(0, 0x00000903); // TRAP 0, Fputc (9), 3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(255), 0);

    mmix.set_pc(4);
    mmix.set_register(255, fd as u64);
    mmix.write_tetra(4, 0x00000300); // TRAP 0, Fclose(3), 0
    assert!(mmix.execute_instruction());

    let contents = fs::read(&path).unwrap();
    assert_eq!(contents, vec![0xFFu8]);

    drop(guard);
    assert!(!path.exists());
}

#[test]
fn trap_fputws_to_file_descriptor() {
    let mut mmix = MMix::new();
    let path = unique_tmp_path("fputws_to_fd.txt");
    let guard = TempFileGuard(path.clone());

    let fd = open_via_trap(&mut mmix, &path, 1);
    assert_eq!(fd, 3, "first fd allocated by a fresh MMix is always 3");

    let test_string = b"wide\0";
    let str_addr = 800u64;
    for (i, &byte) in test_string.iter().enumerate() {
        mmix.write_byte(str_addr + i as u64, byte);
    }
    mmix.set_register(255, str_addr);
    mmix.write_tetra(0, 0x00000A03); // TRAP 0, Fputws (10), 3
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(255), 4);

    mmix.set_pc(4);
    mmix.set_register(255, fd as u64);
    mmix.write_tetra(4, 0x00000300); // TRAP 0, Fclose(3), 0
    assert!(mmix.execute_instruction());

    let contents = fs::read(&path).unwrap();
    assert_eq!(contents, b"wide");

    drop(guard);
    assert!(!path.exists());
}
