//! TRAP opcodes (file I/O, time, debug) and the Host they route through.

use super::*;

// ========== TRAP Handler Tests ==========

#[test]
fn test_trap_halt() {
    let mut mmix = MMix::new();
    // TRAP 0, Halt, 0
    mmix.write_tetra(0, 0x00000000); // TRAP 0,0,0
    let should_continue = mmix.execute_instruction();
    assert!(!should_continue); // Should halt
    assert_eq!(mmix.get_pc(), 4); // PC still advances
}

/// the register form (`X != 0`) halts and exits 1, like every
/// other halt but the `Halt` trap.
#[test]
fn test_trap_register_form_halts_and_exits_1() {
    let (host, handle) = CaptureHost::new();
    let mut mmix = MMix::with_host(host);
    mmix.set_register(2, 10);
    mmix.set_register(3, 20);
    mmix.write_tetra(0, 0x00010203); // TRAP 1,$2,$3 -- register form
    assert!(!mmix.execute_instruction());
    assert_eq!(mmix.get_exit_code(), 1);
    assert_eq!(handle.diagnostics().len(), 1);
    assert!(handle.diagnostics()[0].contains("Register TRAP"));
}

#[test]
fn test_trap_fputs_stdout() {
    let (host, handle) = CaptureHost::new();
    let mut mmix = MMix::with_host(host);
    let test_string = b"Hello, MMIX!\0";
    let str_addr = 1000u64;

    for (i, &byte) in test_string.iter().enumerate() {
        mmix.write_byte(str_addr + i as u64, byte);
    }

    mmix.set_register(255, str_addr); // Fputs reads string address from $255
    mmix.write_tetra(0, 0x00000701); // TRAP 0, Fputs (7), 1 (stdout)
    let should_continue = mmix.execute_instruction();
    assert!(should_continue);
    assert_eq!(mmix.get_pc(), 4);
    assert_eq!(mmix.get_register(255), 12); // bytes written
    assert_eq!(handle.stdout(), b"Hello, MMIX!");
    assert!(handle.stderr().is_empty());
}

#[test]
fn test_trap_fputs_stderr() {
    let (host, handle) = CaptureHost::new();
    let mut mmix = MMix::with_host(host);
    let test_string = b"Error message\0";
    let str_addr = 2000u64;

    for (i, &byte) in test_string.iter().enumerate() {
        mmix.write_byte(str_addr + i as u64, byte);
    }

    mmix.set_register(255, str_addr);
    mmix.write_tetra(0, 0x00000702); // TRAP 0, Fputs (7), 2 (stderr)
    let should_continue = mmix.execute_instruction();
    assert!(should_continue);
    assert_eq!(mmix.get_pc(), 4);
    assert_eq!(mmix.get_register(255), 13);
    assert_eq!(handle.stderr(), b"Error message");
    assert!(handle.stdout().is_empty());
}

#[test]
fn test_trap_fputc_stdout() {
    let (host, handle) = CaptureHost::new();
    let mut mmix = MMix::with_host(host);
    mmix.set_register(255, b'X' as u64);
    mmix.write_tetra(0, 0x00008001); // TRAP 0, Fputc (#80), 1 (stdout)
    let should_continue = mmix.execute_instruction();
    assert!(should_continue);
    assert_eq!(mmix.get_register(255), 0); // Success (return code 0 in $255)
    assert_eq!(mmix.get_pc(), 4);
    assert_eq!(handle.stdout(), b"X");
}

#[test]
fn test_trap_fputws() {
    let (host, handle) = CaptureHost::new();
    let mut mmix = MMix::with_host(host);
    // One wyde ("Hi") then a terminating zero wyde.
    let str_addr = 3000u64;
    for (i, &byte) in [b'H', b'i', 0x00, 0x00].iter().enumerate() {
        mmix.write_byte(str_addr + i as u64, byte);
    }

    mmix.set_register(255, str_addr); // $255 contains string address
    mmix.write_tetra(0, 0x00000801); // TRAP 0, Fputws (8), 1 (stdout)
    let should_continue = mmix.execute_instruction();
    assert!(should_continue);
    assert_eq!(mmix.get_register(255), 1); // wyde count returned in $255
    assert_eq!(handle.stdout(), b"Hi");
}

