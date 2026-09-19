use std::collections::{BTreeSet, HashMap, HashSet};

use tracing::{debug, instrument};

#[macro_use]
mod macros;
mod dispatch;
mod display;
mod exceptions;
mod float;
mod host;
mod memory;
mod registers;
mod stack;
mod trap;

pub use display::ValueFormat;
pub use host::{Host, StdHost};
use registers::{RA_D, RA_I, RA_O, RA_ROUND_SHIFT, RA_U, RA_V, RA_W, RA_X, RA_Z, SAVE_SPECIALS};
pub use registers::{RA_MAX, SpecialReg};
use stack::PopFrame;
pub(crate) use stack::STACK_SEGMENT_START;
use trap::FileHandle;
pub use trap::TrapCode;

/// The MMIX computer architecture.
///
/// MMIX has:
/// - 256 general-purpose registers ($0-$255), each holding 64 bits (an octabyte)
/// - 32 special-purpose registers (rA-rZ, rBB, rTT, rWW, rXX, rYY, rZZ)
/// - 2^64 bytes of virtual memory
///
/// Instructions are tetrybytes (4 bytes) with format: OP X Y Z
/// where OP is the opcode and X, Y, Z are operands.
///
/// # Thread safety
///
/// `MMix` owns a [`Host`] as `Box<dyn Host>` and is therefore none of `Send`,
/// `Sync`, `UnwindSafe`, or `RefUnwindSafe` — a change from 0.2.23, where it
/// was all four. This is deliberate: the intended embedders are
/// single-threaded and capture into `Rc<RefCell<_>>`, which a `Send` bound
/// would forbid. Move the program, not the machine — construct an `MMix` on
/// the thread that runs it, and wrap it in `std::panic::AssertUnwindSafe` to
/// put one through `catch_unwind`.
pub struct MMix {
    /// 256 general-purpose registers, each 64 bits
    general_regs: [u64; 256],

    /// 32 special-purpose registers, each 64 bits
    /// Indexed by SpecialReg enum values
    special_regs: [u64; 32],

    /// Virtual memory
    /// this should use paging/segmentation
    /// Key is the memory address, value is the byte
    /// Iteration order is unspecified; [`MMix::occupied`] sorts it.
    memory: HashMap<u64, u8>,

    /// Program counter (location of next instruction)
    pc: u64,

    /// Open TRAP handles, numbered 0-255 as the caller chooses (`Fopen`
    /// picks 3+; 0-2 are the standard streams, seeded at [`MMix::initialize`]
    /// and never reassigned).
    file_handles: HashMap<u8, FileHandle>,

    /// Exit code from HALT trap (to be returned as process exit code)
    exit_code: u64,

    /// Where process-level effects (writes, the clock, diagnostics, trap
    /// events) go. `MMix::new()` installs `StdHost`; `MMix::with_host`
    /// installs anything else.
    host: Box<dyn Host>,

    /// The strings every `debug` directive collected, `K`-indexed. Installed
    /// by [`MMix::set_debug_strings`] at load time; [`TrapCode::Debug`]
    /// reads it and touches nothing else.
    debug_strings: Vec<Vec<u8>>,

    /// Whether [`MMix::write_byte`] records the address it touches. Survives
    /// [`MMix::reset`]; see [`MMix::set_journal`].
    journal_enabled: bool,

    /// Addresses [`MMix::write_byte`] has touched since the last
    /// [`MMix::take_journal`], while the journal is enabled.
    journal: HashSet<u64>,

    /// Addresses `write_image` loaded, zero byte or not. Only
    /// [`MMix::write_loaded_byte`] records into this; plain
    /// [`MMix::write_byte`] calls (including everything a running program
    /// does after load) never touch it. Backs [`MMix::loaded_extent`].
    loaded: BTreeSet<u64>,
}

impl Default for MMix {
    fn default() -> Self {
        Self::new()
    }
}

impl MMix {
    /// Create a new MMIX computer with all registers and memory initialized
    /// to zero, and process I/O routed through `StdHost` — today's behavior,
    /// unchanged.
    pub fn new() -> Self {
        Self::with_host(StdHost)
    }

