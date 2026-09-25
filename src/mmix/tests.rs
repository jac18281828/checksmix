//! Shared unit-test fixtures and helpers used across more than one area,
//! plus one child module per area under test.

use super::*;
use std::cell::RefCell;
use std::rc::Rc;

/// What a `CaptureHost` records, shared with the test via `CaptureHandle`.
#[derive(Default)]
struct CaptureLog {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    diagnostics: Vec<String>,
    traps: Vec<(TrapCode, u8, u64, u64)>,
    flushes: usize,
}

/// A clone of a `CaptureHost`'s buffers, held by the test after the host
/// is moved into `MMix::with_host`.
#[derive(Clone, Default)]
struct CaptureHandle(Rc<RefCell<CaptureLog>>);

impl CaptureHandle {
    fn stdout(&self) -> Vec<u8> {
        self.0.borrow().stdout.clone()
    }

    fn stderr(&self) -> Vec<u8> {
        self.0.borrow().stderr.clone()
    }

    fn diagnostics(&self) -> Vec<String> {
        self.0.borrow().diagnostics.clone()
    }

    fn traps(&self) -> Vec<(TrapCode, u8, u64, u64)> {
        self.0.borrow().traps.clone()
    }

    fn flushes(&self) -> usize {
        self.0.borrow().flushes
    }
}

/// A `Host` that records writes, diagnostics, and trap events instead of
/// sending them to the process, and reports a fixed clock rather than
/// `SystemTime::now()`.
struct CaptureHost {
    log: Rc<RefCell<CaptureLog>>,
    clock_micros: u64,
}

impl CaptureHost {
    fn new() -> (Self, CaptureHandle) {
        let log = Rc::new(RefCell::new(CaptureLog::default()));
        let handle = CaptureHandle(log.clone());
        (
            Self {
                log,
                clock_micros: 0,
            },
            handle,
        )
    }

    fn with_clock(clock_micros: u64) -> (Self, CaptureHandle) {
        let (mut host, handle) = Self::new();
        host.clock_micros = clock_micros;
        (host, handle)
    }
}

impl Host for CaptureHost {
    fn write(&mut self, fd: u8, bytes: &[u8]) -> std::io::Result<()> {
        let mut log = self.log.borrow_mut();
        match fd {
            1 => log.stdout.extend_from_slice(bytes),
            2 => log.stderr.extend_from_slice(bytes),
            _ => return Err(std::io::Error::other("CaptureHost: unsupported fd")),
        }
        Ok(())
    }

    fn flush(&mut self) {
        self.log.borrow_mut().flushes += 1;
    }

    fn now_micros(&mut self) -> u64 {
        self.clock_micros
    }

    fn diagnostic(&mut self, msg: &str) {
        self.log.borrow_mut().diagnostics.push(msg.to_string());
    }

    fn trap(&mut self, code: TrapCode, arg: u8, arg255: u64, result255: u64) {
        self.log
            .borrow_mut()
            .traps
            .push((code, arg, arg255, result255));
    }
}

/// Assemble a whole program, load it and run it to HALT, returning $255.
/// Zeroed memory decodes as TRAP 0,Halt,0, so a mis-jump halts with 0
/// rather than hanging -- which is what makes these assertions bite.
fn run_to_halt(source: &str) -> u64 {
    use crate::debugger::{entry_point, write_image};
    use crate::mmixal::MMixAssembler;

    let mut asm = MMixAssembler::new(source, "<test>");
    asm.parse().expect("program must assemble");
    let mut mmix = MMix::new();
    write_image(&mut mmix, &asm);
    mmix.set_pc(entry_point(&asm));
    mmix.run();
    mmix.get_register(255)
}

/// Write `value`'s four bytes as loaded, the way `write_image` does, so a
/// trip landing here does not read the address as an unloaded vector.
fn load_tetra(mmix: &mut MMix, addr: u64, value: u32) {
    for i in 0..4 {
        let shift = 24 - 8 * i;
        mmix.write_loaded_byte(addr + i as u64, (value >> shift) as u8);
    }
}

mod dispatch_arithmetic;
mod dispatch_branches;
mod dispatch_misc;
mod display;
mod exceptions;
mod float;
mod lifecycle;
mod memory;
mod registers;
mod stack;
mod trap;