#[test]
fn test_host_trap_hook_reports_arg_and_both_255_values() {
    let (host, handle) = CaptureHost::new();
    let mut mmix = MMix::with_host(host);
    let test_string = b"Hi\0";
    let str_addr = 4000u64;

    for (i, &byte) in test_string.iter().enumerate() {
        mmix.write_byte(str_addr + i as u64, byte);
    }

    mmix.set_register(255, str_addr);
    mmix.write_tetra(0, 0x00000701); // TRAP 0, Fputs (7), 1 (stdout)
    assert!(mmix.execute_instruction());

    let traps = handle.traps();
    assert_eq!(traps.len(), 1);
    let (code, arg, arg255, result255) = traps[0];
    assert_eq!(code, TrapCode::Fputs);
    assert_eq!(arg, 1); // fd 1 (stdout)
    assert_eq!(arg255, str_addr); // $255 before: the string address
    assert_eq!(result255, 2); // $255 after: the byte count
    assert!(handle.diagnostics().is_empty()); // a clean write logs no diagnostic
}

#[test]
fn test_halt_routes_diagnostic_and_flush_to_host() {
    let (host, handle) = CaptureHost::new();
    let mut mmix = MMix::with_host(host);
    mmix.set_register(255, 42);
    mmix.write_tetra(0, 0x00000000); // TRAP 0, Halt (0), 0
    let should_continue = mmix.execute_instruction();
    assert!(!should_continue);
    assert_eq!(
        handle.diagnostics(),
        vec!["HALT trap at PC=0x0000000000000000, exit code=42".to_string()]
    );
    assert_eq!(handle.flushes(), 1);
    assert!(handle.stderr().is_empty()); // no stray fd-2 write alongside it
    assert_eq!(handle.traps(), vec![(TrapCode::Halt, 0, 42, 42)]);
}

#[test]
fn test_boxed_host_delegates_every_method() {
    let (host, handle) = CaptureHost::with_clock(7_000_000);
    let boxed: Box<dyn Host> = Box::new(host);
    let mut mmix = MMix::with_host(boxed);

    mmix.set_register(255, u64::from(b'Z'));
    mmix.write_tetra(0, 0x00008001); // TRAP 0, Fputc (#80), fd 1 -> write
    assert!(mmix.execute_instruction());

    mmix.set_pc(4);
    mmix.write_tetra(4, 0x00008100); // TRAP 0, Time (#81), unit 0 -> now_micros
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(255), 7);

    mmix.set_pc(8);
    mmix.set_register(255, 9);
    mmix.write_tetra(8, 0x00000000); // TRAP 0, Halt (0) -> flush + diagnostic
    assert!(!mmix.execute_instruction());

    // One assertion per trait method, so a missed delegation names itself.
    assert_eq!(handle.stdout(), b"Z"); // write
    assert_eq!(handle.flushes(), 1); // flush
    assert_eq!(handle.diagnostics().len(), 1); // diagnostic
    assert_eq!(
        handle.traps(),
        vec![
            (TrapCode::Fputc, 1, u64::from(b'Z'), 0),
            // $255 enters Time as 0: Fputc stored 0 there on success.
            (TrapCode::Time, 0, 0, 7),
            (TrapCode::Halt, 0, 9, 9),
        ]
    ); // trap
}

#[test]
fn test_injected_clock_drives_handle_time() {
    let (host, _handle) = CaptureHost::with_clock(5_000_000); // 5s since epoch
    let mut mmix = MMix::with_host(host);
    mmix.write_tetra(0, 0x00008100); // TRAP 0, Time (#81), unit=0 (seconds)
    let should_continue = mmix.execute_instruction();
    assert!(should_continue);
    assert_eq!(mmix.get_register(255), 5);
}

#[test]
fn test_trap_read_cstring() {
    let mut mmix = MMix::new();
    // Test the helper function read_cstring
    let test_string = b"Test String\0";
    let addr = 1000u64;

    for (i, &byte) in test_string.iter().enumerate() {
        mmix.write_byte(addr + i as u64, byte);
    }

    let result = mmix.read_cstring(addr, 256);
    assert_eq!(result, "Test String");
}

#[test]
fn test_trap_fclose_error() {
    let mut mmix = MMix::new();
    // Try to close a handle that was never opened.
    mmix.write_tetra(0, 0x00000263); // TRAP 0, Fclose (2), 99
    let should_continue = mmix.execute_instruction();
    assert!(should_continue);
    assert_eq!(mmix.get_register(255), (-1i64) as u64); // Error returned in $255
}

