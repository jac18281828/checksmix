//! MMix construction, reset, run/run_bounded, and Stop.

use super::super::*;
use super::*;

#[test]
fn test_mmix_new() {
    let mmix = MMix::new();
    assert_eq!(mmix.get_register(0), 0);
    assert_eq!(mmix.get_register(255), 0);
    assert_eq!(mmix.get_special(SpecialReg::RA), 0);
    assert_eq!(mmix.get_pc(), 0);
}

#[test]
fn test_pc_operations() {
    let mut mmix = MMix::new();
    assert_eq!(mmix.get_pc(), 0);
    mmix.set_pc(0x1000);
    assert_eq!(mmix.get_pc(), 0x1000);
    mmix.advance_pc();
    assert_eq!(mmix.get_pc(), 0x1004);
}

#[test]
fn test_reset_restores_a_dirtied_machine_and_keeps_the_host() {
    let (host, handle) = CaptureHost::with_clock(11);
    let mut mmix = MMix::with_host(host);

    // Dirty most fields directly; `file_handles` needs a real file, so
    // it is left to `blank`'s exhaustive struct literal.
    const SCRATCH: u64 = 0x5000; // never executed, so nothing overwrites it
    for reg in 0..=255u8 {
        mmix.set_register(reg, 0xDEAD_0000 | u64::from(reg));
    }
    mmix.set_special(SpecialReg::RA, 0x1234);
    mmix.set_special(SpecialReg::RG, 200);
    mmix.write_tetra(SCRATCH, 0xFFFF_FFFF);
    assert_ne!(mmix.read_tetra(SCRATCH), 0, "memory must start dirty");

    // A real PUSHJ, so `call_depth` is genuinely nonzero.
    mmix.set_pc(0x4000);
    mmix.write_tetra(0x4000, 0xF2_02_00_01); // PUSHJ $2, forward 1
    assert!(mmix.execute_instruction());
    assert_eq!(mmix.call_depth(), 1, "PUSHJ must push a frame");

    mmix.set_pc(0x4100);
    mmix.set_register(255, 77);
    mmix.write_tetra(0x4100, 0x00000000); // TRAP 0, Halt -> sets exit_code
    assert!(!mmix.execute_instruction());
    assert_eq!(mmix.get_exit_code(), 77);

    mmix.reset();

    let fresh = MMix::new();
    for reg in 0..=255u8 {
        assert_eq!(mmix.get_register(reg), fresh.get_register(reg), "$#{reg}");
    }
    for spec in [
        SpecialReg::RA,
        SpecialReg::RG,
        SpecialReg::RL,
        SpecialReg::RN,
        SpecialReg::RO,
        SpecialReg::RS,
    ] {
        assert_eq!(mmix.get_special(spec), fresh.get_special(spec), "{spec:?}");
    }
    assert_eq!(mmix.get_pc(), fresh.get_pc());
    assert_eq!(mmix.get_exit_code(), fresh.get_exit_code());
    assert_eq!(mmix.call_depth(), fresh.call_depth(), "frames");
    assert_eq!(mmix.read_tetra(SCRATCH), 0, "memory");

    // The host survives, and is still the injected one.
    mmix.set_register(255, u64::from(b'A'));
    mmix.write_tetra(0, 0x00008001); // TRAP 0, Fputc (#80), fd 1
    assert!(mmix.execute_instruction());
    assert_eq!(handle.stdout(), b"A");
}

/// `JMP 0,0,0` at address `addr`: offset 0 branches back to itself,
/// forever. Shared by every never-halts test below.
fn write_infinite_loop(mmix: &mut MMix, addr: u64) {
    mmix.write_tetra(addr, 0xF0000000);
}

#[test]
fn run_bounded_halts_normally_and_reports_the_count() {
    let mut mmix = MMix::new();
    mmix.write_tetra(0, 0xE7010000); // INCL $1, YZ=0
    mmix.write_tetra(4, 0xE7010203); // INCL $1, YZ=0x0203
    mmix.write_tetra(8, 0xE7010203); // INCL $1, YZ=0x0203
    mmix.write_tetra(12, 0xFF000000); // TRIP (halt)

    let (count, stop) = mmix.run_bounded(100);
    assert_eq!(count, 3);
    assert_eq!(stop, Stop::Halted);
}

#[test]
fn run_bounded_stops_at_the_budget_on_a_program_that_never_halts() {
    let mut mmix = MMix::new();
    write_infinite_loop(&mut mmix, 0);

    let (count, stop) = mmix.run_bounded(1_000);
    assert_eq!(count, 1_000);
    assert_eq!(stop, Stop::BudgetExhausted);
}

#[test]
fn run_bounded_halted_diagnostic_matches_runs_pre_existing_text() {
    let (host, handle) = CaptureHost::new();
    let mut mmix = MMix::with_host(host);
    mmix.write_tetra(0, 0xFF000000); // TRIP (halt)

    let (count, stop) = mmix.run_bounded(100);
    assert_eq!(stop, Stop::Halted);
    // TRIP itself emits its own diagnostic first; run_bounded's is last.
    assert_eq!(
        handle.diagnostics().last(),
        Some(&format!(
            "Execution stopped at PC={:#018x} after {} instructions",
            mmix.get_pc(),
            count
        ))
    );
}

#[test]
fn run_bounded_exhausted_diagnostic_is_visibly_different() {
    let (host, handle) = CaptureHost::new();
    let mut mmix = MMix::with_host(host);
    write_infinite_loop(&mut mmix, 0);

    let (count, stop) = mmix.run_bounded(1_000);
    assert_eq!(stop, Stop::BudgetExhausted);
    assert_eq!(
        handle.diagnostics().last(),
        Some(&format!(
            "Execution paused at PC={:#018x} after {} instructions (budget exhausted)",
            mmix.get_pc(),
            count
        ))
    );
}