    /// Create a new MMIX computer with all registers and memory initialized
    /// to zero, routing every process-level write, the clock, diagnostics,
    /// and trap events through `host` instead of the process.
    ///
    /// The resulting `MMix` is none of `Send`, `Sync`, `UnwindSafe`, or
    /// `RefUnwindSafe` — see the [`MMix`] docs.
    pub fn with_host<H: Host + 'static>(host: H) -> Self {
        Self::blank(Box::new(host))
    }

    /// A machine in its starting state, owning `host`. The single place every
    /// field is named, so construction and [`MMix::reset`] cannot disagree
    /// about what "fresh" means — adding a field to `MMix` fails to compile
    /// here rather than silently surviving a reset.
    fn blank(host: Box<dyn Host>) -> Self {
        let mut mmix = Self {
            general_regs: [0; 256],
            special_regs: [0; 32],
            memory: HashMap::new(),
            pc: 0,
            file_handles: HashMap::new(),
            exit_code: 0,
            host,
            debug_strings: Vec::new(),
            journal_enabled: false,
            journal: HashSet::new(),
            loaded: BTreeSet::new(),
        };
        mmix.initialize();
        mmix
    }

    /// Return the machine to its freshly-constructed state — every register,
    /// all of memory, the program counter, the call-frame stack, open file
    /// handles, and the exit code — while keeping the installed [`Host`].
    ///
    /// A caller that injected a host to capture output needs this: dropping
    /// the machine and building another would take the host with it, and only
    /// the machine's state is stale between runs. The host's own buffers are
    /// untouched, so a host that accumulates sees successive runs appended;
    /// clear them through your own handle if that is not what you want.
    ///
    /// The journal's enabled flag (see [`MMix::set_journal`]) survives; the
    /// accumulated buffer does not, since a fresh machine has written
    /// nothing yet.
    pub fn reset(&mut self) {
        let host = std::mem::replace(&mut self.host, Box::new(StdHost));
        let journal_enabled = self.journal_enabled;
        *self = Self::blank(host);
        self.journal_enabled = journal_enabled;
    }

    /// The installed [`Host`], for a caller that needs to reach it after
    /// construction rather than keeping a shared handle.
    ///
    /// [`Host`] requires [`Any`], so an embedder can recover its concrete
    /// host type:
    ///
    /// ```
    /// # use checksmix::{Host, MMix};
    /// # use std::any::Any;
    /// # struct Capture(Vec<u8>);
    /// # impl Host for Capture {
    /// #     fn write(&mut self, _fd: u8, b: &[u8]) -> std::io::Result<()> {
    /// #         self.0.extend_from_slice(b); Ok(())
    /// #     }
    /// #     fn now_micros(&mut self) -> u64 { 0 }
    /// #     fn diagnostic(&mut self, _m: &str) {}
    /// # }
    /// let mut mmix = MMix::with_host(Capture(Vec::new()));
    /// let host: &mut dyn Any = mmix.host_mut();
    /// let capture = host.downcast_mut::<Capture>().expect("our own host type");
    /// capture.0.clear();
    /// ```
    pub fn host_mut(&mut self) -> &mut dyn Host {
        &mut *self.host
    }

    /// Consume the machine and return its [`Host`], for a caller that wants
    /// what the host captured once the program is done.
    pub fn into_host(self) -> Box<dyn Host> {
        self.host
    }

    /// The special-register values a machine starts life with.
    fn initialize(&mut self) {
        // Initialize rN (serial number register) to a default value
        // The MMIX specification says this should be a unique machine serial number
        self.set_special(SpecialReg::RN, 2009);
        // Initialize rG (global threshold register) - registers $rG..$255 are global
        // With no GREG declarations, rG=32
        self.set_special(SpecialReg::RG, 32);
        // Initialize rL (local threshold register) - number of local registers in use
        // Start with 0, will be updated by PUSHJ/POP
        self.set_special(SpecialReg::RL, 0);
        // rO and rS both start at the register stack's base, segment 6 per
        // MMIX convention. PUSHJ/POP store eagerly, so the two stay equal
        // and move together for the life of the machine.
        self.set_special(SpecialReg::RO, STACK_SEGMENT_START);
        self.set_special(SpecialReg::RS, STACK_SEGMENT_START);

        // StdIn/StdOut/StdErr are open at start, TextRead/TextWrite/TextWrite
        // per the reference. None carries a `File`: fd 0's reads and fd 1/2's
        // writes route through the installed `Host`.
        self.file_handles.insert(
            0,
            FileHandle {
                file: None,
                read: true,
                write: false,
                seek: false,
                read_write: false,
            },
        );
        for fd in [1u8, 2] {
            self.file_handles.insert(
                fd,
                FileHandle {
                    file: None,
                    read: false,
                    write: true,
                    seek: false,
                    read_write: false,
                },
            );
        }
    }

    /// Install the string table every `debug` directive's `TRAP 0,Debug,K`
    /// reads from, `K`-indexed. The loader (`write_image`, `run_mmo`) calls
    /// this once, before the program runs.
    pub fn set_debug_strings(&mut self, strings: Vec<Vec<u8>>) {
        self.debug_strings = strings;
    }

    /// Get the value of a general-purpose register.
    pub fn get_register(&self, reg: u8) -> u64 {
        self.general_regs[reg as usize]
    }

    /// Set the value of a general-purpose register.
    ///
    /// Writing to a local register $i with i >= rL grows rL to i+1 and zeroes
    /// every register the growth exposes, so a register between the old and
    /// new rL never reads a stale value.
    pub fn set_register(&mut self, reg: u8, value: u64) {
        self.claim_local(reg);
        self.general_regs[reg as usize] = value;
    }

    /// Raise rL to cover `reg` when it names a marginal local register,
    /// zeroing every register from the old rL through `reg`. A no-op when
    /// `reg` is already local or is a global register (`reg >= rG`).
    fn claim_local(&mut self, reg: u8) {
        // UNSAVE restores rL and rG straight from guest memory, so either can
        // name a register the file does not have. Compare at full width: a
        // truncated rL would read as a low register and drop live locals.
        let rg = self.special_regs[SpecialReg::RG as usize];
        if (reg as u64) >= rg {
            return;
        }
        let rl = self.special_regs[SpecialReg::RL as usize];
        if (reg as u64) < rl {
            return;
        }
        let rl = rl as u8;
        for marginal in rl..=reg {
            self.general_regs[marginal as usize] = 0;
        }
        self.special_regs[SpecialReg::RL as usize] = (reg as u64) + 1;
    }

    /// True when `op_byte`'s X field names a general-register destination:
    /// the floating-point and integer arithmetic and compare instructions,
    /// the conditional- and zero-set instructions, the loads, GO, PUSHGO,
    /// the bitwise and SETL-family instructions, PUSHJ, GETA, SAVE and GET.
    /// Branches read a register as a source (not a destination) and are excluded.
    /// Stores, PUT, POP, UNSAVE and the opcodes with no register operand are not.
    fn writes_general_register_x(op_byte: u8) -> bool {
        matches!(
            op_byte,
            0x01..=0x3F | 0x60..=0x99 | 0x9E..=0x9F | 0xBE..=0xEF | 0xF2..=0xF5 | 0xFA | 0xFE
        )
    }

    /// Get the value of a special-purpose register.
    pub fn get_special(&self, reg: SpecialReg) -> u64 {
        self.special_regs[reg as usize]
    }

    /// Set the value of a special-purpose register.
    pub fn set_special(&mut self, reg: SpecialReg, value: u64) {
        self.special_regs[reg as usize] = value;
    }

    /// Emit a diagnostic and report the halt `execute_instruction` should
    /// propagate: no register or memory change and no PC advance, on the
    /// caller's promise that it made none before calling this. Every
    /// `PUT`/`PUTI` rejection and `SAVE`/`UNSAVE`'s validation failures route
    /// through this one diagnose-then-refuse path.
    fn reject(&mut self, message: &str) -> bool {
        self.host.diagnostic(message);
        false
    }

    /// Get the current program counter.
    pub fn get_pc(&self) -> u64 {
        self.pc
    }

    /// Set the program counter.
    pub fn set_pc(&mut self, pc: u64) {
        self.pc = pc;
    }

    /// Advance the program counter by 4 bytes (one instruction).
    pub fn advance_pc(&mut self) {
        self.pc = self.pc.wrapping_add(4);
    }

    /// Get the exit code set by TRAP 0 (Halt).
    pub fn get_exit_code(&self) -> u64 {
        self.exit_code
    }

    /// Execute instructions starting from the current PC until a halt condition.
    /// Returns the number of instructions executed.
    #[instrument(skip(self))]
    pub fn run(&mut self) -> usize {
        self.run_bounded(usize::MAX).0
    }

    /// Execute instructions starting from the current PC until the machine
    /// halts or `budget` instructions have run, whichever comes first.
    /// Returns the instruction count and which condition stopped it.
    ///
    /// `MMix` has no breakpoint concept, so the result is never
    /// [`Stop::Breakpoint`] — that variant exists for [`crate::Debugger`]'s
    /// use.
    #[instrument(skip(self))]
    pub fn run_bounded(&mut self, budget: usize) -> (usize, Stop) {
        debug!("Starting MMIX execution");
        let mut count = 0;
        let stop = loop {
            if count >= budget {
                break Stop::BudgetExhausted;
            }
            if !self.execute_instruction() {
                break Stop::Halted;
            }
            count += 1;
        };
        match stop {
            Stop::Halted => self.host.diagnostic(&format!(
                "Execution stopped at PC={:#018x} after {} instructions",
                self.pc, count
            )),
            Stop::BudgetExhausted => self.host.diagnostic(&format!(
                "Execution paused at PC={:#018x} after {} instructions (budget exhausted)",
                self.pc, count
            )),
            Stop::Breakpoint(_) => unreachable!("run_bounded never returns Stop::Breakpoint"),
        }
        debug!(instruction_count = count, "Execution completed");
        (count, stop)
    }
}

/// Why [`MMix::run_bounded`] or a [`crate::Debugger`] step loop stopped.
///
/// More variants may be added in future releases, so downstream matches
/// must carry a wildcard arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Stop {
    /// The machine halted (TRAP 0, an unhandled TRIP, or an unhandled
    /// register trap — `execute_instruction` returning `false` covers all
    /// three with no finer distinction).
    Halted,
    /// The instruction budget ran out before the machine halted. The
    /// machine is unchanged; resuming from here is calling the same
    /// bounded-run method again.
    BudgetExhausted,
    /// A breakpoint address was reached. `MMix` itself has no breakpoint
    /// concept, so [`MMix::run_bounded`] never produces this — it exists for
    /// [`crate::Debugger`], which owns the breakpoint set.
    Breakpoint(u64),
}

#[cfg(test)]
mod tests;