#[test]
fn test_trap_time_microseconds() {
    let mut mmix = MMix::new();
    // TRAP 0, Time, 2 (get time in microseconds)
    mmix.write_tetra(0, 0x00008102); // TRAP 0, Time (#81), 2 (microseconds)

    let should_continue = mmix.execute_instruction();
    assert!(should_continue);
    assert_eq!(mmix.get_pc(), 4); // PC advanced

    let time_us = mmix.get_register(255);
    // Time should be greater than 0 (some time has passed since Unix epoch)
    assert!(time_us > 0);
    // Time should be reasonable (after Jan 1, 2020)
    // Jan 1, 2020 00:00:00 UTC = 1577836800 seconds = 1577836800000000 microseconds
    assert!(time_us > 1_577_836_800_000_000);
    // Time should be before year 3000 (approximately)
    // Jan 1, 3000 00:00:00 UTC ≈ 32503680000 seconds ≈ 32503680000000000 microseconds
    assert!(time_us < 32_503_680_000_000_000);
}

#[test]
fn test_trap_time_milliseconds() {
    let mut mmix = MMix::new();
    // TRAP 0, Time, 1 (get time in milliseconds)
    mmix.write_tetra(0, 0x00008101); // TRAP 0, Time (#81), 1 (milliseconds)

    let should_continue = mmix.execute_instruction();
    assert!(should_continue);
    assert_eq!(mmix.get_pc(), 4);

    let time_ms = mmix.get_register(255);
    assert!(time_ms > 0);
    // After Jan 1, 2020 in milliseconds
    assert!(time_ms > 1_577_836_800_000);
    // Before year 3000 in milliseconds
    assert!(time_ms < 32_503_680_000_000);
}

#[test]
fn test_trap_time_seconds() {
    let mut mmix = MMix::new();
    // TRAP 0, Time, 0 (get time in seconds)
    mmix.write_tetra(0, 0x00008100); // TRAP 0, Time (#81), 0 (seconds)

    let should_continue = mmix.execute_instruction();
    assert!(should_continue);
    assert_eq!(mmix.get_pc(), 4);

    let time_s = mmix.get_register(255);
    assert!(time_s > 0);
    // After Jan 1, 2020 in seconds
    assert!(time_s > 1_577_836_800);
    // Before year 3000 in seconds
    assert!(time_s < 32_503_680_000);
}

#[test]
fn test_trap_time_monotonic() {
    let mut mmix = MMix::new();
    // Get time twice and ensure second is >= first (monotonic)
    mmix.write_tetra(0, 0x00008102); // TRAP 0, Time (#81), 2 (microseconds)
    mmix.execute_instruction();
    let time1 = mmix.get_register(255);

    // Reset PC and execute again
    mmix.set_pc(0);
    mmix.execute_instruction();
    let time2 = mmix.get_register(255);

    // Time should be monotonic (second time >= first time)
    assert!(time2 >= time1);
}

#[test]
fn test_trap_fputs_unknown_fd_returns_error() {
    // Fputs to a closed/unknown fd must report -1, not the byte count.
    let mut mmix = MMix::new();
    let test_string = b"data\0";
    let str_addr = 200u64;
    for (i, &byte) in test_string.iter().enumerate() {
        mmix.write_byte(str_addr + i as u64, byte);
    }
    mmix.set_register(255, str_addr);
    // TRAP 0, Fputs (7), 99 (no such fd)
    mmix.write_tetra(0, 0x00000763);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(255), (-1i64) as u64);
}

#[test]
fn test_trap_fputc_unknown_fd_returns_error() {
    let mut mmix = MMix::new();
    mmix.set_register(255, b'A' as u64);
    // TRAP 0, Fputc (#80), 99
    mmix.write_tetra(0, 0x00008063);
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.get_register(255), (-1i64) as u64);
}

/// End-to-end proof of the `debug` contract: a subroutine that sets
/// locals, prints, makes a nested call with `rJ` saved and restored,
/// prints again, then returns, leaves its caller's locals, `rL`, `rJ`
/// and the returned value exactly as an equivalent program without the
/// two `debug` lines would.
#[test]
fn test_debug_preserves_every_register_around_a_nested_call() {
    use crate::debugger::{entry_point, write_image};
    use crate::mmixal::MMixAssembler;

    const SOURCE: &str = "\
\tLOC\t#100
Main\tSET\t$1,11
\tPUSHJ\t$2,Sub
\tTRAP\t0,Halt,0
Sub\tSET\t$0,5
\tSET\t$1,7
\tdebug\t\"first\"
\tGET\t$2,rJ
\tPUSHJ\t$3,Nested
\tPUT\trJ,$2
\tdebug\t\"second\"
\tPOP\t1,0
Nested\tSET\t$0,42
\tPOP\t0,0
";
    let mut asm = MMixAssembler::new(SOURCE, "<test>");
    asm.parse().expect("program must assemble");
    let main_addr = *asm.labels.get("Main").expect("Main label");

    let (host, handle) = CaptureHost::new();
    let mut mmix = MMix::with_host(host);
    write_image(&mut mmix, &asm);
    mmix.set_pc(entry_point(&asm));

    let (_, stop) = mmix.run_bounded(10_000);
    assert_eq!(stop, Stop::Halted);

    let stdout = String::from_utf8(handle.stdout()).expect("valid utf8");
    assert!(stdout.contains("first\n"), "got {stdout:?}");
    assert!(stdout.contains("second\n"), "got {stdout:?}");
    assert!(
        stdout.find("first").unwrap() < stdout.find("second").unwrap(),
        "the two debug lines must print in order, got {stdout:?}"
    );

    // Main's own local, staged below PUSHJ's hole, survives Sub's whole
    // call -- including both debug lines and the nested call inside it.
    assert_eq!(mmix.get_register(1), 11, "caller's local $1 must survive");
    // Sub's $0, set before either debug line, is what POP 1,0 returns
    // to the hole -- proof debug left it untouched across both calls.
    assert_eq!(mmix.get_register(2), 5, "returned value");
    assert_eq!(mmix.get_special(SpecialReg::RL), 3);
    // rJ lands back on Main's own PUSHJ (the second instruction) + 4:
    // Sub's own POP read it straight, with nothing to restore, since
    // nothing after Sub's own GET/PUT round trip ever touched it.
    assert_eq!(mmix.get_special(SpecialReg::RJ), main_addr + 8);
    // TRAP's own handler advances pc past itself before halting.
    assert_eq!(mmix.get_pc(), main_addr + 12, "halt address");
    assert_eq!(mmix.call_depth(), 0);
}

/// At `rG = 255`, every register but `rG` itself is local. `debug`
/// assembles to a single `TRAP` that touches no register, so it neither
/// inspects nor cares about `rG`'s value: the directive prints and the
/// program halts normally.
#[test]
fn test_debug_at_rg_255_prints_and_continues() {
    use crate::debugger::{entry_point, write_image};
    use crate::mmixal::MMixAssembler;

    const SOURCE: &str = "\
\tLOC\t#100
\tSET\t$1,255
\tPUT\trG,$1
\tdebug\t\"reachable\"
\tTRAP\t0,Halt,0
";
    let mut asm = MMixAssembler::new(SOURCE, "<test>");
    asm.parse().expect("program must assemble");

    let (host, handle) = CaptureHost::new();
    let mut mmix = MMix::with_host(host);
    write_image(&mut mmix, &asm);
    mmix.set_pc(entry_point(&asm));

    let (_, stop) = mmix.run_bounded(100);
    assert_eq!(stop, Stop::Halted);
    assert_eq!(handle.stdout(), b"reachable\n");
    assert_eq!(mmix.get_special(SpecialReg::RG), 255);
}

/// Assembles `source`, runs it under a fresh `CaptureHost` for up to
/// `budget` instructions, and returns the machine, the run's outcome,
/// and captured stdout. Shared by the `debug`-expansion regression
/// tests below.
fn assemble_and_run_bounded(source: &str, budget: usize) -> (MMix, Stop, String) {
    use crate::debugger::{entry_point, write_image};
    use crate::mmixal::MMixAssembler;

    let mut asm = MMixAssembler::new(source, "<test>");
    asm.parse().expect("program must assemble");

    let (host, handle) = CaptureHost::new();
    let mut mmix = MMix::with_host(host);
    write_image(&mut mmix, &asm);
    mmix.set_pc(entry_point(&asm));
    let (_, stop) = mmix.run_bounded(budget);

    let stdout = String::from_utf8(handle.stdout()).expect("valid utf8");
    (mmix, stop, stdout)
}

/// A `debug` line followed by a trailing blank line at end of file, with
/// nothing after it to halt on: falling off the end reads zeroed
/// memory, which decodes as `TRAP 0,Halt,0`, so the program halts and
/// the text prints exactly once.
#[test]
fn test_debug_directive_followed_by_a_blank_line_at_eof() {
    let source = "\tLOC\t#100\nMain\tdebug \"hi\"\n\n";
    let (_, stop, stdout) = assemble_and_run_bounded(source, 1_000);
    assert_eq!(stop, Stop::Halted, "must not loop, got {stdout:?}");
    assert_eq!(stdout, "hi\n");
}

/// A `debug` line followed by a trailing comment line at end of file.
#[test]
fn test_debug_directive_followed_by_a_comment_line_at_eof() {
    let source = "\tLOC\t#100\nMain\tdebug \"hi\"\n% nothing else follows\n";
    let (_, stop, stdout) = assemble_and_run_bounded(source, 1_000);
    assert_eq!(stop, Stop::Halted, "must not loop, got {stdout:?}");
    assert_eq!(stdout, "hi\n");
}

/// A `debug` line as the source's last statement, with nothing after
/// it at all -- not even a blank or comment line.
#[test]
fn test_debug_directive_as_the_last_statement() {
    let source = "\tLOC\t#100\nMain\tdebug \"hi\"\n";
    let (_, stop, stdout) = assemble_and_run_bounded(source, 1_000);
    assert_eq!(stop, Stop::Halted, "must not loop, got {stdout:?}");
    assert_eq!(stdout, "hi\n");
}

/// An `IS` line right after `debug`: unrelated to the call's own return
/// address, it binds its own constant and every later instruction
/// still runs in order.
#[test]
fn test_debug_directive_followed_by_an_is_line() {
    let source = "\
\tLOC\t#100
Main\tdebug\t\"hi\"
Ret2\tIS\t#10C
\tSET\t$1,7
\tSET\t$2,Ret2
\tTRAP\t0,Halt,0
";
    let (mmix, stop, stdout) = assemble_and_run_bounded(source, 1_000);
    assert_eq!(stop, Stop::Halted);
    assert_eq!(stdout, "hi\n");
    assert_eq!(mmix.get_register(1), 7);
    assert_eq!(mmix.get_register(2), 0x10C, "Ret2 must still bind #10C");
}

/// A `GREG` line right after `debug`: it allocates its own register
/// exactly as it would without `debug` in front of it.
#[test]
fn test_debug_directive_followed_by_a_greg_line() {
    let source = "\
\tLOC\t#100
Main\tdebug\t\"hi\"
Foo\tGREG\t@
\tSET\tFoo,42
\tSET\t$1,Foo
\tTRAP\t0,Halt,0
";
    let (mmix, stop, stdout) = assemble_and_run_bounded(source, 1_000);
    assert_eq!(stop, Stop::Halted);
    assert_eq!(stdout, "hi\n");
    assert_eq!(mmix.get_register(1), 42, "Foo must still hold 42");
}

/// A label-only line (no instruction) right after `debug`.
#[test]
fn test_debug_directive_followed_by_a_label_only_line() {
    let source = "\
\tLOC\t#100
Main\tdebug\t\"hi\"
Done
\tSET\t$1,7
\tTRAP\t0,Halt,0
";
    let (mmix, stop, stdout) = assemble_and_run_bounded(source, 1_000);
    assert_eq!(stop, Stop::Halted);
    assert_eq!(stdout, "hi\n");
    assert_eq!(mmix.get_register(1), 7);
}

/// A `:`-prefixed (global-namespace) label-only line right after `debug`.
#[test]
fn test_debug_directive_followed_by_a_colon_prefixed_label() {
    let source = "\
\tLOC\t#100
Main\tdebug\t\"hi\"
:Done
\tSET\t$1,7
\tTRAP\t0,Halt,0
";
    let (mmix, stop, stdout) = assemble_and_run_bounded(source, 1_000);
    assert_eq!(stop, Stop::Halted);
    assert_eq!(stdout, "hi\n");
    assert_eq!(mmix.get_register(1), 7);
}

/// `TRAP 0,Debug,K` writes its string and a newline to handle 1 and
/// changes no register -- not even `$255`. Reverting to the stub
/// expansion (a `JMP`/`SAVE`/`GETA`/`TRAP`/`UNSAVE` sequence) would move
/// the PC through several extra instructions and touch `$254`/`$255`.
#[test]
fn test_debug_trap_writes_text_and_a_newline_touching_no_register() {
    let (host, handle) = CaptureHost::new();
    let mut mmix = MMix::with_host(host);
    mmix.set_debug_strings(vec![b"hello".to_vec()]);

    for reg in 0..=255u8 {
        mmix.set_register(reg, 0xABCD_0000 | u64::from(reg));
    }
    let before: Vec<u64> = (0..=255u8).map(|r| mmix.get_register(r)).collect();

    mmix.write_tetra(0, 0x00008200); // TRAP 0, Debug (#82), K=0
    assert!(mmix.execute_instruction());

    assert_eq!(handle.stdout(), b"hello\n");
    assert_eq!(mmix.get_pc(), 4);
    for reg in 0..=255u8 {
        assert_eq!(
            mmix.get_register(reg),
            before[reg as usize],
            "$#{reg} must survive debug untouched"
        );
    }
}

/// A `K` past the table's end prints nothing, reports a diagnostic, and
/// leaves execution running.
#[test]
fn test_debug_trap_index_past_the_table_reports_and_continues() {
    let (host, handle) = CaptureHost::new();
    let mut mmix = MMix::with_host(host);
    mmix.set_debug_strings(vec![b"only one".to_vec()]);

    mmix.write_tetra(0, 0x00008201); // TRAP 0, Debug (#82), K=1 (no such string)
    assert!(mmix.execute_instruction());

    assert!(handle.stdout().is_empty());
    assert_eq!(mmix.get_pc(), 4);
    assert_eq!(handle.diagnostics().len(), 1);
    assert!(handle.diagnostics()[0].contains('1'));
}

/// Two translation units each print their own `debug` string: the
/// second unit's directive picks up `K` where the first left off.
#[test]
fn test_two_translation_units_each_print_their_own_debug_string() {
    use crate::debugger::{entry_point, write_image};
    use crate::mmixal::MMixAssembler;

    let mut asm = MMixAssembler::new("\tLOC\t#100\nMain\tdebug\t\"from a\"\n", "a.mms");
    asm.add_source("\tdebug\t\"from b\"\n\tTRAP\t0,Halt,0\n", "b.mms");
    asm.parse().expect("program must assemble");

    let (host, handle) = CaptureHost::new();
    let mut mmix = MMix::with_host(host);
    write_image(&mut mmix, &asm);
    mmix.set_pc(entry_point(&asm));
    let (_, stop) = mmix.run_bounded(1_000);

    // Wrong K assignment across units would print "from a" twice.
    assert_eq!(stop, Stop::Halted);
    assert_eq!(handle.stdout(), b"from a\nfrom b\n");
}

/// The `.mmo` round trip prints the same text a direct run does: the
/// string table survives `generate_object_code` -> `MmoDecoder::decode`
/// -> `set_debug_strings` unchanged.
#[test]
fn test_debug_string_survives_the_mmo_round_trip() {
    use crate::debugger::{entry_point, write_image};
    use crate::mmixal::MMixAssembler;
    use crate::mmo::MmoDecoder;

    let source = "\tLOC\t#100\nMain\tdebug\t\"roundtrip\"\n\tTRAP\t0,Halt,0\n";
    let mut asm = MMixAssembler::new(source, "<test>");
    asm.parse().expect("program must assemble");
    let object_code = asm.generate_object_code();

    let (host, handle) = CaptureHost::new();
    let mut mmix = MMix::with_host(host);
    let decoder = MmoDecoder::new(object_code);
    let entry = decoder.decode(|addr, byte| mmix.write_byte(addr, byte));
    mmix.set_debug_strings(decoder.debug_strings());
    mmix.set_pc(entry);

    let (_, stop) = mmix.run_bounded(1_000);
    assert_eq!(stop, Stop::Halted);
    assert_eq!(handle.stdout(), b"roundtrip\n");

    // The direct-run reference: write_image's table must match too.
    let (host2, handle2) = CaptureHost::new();
    let mut mmix2 = MMix::with_host(host2);
    write_image(&mut mmix2, &asm);
    mmix2.set_pc(entry_point(&asm));
    mmix2.run_bounded(1_000);
    assert_eq!(handle.stdout(), handle2.stdout());
}
