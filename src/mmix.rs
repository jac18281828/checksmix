use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt;

use tracing::{debug, instrument};

#[macro_use]
mod macros;
mod exceptions;
mod float;
mod host;
mod memory;
mod registers;
mod stack;
mod trap;

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

    // ========== Internal Helpers ==========

    /// Conditional branch forward: if cond, PC = PC + 4*YZ.
    #[inline]
    fn branch_forward(&mut self, cond: bool, y: u8, z: u8) {
        if cond {
            let yz = ((y as u16) << 8) | (z as u16);
            self.pc = self.pc.wrapping_add((yz as u64) * 4);
        } else {
            self.advance_pc();
        }
    }

    /// Conditional branch backward: if cond, PC = PC + 4*(YZ - 65536).
    #[inline]
    fn branch_backward(&mut self, cond: bool, y: u8, z: u8) {
        if cond {
            // YZ is unsigned in both directions; the backward opcode carries the
            // sign, so YZ = 0 is -65536 tetras and YZ = 65535 is -1.
            let yz = ((y as u16) << 8) | (z as u16);
            let offset = yz as i64 - 65536;
            self.pc = self.pc.wrapping_add((offset * 4) as u64);
        } else {
            self.advance_pc();
        }
    }

    /// Conditional set: if cond($Y), $X = $Z, else do nothing
    #[inline]
    fn cond_set_rr(&mut self, x: u8, _y: u8, z: u8, cond: bool) {
        if cond {
            let val_z = self.get_register(z);
            self.set_register(x, val_z);
        }
        self.advance_pc();
    }

    /// Conditional set with immediate: if cond($Y), $X = Z, else do nothing
    #[inline]
    fn cond_set_ri(&mut self, x: u8, _y: u8, z: u8, cond: bool) {
        if cond {
            self.set_register(x, z as u64);
        }
        self.advance_pc();
    }

    /// Zero or set with register: if cond($Y), $X = $Z, else $X = 0
    #[inline]
    fn zero_set_rr(&mut self, x: u8, _y: u8, z: u8, cond: bool) {
        let result = if cond { self.get_register(z) } else { 0 };
        self.set_register(x, result);
        self.advance_pc();
    }

    /// Zero or set with immediate: if cond($Y), $X = Z, else $X = 0
    #[inline]
    fn zero_set_ri(&mut self, x: u8, _y: u8, z: u8, cond: bool) {
        let result = if cond { z as u64 } else { 0 };
        self.set_register(x, result);
        self.advance_pc();
    }

    // ========== Instruction Execution ==========

    /// Execute a single instruction at the current program counter.
    /// Returns true if execution should continue, false if halted.
    #[instrument(skip(self), fields(pc = format!("0x{:X}", self.pc)))]
    pub fn execute_instruction(&mut self) -> bool {
        let (op_byte, x, y, z) = self.fetch_instruction();
        debug!(
            op = format!("0x{:02X}", op_byte),
            x, y, z, "Executing instruction"
        );

        use crate::mmixal::Opcode;
        let opcode = Opcode::try_from(op_byte).unwrap_or_else(|_| {
            panic!("Invalid opcode {:#04x} at PC {:#018x}", op_byte, self.pc);
        });

        self.dispatch(opcode, op_byte, x, y, z)
    }

    /// Decode and run one instruction given its already-resolved opcode and
    /// fields. Split out of [`MMix::execute_instruction`] so `RESUME` can run
    /// the instruction carried in `rX` (§1 rule 4) as if fetched at a chosen
    /// address, with no real memory read.
    fn dispatch(
        &mut self,
        opcode: crate::mmixal::Opcode,
        op_byte: u8,
        x: u8,
        y: u8,
        z: u8,
    ) -> bool {
        use crate::mmixal::Opcode;

        // Operands are read before the destination raises rL. A marginal $Y
        // or $Z still reads as zero when the instruction executes.
        //
        // SAVE is excluded even though its X is a genuine destination: X
        // must already be global, so a legal SAVE never needs this claim,
        // and claiming a local X here would raise rL before SAVE's own
        // rejection runs, breaking its promise to leave a rejected machine
        // unchanged. SAVE's arm validates X itself.
        if Self::writes_general_register_x(op_byte) && opcode != Opcode::SAVE {
            self.claim_local(x);
        }

        match opcode {
            // Floating Point instructions
            Opcode::TRAP => {
                // TRAP X, YZ or TRAP X, Y, Z - Force trap interrupt
                // X = 0 for immediate (YZ), X > 0 for register ($Y, $Z)
                // For immediate form: Y is the trap number, Z is an argument
                if x == 0 {
                    // Immediate TRAP - handle system calls
                    match TrapCode::from_u8(y) {
                        Some(trap) => self.handle_trap(trap, z),
                        None => {
                            debug!(trap_code = y, arg = z, "TRAP: Unhandled trap code");
                            self.host.diagnostic(&format!(
                                "Unhandled TRAP code {} at PC={:#018x}",
                                y, self.pc
                            ));
                            self.advance_pc();
                            true
                        }
                    }
                } else {
                    // Register form - not commonly used for syscalls
                    let trap_val = {
                        let y_val = self.get_register(y);
                        let z_val = self.get_register(z);
                        (y_val << 32) | z_val
                    };
                    self.host.diagnostic(&format!(
                        "Register TRAP x={} y=$ {} z=$ {} val=0x{:016X} at PC={:#018x}",
                        x, y, z, trap_val, self.pc
                    ));
                    self.set_special(SpecialReg::RBB, trap_val);
                    self.advance_pc();
                    false // Halt by default for unhandled register traps
                }
            }
            Opcode::FCMP => {
                // FCMP $X, $Y, $Z - Floating compare. Raises I when an operand is NaN.
                let y_raw = self.get_register(y);
                let z_raw = self.get_register(z);
                let y_val = Self::u64_to_f64(y_raw);
                let z_val = Self::u64_to_f64(z_raw);
                let flags = if y_val.is_nan() || z_val.is_nan() {
                    RA_I
                } else {
                    0
                };
                self.set_register(x, Self::fcmp(y_val, z_val));
                self.raise_exceptions(flags, op_byte, x, y, z, y_raw, z_raw)
            }
            Opcode::FUN => {
                // FUN $X, $Y, $Z - Floating unordered (no exception flag)
                fcmp_rr!(
                    self,
                    x,
                    y,
                    z,
                    |y: f64, z: f64| if y.is_nan() || z.is_nan() { 1 } else { 0 }
                )
            }
            Opcode::FEQL => {
                // FEQL $X, $Y, $Z - Floating equal to (no exception flag)
                fcmp_rr!(self, x, y, z, |y: f64, z: f64| if y == z { 1 } else { 0 })
            }
            Opcode::FADD => {
                let y_val = self.get_register(y);
                let z_val = self.get_register(z);
                let a = Self::u64_to_f64(y_val);
                let b = Self::u64_to_f64(z_val);
                let r_near = a + b;
                let (_, err) = Self::two_sum(a, b);
                // `two_sum` is exact for finite operands, so a zero sum with a
                // zero residual is exact cancellation, not an underflow.
                let (r, flags) = self.finalize_fp_binop(a, b, r_near, err, err == 0.0);
                self.set_register(x, Self::f64_to_u64(r));
                self.raise_exceptions(flags, op_byte, x, y, z, y_val, z_val)
            }
            Opcode::FIX => {
                // FIX $X, Y, $Z - Convert floating to fixed (signed). Raises X
                // on inexact and W when the value is out of i64 range or NaN/Inf.
                let mode = match self.resolved_round_mode(y) {
                    Ok(m) => m,
                    Err(()) => return self.illegal_round_mode("FIX", y),
                };
                // Y is the rounding-mode field, not a register operand.
                let y_val = y as u64;
                let z_val = self.get_register(z);
                let f = Self::u64_to_f64(z_val);
                let rounded = Self::round_with_mode(f, mode);
                let mut flags = 0u64;
                let value = if !f.is_finite() {
                    flags |= RA_W;
                    if f.is_nan() {
                        flags |= RA_I;
                    }
                    0u64
                } else if rounded > i64::MAX as f64 || rounded < i64::MIN as f64 {
                    flags |= RA_W;
                    rounded as i64 as u64
                } else {
                    rounded as i64 as u64
                };
                if rounded != f && f.is_finite() {
                    flags |= RA_X;
                }
                self.set_register(x, value);
                self.raise_exceptions(flags, op_byte, x, y, z, y_val, z_val)
            }
            Opcode::FSUB => {
                let y_val = self.get_register(y);
                let z_val = self.get_register(z);
                let a = Self::u64_to_f64(y_val);
                let b = Self::u64_to_f64(z_val);
                let r_near = a - b;
                let (_, err) = Self::two_sum(a, -b);
                let (r, flags) = self.finalize_fp_binop(a, b, r_near, err, err == 0.0);
                self.set_register(x, Self::f64_to_u64(r));
                self.raise_exceptions(flags, op_byte, x, y, z, y_val, z_val)
            }
            Opcode::FIXU => {
                // FIXU $X, Y, $Z - Convert floating to fixed unsigned
                let mode = match self.resolved_round_mode(y) {
                    Ok(m) => m,
                    Err(()) => return self.illegal_round_mode("FIXU", y),
                };
                // Y is the rounding-mode field, not a register operand.
                let y_val = y as u64;
                let z_val = self.get_register(z);
                let f = Self::u64_to_f64(z_val);
                let rounded = Self::round_with_mode(f, mode);
                let mut flags = 0u64;
                let value = if !f.is_finite() {
                    flags |= RA_W;
                    if f.is_nan() {
                        flags |= RA_I;
                    }
                    0u64
                } else {
                    Self::wrap_to_u64(rounded)
                };
                if rounded != f && f.is_finite() {
                    flags |= RA_X;
                }
                self.set_register(x, value);
                self.raise_exceptions(flags, op_byte, x, y, z, y_val, z_val)
            }
            Opcode::FLOT => {
                // FLOT $X, Y, $Z - Convert fixed to floating (signed)
                i2f_conv_rr!(self, op_byte, x, y, z, true, "FLOT")
            }
            Opcode::FLOTI => {
                // FLOTI $X, Y, Z - Convert fixed to floating immediate (signed)
                i2f_conv_ri!(self, op_byte, x, y, z, true, "FLOTI")
            }
            Opcode::FLOTU => {
                // FLOTU $X, Y, $Z - Convert fixed unsigned to floating
                i2f_conv_rr!(self, op_byte, x, y, z, false, "FLOTU")
            }
            Opcode::FLOTUI => {
                // FLOTUI $X, Y, Z - Convert fixed unsigned to floating immediate
                i2f_conv_ri!(self, op_byte, x, y, z, false, "FLOTUI")
            }
            Opcode::SFLOT => {
                // SFLOT $X, Y, $Z - Convert signed integer to f32 (in f64 register)
                let mode = match self.resolved_round_mode(y) {
                    Ok(m) => m,
                    Err(()) => return self.illegal_round_mode("SFLOT", y),
                };
                // Y is the rounding-mode field, not a register operand.
                let y_val = y as u64;
                let z_val = self.get_register(z);
                let v = z_val as i64;
                let flags = Self::int_to_f64_inexact(v.unsigned_abs());
                let (narrowed, narrow_flags) = self.f64_to_f32_rounded(v as f64, mode);
                self.set_register(x, Self::f64_to_u64(narrowed));
                self.raise_exceptions(flags | narrow_flags, op_byte, x, y, z, y_val, z_val)
            }
            Opcode::SFLOTI => {
                let mode = match self.resolved_round_mode(y) {
                    Ok(m) => m,
                    Err(()) => return self.illegal_round_mode("SFLOTI", y),
                };
                // Y is the rounding-mode field and Z the literal operand,
                // neither a register.
                let y_val = y as u64;
                let z_val = z as u64;
                let v = (z as i8) as i64;
                let flags = Self::int_to_f64_inexact(v.unsigned_abs());
                let (narrowed, narrow_flags) = self.f64_to_f32_rounded(v as f64, mode);
                self.set_register(x, Self::f64_to_u64(narrowed));
                self.raise_exceptions(flags | narrow_flags, op_byte, x, y, z, y_val, z_val)
            }
            Opcode::SFLOTU => {
                let mode = match self.resolved_round_mode(y) {
                    Ok(m) => m,
                    Err(()) => return self.illegal_round_mode("SFLOTU", y),
                };
                // Y is the rounding-mode field, not a register operand.
                let y_val = y as u64;
                let v = self.get_register(z);
                let flags = Self::int_to_f64_inexact(v);
                let (narrowed, narrow_flags) = self.f64_to_f32_rounded(v as f64, mode);
                self.set_register(x, Self::f64_to_u64(narrowed));
                self.raise_exceptions(flags | narrow_flags, op_byte, x, y, z, y_val, v)
            }
            Opcode::SFLOTUI => {
                let mode = match self.resolved_round_mode(y) {
                    Ok(m) => m,
                    Err(()) => return self.illegal_round_mode("SFLOTUI", y),
                };
                // Y is the rounding-mode field and Z the literal operand,
                // neither a register.
                let y_val = y as u64;
                let z_val = z as u64;
                let flags = Self::int_to_f64_inexact(z as u64);
                let (narrowed, narrow_flags) = self.f64_to_f32_rounded(z as f64, mode);
                self.set_register(x, Self::f64_to_u64(narrowed));
                self.raise_exceptions(flags | narrow_flags, op_byte, x, y, z, y_val, z_val)
            }
            Opcode::FMUL => {
                let y_val = self.get_register(y);
                let z_val = self.get_register(z);
                let a = Self::u64_to_f64(y_val);
                let b = Self::u64_to_f64(z_val);
                let r_near = a * b;
                // FMA gives the exact residual: a*b - r_near.
                let err = a.mul_add(b, -r_near);
                // A product of nonzero finite operands is never mathematically
                // zero, and the residual cannot witness that: for operands near
                // MIN_POSITIVE the exact product lies below the subnormal range,
                // so the FMA rounds the residual to zero on a real underflow.
                let (r, flags) = self.finalize_fp_binop(a, b, r_near, err, false);
                self.set_register(x, Self::f64_to_u64(r));
                self.raise_exceptions(flags, op_byte, x, y, z, y_val, z_val)
            }
            Opcode::FCMPE => {
                // FCMPE $X, $Y, $Z - the ε-relation: −1 (≺), 0 (∼, an
                // epsilon-close pair), or +1 (≻). Forces 0 and raises I on
                // an exceptional input; never both -1/+1 and I.
                let y_raw = self.get_register(y);
                let z_raw = self.get_register(z);
                let y_val = Self::u64_to_f64(y_raw);
                let z_val = Self::u64_to_f64(z_raw);
                let epsilon = Self::u64_to_f64(self.get_special(SpecialReg::RE));
                let exceptional = Self::epsilon_exceptional(y_val, z_val, epsilon);
                let result = if exceptional
                    || Self::in_epsilon_neighborhood(y_val, z_val, epsilon)
                    || Self::in_epsilon_neighborhood(z_val, y_val, epsilon)
                {
                    0
                } else if y_val < z_val {
                    (-1i64) as u64
                } else {
                    1
                };
                self.set_register(x, result);
                let flags = if exceptional { RA_I } else { 0 };
                self.raise_exceptions(flags, op_byte, x, y, z, y_raw, z_raw)
            }
            Opcode::FUNE => {
                // FUNE $X, $Y, $Z - reports only whether $Y, $Z, or rE is
                // exceptional (NaN operand, or rE NaN/negative); says
                // nothing about proximity, unlike FCMPE/FEQLE's ∼. Exempt
                // from the invalid exception FCMPE/FEQLE raise on that same
                // condition — raises no flag either way.
                let y_val = Self::u64_to_f64(self.get_register(y));
                let z_val = Self::u64_to_f64(self.get_register(z));
                let epsilon = Self::u64_to_f64(self.get_special(SpecialReg::RE));
                let result = if Self::epsilon_exceptional(y_val, z_val, epsilon) {
                    1
                } else {
                    0
                };
                self.set_register(x, result);
                self.advance_pc();
                true
            }
            Opcode::FEQLE => {
                // FEQLE $X, $Y, $Z - ≈: both directions of Nε membership
                // must hold, stronger than FCMPE's ∼.
                let y_raw = self.get_register(y);
                let z_raw = self.get_register(z);
                let y_val = Self::u64_to_f64(y_raw);
                let z_val = Self::u64_to_f64(z_raw);
                let epsilon = Self::u64_to_f64(self.get_special(SpecialReg::RE));
                let exceptional = Self::epsilon_exceptional(y_val, z_val, epsilon);
                let result = if exceptional {
                    0
                } else if Self::in_epsilon_neighborhood(y_val, z_val, epsilon)
                    && Self::in_epsilon_neighborhood(z_val, y_val, epsilon)
                {
                    1
                } else {
                    0
                };
                self.set_register(x, result);
                let flags = if exceptional { RA_I } else { 0 };
                self.raise_exceptions(flags, op_byte, x, y, z, y_raw, z_raw)
            }
            Opcode::FDIV => {
                let y_val = self.get_register(y);
                let z_val = self.get_register(z);
                let a = Self::u64_to_f64(y_val);
                let b = Self::u64_to_f64(z_val);
                let div_by_zero = if b == 0.0 && !a.is_nan() && a != 0.0 {
                    RA_Z
                } else {
                    0
                };
                let r_near = a / b;
                // residual = a - r_near*b is exact via FMA.
                // sign(true - r_near) = sign(residual) * sign(b).
                let residual = (-r_near).mul_add(b, a);
                let err = if b.is_sign_negative() {
                    -residual
                } else {
                    residual
                };
                // A quotient of nonzero finite operands is never mathematically
                // zero, so a zero result from such operands underflowed.
                let (r, flags) = self.finalize_fp_binop(a, b, r_near, err, false);
                self.set_register(x, Self::f64_to_u64(r));
                self.raise_exceptions(div_by_zero | flags, op_byte, x, y, z, y_val, z_val)
            }
            Opcode::FSQRT => {
                let mode = match self.resolved_round_mode(y) {
                    Ok(m) => m,
                    Err(()) => return self.illegal_round_mode("FSQRT", y),
                };
                // Y is the rounding-mode field, not a register operand.
                let y_val = y as u64;
                let z_val = self.get_register(z);
                let a = Self::u64_to_f64(z_val);
                let r_near = a.sqrt();
                // residual = a - r_near^2, exact via FMA.
                // sign(true - r_near) = sign(residual) when r_near >= 0 (always
                // true here since sqrt returns ≥0 or NaN).
                let err = (-r_near).mul_add(r_near, a);
                let (r, flags) = self.finalize_fp_unop(a, r_near, err, mode);
                self.set_register(x, Self::f64_to_u64(r));
                self.raise_exceptions(flags, op_byte, x, y, z, y_val, z_val)
            }
            Opcode::FREM => {
                // FREM $X, $Y, $Z - IEEE 754 floating remainder.
                let y_val = self.get_register(y);
                let z_val = self.get_register(z);
                let a = Self::u64_to_f64(y_val);
                let b = Self::u64_to_f64(z_val);
                let r = Self::ieee_remainder(a, b);
                let flags = Self::fp_arith_flags(a, b, r);
                self.set_register(x, Self::f64_to_u64(r));
                self.raise_exceptions(flags, op_byte, x, y, z, y_val, z_val)
            }
            Opcode::FINT => {
                // FINT $X, Y, $Z — Integerize under Y's rounding-mode
                // override, or rA's own mode when Y is 0.
                let mode = match self.resolved_round_mode(y) {
                    Ok(m) => m,
                    Err(()) => return self.illegal_round_mode("FINT", y),
                };
                // Y is the rounding-mode field, not a register operand.
                let y_val = y as u64;
                let z_val = self.get_register(z);
                let v = Self::u64_to_f64(z_val);
                let r = if v.is_finite() {
                    Self::round_with_mode(v, mode)
                } else {
                    v
                };
                let mut flags = 0u64;
                if v.is_nan() {
                    flags |= RA_I;
                } else if v.is_finite() && r != v {
                    flags |= RA_X;
                }
                self.set_register(x, Self::f64_to_u64(r));
                self.raise_exceptions(flags, op_byte, x, y, z, y_val, z_val)
            }

            // Load instructions
            Opcode::LDB => {
                // LDB $X, $Y, $Z - Load byte signed
                // s($X) <- s(M[$Y + $Z])
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let byte = self.read_byte(addr);
                let value = (byte as i8) as i64 as u64; // Sign extend
                self.set_register(x, value);
                self.advance_pc();
                true
            }
            Opcode::LDBI => {
                // LDB $X, $Y, Z - Load byte signed (immediate)
                let addr = self.get_register(y).wrapping_add(z as u64);
                let byte = self.read_byte(addr);
                let value = (byte as i8) as i64 as u64; // Sign extend
                self.set_register(x, value);
                self.advance_pc();
                true
            }
            Opcode::LDBU => {
                // LDBU $X, $Y, $Z - Load byte unsigned
                // u($X) <- M[$Y + $Z]
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let byte = self.read_byte(addr);
                self.set_register(x, byte as u64);
                self.advance_pc();
                true
            }
            Opcode::LDBUI => {
                // LDBU $X, $Y, Z - Load byte unsigned (immediate)
                let addr = self.get_register(y).wrapping_add(z as u64);
                let byte = self.read_byte(addr);
                self.set_register(x, byte as u64);
                self.advance_pc();
                true
            }
            Opcode::LDW => {
                // LDW $X, $Y, $Z - Load wyde signed
                // s($X) <- s(M2[$Y + $Z])
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let wyde = self.read_wyde(addr);
                let value = (wyde as i16) as i64 as u64; // Sign extend
                self.set_register(x, value);
                self.advance_pc();
                true
            }
            Opcode::LDWI => {
                // LDW $X, $Y, Z - Load wyde signed (immediate)
                let addr = self.get_register(y).wrapping_add(z as u64);
                let wyde = self.read_wyde(addr);
                let value = (wyde as i16) as i64 as u64; // Sign extend
                self.set_register(x, value);
                self.advance_pc();
                true
            }
            Opcode::LDWU => {
                // LDWU $X, $Y, $Z - Load wyde unsigned
                // u($X) <- M2[$Y + $Z]
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let wyde = self.read_wyde(addr);
                self.set_register(x, wyde as u64);
                self.advance_pc();
                true
            }
            Opcode::LDWUI => {
                // LDWU $X, $Y, Z - Load wyde unsigned (immediate)
                let addr = self.get_register(y).wrapping_add(z as u64);
                let wyde = self.read_wyde(addr);
                self.set_register(x, wyde as u64);
                self.advance_pc();
                true
            }
            Opcode::LDT => {
                // LDT $X, $Y, $Z - Load tetra signed
                // s($X) <- s(M4[$Y + $Z])
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let tetra = self.read_tetra(addr);
                let value = (tetra as i32) as i64 as u64; // Sign extend
                self.set_register(x, value);
                self.advance_pc();
                true
            }
            Opcode::LDTI => {
                // LDT $X, $Y, Z - Load tetra signed (immediate)
                let addr = self.get_register(y).wrapping_add(z as u64);
                let tetra = self.read_tetra(addr);
                let value = (tetra as i32) as i64 as u64; // Sign extend
                self.set_register(x, value);
                self.advance_pc();
                true
            }
            Opcode::LDTU => {
                // LDTU $X, $Y, $Z - Load tetra unsigned
                // u($X) <- M4[$Y + $Z]
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let tetra = self.read_tetra(addr);
                self.set_register(x, tetra as u64);
                self.advance_pc();
                true
            }
            Opcode::LDTUI => {
                // LDTU $X, $Y, Z - Load tetra unsigned (immediate)
                let addr = self.get_register(y).wrapping_add(z as u64);
                let tetra = self.read_tetra(addr);
                self.set_register(x, tetra as u64);
                self.advance_pc();
                true
            }
            Opcode::LDO => {
                // LDO $X, $Y, $Z - Load octa
                // u($X) <- M8[$Y + $Z]
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let octa = self.read_octa(addr);
                self.set_register(x, octa);
                self.advance_pc();
                true
            }
            Opcode::LDOI => {
                // LDO $X, $Y, Z - Load octa (immediate)
                let addr = self.get_register(y).wrapping_add(z as u64);
                let octa = self.read_octa(addr);
                self.set_register(x, octa);
                self.advance_pc();
                true
            }
            Opcode::LDOU => {
                // LDOU $X, $Y, $Z - Load octa unsigned (same as LDO)
                // u($X) <- M8[$Y + $Z]
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let octa = self.read_octa(addr);
                self.set_register(x, octa);
                self.advance_pc();
                true
            }
            Opcode::LDOUI => {
                // LDOU $X, $Y, Z - Load octa unsigned (immediate, same as LDO)
                let addr = self.get_register(y).wrapping_add(z as u64);
                let octa = self.read_octa(addr);
                self.set_register(x, octa);
                self.advance_pc();
                true
            }
            Opcode::LDSF => {
                // LDSF $X, $Y, $Z - Load short float (32-bit float to 64-bit)
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let tetra = self.read_tetra(addr);
                let short_float = f32::from_bits(tetra);
                let value = short_float as f64;
                self.set_register(x, Self::f64_to_u64(value));
                self.advance_pc();
                true
            }
            Opcode::LDSFI => {
                // LDSFI $X, $Y, Z - Load short float immediate (32-bit float to 64-bit)
                let addr = self.get_register(y).wrapping_add(z as u64);
                let tetra = self.read_tetra(addr);
                let short_float = f32::from_bits(tetra);
                let value = short_float as f64;
                self.set_register(x, Self::f64_to_u64(value));
                self.advance_pc();
                true
            }
            Opcode::ADDU => {
                // ADDU $X, $Y, $Z
                binop_rr!(self, x, y, z, u64::wrapping_add)
            }
            Opcode::ADDUI => {
                // ADDUI $X, $Y, Z
                binop_ri!(self, x, y, z, u64::wrapping_add)
            }
            // Opcodes 0xE0-0xE3. Each places YZ in its own wyde and zeroes the
            // other 48 bits; the INC/OR/ANDN families below preserve them.
            Opcode::SETH => {
                // SETH $X, YZ - u($X) <- YZ * 2^48
                let yz = ((y as u64) << 8) | (z as u64);
                self.set_register(x, yz << 48);
                self.advance_pc();
                true
            }
            Opcode::SETMH => {
                // SETMH $X, YZ - u($X) <- YZ * 2^32
                let yz = ((y as u64) << 8) | (z as u64);
                self.set_register(x, yz << 32);
                self.advance_pc();
                true
            }
            Opcode::SETML => {
                // SETML $X, YZ - u($X) <- YZ * 2^16
                let yz = ((y as u64) << 8) | (z as u64);
                self.set_register(x, yz << 16);
                self.advance_pc();
                true
            }
            Opcode::SETL => {
                // SETL $X, YZ - u($X) <- YZ
                let yz = ((y as u64) << 8) | (z as u64);
                self.set_register(x, yz);
                self.advance_pc();
                true
            }
            Opcode::INCH => {
                // INCH $X, YZ - Increase by high wyde
                let yz = ((y as u64) << 8) | (z as u64);
                let value = yz << 48;
                let current = self.get_register(x);
                self.set_register(x, current.wrapping_add(value));
                self.advance_pc();
                true
            }
            Opcode::INCMH => {
                // INCMH $X, YZ - Increase by medium high wyde
                let yz = ((y as u64) << 8) | (z as u64);
                let value = yz << 32;
                let current = self.get_register(x);
                self.set_register(x, current.wrapping_add(value));
                self.advance_pc();
                true
            }
            Opcode::INCML => {
                // INCML $X, YZ - Increase by medium low wyde
                let yz = ((y as u64) << 8) | (z as u64);
                let value = yz << 16;
                let current = self.get_register(x);
                self.set_register(x, current.wrapping_add(value));
                self.advance_pc();
                true
            }
            Opcode::INCL => {
                // INCL $X, YZ - Increase by low wyde
                let yz = ((y as u64) << 8) | (z as u64);
                let current = self.get_register(x);
                self.set_register(x, current.wrapping_add(yz));
                self.advance_pc();
                true
            }
            Opcode::ORH => {
                // ORH $X, YZ - OR with high wyde
                let yz = ((y as u64) << 8) | (z as u64);
                let value = yz << 48;
                let current = self.get_register(x);
                self.set_register(x, current | value);
                self.advance_pc();
                true
            }
            Opcode::ORMH => {
                // ORMH $X, YZ - OR with medium high wyde
                let yz = ((y as u64) << 8) | (z as u64);
                let value = yz << 32;
                let current = self.get_register(x);
                self.set_register(x, current | value);
                self.advance_pc();
                true
            }
            Opcode::ORML => {
                // ORML $X, YZ - OR with medium low wyde
                let yz = ((y as u64) << 8) | (z as u64);
                let value = yz << 16;
                let current = self.get_register(x);
                self.set_register(x, current | value);
                self.advance_pc();
                true
            }
            Opcode::ORL => {
                // ORL $X, YZ - OR with low wyde
                let yz = ((y as u64) << 8) | (z as u64);
                let current = self.get_register(x);
                self.set_register(x, current | yz);
                self.advance_pc();
                true
            }
            Opcode::ANDNH => {
                // ANDNH $X, YZ - AND-NOT with high wyde
                let yz = ((y as u64) << 8) | (z as u64);
                let mask = !(yz << 48);
                let current = self.get_register(x);
                self.set_register(x, current & mask);
                self.advance_pc();
                true
            }
            Opcode::ANDNMH => {
                // ANDNMH $X, YZ - AND-NOT with medium high wyde
                let yz = ((y as u64) << 8) | (z as u64);
                let mask = !(yz << 32);
                let current = self.get_register(x);
                self.set_register(x, current & mask);
                self.advance_pc();
                true
            }
            Opcode::ANDNML => {
                // ANDNML $X, YZ - AND-NOT with medium low wyde
                let yz = ((y as u64) << 8) | (z as u64);
                let mask = !(yz << 16);
                let current = self.get_register(x);
                self.set_register(x, current & mask);
                self.advance_pc();
                true
            }
            Opcode::ANDNL => {
                // ANDNL $X, YZ - AND-NOT with low wyde
                let yz = ((y as u64) << 8) | (z as u64);
                let mask = !yz;
                let current = self.get_register(x);
                self.set_register(x, current & mask);
                self.advance_pc();
                true
            }

            // Special Load/Store instructions (0x92-0x9F)
            Opcode::LDHT => {
                // LDHT $X, $Y, $Z - Load high tetra
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let tetra = self.read_tetra(addr);
                let value = (tetra as u64) << 32;
                self.set_register(x, value);
                self.advance_pc();
                true
            }
            Opcode::LDHTI => {
                // LDHTI $X, $Y, Z - Load high tetra immediate
                let addr = self.get_register(y).wrapping_add(z as u64);
                let tetra = self.read_tetra(addr);
                let value = (tetra as u64) << 32;
                self.set_register(x, value);
                self.advance_pc();
                true
            }
            Opcode::CSWAP => {
                // CSWAP $X, $Y, $Z - Compare and swap octabytes
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let mem_value = self.read_octa(addr);
                let compare_value = self.get_special(SpecialReg::RP);
                if mem_value == compare_value {
                    // Values match, perform swap
                    self.write_octa(addr, self.get_register(x));
                    self.set_register(x, 1); // Success
                } else {
                    // Values don't match: on failure rP <- M8[$Y+$Z], giving the
                    // caller the current value to retry with.
                    self.set_special(SpecialReg::RP, mem_value);
                    self.set_register(x, 0); // Failure
                }
                self.advance_pc();
                true
            }
            Opcode::CSWAPI => {
                // CSWAPI $X, $Y, Z - Compare and swap octabytes immediate
                let addr = self.get_register(y).wrapping_add(z as u64);
                let mem_value = self.read_octa(addr);
                let compare_value = self.get_special(SpecialReg::RP);
                if mem_value == compare_value {
                    // Values match, perform swap
                    self.write_octa(addr, self.get_register(x));
                    self.set_register(x, 1); // Success
                } else {
                    // Values don't match: on failure rP <- M8[$Y+Z], giving the
                    // caller the current value to retry with.
                    self.set_special(SpecialReg::RP, mem_value);
                    self.set_register(x, 0); // Failure
                }
                self.advance_pc();
                true
            }
            Opcode::LDUNC => {
                // LDUNC $X, $Y, $Z - Load uncached (treat as normal load)
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let value = self.read_octa(addr);
                self.set_register(x, value);
                self.advance_pc();
                true
            }
            Opcode::LDUNCI => {
                // LDUNCI $X, $Y, Z - Load uncached immediate
                let addr = self.get_register(y).wrapping_add(z as u64);
                let value = self.read_octa(addr);
                self.set_register(x, value);
                self.advance_pc();
                true
            }
            Opcode::LDVTS => {
                // LDVTS $X, $Y, $Z - Load virtual translation status (simplified)
                // In a full implementation, this would interact with virtual memory
                // For now, return 0 (no translation)
                self.set_register(x, 0);
                self.advance_pc();
                true
            }
            Opcode::LDVTSI => {
                // LDVTSI $X, $Y, Z - Load virtual translation status immediate
                self.set_register(x, 0);
                self.advance_pc();
                true
            }
            Opcode::PRELD => {
                // PRELD $X, $Y, $Z - Preload data (hint, no-op in simulation)
                self.advance_pc();
                true
            }
            Opcode::PRELDI => {
                // PRELDI $X, $Y, Z - Preload data immediate (hint, no-op)
                self.advance_pc();
                true
            }
            Opcode::PREGO => {
                // PREGO $X, $Y, $Z - Preload to go (hint, no-op in simulation)
                self.advance_pc();
                true
            }
            Opcode::PREGOI => {
                // PREGOI $X, $Y, Z - Preload to go immediate (hint, no-op)
                self.advance_pc();
                true
            }
            Opcode::GO => {
                // GO $X, $Y, $Z - Go to location
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                self.set_register(x, self.pc + 4); // Save return address
                self.pc = addr;
                true
            }
            Opcode::GOI => {
                // GOI $X, $Y, Z - Go to location immediate
                let addr = self.get_register(y).wrapping_add(z as u64);
                self.set_register(x, self.pc + 4); // Save return address
                self.pc = addr;
                true
            }

            // Store instructions
            Opcode::STB => {
                // STB $X, $Y, $Z - Store byte (with overflow check). A trip
                // sets rY to the address and rZ to the merged octabyte after
                // the store.
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let value = self.get_register(x);
                let flags = Self::store_overflow_flag(value, i8::MIN as i64, i8::MAX as i64);
                self.write_byte(addr, value as u8);
                let merged = self.merged_store_octa(addr);
                self.raise_exceptions(flags, op_byte, x, y, z, addr, merged)
            }
            Opcode::STBI => {
                // STB $X, $Y, Z - Store byte immediate (with overflow check)
                let addr = self.get_register(y).wrapping_add(z as u64);
                let value = self.get_register(x);
                let flags = Self::store_overflow_flag(value, i8::MIN as i64, i8::MAX as i64);
                self.write_byte(addr, value as u8);
                let merged = self.merged_store_octa(addr);
                self.raise_exceptions(flags, op_byte, x, y, z, addr, merged)
            }
            Opcode::STBU => {
                // STBU $X, $Y, $Z - Store byte unsigned (no overflow check)
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let value = self.get_register(x);
                self.write_byte(addr, value as u8);
                self.advance_pc();
                true
            }
            Opcode::STBUI => {
                // STBU $X, $Y, Z - Store byte unsigned immediate
                let addr = self.get_register(y).wrapping_add(z as u64);
                let value = self.get_register(x);
                self.write_byte(addr, value as u8);
                self.advance_pc();
                true
            }
            Opcode::STW => {
                // STW $X, $Y, $Z - Store wyde (with overflow check). A trip
                // sets rY to the address and rZ to the merged octabyte after
                // the store.
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let value = self.get_register(x);
                let flags = Self::store_overflow_flag(value, i16::MIN as i64, i16::MAX as i64);
                self.write_wyde(addr, value as u16);
                let merged = self.merged_store_octa(addr);
                self.raise_exceptions(flags, op_byte, x, y, z, addr, merged)
            }
            Opcode::STWI => {
                // STW $X, $Y, Z - Store wyde immediate (with overflow check)
                let addr = self.get_register(y).wrapping_add(z as u64);
                let value = self.get_register(x);
                let flags = Self::store_overflow_flag(value, i16::MIN as i64, i16::MAX as i64);
                self.write_wyde(addr, value as u16);
                let merged = self.merged_store_octa(addr);
                self.raise_exceptions(flags, op_byte, x, y, z, addr, merged)
            }
            Opcode::STWU => {
                // STWU $X, $Y, $Z - Store wyde unsigned (no overflow check)
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let value = self.get_register(x);
                self.write_wyde(addr, value as u16);
                self.advance_pc();
                true
            }
            Opcode::STWUI => {
                // STWU $X, $Y, Z - Store wyde unsigned immediate
                let addr = self.get_register(y).wrapping_add(z as u64);
                let value = self.get_register(x);
                self.write_wyde(addr, value as u16);
                self.advance_pc();
                true
            }
            Opcode::STT => {
                // STT $X, $Y, $Z - Store tetra (with overflow check). A trip
                // sets rY to the address and rZ to the merged octabyte after
                // the store.
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let value = self.get_register(x);
                let flags = Self::store_overflow_flag(value, i32::MIN as i64, i32::MAX as i64);
                self.write_tetra(addr, value as u32);
                let merged = self.merged_store_octa(addr);
                self.raise_exceptions(flags, op_byte, x, y, z, addr, merged)
            }
            Opcode::STTI => {
                // STT $X, $Y, Z - Store tetra immediate (with overflow check)
                let addr = self.get_register(y).wrapping_add(z as u64);
                let value = self.get_register(x);
                let flags = Self::store_overflow_flag(value, i32::MIN as i64, i32::MAX as i64);
                self.write_tetra(addr, value as u32);
                let merged = self.merged_store_octa(addr);
                self.raise_exceptions(flags, op_byte, x, y, z, addr, merged)
            }
            Opcode::STTU => {
                // STTU $X, $Y, $Z - Store tetra unsigned (no overflow check)
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let value = self.get_register(x);
                self.write_tetra(addr, value as u32);
                self.advance_pc();
                true
            }
            Opcode::STTUI => {
                // STTU $X, $Y, Z - Store tetra unsigned immediate
                let addr = self.get_register(y).wrapping_add(z as u64);
                let value = self.get_register(x);
                self.write_tetra(addr, value as u32);
                self.advance_pc();
                true
            }
            Opcode::STO => {
                // STO $X, $Y, $Z - Store octa
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let value = self.get_register(x);
                self.write_octa(addr, value);
                self.advance_pc();
                true
            }
            Opcode::STOI => {
                // STO $X, $Y, Z - Store octa immediate
                let addr = self.get_register(y).wrapping_add(z as u64);
                let value = self.get_register(x);
                self.write_octa(addr, value);
                self.advance_pc();
                true
            }
            Opcode::STOU => {
                // STOU $X, $Y, $Z - Store octa unsigned (same as STO)
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let value = self.get_register(x);
                self.write_octa(addr, value);
                self.advance_pc();
                true
            }
            Opcode::STOUI => {
                // STOU $X, $Y, Z - Store octa unsigned immediate
                let addr = self.get_register(y).wrapping_add(z as u64);
                let value = self.get_register(x);
                self.write_octa(addr, value);
                self.advance_pc();
                true
            }
            Opcode::STSF => {
                // STSF $X, $Y, $Z - Narrow $X to f32 using rA mode and store at $Y+$Z.
                // No Y-operand override: STSF takes no rounding-mode field.
                // A store trip, so a trip sets rY to the address and rZ to
                // the merged octabyte after the store, per §1 rule 3.
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let value = Self::u64_to_f64(self.get_register(x));
                let mode = (self.get_special(SpecialReg::RA) >> RA_ROUND_SHIFT) & 0x3;
                let (narrowed, flags) = self.f64_to_f32_rounded(value, mode);
                self.write_tetra(addr, (narrowed as f32).to_bits());
                let merged = self.merged_store_octa(addr);
                self.raise_exceptions(flags, op_byte, x, y, z, addr, merged)
            }
            Opcode::STSFI => {
                // A store trip: rY takes the address, rZ the merged octabyte
                // after the store, per §1 rule 3.
                let addr = self.get_register(y).wrapping_add(z as u64);
                let value = Self::u64_to_f64(self.get_register(x));
                let mode = (self.get_special(SpecialReg::RA) >> RA_ROUND_SHIFT) & 0x3;
                let (narrowed, flags) = self.f64_to_f32_rounded(value, mode);
                self.write_tetra(addr, (narrowed as f32).to_bits());
                let merged = self.merged_store_octa(addr);
                self.raise_exceptions(flags, op_byte, x, y, z, addr, merged)
            }
            Opcode::STHT => {
                // STHT $X, $Y, $Z - Store high tetra
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let value = self.get_register(x);
                let high_tetra = (value >> 32) as u32;
                self.write_tetra(addr, high_tetra);
                self.advance_pc();
                true
            }
            Opcode::STHTI => {
                // STHTI $X, $Y, Z - Store high tetra immediate
                let addr = self.get_register(y).wrapping_add(z as u64);
                let value = self.get_register(x);
                let high_tetra = (value >> 32) as u32;
                self.write_tetra(addr, high_tetra);
                self.advance_pc();
                true
            }
            Opcode::STCO => {
                // STCO X, $Y, $Z - Store constant octabyte (X is immediate value)
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                self.write_octa(addr, x as u64);
                self.advance_pc();
                true
            }
            Opcode::STCOI => {
                // STCOI X, $Y, Z - Store constant octabyte immediate
                let addr = self.get_register(y).wrapping_add(z as u64);
                self.write_octa(addr, x as u64);
                self.advance_pc();
                true
            }
            Opcode::STUNC => {
                // STUNC $X, $Y, $Z - Store uncached (treat as normal store)
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let value = self.get_register(x);
                self.write_octa(addr, value);
                self.advance_pc();
                true
            }
            Opcode::STUNCI => {
                // STUNCI $X, $Y, Z - Store uncached immediate
                let addr = self.get_register(y).wrapping_add(z as u64);
                let value = self.get_register(x);
                self.write_octa(addr, value);
                self.advance_pc();
                true
            }
            Opcode::SYNCD => {
                // SYNCD X, $Y, $Z - Synchronize data (no-op in simulation)
                self.advance_pc();
                true
            }
            Opcode::SYNCDI => {
                // SYNCDI X, $Y, Z - Synchronize data immediate (no-op)
                self.advance_pc();
                true
            }
            Opcode::PREST => {
                // PREST X, $Y, $Z - Prestore (hint, no-op in simulation)
                self.advance_pc();
                true
            }
            Opcode::PRESTI => {
                // PRESTI X, $Y, Z - Prestore immediate (hint, no-op)
                self.advance_pc();
                true
            }
            Opcode::SYNCID => {
                // SYNCID X, $Y, $Z - Synchronize instruction data (no-op in simulation)
                self.advance_pc();
                true
            }
            Opcode::SYNCIDI => {
                // SYNCIDI X, $Y, Z - Synchronize instruction data immediate (no-op)
                self.advance_pc();
                true
            }
            Opcode::PUSHGO => {
                // PUSHGO $X, $Y, $Z - Push registers and go (absolute target $Y+$Z)
                let target = self.get_register(y).wrapping_add(self.get_register(z));
                self.push_frame(x);
                self.set_pc(target);
                true
            }
            Opcode::PUSHGOI => {
                // PUSHGOI $X, $Y, Z - Push registers and go (absolute target $Y+Z)
                let target = self.get_register(y).wrapping_add(z as u64);
                self.push_frame(x);
                self.set_pc(target);
                true
            }
            // Arithmetic instructions - MUL/DIV opcodes 0x18-0x1F
            Opcode::MUL => {
                // MUL $X, $Y, $Z - Multiply signed with overflow
                mul_rr!(self, op_byte, x, y, z)
            }
            Opcode::MULI => {
                // MULI $X, $Y, Z - Multiply signed immediate with overflow
                mul_ri!(self, op_byte, x, y, z)
            }
            Opcode::MULU => {
                // MULU $X, $Y, $Z - Multiply unsigned
                mulu_rr!(self, x, y, z)
            }
            Opcode::MULUI => {
                // MULUI $X, $Y, Z - Multiply unsigned immediate
                mulu_ri!(self, x, y, z)
            }
            Opcode::DIV => {
                // DIV $X, $Y, $Z - Divide signed
                div_rr!(self, op_byte, x, y, z)
            }
            Opcode::DIVI => {
                // DIVI $X, $Y, Z - Divide signed immediate
                div_ri!(self, op_byte, x, y, z)
            }
            Opcode::DIVU => {
                // DIVU $X, $Y, $Z - Divide unsigned
                divu_rr!(self, x, y, z)
            }
            Opcode::DIVUI => {
                // DIVUI $X, $Y, Z - Divide unsigned immediate
                divu_ri!(self, x, y, z)
            }
            // ADD/SUB and variants - opcodes 0x20-0x2F
            Opcode::ADD => {
                // ADD $X, $Y, $Z - Add signed with overflow check
                add_rr!(self, op_byte, x, y, z)
            }
            Opcode::ADDI => {
                // ADDI $X, $Y, Z - Add signed immediate with overflow check
                add_ri!(self, op_byte, x, y, z)
            }
            // 0x22 and 0x23 are ADDU/ADDUI, already implemented above
            Opcode::SUB => {
                // SUB $X, $Y, $Z - Subtract signed with overflow check
                sub_rr!(self, op_byte, x, y, z)
            }
            Opcode::SUBI => {
                // SUBI $X, $Y, Z - Subtract signed immediate with overflow check
                sub_ri!(self, op_byte, x, y, z)
            }
            Opcode::SUBU => {
                // SUBU $X, $Y, $Z
                binop_rr!(self, x, y, z, u64::wrapping_sub)
            }
            Opcode::SUBUI => {
                // SUBUI $X, $Y, Z
                binop_ri!(self, x, y, z, u64::wrapping_sub)
            }
            Opcode::ADDU2 => {
                // 2ADDU $X, $Y, $Z
                muladd_rr!(self, x, y, z, 2)
            }
            Opcode::ADDU2I => {
                // 2ADDUI $X, $Y, Z
                muladd_ri!(self, x, y, z, 2)
            }
            Opcode::ADDU4 => {
                // 4ADDU $X, $Y, $Z
                muladd_rr!(self, x, y, z, 4)
            }
            Opcode::ADDU4I => {
                // 4ADDUI $X, $Y, Z
                muladd_ri!(self, x, y, z, 4)
            }
            Opcode::ADDU8 => {
                // 8ADDU $X, $Y, $Z
                muladd_rr!(self, x, y, z, 8)
            }
            Opcode::ADDU8I => {
                // 8ADDUI $X, $Y, Z
                muladd_ri!(self, x, y, z, 8)
            }
            Opcode::ADDU16 => {
                // 16ADDU $X, $Y, $Z
                muladd_rr!(self, x, y, z, 16)
            }
            Opcode::ADDU16I => {
                // 16ADDUI $X, $Y, Z
                muladd_ri!(self, x, y, z, 16)
            }
            // CMP instructions - opcodes 0x30-0x33
            Opcode::CMP => {
                // CMP $X, $Y, $Z
                cmp_rr!(self, x, y, z, |v| v as i64)
            }
            Opcode::CMPI => {
                // CMPI $X, $Y, Z
                cmp_ri!(self, x, y, z, |v| v as i64, |v| v as i64)
            }
            Opcode::CMPU => {
                // CMPU $X, $Y, $Z
                cmp_rr!(self, x, y, z, |v| v)
            }
            Opcode::CMPUI => {
                // CMPUI $X, $Y, Z
                cmp_ri!(self, x, y, z, |v| v, |v| v as u64)
            }
            Opcode::NEG => {
                // NEG $X, Y, $Z - Negate with overflow check
                // Y is immediate constant, $Z is register
                let y_val = y as u64;
                let z_val = self.get_register(z);
                let a = y as i64;
                let b = z_val as i64;
                let flags = match a.checked_sub(b) {
                    Some(result) => {
                        self.set_register(x, result as u64);
                        0
                    }
                    None => {
                        self.set_register(x, a.wrapping_sub(b) as u64);
                        RA_V
                    }
                };
                self.raise_exceptions(flags, op_byte, x, y, z, y_val, z_val)
            }
            Opcode::NEGI => {
                // NEG $X, Y, Z - Negate immediate with overflow check
                // Both Y and Z are immediate constants
                let y_val = y as u64;
                let z_val = z as u64;
                let a = y as i64;
                let b = z as i64;
                let flags = match a.checked_sub(b) {
                    Some(result) => {
                        self.set_register(x, result as u64);
                        0
                    }
                    // Y and Z are bytes, so a - b lies in [-255, 255] and this
                    // arm is unreachable from the immediate encoding. It mirrors
                    // NEG, where s($Z) can carry the difference out of range.
                    None => {
                        self.set_register(x, a.wrapping_sub(b) as u64);
                        RA_V
                    }
                };
                self.raise_exceptions(flags, op_byte, x, y, z, y_val, z_val)
            }
            Opcode::NEGU => {
                // NEGU $X, Y, $Z - Negate unsigned
                let a = y as u64;
                let b = self.get_register(z);
                self.set_register(x, a.wrapping_sub(b));
                self.advance_pc();
                true
            }
            Opcode::NEGUI => {
                // NEGU $X, Y, Z - Negate unsigned immediate
                let a = y as u64;
                let b = z as u64;
                self.set_register(x, a.wrapping_sub(b));
                self.advance_pc();
                true
            }
            // Shift instructions - opcodes 0x38-0x3F
            Opcode::SL => {
                // SL $X, $Y, $Z - Shift left with overflow check
                let y_val = self.get_register(y);
                let z_val = self.get_register(z);
                let val_y = y_val as i64;
                let shift = z_val;
                let flags = if shift >= 64 {
                    // Shift by 64 or more: result is 0, overflow unless Y was 0
                    self.set_register(x, 0);
                    if val_y != 0 { RA_V } else { 0 }
                } else {
                    let result = (val_y as u64) << shift;
                    self.set_register(x, result);
                    // Overflow exactly when s($Y)·2^u($Z) leaves the signed
                    // range, which is when shifting back does not restore $Y.
                    if ((result as i64) >> shift) != val_y {
                        RA_V
                    } else {
                        0
                    }
                };
                self.raise_exceptions(flags, op_byte, x, y, z, y_val, z_val)
            }
            Opcode::SLI => {
                // SLI $X, $Y, Z - Shift left immediate with overflow check
                let y_val = self.get_register(y);
                // Z is the literal shift amount, not a register.
                let z_val = z as u64;
                let val_y = y_val as i64;
                let shift = z as u64;
                let flags = if shift >= 64 {
                    self.set_register(x, 0);
                    if val_y != 0 { RA_V } else { 0 }
                } else {
                    let result = (val_y as u64) << shift;
                    self.set_register(x, result);
                    if ((result as i64) >> shift) != val_y {
                        RA_V
                    } else {
                        0
                    }
                };
                self.raise_exceptions(flags, op_byte, x, y, z, y_val, z_val)
            }
            Opcode::SLU => {
                // SLU $X, $Y, $Z - Shift left unsigned (no overflow check)
                let val_y = self.get_register(y);
                let shift = self.get_register(z);
                let result = if shift >= 64 { 0 } else { val_y << shift };
                self.set_register(x, result);
                self.advance_pc();
                true
            }
            Opcode::SLUI => {
                // SLUI $X, $Y, Z - Shift left unsigned immediate (no overflow check)
                let val_y = self.get_register(y);
                let shift = z as u64;
                let result = if shift >= 64 { 0 } else { val_y << shift };
                self.set_register(x, result);
                self.advance_pc();
                true
            }
            Opcode::SR => {
                // SR $X, $Y, $Z - Shift right (arithmetic)
                let val_y = self.get_register(y) as i64;
                let shift = self.get_register(z);
                let result = if shift >= 64 {
                    if val_y < 0 { -1i64 as u64 } else { 0 }
                } else {
                    (val_y >> shift) as u64
                };
                self.set_register(x, result);
                self.advance_pc();
                true
            }
            Opcode::SRI => {
                // SRI $X, $Y, Z - Shift right immediate (arithmetic)
                let val_y = self.get_register(y) as i64;
                let shift = z as u64;
                let result = if shift >= 64 {
                    if val_y < 0 { -1i64 as u64 } else { 0 }
                } else {
                    (val_y >> shift) as u64
                };
                self.set_register(x, result);
                self.advance_pc();
                true
            }
            Opcode::SRU => {
                // SRU $X, $Y, $Z - Shift right unsigned (logical)
                let val_y = self.get_register(y);
                let shift = self.get_register(z);
                let result = if shift >= 64 { 0 } else { val_y >> shift };
                self.set_register(x, result);
                self.advance_pc();
                true
            }
            Opcode::SRUI => {
                // SRUI $X, $Y, Z - Shift right unsigned immediate (logical)
                let val_y = self.get_register(y);
                let shift = z as u64;
                let result = if shift >= 64 { 0 } else { val_y >> shift };
                self.set_register(x, result);
                self.advance_pc();
                true
            }
            // Branch instructions - opcodes 0x40-0x5F
            Opcode::BN => {
                // BN $X, $Y, Z - Branch if negative
                let cond = (self.get_register(x) as i64) < 0;
                self.branch_forward(cond, y, z);
                true
            }
            Opcode::BNB => {
                // BNB $X, $Y, Z - Branch if negative (backward)
                let cond = (self.get_register(x) as i64) < 0;
                self.branch_backward(cond, y, z);
                true
            }
            Opcode::BZ => {
                // BZ $X, $Y, Z - Branch if zero
                let cond = self.get_register(x) == 0;
                self.branch_forward(cond, y, z);
                true
            }
            Opcode::BZB => {
                // BZB $X, $Y, Z - Branch if zero (backward)
                let cond = self.get_register(x) == 0;
                self.branch_backward(cond, y, z);
                true
            }
            Opcode::BP => {
                // BP $X, $Y, Z - Branch if positive
                let cond = (self.get_register(x) as i64) > 0;
                self.branch_forward(cond, y, z);
                true
            }
            Opcode::BPB => {
                // BPB $X, $Y, Z - Branch if positive (backward)
                let cond = (self.get_register(x) as i64) > 0;
                self.branch_backward(cond, y, z);
                true
            }
            Opcode::BOD => {
                // BOD $X, $Y, Z - Branch if odd
                let cond = (self.get_register(x) & 1) != 0;
                self.branch_forward(cond, y, z);
                true
            }
            Opcode::BODB => {
                // BODB $X, $Y, Z - Branch if odd (backward)
                let cond = (self.get_register(x) & 1) != 0;
                self.branch_backward(cond, y, z);
                true
            }
            Opcode::BNN => {
                // BNN $X, $Y, Z - Branch if non-negative
                let cond = (self.get_register(x) as i64) >= 0;
                self.branch_forward(cond, y, z);
                true
            }
            Opcode::BNNB => {
                // BNNB $X, $Y, Z - Branch if non-negative (backward)
                let cond = (self.get_register(x) as i64) >= 0;
                self.branch_backward(cond, y, z);
                true
            }
            Opcode::BNZ => {
                // BNZ $X, $Y, Z - Branch if non-zero
                let cond = self.get_register(x) != 0;
                self.branch_forward(cond, y, z);
                true
            }
            Opcode::BNZB => {
                // BNZB $X, $Y, Z - Branch if non-zero (backward)
                let cond = self.get_register(x) != 0;
                self.branch_backward(cond, y, z);
                true
            }
            Opcode::BNP => {
                // BNP $X, $Y, Z - Branch if non-positive
                let cond = (self.get_register(x) as i64) <= 0;
                self.branch_forward(cond, y, z);
                true
            }
            Opcode::BNPB => {
                // BNPB $X, $Y, Z - Branch if non-positive (backward)
                let cond = (self.get_register(x) as i64) <= 0;
                self.branch_backward(cond, y, z);
                true
            }
            Opcode::BEV => {
                // BEV $X, $Y, Z - Branch if even
                let cond = (self.get_register(x) & 1) == 0;
                self.branch_forward(cond, y, z);
                true
            }
            Opcode::BEVB => {
                // BEVB $X, $Y, Z - Branch if even (backward)
                let cond = (self.get_register(x) & 1) == 0;
                self.branch_backward(cond, y, z);
                true
            }
            Opcode::PBN => {
                // PBN $X, $Y, Z - Probable branch if negative
                let cond = (self.get_register(x) as i64) < 0;
                self.branch_forward(cond, y, z);
                true
            }
            Opcode::PBNB => {
                // PBNB $X, $Y, Z - Probable branch if negative (backward)
                let cond = (self.get_register(x) as i64) < 0;
                self.branch_backward(cond, y, z);
                true
            }
            Opcode::PBZ => {
                // PBZ $X, $Y, Z - Probable branch if zero
                let cond = self.get_register(x) == 0;
                self.branch_forward(cond, y, z);
                true
            }
            Opcode::PBZB => {
                // PBZB $X, $Y, Z - Probable branch if zero (backward)
                let cond = self.get_register(x) == 0;
                self.branch_backward(cond, y, z);
                true
            }
            Opcode::PBP => {
                // PBP $X, $Y, Z - Probable branch if positive
                let cond = (self.get_register(x) as i64) > 0;
                self.branch_forward(cond, y, z);
                true
            }
            Opcode::PBPB => {
                // PBPB $X, $Y, Z - Probable branch if positive (backward)
                let cond = (self.get_register(x) as i64) > 0;
                self.branch_backward(cond, y, z);
                true
            }
            Opcode::PBOD => {
                // PBOD $X, $Y, Z - Probable branch if odd
                let cond = (self.get_register(x) & 1) != 0;
                self.branch_forward(cond, y, z);
                true
            }
            Opcode::PBODB => {
                // PBODB $X, $Y, Z - Probable branch if odd (backward)
                let cond = (self.get_register(x) & 1) != 0;
                self.branch_backward(cond, y, z);
                true
            }
            Opcode::PBNN => {
                // PBNN $X, $Y, Z - Probable branch if non-negative
                let cond = (self.get_register(x) as i64) >= 0;
                self.branch_forward(cond, y, z);
                true
            }
            Opcode::PBNNB => {
                // PBNNB $X, $Y, Z - Probable branch if non-negative (backward)
                let cond = (self.get_register(x) as i64) >= 0;
                self.branch_backward(cond, y, z);
                true
            }
            Opcode::PBNZ => {
                // PBNZ $X, $Y, Z - Probable branch if non-zero
                let cond = self.get_register(x) != 0;
                self.branch_forward(cond, y, z);
                true
            }
            Opcode::PBNZB => {
                // PBNZB $X, $Y, Z - Probable branch if non-zero (backward)
                let cond = self.get_register(x) != 0;
                self.branch_backward(cond, y, z);
                true
            }
            Opcode::PBNP => {
                // PBNP $X, $Y, Z - Probable branch if non-positive
                let cond = (self.get_register(x) as i64) <= 0;
                self.branch_forward(cond, y, z);
                true
            }
            Opcode::PBNPB => {
                // PBNPB $X, $Y, Z - Probable branch if non-positive (backward)
                let cond = (self.get_register(x) as i64) <= 0;
                self.branch_backward(cond, y, z);
                true
            }
            Opcode::PBEV => {
                // PBEV $X, $Y, Z - Probable branch if even
                let cond = (self.get_register(x) & 1) == 0;
                self.branch_forward(cond, y, z);
                true
            }
            Opcode::PBEVB => {
                // PBEVB $X, $Y, Z - Probable branch if even (backward)
                let cond = (self.get_register(x) & 1) == 0;
                self.branch_backward(cond, y, z);
                true
            }
            // Conditional Set instructions - opcodes 0x60-0x6F
            Opcode::CSN => {
                // CSN $X, $Y, $Z - Conditional Set if Negative (checks $Y)
                let cond = (self.get_register(y) as i64) < 0;
                self.cond_set_rr(x, y, z, cond);
                true
            }
            Opcode::CSNI => {
                // CSNI $X, $Y, Z - Conditional Set if Negative (immediate, checks $Y)
                let cond = (self.get_register(y) as i64) < 0;
                self.cond_set_ri(x, y, z, cond);
                true
            }
            Opcode::CSZ => {
                // CSZ $X, $Y, $Z - Conditional Set if Zero (checks $Y)
                let cond = self.get_register(y) == 0;
                self.cond_set_rr(x, y, z, cond);
                true
            }
            Opcode::CSZI => {
                // CSZI $X, $Y, Z - Conditional Set if Zero (immediate, checks $Y)
                let cond = self.get_register(y) == 0;
                self.cond_set_ri(x, y, z, cond);
                true
            }
            Opcode::CSP => {
                // CSP $X, $Y, $Z - Conditional Set if Positive (checks $Y)
                let cond = (self.get_register(y) as i64) > 0;
                self.cond_set_rr(x, y, z, cond);
                true
            }
            Opcode::CSPI => {
                // CSPI $X, $Y, Z - Conditional Set if Positive (immediate, checks $Y)
                let cond = (self.get_register(y) as i64) > 0;
                self.cond_set_ri(x, y, z, cond);
                true
            }
            Opcode::CSOD => {
                // CSOD $X, $Y, $Z - Conditional Set if Odd (checks $Y)
                let cond = (self.get_register(y) & 1) != 0;
                self.cond_set_rr(x, y, z, cond);
                true
            }
            Opcode::CSODI => {
                // CSODI $X, $Y, Z - Conditional Set if Odd (immediate, checks $Y)
                let cond = (self.get_register(y) & 1) != 0;
                self.cond_set_ri(x, y, z, cond);
                true
            }
            Opcode::CSNN => {
                // CSNN $X, $Y, $Z - Conditional Set if Non-Negative (checks $Y)
                let cond = (self.get_register(y) as i64) >= 0;
                self.cond_set_rr(x, y, z, cond);
                true
            }
            Opcode::CSNNI => {
                // CSNNI $X, $Y, Z - Conditional Set if Non-Negative (immediate, checks $Y)
                let cond = (self.get_register(y) as i64) >= 0;
                self.cond_set_ri(x, y, z, cond);
                true
            }
            Opcode::CSNZ => {
                // CSNZ $X, $Y, $Z - Conditional Set if Non-Zero (checks $Y)
                let cond = self.get_register(y) != 0;
                self.cond_set_rr(x, y, z, cond);
                true
            }
            Opcode::CSNZI => {
                // CSNZI $X, $Y, Z - Conditional Set if Non-Zero (immediate, checks $Y)
                let cond = self.get_register(y) != 0;
                self.cond_set_ri(x, y, z, cond);
                true
            }
            Opcode::CSNP => {
                // CSNP $X, $Y, $Z - Conditional Set if Non-Positive (checks $Y)
                let cond = (self.get_register(y) as i64) <= 0;
                self.cond_set_rr(x, y, z, cond);
                true
            }
            Opcode::CSNPI => {
                // CSNPI $X, $Y, Z - Conditional Set if Non-Positive (immediate, checks $Y)
                let cond = (self.get_register(y) as i64) <= 0;
                self.cond_set_ri(x, y, z, cond);
                true
            }
            Opcode::CSEV => {
                // CSEV $X, $Y, $Z - Conditional Set if Even (checks $Y)
                let cond = (self.get_register(y) & 1) == 0;
                self.cond_set_rr(x, y, z, cond);
                true
            }
            Opcode::CSEVI => {
                // CSEVI $X, $Y, Z - Conditional Set if Even (immediate, checks $Y)
                let cond = (self.get_register(y) & 1) == 0;
                self.cond_set_ri(x, y, z, cond);
                true
            }

            // Zero or Set instructions (0x70-0x7F)
            Opcode::ZSN => {
                // ZSN $X, $Y, $Z - Zero or Set if Negative (checks $Y)
                let cond = (self.get_register(y) as i64) < 0;
                self.zero_set_rr(x, y, z, cond);
                true
            }
            Opcode::ZSNI => {
                // ZSNI $X, $Y, Z - Zero or Set if Negative (immediate, checks $Y)
                let cond = (self.get_register(y) as i64) < 0;
                self.zero_set_ri(x, y, z, cond);
                true
            }
            Opcode::ZSZ => {
                // ZSZ $X, $Y, $Z - Zero or Set if Zero (checks $Y)
                let cond = self.get_register(y) == 0;
                self.zero_set_rr(x, y, z, cond);
                true
            }
            Opcode::ZSZI => {
                // ZSZI $X, $Y, Z - Zero or Set if Zero (immediate, checks $Y)
                let cond = self.get_register(y) == 0;
                self.zero_set_ri(x, y, z, cond);
                true
            }
            Opcode::ZSP => {
                // ZSP $X, $Y, $Z - Zero or Set if Positive (checks $Y)
                let cond = (self.get_register(y) as i64) > 0;
                self.zero_set_rr(x, y, z, cond);
                true
            }
            Opcode::ZSPI => {
                // ZSPI $X, $Y, Z - Zero or Set if Positive (immediate, checks $Y)
                let cond = (self.get_register(y) as i64) > 0;
                self.zero_set_ri(x, y, z, cond);
                true
            }
            Opcode::ZSOD => {
                // ZSOD $X, $Y, $Z - Zero or Set if Odd (checks $Y)
                let cond = (self.get_register(y) & 1) != 0;
                self.zero_set_rr(x, y, z, cond);
                true
            }
            Opcode::ZSODI => {
                // ZSODI $X, $Y, Z - Zero or Set if Odd (immediate, checks $Y)
                let cond = (self.get_register(y) & 1) != 0;
                self.zero_set_ri(x, y, z, cond);
                true
            }
            Opcode::ZSNN => {
                // ZSNN $X, $Y, $Z - Zero or Set if Non-Negative (checks $Y)
                let cond = (self.get_register(y) as i64) >= 0;
                self.zero_set_rr(x, y, z, cond);
                true
            }
            Opcode::ZSNNI => {
                // ZSNNI $X, $Y, Z - Zero or Set if Non-Negative (immediate, checks $Y)
                let cond = (self.get_register(y) as i64) >= 0;
                self.zero_set_ri(x, y, z, cond);
                true
            }
            Opcode::ZSNZ => {
                // ZSNZ $X, $Y, $Z - Zero or Set if Non-Zero (checks $Y)
                let cond = self.get_register(y) != 0;
                self.zero_set_rr(x, y, z, cond);
                true
            }
            Opcode::ZSNZI => {
                // ZSNZI $X, $Y, Z - Zero or Set if Non-Zero (immediate, checks $Y)
                let cond = self.get_register(y) != 0;
                self.zero_set_ri(x, y, z, cond);
                true
            }
            Opcode::ZSNP => {
                // ZSNP $X, $Y, $Z - Zero or Set if Non-Positive (checks $Y)
                let cond = (self.get_register(y) as i64) <= 0;
                self.zero_set_rr(x, y, z, cond);
                true
            }
            Opcode::ZSNPI => {
                // ZSNPI $X, $Y, Z - Zero or Set if Non-Positive (immediate, checks $Y)
                let cond = (self.get_register(y) as i64) <= 0;
                self.zero_set_ri(x, y, z, cond);
                true
            }
            Opcode::ZSEV => {
                // ZSEV $X, $Y, $Z - Zero or Set if Even (checks $Y)
                let cond = (self.get_register(y) & 1) == 0;
                self.zero_set_rr(x, y, z, cond);
                true
            }
            Opcode::ZSEVI => {
                // ZSEVI $X, $Y, Z - Zero or Set if Even (immediate, checks $Y)
                let cond = (self.get_register(y) & 1) == 0;
                self.zero_set_ri(x, y, z, cond);
                true
            }

            // Bitwise operations - opcodes 0xC0-0xCF, 0xD8-0xD9
            Opcode::OR => {
                // OR $X, $Y, $Z
                binop_rr!(self, x, y, z, |a, b| a | b)
            }
            Opcode::ORI => {
                // ORI $X, $Y, Z
                binop_ri!(self, x, y, z, |a, b| a | b)
            }
            Opcode::ORN => {
                // ORN $X, $Y, $Z
                binop_rr!(self, x, y, z, |a: u64, b: u64| a | !b)
            }
            Opcode::ORNI => {
                // ORNI $X, $Y, Z
                binop_ri!(self, x, y, z, |a: u64, b: u64| a | !b)
            }
            Opcode::NOR => {
                // NOR $X, $Y, $Z
                binop_rr!(self, x, y, z, |a: u64, b: u64| !(a | b))
            }
            Opcode::NORI => {
                // NORI $X, $Y, Z
                binop_ri!(self, x, y, z, |a: u64, b: u64| !(a | b))
            }
            Opcode::XOR => {
                // XOR $X, $Y, $Z
                binop_rr!(self, x, y, z, |a, b| a ^ b)
            }
            Opcode::XORI => {
                // XORI $X, $Y, Z
                binop_ri!(self, x, y, z, |a, b| a ^ b)
            }
            Opcode::AND => {
                // AND $X, $Y, $Z
                binop_rr!(self, x, y, z, |a, b| a & b)
            }
            Opcode::ANDI => {
                // ANDI $X, $Y, Z
                binop_ri!(self, x, y, z, |a, b| a & b)
            }
            Opcode::ANDN => {
                // ANDN $X, $Y, $Z
                binop_rr!(self, x, y, z, |a: u64, b: u64| a & !b)
            }
            Opcode::ANDNI => {
                // ANDNI $X, $Y, Z
                binop_ri!(self, x, y, z, |a: u64, b: u64| a & !b)
            }
            Opcode::NAND => {
                // NAND $X, $Y, $Z
                binop_rr!(self, x, y, z, |a: u64, b: u64| !(a & b))
            }
            Opcode::NANDI => {
                // NANDI $X, $Y, Z
                binop_ri!(self, x, y, z, |a: u64, b: u64| !(a & b))
            }
            Opcode::NXOR => {
                // NXOR $X, $Y, $Z
                binop_rr!(self, x, y, z, |a: u64, b: u64| !(a ^ b))
            }
            Opcode::NXORI => {
                // NXORI $X, $Y, Z
                binop_ri!(self, x, y, z, |a: u64, b: u64| !(a ^ b))
            }
            // Bit fiddling operations - opcodes 0xD0-0xDF
            Opcode::BDIF => {
                // BDIF $X, $Y, $Z - Byte difference
                let val_y = self.get_register(y);
                let val_z = self.get_register(z);
                let mut result: u64 = 0;
                for i in 0..8 {
                    let byte_y = ((val_y >> (i * 8)) & 0xFF) as u8;
                    let byte_z = ((val_z >> (i * 8)) & 0xFF) as u8;
                    let diff = byte_y.saturating_sub(byte_z);
                    result |= (diff as u64) << (i * 8);
                }
                self.set_register(x, result);
                self.advance_pc();
                true
            }
            Opcode::BDIFI => {
                // BDIFI $X, $Y, Z - Byte difference immediate
                let val_y = self.get_register(y);
                let mut result: u64 = 0;
                for i in 0..8 {
                    let byte_y = ((val_y >> (i * 8)) & 0xFF) as u8;
                    let diff = byte_y.saturating_sub(z);
                    result |= (diff as u64) << (i * 8);
                }
                self.set_register(x, result);
                self.advance_pc();
                true
            }
            Opcode::WDIF => {
                // WDIF $X, $Y, $Z - Wyde difference
                let val_y = self.get_register(y);
                let val_z = self.get_register(z);
                let mut result: u64 = 0;
                for i in 0..4 {
                    let wyde_y = ((val_y >> (i * 16)) & 0xFFFF) as u16;
                    let wyde_z = ((val_z >> (i * 16)) & 0xFFFF) as u16;
                    let diff = wyde_y.saturating_sub(wyde_z);
                    result |= (diff as u64) << (i * 16);
                }
                self.set_register(x, result);
                self.advance_pc();
                true
            }
            Opcode::WDIFI => {
                // WDIFI $X, $Y, Z - Wyde difference immediate
                let val_y = self.get_register(y);
                let z_wyde = z as u16;
                let mut result: u64 = 0;
                for i in 0..4 {
                    let wyde_y = ((val_y >> (i * 16)) & 0xFFFF) as u16;
                    let diff = wyde_y.saturating_sub(z_wyde);
                    result |= (diff as u64) << (i * 16);
                }
                self.set_register(x, result);
                self.advance_pc();
                true
            }
            Opcode::TDIF => {
                // TDIF $X, $Y, $Z - Tetra difference
                let val_y = self.get_register(y);
                let val_z = self.get_register(z);
                let mut result: u64 = 0;
                for i in 0..2 {
                    let tetra_y = ((val_y >> (i * 32)) & 0xFFFFFFFF) as u32;
                    let tetra_z = ((val_z >> (i * 32)) & 0xFFFFFFFF) as u32;
                    let diff = tetra_y.saturating_sub(tetra_z);
                    result |= (diff as u64) << (i * 32);
                }
                self.set_register(x, result);
                self.advance_pc();
                true
            }
            Opcode::TDIFI => {
                // TDIFI $X, $Y, Z - Tetra difference immediate
                let val_y = self.get_register(y);
                let z_tetra = z as u32;
                let mut result: u64 = 0;
                for i in 0..2 {
                    let tetra_y = ((val_y >> (i * 32)) & 0xFFFFFFFF) as u32;
                    let diff = tetra_y.saturating_sub(z_tetra);
                    result |= (diff as u64) << (i * 32);
                }
                self.set_register(x, result);
                self.advance_pc();
                true
            }
            Opcode::ODIF => {
                // ODIF $X, $Y, $Z
                binop_rr!(self, x, y, z, u64::saturating_sub)
            }
            Opcode::ODIFI => {
                // ODIFI $X, $Y, Z
                binop_ri!(self, x, y, z, u64::saturating_sub)
            }
            Opcode::SADD => {
                // SADD $X, $Y, $Z
                binop_rr!(self, x, y, z, |a: u64, b: u64| (a & !b).count_ones() as u64)
            }
            Opcode::SADDI => {
                // SADDI $X, $Y, Z
                binop_ri!(self, x, y, z, |a: u64, b: u64| (a & !b).count_ones() as u64)
            }
            Opcode::MOR => {
                // MOR $X, $Y, $Z - Multiple or (Boolean matrix multiplication)
                let val_y = self.get_register(y);
                let val_z = self.get_register(z);
                let mut result: u64 = 0;
                for i in 0..8 {
                    for j in 0..8 {
                        let mut bit = false;
                        for k in 0..8 {
                            let y_bit = (val_y >> (k * 8 + j)) & 1;
                            let z_bit = (val_z >> (i * 8 + k)) & 1;
                            if y_bit != 0 && z_bit != 0 {
                                bit = true;
                                break;
                            }
                        }
                        if bit {
                            result |= 1 << (i * 8 + j);
                        }
                    }
                }
                self.set_register(x, result);
                self.advance_pc();
                true
            }
            Opcode::MORI => {
                // MORI $X, $Y, Z - Multiple or immediate
                let val_y = self.get_register(y);
                let mut result: u64 = 0;
                // For immediate form, only bottom byte of result is non-zero
                for j in 0..8 {
                    let mut bit = false;
                    for k in 0..8 {
                        let y_bit = (val_y >> (k * 8 + j)) & 1;
                        let z_bit = (z >> k) & 1;
                        if y_bit != 0 && z_bit != 0 {
                            bit = true;
                            break;
                        }
                    }
                    if bit {
                        result |= 1 << j;
                    }
                }
                self.set_register(x, result);
                self.advance_pc();
                true
            }
            Opcode::MXOR => {
                // MXOR $X, $Y, $Z - Multiple exclusive-or (matrix product over GF(2))
                let val_y = self.get_register(y);
                let val_z = self.get_register(z);
                let mut result: u64 = 0;
                for i in 0..8 {
                    for j in 0..8 {
                        let mut bit = false;
                        for k in 0..8 {
                            let y_bit = (val_y >> (k * 8 + j)) & 1;
                            let z_bit = (val_z >> (i * 8 + k)) & 1;
                            if y_bit != 0 && z_bit != 0 {
                                bit = !bit;
                            }
                        }
                        if bit {
                            result |= 1 << (i * 8 + j);
                        }
                    }
                }
                self.set_register(x, result);
                self.advance_pc();
                true
            }
            Opcode::MXORI => {
                // MXORI $X, $Y, Z - Multiple exclusive-or immediate
                let val_y = self.get_register(y);
                let mut result: u64 = 0;
                for j in 0..8 {
                    let mut bit = false;
                    for k in 0..8 {
                        let y_bit = (val_y >> (k * 8 + j)) & 1;
                        let z_bit = (z >> k) & 1;
                        if y_bit != 0 && z_bit != 0 {
                            bit = !bit;
                        }
                    }
                    if bit {
                        result |= 1 << j;
                    }
                }
                self.set_register(x, result);
                self.advance_pc();
                true
            }
            Opcode::MUX => {
                // MUX $X, $Y, $Z - Bitwise multiplex
                let val_y = self.get_register(y);
                let val_z = self.get_register(z);
                let mask = self.special_regs[SpecialReg::RM as usize];
                self.set_register(x, (val_y & mask) | (val_z & !mask));
                self.advance_pc();
                true
            }
            Opcode::MUXI => {
                // MUXI $X, $Y, Z - Bitwise multiplex immediate
                let val_y = self.get_register(y);
                let mask = self.special_regs[SpecialReg::RM as usize];
                self.set_register(x, (val_y & mask) | ((z as u64) & !mask));
                self.advance_pc();
                true
            }
            // Jump/Stack/System instructions - opcodes 0xF0-0xFF
            Opcode::JMP => {
                // JMP XYZ - Jump to PC + 4*XYZ.
                let xyz = ((x as u32) << 16) | ((y as u32) << 8) | (z as u32);
                self.pc = self.pc.wrapping_add((xyz as u64) * 4);
                true
            }
            Opcode::JMPB => {
                // JMPB XYZ - Jump to PC + 4*(XYZ - 2^24).
                let xyz = ((x as u32) << 16) | ((y as u32) << 8) | (z as u32);
                let offset = xyz as i64 - (1 << 24);
                self.pc = self.pc.wrapping_add((offset * 4) as u64);
                true
            }
            Opcode::PUSHJ => {
                // PUSHJ $X, YZ - Push registers and jump to PC + 4*YZ.
                let pc_at_inst = self.pc;
                self.push_frame(x);
                let yz = ((y as u16) << 8) | z as u16;
                self.pc = pc_at_inst.wrapping_add((yz as u64) * 4);
                true
            }
            Opcode::PUSHJB => {
                // PUSHJB $X, YZ - Push registers and jump to PC + 4*(YZ - 65536).
                let pc_at_inst = self.pc;
                self.push_frame(x);
                let yz = ((y as u16) << 8) | z as u16;
                let offset = yz as i64 - 65536;
                self.pc = pc_at_inst.wrapping_add((offset * 4) as u64);
                true
            }
            Opcode::GETA => {
                // GETA $X, YZ - Address PC + 4*YZ.
                let yz = ((y as u16) << 8) | z as u16;
                let addr = self.pc.wrapping_add((yz as u64) * 4);
                self.set_register(x, addr);
                self.advance_pc();
                true
            }
            Opcode::GETAB => {
                // GETAB $X, YZ - Address PC + 4*(YZ - 65536).
                let yz = ((y as u16) << 8) | z as u16;
                let offset = yz as i64 - 65536;
                let addr = self.pc.wrapping_add((offset * 4) as u64);
                self.set_register(x, addr);
                self.advance_pc();
                true
            }
            Opcode::PUT => {
                // PUT X, $Z - Put $Z into special register X.
                let value = self.get_register(z);
                if !self.put_special("PUT", x, value) {
                    return false;
                }
                self.advance_pc();
                true
            }
            Opcode::PUTI => {
                // PUT X, Z - Put immediate Z into special register X. Y is
                // ignored; the value is Z alone, eight bits.
                if !self.put_special("PUTI", x, z as u64) {
                    return false;
                }
                self.advance_pc();
                true
            }
            Opcode::POP => {
                // POP X, YZ - Pop frame and return.
                // Puts the callee's last output in the hole and the rest
                // above it in order, restores caller's locals below the
                // hole, and branches to rJ + 4·YZ. rJ itself is untouched;
                // a subroutine that calls another must save and restore it.
                let yz = ((y as u16) << 8) | z as u16;
                match self.pop_frame(x, yz) {
                    PopFrame::Frame(target) => {
                        self.pc = target;
                        true
                    }
                    PopFrame::NoFrame => {
                        // No frame to pop: branch via current rJ as a defensive fallback.
                        self.pc = self
                            .get_special(SpecialReg::RJ)
                            .wrapping_add((yz as u64) * 4);
                        true
                    }
                    PopFrame::Rejected => false,
                }
            }
            Opcode::RESUME => {
                // RESUME Z (gitraptrip.html "RESUME"; §1 rule 4): X and Y
                // are always zero, so only Z varies. Z != 0 is RESUME 1,
                // which restores from the kernel's rWW/rXX/rYY/rZZ and is
                // privileged. rX < 0 resumes at rW. Otherwise rX's top byte
                // is the ropcode: 0 inserts rX's instruction as if it stood
                // at rW-4 and resumes at rW; 1 substitutes rY/rZ into an
                // interruptible instruction; 2 serves forced-trap emulation;
                // 3 inserts a page-table entry. checksmix has no
                // interruptible instruction, no emulated opcode and no page
                // table, so ropcodes 1-3 never arise legitimately. Each
                // unsupported form halts through the shared MMix::reject
                // path: diagnostic, false, PC unmoved.
                if z != 0 {
                    return self.reject(&format!(
                        "RESUME {z}: privileged form (RESUME 1) at PC={:#018x}",
                        self.pc
                    ));
                }
                let rx = self.get_special(SpecialReg::RX);
                if (rx as i64) < 0 {
                    self.pc = self.get_special(SpecialReg::RW);
                    return true;
                }
                let ropcode = rx >> 56;
                if ropcode != 0 {
                    return self.reject(&format!(
                        "RESUME: ropcode {ropcode} at PC={:#018x} \
                         (only ropcode 0 and rX < 0 are supported)",
                        self.pc
                    ));
                }
                let word = rx as u32;
                let ins_op = (word >> 24) as u8;
                let ins_x = (word >> 16) as u8;
                let ins_y = (word >> 8) as u8;
                let ins_z = word as u8;
                let ins_opcode = match Opcode::try_from(ins_op) {
                    Ok(op) => op,
                    Err(_) => {
                        return self.reject(&format!(
                            "RESUME: invalid opcode {ins_op:#04x} in rX at PC={:#018x}",
                            self.pc
                        ));
                    }
                };
                self.pc = self.get_special(SpecialReg::RW).wrapping_sub(4);
                self.dispatch(ins_opcode, ins_op, ins_x, ins_y, ins_z)
            }
            Opcode::SAVE => {
                // SAVE $X,0 - push the machine's context onto the register
                // stack. See MMix::save_context.
                if !self.save_context(x) {
                    return false;
                }
                self.advance_pc();
                true
            }
            Opcode::UNSAVE => {
                // UNSAVE 0,$Z - restore the context $Z addresses. See
                // MMix::unsave_context.
                let packed_addr = self.get_register(z);
                if !self.unsave_context(packed_addr) {
                    return false;
                }
                self.advance_pc();
                true
            }
            Opcode::SYNC => {
                // SYNC XYZ - Synchronize
                // Memory synchronization barrier
                // For a simulator, this is typically a no-op
                self.advance_pc();
                true
            }
            Opcode::SWYM => {
                // SWYM XYZ - Sympathize with your machinery (no-op)
                self.advance_pc();
                true
            }
            Opcode::GET => {
                // GET $X, $Z - Get from special register
                let special_reg_num = z;
                if let Some(special_reg) = SpecialReg::from_u8(special_reg_num) {
                    let value = self.get_special(special_reg);
                    self.set_register(x, value);
                }
                self.advance_pc();
                true
            }
            Opcode::TRIP => {
                // TRIP X,Y,Z: user trip to the handler at #00 (trip.html;
                // §1 rule 3).
                let y_val = self.get_register(y);
                let z_val = self.get_register(z);
                self.trip(0x00, "TRIP", op_byte, x, y, z, y_val, z_val)
            }
        }
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

impl fmt::Display for MMix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_with_mode(f, ValueFormat::Signed)
    }
}

#[derive(Copy, Clone)]
pub enum ValueFormat {
    Signed,
    Unsigned,
}

pub struct MMixDisplay<'a> {
    mmix: &'a MMix,
    format: ValueFormat,
}

fn display_value(value: u64, format: ValueFormat) -> String {
    match format {
        ValueFormat::Signed => (value as i64).to_string(),
        ValueFormat::Unsigned => value.to_string(),
    }
}

impl<'a> fmt::Display for MMixDisplay<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.mmix.fmt_with_mode(f, self.format)
    }
}

impl MMix {
    pub fn display_with(&self, format: ValueFormat) -> MMixDisplay<'_> {
        MMixDisplay { mmix: self, format }
    }

    fn fmt_with_mode(&self, f: &mut fmt::Formatter<'_>, format: ValueFormat) -> fmt::Result {
        writeln!(f, "MMIX Computer State:")?;
        writeln!(f, "  PC = {:#018x}", self.pc)?;
        writeln!(f)?;

        // Display non-zero general registers
        writeln!(f, "General Registers:")?;
        let mut any_nonzero = false;
        for (i, &value) in self.general_regs.iter().enumerate() {
            if value != 0 {
                writeln!(
                    f,
                    "  ${:<3} = {:#018x} ({})",
                    i,
                    value,
                    display_value(value, format)
                )?;
                any_nonzero = true;
            }
        }
        if !any_nonzero {
            writeln!(f, "  (all zero)")?;
        }
        writeln!(f)?;

        // Display non-zero special registers
        writeln!(f, "Special Registers:")?;
        any_nonzero = false;
        for (i, &value) in self.special_regs.iter().enumerate() {
            if value != 0 {
                let name = match SpecialReg::from_u8(i as u8) {
                    Some(reg) => reg.name(),
                    // No register carries this number; still show the value.
                    None => "r??",
                };
                writeln!(
                    f,
                    "  {:<4} = {:#018x} ({})",
                    name,
                    value,
                    display_value(value, format)
                )?;
                any_nonzero = true;
            }
        }
        if !any_nonzero {
            writeln!(f, "  (all zero)")?;
        }
        writeln!(f)?;

        // Display memory usage
        writeln!(f, "Memory: {} bytes used", self.memory.len())?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
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

    #[test]
    fn test_mmix_new() {
        let mmix = MMix::new();
        assert_eq!(mmix.get_register(0), 0);
        assert_eq!(mmix.get_register(255), 0);
        assert_eq!(mmix.get_special(SpecialReg::RA), 0);
        assert_eq!(mmix.get_pc(), 0);
    }

    #[test]
    fn test_general_registers() {
        let mut mmix = MMix::new();
        mmix.set_register(1, 0x123456789ABCDEF0);
        assert_eq!(mmix.get_register(1), 0x123456789ABCDEF0);
        assert_eq!(mmix.get_register(2), 0);
    }

    #[test]
    fn test_special_registers() {
        let mut mmix = MMix::new();
        mmix.set_special(SpecialReg::RR, 42);
        assert_eq!(mmix.get_special(SpecialReg::RR), 42);
        assert_eq!(mmix.get_special(SpecialReg::RA), 0);
    }

    #[test]
    fn test_memory_byte() {
        let mut mmix = MMix::new();
        mmix.write_byte(0x1000, 0x42);
        assert_eq!(mmix.read_byte(0x1000), 0x42);
        assert_eq!(mmix.read_byte(0x1001), 0);
    }

    #[test]
    fn test_memory_wyde() {
        let mut mmix = MMix::new();
        mmix.write_wyde(0x1000, 0x1234);
        assert_eq!(mmix.read_wyde(0x1000), 0x1234);
        assert_eq!(mmix.read_byte(0x1000), 0x12);
        assert_eq!(mmix.read_byte(0x1001), 0x34);
    }

    #[test]
    fn test_memory_tetra() {
        let mut mmix = MMix::new();
        mmix.write_tetra(0x1000, 0x12345678);
        assert_eq!(mmix.read_tetra(0x1000), 0x12345678);
    }

    #[test]
    fn test_memory_octa() {
        let mut mmix = MMix::new();
        mmix.write_octa(0x1000, 0x123456789ABCDEF0);
        assert_eq!(mmix.read_octa(0x1000), 0x123456789ABCDEF0);
    }

    #[test]
    fn test_fetch_instruction() {
        let mut mmix = MMix::new();
        // Store instruction #20010203 (ADD $1, $2, $3)
        mmix.write_tetra(0, 0x20010203);
        let (op, x, y, z) = mmix.fetch_instruction();
        assert_eq!(op, 0x20);
        assert_eq!(x, 0x01);
        assert_eq!(y, 0x02);
        assert_eq!(z, 0x03);
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
    fn test_sparse_memory() {
        let mut mmix = MMix::new();
        mmix.write_byte(0x1000, 0x42);
        mmix.write_byte(0x1000, 0); // Writing zero should remove it
        assert_eq!(mmix.memory.len(), 0);
    }

    #[test]
    fn test_register_stack_initialization() {
        let mmix = MMix::new();
        assert_eq!(mmix.get_special(SpecialReg::RO), STACK_SEGMENT_START);
        assert_eq!(mmix.get_special(SpecialReg::RS), STACK_SEGMENT_START);
        assert_eq!(mmix.get_special(SpecialReg::RL), 0);
        assert_eq!(mmix.get_special(SpecialReg::RG), 32);
        assert_eq!(mmix.call_depth(), 0);
    }

    #[test]
    fn test_pushj_basic() {
        let mut mmix = MMix::new();
        mmix.set_pc(0x100);
        mmix.set_register(0, 100);
        mmix.set_register(1, 101);
        mmix.set_register(2, 102);
        mmix.set_special(SpecialReg::RL, 3);
        let ro = mmix.get_special(SpecialReg::RO);

        // PUSHJ $3, +1 pushes X+1 = 4 entries: $0..$2 and the hole marker.
        mmix.write_tetra(0x100, 0xF2030001);
        mmix.execute_instruction();

        // rO and rS both advance by X+1 = 4 octas.
        assert_eq!(mmix.get_special(SpecialReg::RO), ro + 32);
        assert_eq!(mmix.get_special(SpecialReg::RS), ro + 32);
        // new rL = max(0, rL_old - X - 1) = max(0, 3-3-1) = 0
        assert_eq!(mmix.get_special(SpecialReg::RL), 0);
        assert_eq!(mmix.get_special(SpecialReg::RJ), 0x104);
        assert_eq!(mmix.get_pc(), 0x104);

        // Saved frame in memory: $0, $1, $2, then the hole marker = X.
        assert_eq!(mmix.read_octa(ro), 100);
        assert_eq!(mmix.read_octa(ro + 8), 101);
        assert_eq!(mmix.read_octa(ro + 16), 102);
        assert_eq!(mmix.read_octa(ro + 24), 3); // hole marker = X

        // Live register file: caller's $0..$2 zeroed (no slide source above $X here).
        assert_eq!(mmix.get_register(0), 0);
        assert_eq!(mmix.get_register(1), 0);
        assert_eq!(mmix.get_register(2), 0);

        assert_eq!(mmix.call_depth(), 1);
    }

    /// The only test pinning part 1's destination rise through PUSHJ:
    /// writing to a marginal $X raises rL and zeroes $rL..$X before
    /// push_frame runs, so the spill holds zeros, not whatever the flat
    /// array held from an earlier, unrelated frame.
    #[test]
    fn test_pushj_spills_zero_not_stale_content_when_caller_rl_is_zero() {
        let mut mmix = MMix::new();
        mmix.set_pc(0x100);
        // Dirty the register file directly, then drop rL to 0 without
        // going through claim_local, so $0..$2 are marginal but the
        // physical array still holds the dirty values.
        mmix.set_register(0, 0xDEAD);
        mmix.set_register(1, 0xBEEF);
        mmix.set_register(2, 0xCAFE);
        mmix.set_special(SpecialReg::RL, 0);
        let ro = mmix.get_special(SpecialReg::RO);

        // PUSHJ $2, +1: X=2, caller's rL = 0.
        mmix.write_tetra(0x100, 0xF2020001);
        mmix.execute_instruction();

        // The destination rise zeroes $0..$2 before the spill runs, so
        // memory holds zeros, not the dirty values.
        assert_eq!(mmix.read_octa(ro), 0);
        assert_eq!(mmix.read_octa(ro + 8), 0);
        assert_eq!(mmix.read_octa(ro + 16), 2); // hole marker = X
        // The entries are fixed by X, not by the raised rL: the new rL
        // settles back at 0 either way.
        assert_eq!(mmix.get_special(SpecialReg::RL), 0);
    }

    /// `POP` reads its hole from `M8[rO-8]` at the moment it runs, not from
    /// any count `PUSHJ` cached. Rewriting that word between the two changes
    /// how far `POP` retracts: no implementation that ignores memory can
    /// pass this.
    #[test]
    fn test_pop_reads_the_hole_from_memory_even_when_rewritten() {
        let mut mmix = MMix::new();
        mmix.set_pc(0x100);
        mmix.set_register(0, 100);
        mmix.set_register(1, 101);
        mmix.set_register(2, 102);
        mmix.set_special(SpecialReg::RL, 3);
        let ro = mmix.get_special(SpecialReg::RO);

        // PUSHJ $3, +1: hole marker 3 lands at ro+24; rO advances to ro+32.
        mmix.write_tetra(0x100, 0xF2030001);
        mmix.execute_instruction();
        assert_eq!(mmix.get_special(SpecialReg::RO), ro + 32);

        // Overwrite the hole in memory: POP must read this value, 1, not
        // the pushed x = 3.
        mmix.write_octa(ro + 24, 1);

        // POP 0, 0
        mmix.write_tetra(0x104, 0xF8000000);
        mmix.execute_instruction();

        // Reading x = 3 (the pushed value) would retract by 4 octas,
        // landing back at ro. Reading the rewritten hole (1) retracts by
        // only 2, landing short of it -- proof POP read memory, not a
        // cached count.
        assert_eq!(mmix.get_special(SpecialReg::RO), ro + 16);
        assert_eq!(mmix.get_special(SpecialReg::RS), ro + 16);
    }

    #[test]
    fn test_pushj_window_slide_argument_passing() {
        // Stage an arg at $5; PUSHJ $4 should make it visible to the callee at $0.
        let mut mmix = MMix::new();
        mmix.set_pc(0x100);
        mmix.set_register(5, 0xAA);
        mmix.set_special(SpecialReg::RL, 8); // 8 active locals

        // PUSHJ $4, +1
        mmix.write_tetra(0x100, 0xF2040001);
        mmix.execute_instruction();

        // Callee sees caller's $5 as $0 (slid down by X+1 = 5).
        assert_eq!(mmix.get_register(0), 0xAA);
        // rL = 8 - 5 = 3
        assert_eq!(mmix.get_special(SpecialReg::RL), 3);

        // POP 0,0 — no output reaches $5, so it is marginal and reads zero.
        mmix.write_tetra(0x104, 0xF8000000);
        mmix.execute_instruction();
        assert_eq!(mmix.get_special(SpecialReg::RL), 4);
        assert_eq!(mmix.get_register(5), 0);
        // $4 was the marginal hole — it's consumed by PUSHJ and reads as zero
        // after POP 0 since no return value lands there.
        assert_eq!(mmix.get_register(4), 0);
    }

    #[test]
    fn test_pushj_window_slide_return_value() {
        // POP 1 lands the single return value at the caller's hole position $X.
        let mut mmix = MMix::new();
        mmix.set_pc(0x100);
        mmix.set_special(SpecialReg::RL, 5);

        // PUSHJ $4, +1
        mmix.write_tetra(0x100, 0xF2040001);
        mmix.execute_instruction();

        // Callee writes 0xBB into $0 as the return value.
        mmix.set_register(0, 0xBB);

        // POP 1, 0
        mmix.write_tetra(0x104, 0xF8010000);
        mmix.execute_instruction();

        // Return value lands at caller's $4 (the hole).
        assert_eq!(mmix.get_register(4), 0xBB);
        // rL = min(x+n, rG) = min(4+1, 32) = 5
        assert_eq!(mmix.get_special(SpecialReg::RL), 5);
    }

    #[test]
    fn test_pushj_zeros_freshly_allocated_locals() {
        // After PUSHJ, callee locals beyond the slide window must read as zero.
        let mut mmix = MMix::new();
        mmix.set_pc(0x100);
        for i in 0..8u8 {
            mmix.set_register(i, 1000 + i as u64);
        }
        mmix.set_special(SpecialReg::RL, 8);

        // PUSHJ $4, +1
        mmix.write_tetra(0x100, 0xF2040001);
        mmix.execute_instruction();

        // Slid-down values: caller $5..$7 → callee $0..$2.
        assert_eq!(mmix.get_register(0), 1005);
        assert_eq!(mmix.get_register(1), 1006);
        assert_eq!(mmix.get_register(2), 1007);
        // Vacated tail must be zero.
        assert_eq!(mmix.get_register(3), 0);
        assert_eq!(mmix.get_register(4), 0);
        assert_eq!(mmix.get_register(5), 0);
        assert_eq!(mmix.get_register(7), 0);
    }

    #[test]
    fn test_pop_with_return_value_shift() {
        // PUSHJ $3 + POP 2: the last output lands in the hole.
        let mut mmix = MMix::new();
        mmix.set_pc(0x100);
        mmix.set_special(SpecialReg::RL, 4);

        // PUSHJ $3, +1
        mmix.write_tetra(0x100, 0xF2030001);
        mmix.execute_instruction();

        mmix.set_register(0, 0x111);
        mmix.set_register(1, 0x222);

        // POP 2, 0
        mmix.write_tetra(0x104, 0xF8020000);
        mmix.execute_instruction();

        // x=3, n=2: the hole $3 gets the last output (0x222); $4 gets 0x111.
        assert_eq!(mmix.get_register(3), 0x222);
        assert_eq!(mmix.get_register(4), 0x111);
        // rL = min(x+n, rG) = min(3+2, 32) = 5
        assert_eq!(mmix.get_special(SpecialReg::RL), 5);
        // Caller's $0..$2 are restored from memory (originally zero).
        assert_eq!(mmix.get_register(0), 0);
        assert_eq!(mmix.get_register(1), 0);
        assert_eq!(mmix.get_register(2), 0);
    }

    #[test]
    fn test_pushgo_pop_basic() {
        let mut mmix = MMix::new();
        mmix.set_pc(0x100);
        mmix.set_register(0, 10);
        mmix.set_register(1, 11);
        // Target address: $3 + $4 = 0x200
        mmix.set_register(3, 0x200);
        mmix.set_register(4, 0);
        mmix.set_special(SpecialReg::RL, 2);
        let ro = mmix.get_special(SpecialReg::RO);

        // PUSHGO $2, $3, $4
        mmix.write_tetra(0x100, 0xBE020304);
        mmix.execute_instruction();

        // X=2: rO and rS both advance by X+1 = 3 octas.
        assert_eq!(mmix.get_special(SpecialReg::RO), ro + 24);
        assert_eq!(mmix.get_special(SpecialReg::RS), ro + 24);
        // new rL = saturating_sub(2, 3) = 0
        assert_eq!(mmix.get_special(SpecialReg::RL), 0);
        assert_eq!(mmix.get_special(SpecialReg::RJ), 0x104);
        assert_eq!(mmix.get_pc(), 0x200);

        // Saved $0, $1, then the hole marker = X.
        assert_eq!(mmix.read_octa(ro), 10);
        assert_eq!(mmix.read_octa(ro + 8), 11);
        assert_eq!(mmix.read_octa(ro + 16), 2);
        assert_eq!(mmix.call_depth(), 1);

        // POP 0,0 at target
        mmix.write_tetra(0x200, 0xF8000000);
        mmix.execute_instruction();

        // Caller's $0,$1 restored from memory.
        assert_eq!(mmix.get_register(0), 10);
        assert_eq!(mmix.get_register(1), 11);
        assert_eq!(mmix.get_special(SpecialReg::RO), ro);
        assert_eq!(mmix.get_special(SpecialReg::RS), ro);
        assert_eq!(mmix.get_special(SpecialReg::RL), 2);
        assert_eq!(mmix.get_pc(), 0x104);
        assert_eq!(mmix.call_depth(), 0);
    }

    #[test]
    fn test_pop_basic() {
        // PUSHJ $3 + POP 0: caller's $0..$2 fully restored, rL back to 3.
        let mut mmix = MMix::new();
        mmix.set_pc(0x100);
        mmix.set_register(0, 100);
        mmix.set_register(1, 101);
        mmix.set_register(2, 102);
        mmix.set_special(SpecialReg::RL, 3);
        let ro = mmix.get_special(SpecialReg::RO);

        // PUSHJ $3, +1
        mmix.write_tetra(0x100, 0xF2030001);
        mmix.execute_instruction();

        // Callee scribbles over its locals.
        mmix.set_register(0, 200);
        mmix.set_register(1, 201);
        mmix.set_register(2, 202);

        // POP 0, 0
        mmix.write_tetra(0x104, 0xF8000000);
        mmix.execute_instruction();

        assert_eq!(mmix.get_register(0), 100);
        assert_eq!(mmix.get_register(1), 101);
        assert_eq!(mmix.get_register(2), 102);
        assert_eq!(mmix.get_special(SpecialReg::RO), ro);
        assert_eq!(mmix.get_special(SpecialReg::RS), ro);
        assert_eq!(mmix.get_special(SpecialReg::RL), 3);
        assert_eq!(mmix.get_pc(), 0x104);
        assert_eq!(mmix.call_depth(), 0);
    }

    #[test]
    fn test_pop_with_return_values() {
        // PUSHJ $4 + POP 3: callee's $0..$2 land at caller's $4..$6.
        let mut mmix = MMix::new();
        mmix.set_pc(0x100);
        for i in 0..8u8 {
            mmix.set_register(i, 100 + i as u64);
        }
        mmix.set_special(SpecialReg::RL, 8);

        // PUSHJ $4, +1
        mmix.write_tetra(0x100, 0xF2040001);
        mmix.execute_instruction();

        // Callee sets 3 return values.
        mmix.set_register(0, 300);
        mmix.set_register(1, 301);
        mmix.set_register(2, 302);

        // POP 3, 0
        mmix.write_tetra(0x104, 0xF8030000);
        mmix.execute_instruction();

        // The hole $4 gets the last output (302); $5, $6 get 300, 301 in order.
        assert_eq!(mmix.get_register(4), 302);
        assert_eq!(mmix.get_register(5), 300);
        assert_eq!(mmix.get_register(6), 301);
        // Caller's $0..$3 restored from memory.
        assert_eq!(mmix.get_register(0), 100);
        assert_eq!(mmix.get_register(1), 101);
        assert_eq!(mmix.get_register(2), 102);
        assert_eq!(mmix.get_register(3), 103);
        // rL = min(x+n, rG) = min(4+3, 32) = 7
        assert_eq!(mmix.get_special(SpecialReg::RL), 7);
    }

    #[test]
    fn test_pop_no_return_values() {
        // PUSHJ $5 + POP 0: caller's $0..$4 are restored; $5..$9 are
        // marginal after POP and read zero.
        let mut mmix = MMix::new();
        mmix.set_pc(0x100);
        for i in 0..10u8 {
            mmix.set_register(i, 100 + i as u64);
        }
        mmix.set_special(SpecialReg::RL, 10);

        // PUSHJ $5, +1
        mmix.write_tetra(0x100, 0xF2050001);
        mmix.execute_instruction();

        // Callee scribbles over its locals.
        mmix.set_register(0, 999);
        mmix.set_register(1, 888);
        mmix.set_register(2, 777);

        // POP 0, 0
        mmix.write_tetra(0x104, 0xF8000000);
        mmix.execute_instruction();

        // Caller's $0..$4 restored from memory.
        for i in 0..5u8 {
            assert_eq!(mmix.get_register(i), 100 + i as u64);
        }
        // $5 was the marginal hole — it's consumed by PUSHJ and reads as zero
        // after POP 0 since no return value lands there.
        assert_eq!(mmix.get_register(5), 0);
        // Slots above the hole are marginal after POP — they read zero, not
        // their stale pre-call values.
        for i in 6..10u8 {
            assert_eq!(mmix.get_register(i), 0);
        }
        assert_eq!(mmix.get_special(SpecialReg::RL), 5);
    }

    #[test]
    fn test_pop_mmixware_program1_matches_measured_values() {
        // Measured on MMIXware and checksmix at 91d207f (see MMIX.md's
        // register stack table). Caller's $1..$5 read zero after POP, and
        // the callee's sole return value lands at the hole $0.
        let mut mmix = MMix::new();
        mmix.set_pc(0x100);
        mmix.set_register(1, 111);
        mmix.set_register(2, 222);
        mmix.set_register(3, 333);
        mmix.set_register(4, 444);
        mmix.set_register(5, 555);

        // PUSHJ $0, +1
        mmix.write_tetra(0x100, 0xF2000001);
        mmix.execute_instruction();

        // Callee sets its return values.
        mmix.set_register(0, 999);
        mmix.set_register(1, 777);

        // POP 1, 0
        mmix.write_tetra(0x104, 0xF8010000);
        mmix.execute_instruction();

        assert_eq!(mmix.get_register(0), 999);
        for i in 1..=5u8 {
            assert_eq!(mmix.get_register(i), 0);
        }
        assert_eq!(mmix.get_special(SpecialReg::RL), 1);
    }

    #[test]
    fn test_pop_mmixware_program2_matches_measured_values() {
        // Measured on MMIXware and checksmix at 91d207f. POP 2 puts the
        // callee's last output ($1) in the hole and the first ($0) above
        // it, and registers above the outputs read zero rather than their
        // pre-call values.
        let mut mmix = MMix::new();
        mmix.set_pc(0x100);
        mmix.set_register(0, 10);
        mmix.set_register(1, 20);
        mmix.set_register(2, 30);
        mmix.set_register(3, 40);
        mmix.set_register(4, 50);
        mmix.set_register(5, 60);
        mmix.set_register(6, 70);

        // PUSHJ $3, +1
        mmix.write_tetra(0x100, 0xF2030001);
        mmix.execute_instruction();

        // Callee sets its three return values.
        mmix.set_register(0, 801);
        mmix.set_register(1, 802);
        mmix.set_register(2, 803);

        // POP 2, 0
        mmix.write_tetra(0x104, 0xF8020000);
        mmix.execute_instruction();

        assert_eq!(mmix.get_register(0), 10);
        assert_eq!(mmix.get_register(1), 20);
        assert_eq!(mmix.get_register(2), 30);
        assert_eq!(mmix.get_register(3), 802);
        assert_eq!(mmix.get_register(4), 801);
        assert_eq!(mmix.get_register(5), 0);
        assert_eq!(mmix.get_register(6), 0);
        assert_eq!(mmix.get_special(SpecialReg::RL), 5);
    }

    #[test]
    fn test_pop_0_0_leaves_hole_and_everything_above_it_zero() {
        // POP 0,0 leaves the hole marginal: rL becomes x, and every
        // register from there through rG-1 reads zero.
        let mut mmix = MMix::new();
        mmix.set_pc(0x100);
        mmix.set_register(0, 10);
        mmix.set_register(1, 20);
        mmix.set_register(2, 30);
        mmix.set_register(3, 40);
        mmix.set_special(SpecialReg::RL, 4);

        // PUSHJ $2, +1
        mmix.write_tetra(0x100, 0xF2020001);
        mmix.execute_instruction();

        // Callee scribbles over its one local.
        mmix.set_register(0, 999);

        // POP 0, 0
        mmix.write_tetra(0x104, 0xF8000000);
        mmix.execute_instruction();

        // Caller's $0, $1 restored; $2 (the hole) and everything above it,
        // through rG-1, is marginal and reads zero.
        assert_eq!(mmix.get_register(0), 10);
        assert_eq!(mmix.get_register(1), 20);
        for i in 2..32u8 {
            assert_eq!(mmix.get_register(i), 0);
        }
        assert_eq!(mmix.get_special(SpecialReg::RL), 2);
    }

    #[test]
    fn test_pop_x_greater_than_l_clamps_and_zeros_the_hole() {
        // If X > L, X becomes L+1 and the hole gets zero regardless of what
        // the callee left there.
        let mut mmix = MMix::new();
        mmix.set_pc(0x100);
        mmix.set_register(0, 100);
        mmix.set_register(2, 200);
        mmix.set_special(SpecialReg::RL, 3);

        // PUSHJ $1, +1 — callee's rL is 1 (only $0 is a valid local).
        mmix.write_tetra(0x100, 0xF2010001);
        mmix.execute_instruction();
        assert_eq!(mmix.get_special(SpecialReg::RL), 1);

        // Callee sets its one local as a return value.
        mmix.set_register(0, 555);

        // POP 3, 0 — X (3) exceeds L (1), so X clamps to L+1 = 2.
        mmix.write_tetra(0x104, 0xF8030000);
        mmix.execute_instruction();

        assert_eq!(mmix.get_register(0), 100);
        // The hole reads zero, not the callee's $0.
        assert_eq!(mmix.get_register(1), 0);
        // The clamp still delivers the callee's one real output, one slot up.
        assert_eq!(mmix.get_register(2), 555);
        assert_eq!(mmix.get_special(SpecialReg::RL), 3);
    }

    #[test]
    fn test_pushj_x_at_or_above_rg_saves_all_locals_and_pops_at_the_hole() {
        // PUSHJ $X with X >= rG pushes $0..$(rL-1), the callee starts at
        // rL = 0, and the hole for POP is the caller's rL, not X.
        let mut mmix = MMix::new();
        mmix.set_pc(0x100);
        mmix.set_special(SpecialReg::RG, 10);
        mmix.set_register(0, 1000);
        mmix.set_register(1, 1001);
        mmix.set_register(2, 1002);
        mmix.set_register(3, 1003);
        mmix.set_register(4, 1004);
        mmix.set_special(SpecialReg::RL, 5);
        mmix.set_register(50, 0xBEEF); // a global, above rG

        let rs_before = mmix.get_special(SpecialReg::RS);

        // PUSHJ $255, +1
        mmix.write_tetra(0x100, 0xF2FF0001);
        mmix.execute_instruction();
        assert_eq!(mmix.get_special(SpecialReg::RL), 0);

        // Callee computes its result in its one local.
        mmix.set_register(0, 777);

        // POP 1, 0
        mmix.write_tetra(0x104, 0xF8010000);
        mmix.execute_instruction();

        // Caller's $0..$4 restored, the output lands at $rL (the hole), and
        // rL becomes min(rL+1, rG).
        assert_eq!(mmix.get_register(0), 1000);
        assert_eq!(mmix.get_register(1), 1001);
        assert_eq!(mmix.get_register(2), 1002);
        assert_eq!(mmix.get_register(3), 1003);
        assert_eq!(mmix.get_register(4), 1004);
        assert_eq!(mmix.get_register(5), 777);
        assert_eq!(mmix.get_special(SpecialReg::RL), 6);
        // Globals are untouched throughout.
        assert_eq!(mmix.get_register(50), 0xBEEF);
        // POP retracts rO and rS to exactly the address PUSHJ found them
        // at: a spill sized by rL, not the hole read back from memory,
        // would retract by the wrong amount and land somewhere else.
        assert_eq!(mmix.get_special(SpecialReg::RS), rs_before);
    }

    #[test]
    fn test_pushgoi_pop_basic() {
        let mut mmix = MMix::new();
        mmix.set_pc(0x300);
        mmix.set_register(0, 42);
        mmix.set_register(1, 43);
        mmix.set_register(5, 0x310);
        mmix.set_special(SpecialReg::RL, 2);
        let ro = mmix.get_special(SpecialReg::RO);

        // PUSHGOI $2, $5, #0x10  → target = $5 + 0x10 = 0x320
        mmix.write_tetra(0x300, 0xBF020510);
        mmix.execute_instruction();

        // X=2: rO and rS both advance by X+1 = 3 octas.
        assert_eq!(mmix.get_special(SpecialReg::RO), ro + 24);
        assert_eq!(mmix.get_special(SpecialReg::RS), ro + 24);
        assert_eq!(mmix.get_special(SpecialReg::RL), 0);
        assert_eq!(mmix.get_special(SpecialReg::RJ), 0x304);
        assert_eq!(mmix.get_pc(), 0x320);

        assert_eq!(mmix.read_octa(ro), 42);
        assert_eq!(mmix.read_octa(ro + 8), 43);
        assert_eq!(mmix.read_octa(ro + 16), 2); // hole marker = X
        assert_eq!(mmix.call_depth(), 1);

        // POP 0,0 at target — restore caller's $0,$1.
        mmix.write_tetra(0x320, 0xF8000000);
        mmix.execute_instruction();

        assert_eq!(mmix.get_register(0), 42);
        assert_eq!(mmix.get_register(1), 43);
        assert_eq!(mmix.get_special(SpecialReg::RO), ro);
        assert_eq!(mmix.get_special(SpecialReg::RS), ro);
        assert_eq!(mmix.get_special(SpecialReg::RL), 2);
        assert_eq!(mmix.get_pc(), 0x304);
        assert_eq!(mmix.call_depth(), 0);
    }

    #[test]
    fn test_pushj_pop_nested() {
        // Two-level nested call exercises arg passing in both directions
        // and that the two frames stack contiguously in memory.
        let mut mmix = MMix::new();
        mmix.set_pc(0x100);
        mmix.set_register(0, 10);
        mmix.set_register(1, 11);
        mmix.set_register(2, 0xCAFE); // arg for inner call: caller's $2
        mmix.set_special(SpecialReg::RL, 5);
        let ro = mmix.get_special(SpecialReg::RO);

        // Outer PUSHJ $2, +1 — saves $0,$1; slides $3,$4 to callee's $0,$1; arg at $2 → callee $0? No —
        // PUSHJ $X saves $0..$X-1 (here $0,$1) and the marginal at $X. Caller's $X+1, $X+2, ...
        // become callee's $0, $1, ... .  So $2 (the marginal) is *not* an arg; it becomes X.
        // To pass an arg via slide, stage at $X+1 = $3.
        mmix.set_register(3, 0xCAFE);
        mmix.write_tetra(0x100, 0xF2020001);
        mmix.execute_instruction();

        // Outer callee sees arg at $0.
        assert_eq!(mmix.get_register(0), 0xCAFE);
        assert_eq!(mmix.call_depth(), 1);
        // Outer frame: X+1 = 3 entries.
        assert_eq!(mmix.get_special(SpecialReg::RO), ro + 24);

        // Outer callee stages an arg at $1 then nested PUSHJ $0, +1 (X=0: nothing saved, slide $1→$0).
        mmix.set_register(1, 0xBEEF);
        mmix.write_tetra(0x104, 0xF2000001);
        mmix.execute_instruction();

        assert_eq!(mmix.get_register(0), 0xBEEF);
        assert_eq!(mmix.call_depth(), 2);
        // Inner frame starts exactly where the outer frame's entries end.
        assert_eq!(mmix.get_special(SpecialReg::RO), ro + 32);

        // Inner POP 1, 0 — return 0xD00D at $0; lands at outer callee's $0 (hole=0).
        mmix.set_register(0, 0xD00D);
        mmix.write_tetra(0x108, 0xF8010000);
        mmix.execute_instruction();

        assert_eq!(mmix.get_register(0), 0xD00D);
        assert_eq!(mmix.call_depth(), 1);
        // Popping the inner frame retracts rO to exactly where it started.
        assert_eq!(mmix.get_special(SpecialReg::RO), ro + 24);

        // Outer POP 1, 0 — return 0xD00D, lands at top-level caller's $2 (hole=2).
        mmix.write_tetra(0x10C, 0xF8010000);
        mmix.execute_instruction();

        assert_eq!(mmix.get_register(2), 0xD00D);
        // Caller's $0, $1 restored.
        assert_eq!(mmix.get_register(0), 10);
        assert_eq!(mmix.get_register(1), 11);
        assert_eq!(mmix.get_special(SpecialReg::RO), ro);
        assert_eq!(mmix.call_depth(), 0);
    }

    /// `POP` never writes `rJ`: it only reads it for the branch target.
    /// Set a distinct, nonzero `rJ` before the call so a restore, if `POP`
    /// still did one, would be visible.
    #[test]
    fn test_pop_leaves_rj_as_pushj_set_it() {
        let mut mmix = MMix::new();
        mmix.set_pc(0x100);
        mmix.set_special(SpecialReg::RJ, 0x999);
        mmix.write_tetra(0x100, 0xF2000002); // PUSHJ $0,2 -> 0x108, rJ := 0x104
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_special(SpecialReg::RJ), 0x104);

        mmix.write_tetra(0x108, 0xF8000000); // POP 0,0
        assert!(mmix.execute_instruction());
        assert_eq!(
            mmix.get_special(SpecialReg::RJ),
            0x104,
            "POP must leave rJ exactly as PUSHJ set it, not restore 0x999"
        );
        assert_eq!(mmix.get_pc(), 0x104);
    }

    /// A callee that makes a nested call without saving `rJ` first has its
    /// own `POP` branch to the address after the *nested* `PUSHJ`, not back
    /// to its own caller: a subroutine that calls another must save and
    /// restore `rJ` itself.
    #[test]
    fn test_pop_without_saving_rj_returns_to_the_nested_call_site() {
        let mut mmix = MMix::new();
        mmix.set_pc(0x100);
        // Top-level caller: PUSHJ $0,2 -> callee at 0x108; rJ := 0x104.
        mmix.write_tetra(0x100, 0xF2000002);
        assert!(mmix.execute_instruction());

        // Callee makes a nested call without saving rJ: PUSHJ $0,4 ->
        // nested callee at 0x118; rJ := 0x10C.
        mmix.write_tetra(0x108, 0xF2000004);
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_special(SpecialReg::RJ), 0x10C);

        // Nested callee returns immediately: POP 0,0 branches to rJ + 0,
        // landing back at 0x10C.
        mmix.write_tetra(0x118, 0xF8000000);
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 0x10C);

        // The callee's own POP, resumed right there, reads the SAME stale
        // rJ (0x10C) and branches to it again -- the address after its
        // nested PUSHJ, not the address after the top-level PUSHJ (0x104).
        mmix.write_tetra(0x10C, 0xF8000000);
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 0x10C);
        assert_eq!(mmix.call_depth(), 0);
    }

    /// A callee that saves `rJ` before its nested call and restores it
    /// after, before its own `POP`, returns correctly to its own caller.
    #[test]
    fn test_pop_returns_correctly_when_rj_is_saved_and_restored() {
        let mut mmix = MMix::new();
        mmix.set_pc(0x100);
        // Top-level caller: PUSHJ $0,2 -> callee at 0x108; rJ := 0x104.
        mmix.write_tetra(0x100, 0xF2000002);
        assert!(mmix.execute_instruction());

        // Callee saves rJ: GET $1,rJ.
        mmix.write_tetra(0x108, 0xFE010004);
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 0x104);

        // Callee makes a nested call: PUSHJ $2,3 -- the hole must clear
        // rJ's stash at $1, or the slide swallows it into the nested
        // callee's own $0 -- nested callee at 0x118; rJ := 0x110.
        mmix.write_tetra(0x10C, 0xF2020003);
        assert!(mmix.execute_instruction());

        // Nested callee returns immediately: POP 0,0 branches to rJ + 0,
        // landing back at 0x110.
        mmix.write_tetra(0x118, 0xF8000000);
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 0x110);

        // Callee restores rJ: PUT rJ,$1.
        mmix.write_tetra(0x110, 0xF6040001);
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_special(SpecialReg::RJ), 0x104);

        // Callee's own POP now branches to the address after the
        // top-level PUSHJ, not the nested one.
        mmix.write_tetra(0x114, 0xF8010000);
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 0x104);
        assert_eq!(mmix.call_depth(), 0);
    }

    #[test]
    fn test_incl_instruction() {
        let mut mmix = MMix::new();
        // INCL $1, YZ=0x0203 - opcode 0xE7, X=1, YZ=0x0203
        mmix.write_tetra(0, 0xE7010203);
        mmix.set_register(1, 50);

        let result = mmix.execute_instruction();
        assert!(result); // Should continue
        assert_eq!(mmix.get_register(1), 50 + 0x0203); // 50 + YZ value
        assert_eq!(mmix.get_pc(), 4); // PC advanced
    }

    #[test]
    fn test_incl_with_zero() {
        let mut mmix = MMix::new();
        // INCL $2, YZ=0
        mmix.write_tetra(0, 0xE7020000);
        mmix.set_register(2, 42);

        mmix.execute_instruction();
        assert_eq!(mmix.get_register(2), 42); // No change
    }

    #[test]
    fn test_incl_overflow() {
        let mut mmix = MMix::new();
        // INCL $3, YZ=0x0405
        mmix.write_tetra(0, 0xE7030405);
        mmix.set_register(3, u64::MAX - 5);

        mmix.execute_instruction();
        assert_eq!(mmix.get_register(3), (u64::MAX - 5).wrapping_add(0x0405)); // Wraps around
    }

    #[test]
    fn test_incl_large_values() {
        let mut mmix = MMix::new();
        // INCL $1, YZ=0x0203
        mmix.write_tetra(0, 0xE7010203);
        mmix.set_register(1, 100);

        mmix.execute_instruction();
        assert_eq!(mmix.get_register(1), 100 + 0x0203);
    }

    #[test]
    fn test_incl_register_255() {
        let mut mmix = MMix::new();
        // INCL $255, YZ=0x0102 - should modify $255 like any other register
        mmix.write_tetra(0, 0xE7FF0102);

        mmix.execute_instruction();
        assert_eq!(mmix.get_register(255), 0x0102); // Should have the immediate value
    }

    #[test]
    fn test_incl_using_255() {
        let mut mmix = MMix::new();
        // INCL $1, YZ=0xFF02
        mmix.write_tetra(0, 0xE701FF02);
        mmix.set_register(1, 100);

        mmix.execute_instruction();
        assert_eq!(mmix.get_register(1), 100 + 0xFF02);
    }

    #[test]
    fn test_run_multiple_incl() {
        let mut mmix = MMix::new();
        // Program: 3 INCL instructions followed by TRAP (halt)
        mmix.write_tetra(0, 0xE7010000); // INCL $1, YZ=0 (no change)
        mmix.write_tetra(4, 0xE7010203); // INCL $1, YZ=0x0203
        mmix.write_tetra(8, 0xE7010203); // INCL $1, YZ=0x0203
        mmix.write_tetra(12, 0xFF000000); // TRIP (halt)

        let count = mmix.run();
        assert_eq!(count, 3);
        assert_eq!(mmix.get_register(1), 0x0203 * 2); // 0 + 0x0203 + 0x0203
        assert_eq!(mmix.get_pc(), 12);
    }

    #[test]
    fn test_trip_to_unloaded_vector_halts_with_diagnostic_and_nonzero_exit() {
        // #00 reads zero, which decodes as TRAP 0,0,0 — the case the
        // unloaded-vector halt exists to catch instead of hiding.
        let (host, handle) = CaptureHost::new();
        let mut mmix = MMix::with_host(host);
        mmix.set_pc(0x100);
        mmix.write_tetra(0x100, 0xFF000000); // TRIP 0,0,0
        let result = mmix.execute_instruction();
        assert!(!result);
        assert_eq!(mmix.get_pc(), 0x100, "PC stays on the tripping instruction");
        assert_eq!(mmix.get_exit_code(), 1);
        // Registers are set before the halt is detected, so a debugger sees why.
        assert_eq!(mmix.get_special(SpecialReg::RW), 0x104);
        assert_eq!(handle.diagnostics().len(), 1);
        assert!(handle.diagnostics()[0].contains("TRIP"));
    }

    // Load instruction tests

    #[test]
    fn test_ldb_signed_positive() {
        let mut mmix = MMix::new();
        // LDB $1, $2, $3 - Load signed byte (positive)
        mmix.write_tetra(0, 0x80010203);
        mmix.set_register(2, 100);
        mmix.set_register(3, 50);
        mmix.write_byte(150, 127); // Max positive signed byte

        mmix.execute_instruction();
        assert_eq!(mmix.get_register(1), 127);
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_ldb_signed_negative() {
        let mut mmix = MMix::new();
        // LDB $1, $2, $3 - Load signed byte (negative)
        mmix.write_tetra(0, 0x80010203);
        mmix.set_register(2, 100);
        mmix.set_register(3, 50);
        mmix.write_byte(150, 0xFF); // -1 in signed byte

        mmix.execute_instruction();
        assert_eq!(mmix.get_register(1) as i64, -1);
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_ldb_immediate() {
        let mut mmix = MMix::new();
        // LDB $1, $2, 10 - Load signed byte with immediate offset
        mmix.write_tetra(0, 0x8101020A);
        mmix.set_register(2, 100);
        mmix.write_byte(110, 0x80); // -128 in signed byte

        mmix.execute_instruction();
        assert_eq!(mmix.get_register(1) as i64, -128);
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_ldbu_unsigned() {
        let mut mmix = MMix::new();
        // LDBU $1, $2, $3 - Load unsigned byte
        mmix.write_tetra(0, 0x82010203);
        mmix.set_register(2, 100);
        mmix.set_register(3, 50);
        mmix.write_byte(150, 0xFF); // 255 unsigned

        mmix.execute_instruction();
        assert_eq!(mmix.get_register(1), 255);
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_ldbu_immediate() {
        let mut mmix = MMix::new();
        // LDBU $1, $2, 20 - Load unsigned byte with immediate
        mmix.write_tetra(0, 0x83010214);
        mmix.set_register(2, 100);
        mmix.write_byte(120, 200);

        mmix.execute_instruction();
        assert_eq!(mmix.get_register(1), 200);
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_ldw_signed_positive() {
        let mut mmix = MMix::new();
        // LDW $1, $2, $3 - Load signed wyde (positive)
        mmix.write_tetra(0, 0x84010203);
        mmix.set_register(2, 100);
        mmix.set_register(3, 50);
        mmix.write_wyde(150, 32767); // Max positive signed wyde

        mmix.execute_instruction();
        assert_eq!(mmix.get_register(1), 32767);
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_ldw_signed_negative() {
        let mut mmix = MMix::new();
        // LDW $1, $2, $3 - Load signed wyde (negative)
        mmix.write_tetra(0, 0x84010203);
        mmix.set_register(2, 100);
        mmix.set_register(3, 50);
        mmix.write_wyde(150, 0xFFFF); // -1 in signed wyde

        mmix.execute_instruction();
        assert_eq!(mmix.get_register(1) as i64, -1);
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_ldw_immediate() {
        let mut mmix = MMix::new();
        // LDW $1, $2, 10 - Load signed wyde with immediate
        mmix.write_tetra(0, 0x8501020A);
        mmix.set_register(2, 100);
        mmix.write_wyde(110, 0x8000); // -32768 in signed wyde

        mmix.execute_instruction();
        assert_eq!(mmix.get_register(1) as i64, -32768);
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_ldwu_unsigned() {
        let mut mmix = MMix::new();
        // LDWU $1, $2, $3 - Load unsigned wyde
        mmix.write_tetra(0, 0x86010203);
        mmix.set_register(2, 100);
        mmix.set_register(3, 50);
        mmix.write_wyde(150, 0xFFFF); // 65535 unsigned

        mmix.execute_instruction();
        assert_eq!(mmix.get_register(1), 65535);
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_ldwu_immediate() {
        let mut mmix = MMix::new();
        // LDWU $1, $2, 30 - Load unsigned wyde with immediate
        mmix.write_tetra(0, 0x8701021E);
        mmix.set_register(2, 100);
        mmix.write_wyde(130, 50000);

        mmix.execute_instruction();
        assert_eq!(mmix.get_register(1), 50000);
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_ldt_signed_positive() {
        let mut mmix = MMix::new();
        // LDT $1, $2, $3 - Load signed tetra (positive)
        mmix.write_tetra(0, 0x88010203);
        mmix.set_register(2, 100);
        mmix.set_register(3, 50);
        mmix.write_tetra(150, 2_147_483_647); // Max positive signed tetra

        mmix.execute_instruction();
        assert_eq!(mmix.get_register(1), 2_147_483_647);
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_ldt_signed_negative() {
        let mut mmix = MMix::new();
        // LDT $1, $2, $3 - Load signed tetra (negative)
        mmix.write_tetra(0, 0x88010203);
        mmix.set_register(2, 100);
        mmix.set_register(3, 50);
        mmix.write_tetra(150, 0xFFFFFFFF); // -1 in signed tetra

        mmix.execute_instruction();
        assert_eq!(mmix.get_register(1) as i64, -1);
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_ldt_immediate() {
        let mut mmix = MMix::new();
        // LDT $1, $2, 20 - Load signed tetra with immediate
        mmix.write_tetra(0, 0x89010214);
        mmix.set_register(2, 100);
        mmix.write_tetra(120, 0x80000000); // -2147483648 in signed tetra

        mmix.execute_instruction();
        assert_eq!(mmix.get_register(1) as i64, -2147483648);
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_ldtu_unsigned() {
        let mut mmix = MMix::new();
        // LDTU $1, $2, $3 - Load unsigned tetra
        mmix.write_tetra(0, 0x8A010203);
        mmix.set_register(2, 100);
        mmix.set_register(3, 50);
        mmix.write_tetra(150, 0xFFFFFFFF); // 4294967295 unsigned

        mmix.execute_instruction();
        assert_eq!(mmix.get_register(1), 4294967295);
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_ldtu_immediate() {
        let mut mmix = MMix::new();
        // LDTU $1, $2, 40 - Load unsigned tetra with immediate
        mmix.write_tetra(0, 0x8B010228);
        mmix.set_register(2, 100);
        mmix.write_tetra(140, 3_000_000_000);

        mmix.execute_instruction();
        assert_eq!(mmix.get_register(1), 3_000_000_000);
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_ldo_load_octa() {
        let mut mmix = MMix::new();
        // LDO $1, $2, $3 - Load octa
        mmix.write_tetra(0, 0x8C010203);
        mmix.set_register(2, 100);
        mmix.set_register(3, 50);
        mmix.write_octa(150, 0x123456789ABCDEF0);

        mmix.execute_instruction();
        assert_eq!(mmix.get_register(1), 0x123456789ABCDEF0);
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_ldo_immediate() {
        let mut mmix = MMix::new();
        // LDO $1, $2, 16 - Load octa with immediate
        mmix.write_tetra(0, 0x8D010210);
        mmix.set_register(2, 100);
        mmix.write_octa(116, 0xFEDCBA9876543210);

        mmix.execute_instruction();
        assert_eq!(mmix.get_register(1), 0xFEDCBA9876543210);
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_ldou_same_as_ldo() {
        let mut mmix = MMix::new();
        // LDOU $1, $2, $3 - Load octa unsigned (same as LDO)
        mmix.write_tetra(0, 0x8E010203);
        mmix.set_register(2, 100);
        mmix.set_register(3, 50);
        mmix.write_octa(150, 0x123456789ABCDEF0);

        mmix.execute_instruction();
        assert_eq!(mmix.get_register(1), 0x123456789ABCDEF0);
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_ldou_immediate() {
        let mut mmix = MMix::new();
        // LDOU $1, $2, 8 - Load octa unsigned with immediate
        mmix.write_tetra(0, 0x8F010208);
        mmix.set_register(2, 1000);
        mmix.write_octa(1008, 0xFFFFFFFFFFFFFFFF);

        mmix.execute_instruction();
        assert_eq!(mmix.get_register(1), 0xFFFFFFFFFFFFFFFF);
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_ldsf_short_float() {
        let mut mmix = MMix::new();
        // LDSF $1, $2, $3 - Load short float (32-bit to 64-bit)
        mmix.write_tetra(0, 0x90010203);
        mmix.set_register(2, 100);
        mmix.set_register(3, 50);
        // Write a 32-bit float (e.g., 3.14159 in IEEE 754 single precision)
        let float_val = std::f32::consts::PI;
        mmix.write_tetra(150, float_val.to_bits());

        mmix.execute_instruction();
        // Should be converted to 64-bit float
        let result_f64 = MMix::u64_to_f64(mmix.get_register(1));
        assert!((result_f64 - std::f64::consts::PI).abs() < 0.0001);
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_ldsfi_immediate() {
        let mut mmix = MMix::new();
        // LDSFI $1, $2, 12 - Load short float with immediate offset
        mmix.write_tetra(0, 0x9101020C);
        mmix.set_register(2, 200);
        // Write a 32-bit float (e.g., -2.5 in IEEE 754 single precision)
        let float_val = -2.5f32;
        mmix.write_tetra(212, float_val.to_bits());

        mmix.execute_instruction();
        let result_f64 = MMix::u64_to_f64(mmix.get_register(1));
        assert_eq!(result_f64, -2.5);
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_lda_load_address() {
        let mut mmix = MMix::new();
        // LDA $1, $2, $3 - Load address (same as ADDU)
        mmix.write_tetra(0, 0x22010203);
        mmix.set_register(2, 0x1000);
        mmix.set_register(3, 0x500);

        mmix.execute_instruction();
        assert_eq!(mmix.get_register(1), 0x1500);
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_lda_immediate() {
        let mut mmix = MMix::new();
        // LDA $1, $2, 64 - Load address with immediate (same as ADDU)
        mmix.write_tetra(0, 0x23010240);
        mmix.set_register(2, 0x2000);

        mmix.execute_instruction();
        assert_eq!(mmix.get_register(1), 0x2040);
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_load_from_uninitialized_memory() {
        let mut mmix = MMix::new();
        // LDBU $1, $0, 100 - Load from uninitialized memory (should be 0)
        mmix.write_tetra(0, 0x83010064);

        mmix.execute_instruction();
        assert_eq!(mmix.get_register(1), 0);
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_load_address_wraparound() {
        let mut mmix = MMix::new();
        // LDA $1, $2, $3 - Test address wraparound
        mmix.write_tetra(0, 0x22010203);
        mmix.set_register(2, u64::MAX - 100);
        mmix.set_register(3, 200);

        mmix.execute_instruction();
        assert_eq!(mmix.get_register(1), 99); // Wraps around
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_wyde_set_replaces_whole_register() {
        // SETH, SETMH, SETML, SETL each deposit YZ in one wyde and zero the
        // other 48 bits. The all-ones preload is what distinguishes a set
        // from a merge.
        for (opcode, shift) in [(0xE0_u32, 48_u32), (0xE1, 32), (0xE2, 16), (0xE3, 0)] {
            let mut mmix = MMix::new();
            mmix.set_register(1, u64::MAX);
            mmix.write_tetra(0, (opcode << 24) | (1 << 16) | 0xABCD);

            mmix.execute_instruction();

            let reg = mmix.get_register(1);
            assert_eq!((reg >> shift) & 0xFFFF, 0xABCD, "opcode {opcode:#04X} wyde");
            assert_eq!(
                reg & !(0xFFFF_u64 << shift),
                0,
                "opcode {opcode:#04X} residue"
            );
            assert_eq!(mmix.get_pc(), 4);
        }
    }

    #[test]
    fn test_setl_zero_clears_whole_register() {
        let mut mmix = MMix::new();
        mmix.set_register(1, u64::MAX);
        // SETL $1, 0 - a one-instruction register clear
        mmix.write_tetra(0, 0xE3010000);

        mmix.execute_instruction();
        assert_eq!(mmix.get_register(1), 0);
    }

    /// Assemble `source`, load its image and execute `steps` instructions
    /// from the first assembled address.
    fn assemble_and_run(source: &str, steps: usize) -> MMix {
        use crate::debugger::write_image;
        use crate::mmixal::MMixAssembler;

        let mut asm = MMixAssembler::new(source, "<test>");
        asm.parse().expect("test source must assemble");
        let start = asm.instructions[0].0;

        let mut mmix = MMix::new();
        write_image(&mut mmix, &asm);
        mmix.set_pc(start);
        for _ in 0..steps {
            mmix.execute_instruction();
        }
        mmix
    }

    #[test]
    fn test_seti_lands_a_wide_constant() {
        let mmix = assemble_and_run("SETI $1,#0123456789ABCDEF", 4);
        assert_eq!(mmix.get_register(1), 0x0123456789ABCDEF);
    }

    #[test]
    fn test_lda_large_address_lands_whole_address() {
        // LDA $X,Label above #FF expands to the same four tetras SETI uses;
        // examples/hello_world.mms gets its string pointer this way.
        let mmix = assemble_and_run("\tLOC\t#2000000000001234\nMain\tLDA\t$255,Main\n", 4);
        assert_eq!(mmix.get_register(255), 0x2000_0000_0000_1234);
    }

    #[test]
    fn test_inc_and_or_wydes_preserve_the_others() {
        let mut mmix = MMix::new();
        mmix.set_register(1, 0x1111_2222_3333_4444);

        // INCML $1, 0x0001
        mmix.write_tetra(0, 0xE6010001);
        mmix.execute_instruction();
        assert_eq!(mmix.get_register(1), 0x1111_2222_3334_4444);

        // ORL $1, 0x000F
        mmix.set_pc(4);
        mmix.write_tetra(4, 0xEB01000F);
        mmix.execute_instruction();
        assert_eq!(mmix.get_register(1), 0x1111_2222_3334_444F);
    }

    #[test]
    fn test_incl_has_multiple_tests() {
        // INCL is already tested in:
        // - test_incl_instruction
        // - test_incl_with_zero
        // - test_incl_overflow
        // - test_incl_large_values
        // - test_incl_register_255
        // - test_incl_using_255
        // - test_run_multiple_incl
        // This test just confirms coverage
        let mut mmix = MMix::new();
        mmix.write_tetra(0, 0xE7010203);
        mmix.execute_instruction();
        assert_eq!(mmix.get_register(1), 0x0203);
    }

    // Store instruction tests

    #[test]
    fn test_stb_store_byte() {
        let mut mmix = MMix::new();
        // STB $1, $2, $3 - Store byte
        mmix.write_tetra(0, 0xA0010203);
        mmix.set_register(1, 0x42); // Value to store
        mmix.set_register(2, 100);
        mmix.set_register(3, 50);

        mmix.execute_instruction();
        assert_eq!(mmix.read_byte(150), 0x42);
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_stb_immediate() {
        let mut mmix = MMix::new();
        // STB $1, $2, 10 - Store byte immediate
        mmix.write_tetra(0, 0xA101020A);
        mmix.set_register(1, 0x7F); // Max positive signed byte
        mmix.set_register(2, 200);

        mmix.execute_instruction();
        assert_eq!(mmix.read_byte(210), 0x7F);
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_stbu_store_byte_unsigned() {
        let mut mmix = MMix::new();
        // STBU $1, $2, $3 - Store byte unsigned
        mmix.write_tetra(0, 0xA2010203);
        mmix.set_register(1, 0xFF); // 255 unsigned
        mmix.set_register(2, 100);
        mmix.set_register(3, 50);

        mmix.execute_instruction();
        assert_eq!(mmix.read_byte(150), 0xFF);
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_stbu_immediate() {
        let mut mmix = MMix::new();
        // STBU $1, $2, 20 - Store byte unsigned immediate
        mmix.write_tetra(0, 0xA3010214);
        mmix.set_register(1, 0xAB);
        mmix.set_register(2, 1000);

        mmix.execute_instruction();
        assert_eq!(mmix.read_byte(1020), 0xAB);
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_stw_store_wyde() {
        let mut mmix = MMix::new();
        // STW $1, $2, $3 - Store wyde
        mmix.write_tetra(0, 0xA4010203);
        mmix.set_register(1, 0x1234);
        mmix.set_register(2, 100);
        mmix.set_register(3, 50);

        mmix.execute_instruction();
        assert_eq!(mmix.read_wyde(150), 0x1234);
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_stw_immediate() {
        let mut mmix = MMix::new();
        // STW $1, $2, 30 - Store wyde immediate
        mmix.write_tetra(0, 0xA501021E);
        mmix.set_register(1, 0x7FFF); // Max positive signed wyde
        mmix.set_register(2, 2000);

        mmix.execute_instruction();
        assert_eq!(mmix.read_wyde(2030), 0x7FFF);
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_stwu_store_wyde_unsigned() {
        let mut mmix = MMix::new();
        // STWU $1, $2, $3 - Store wyde unsigned
        mmix.write_tetra(0, 0xA6010203);
        mmix.set_register(1, 0xFFFF); // 65535 unsigned
        mmix.set_register(2, 100);
        mmix.set_register(3, 50);

        mmix.execute_instruction();
        assert_eq!(mmix.read_wyde(150), 0xFFFF);
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_stwu_immediate() {
        let mut mmix = MMix::new();
        // STWU $1, $2, 40 - Store wyde unsigned immediate
        mmix.write_tetra(0, 0xA7010228);
        mmix.set_register(1, 0xABCD);
        mmix.set_register(2, 5000);

        mmix.execute_instruction();
        assert_eq!(mmix.read_wyde(5040), 0xABCD);
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_stt_store_tetra() {
        let mut mmix = MMix::new();
        // STT $1, $2, $3 - Store tetra
        mmix.write_tetra(0, 0xA8010203);
        mmix.set_register(1, 0x12345678);
        mmix.set_register(2, 100);
        mmix.set_register(3, 50);

        mmix.execute_instruction();
        assert_eq!(mmix.read_tetra(150), 0x12345678);
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_stt_immediate() {
        let mut mmix = MMix::new();
        // STT $1, $2, 50 - Store tetra immediate
        mmix.write_tetra(0, 0xA9010232);
        mmix.set_register(1, 0x7FFFFFFF); // Max positive signed tetra
        mmix.set_register(2, 10000);

        mmix.execute_instruction();
        assert_eq!(mmix.read_tetra(10050), 0x7FFFFFFF);
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_sttu_store_tetra_unsigned() {
        let mut mmix = MMix::new();
        // STTU $1, $2, $3 - Store tetra unsigned
        mmix.write_tetra(0, 0xAA010203);
        mmix.set_register(1, 0xFFFFFFFF); // 4294967295 unsigned
        mmix.set_register(2, 100);
        mmix.set_register(3, 50);

        mmix.execute_instruction();
        assert_eq!(mmix.read_tetra(150), 0xFFFFFFFF);
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_sttu_immediate() {
        let mut mmix = MMix::new();
        // STTU $1, $2, 60 - Store tetra unsigned immediate
        mmix.write_tetra(0, 0xAB01023C);
        mmix.set_register(1, 0xDEADBEEF);
        mmix.set_register(2, 20000);

        mmix.execute_instruction();
        assert_eq!(mmix.read_tetra(20060), 0xDEADBEEF);
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_sto_store_octa() {
        let mut mmix = MMix::new();
        // STO $1, $2, $3 - Store octa
        mmix.write_tetra(0, 0xAC010203);
        mmix.set_register(1, 0x123456789ABCDEF0);
        mmix.set_register(2, 100);
        mmix.set_register(3, 50);

        mmix.execute_instruction();
        assert_eq!(mmix.read_octa(150), 0x123456789ABCDEF0);
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_sto_immediate() {
        let mut mmix = MMix::new();
        // STO $1, $2, 70 - Store octa immediate
        mmix.write_tetra(0, 0xAD010246);
        mmix.set_register(1, 0xFEDCBA9876543210);
        mmix.set_register(2, 30000);

        mmix.execute_instruction();
        assert_eq!(mmix.read_octa(30070), 0xFEDCBA9876543210);
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_stou_same_as_sto() {
        let mut mmix = MMix::new();
        // STOU $1, $2, $3 - Store octa unsigned (same as STO)
        mmix.write_tetra(0, 0xAE010203);
        mmix.set_register(1, 0xFFFFFFFFFFFFFFFF);
        mmix.set_register(2, 100);
        mmix.set_register(3, 50);

        mmix.execute_instruction();
        assert_eq!(mmix.read_octa(150), 0xFFFFFFFFFFFFFFFF);
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_stou_immediate() {
        let mut mmix = MMix::new();
        // STOU $1, $2, 80 - Store octa unsigned immediate
        mmix.write_tetra(0, 0xAF010250);
        mmix.set_register(1, 0x0123456789ABCDEF);
        mmix.set_register(2, 40000);

        mmix.execute_instruction();
        assert_eq!(mmix.read_octa(40080), 0x0123456789ABCDEF);
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_stco_store_constant() {
        let mut mmix = MMix::new();
        // STCO 42, $2, $3 - Store constant octabyte
        mmix.write_tetra(0, 0xB42A0203); // X=42 (0x2A)
        mmix.set_register(2, 100);
        mmix.set_register(3, 50);

        mmix.execute_instruction();
        assert_eq!(mmix.read_octa(150), 42);
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_stco_immediate() {
        let mut mmix = MMix::new();
        // STCO 255, $2, 90 - Store constant octabyte immediate
        mmix.write_tetra(0, 0xB5FF025A); // X=255 (0xFF)
        mmix.set_register(2, 50000);

        mmix.execute_instruction();
        assert_eq!(mmix.read_octa(50090), 255);
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_stht_store_high_tetra() {
        let mut mmix = MMix::new();
        // STHT $1, $2, $3 - Store high tetra
        mmix.write_tetra(0, 0xB2010203);
        mmix.set_register(1, 0xDEADBEEF12345678);
        mmix.set_register(2, 100);
        mmix.set_register(3, 50);

        mmix.execute_instruction();
        assert_eq!(mmix.read_tetra(150), 0xDEADBEEF); // High 32 bits
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_stht_immediate() {
        let mut mmix = MMix::new();
        // STHT $1, $2, 100 - Store high tetra immediate
        mmix.write_tetra(0, 0xB3010264);
        mmix.set_register(1, 0xABCD123456789ABC);
        mmix.set_register(2, 60000);

        mmix.execute_instruction();
        assert_eq!(mmix.read_tetra(60100), 0xABCD1234); // High 32 bits
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_store_and_load_roundtrip() {
        let mut mmix = MMix::new();
        let test_addr = 5000u64;

        // Store a value
        mmix.set_register(1, 0x123456789ABCDEF0);
        mmix.set_register(2, test_addr);
        mmix.write_tetra(0, 0xAD010200); // STO $1, $2, 0
        mmix.execute_instruction();

        // Load it back
        mmix.set_pc(4);
        mmix.write_tetra(4, 0x8D030200); // LDO $3, $2, 0
        mmix.execute_instruction();

        assert_eq!(mmix.get_register(3), 0x123456789ABCDEF0);
    }

    #[test]
    fn test_all_store_instructions_have_tests() {
        // Verify all store instructions are covered
        let mut mmix = MMix::new();

        // STB $1, $2, $3
        mmix.write_tetra(0, 0xA0010203);
        mmix.set_register(1, 0x5A);
        mmix.set_register(2, 300);
        mmix.set_register(3, 12);
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.read_byte(312), 0x5A);

        // STBU $1, $2, $3
        mmix.set_pc(0);
        mmix.write_tetra(0, 0xA2010203);
        mmix.set_register(1, 0xE1);
        mmix.set_register(2, 300);
        mmix.set_register(3, 13);
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.read_byte(313), 0xE1);

        // STW $1, $2, $3
        mmix.set_pc(0);
        mmix.write_tetra(0, 0xA4010203);
        mmix.set_register(1, 0x4321);
        mmix.set_register(2, 400);
        mmix.set_register(3, 14);
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.read_wyde(414), 0x4321);

        // STWU $1, $2, $3
        mmix.set_pc(0);
        mmix.write_tetra(0, 0xA6010203);
        mmix.set_register(1, 0xBEEF);
        mmix.set_register(2, 400);
        mmix.set_register(3, 16);
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.read_wyde(416), 0xBEEF);

        // STT $1, $2, $3
        mmix.set_pc(0);
        mmix.write_tetra(0, 0xA8010203);
        mmix.set_register(1, 0x87654321);
        mmix.set_register(2, 500);
        mmix.set_register(3, 20);
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.read_tetra(520), 0x87654321);

        // STTU $1, $2, $3
        mmix.set_pc(0);
        mmix.write_tetra(0, 0xAA010203);
        mmix.set_register(1, 0xCAFEBABE);
        mmix.set_register(2, 500);
        mmix.set_register(3, 24);
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.read_tetra(524), 0xCAFEBABE);

        // STO $1, $2, $3
        mmix.set_pc(0);
        mmix.write_tetra(0, 0xAC010203);
        mmix.set_register(1, 0x0F1E2D3C4B5A6978);
        mmix.set_register(2, 600);
        mmix.set_register(3, 30);
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.read_octa(630), 0x0F1E2D3C4B5A6978);

        // STOU $1, $2, $3
        mmix.set_pc(0);
        mmix.write_tetra(0, 0xAE010203);
        mmix.set_register(1, 0x1122334455667788);
        mmix.set_register(2, 600);
        mmix.set_register(3, 40);
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.read_octa(640), 0x1122334455667788);

        // STCO 17, $2, $3 - Store constant octabyte
        mmix.set_pc(0);
        mmix.write_tetra(0, 0xB4110203); // X=17 (0x11)
        mmix.set_register(2, 700);
        mmix.set_register(3, 5);
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.read_octa(705), 17);

        // STHT $1, $2, $3 - Store high tetra
        mmix.set_pc(0);
        mmix.write_tetra(0, 0xB2010203);
        mmix.set_register(1, 0x1357924600000000);
        mmix.set_register(2, 800);
        mmix.set_register(3, 8);
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.read_tetra(808), 0x13579246); // High 32 bits
    }

    // Arithmetic instruction tests - Add and Subtract

    #[test]
    fn test_add_positive_numbers() {
        let mut mmix = MMix::new();
        // ADD $1, $2, $3
        mmix.write_tetra(0, 0x20010203);
        mmix.set_register(2, 100);
        mmix.set_register(3, 50);

        mmix.execute_instruction();
        assert_eq!(mmix.get_register(1), 150);
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_add_immediate() {
        let mut mmix = MMix::new();
        // ADD $1, $2, 75
        mmix.write_tetra(0, 0x2101024B);
        mmix.set_register(2, 25);

        mmix.execute_instruction();
        assert_eq!(mmix.get_register(1), 100);
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_add_negative_numbers() {
        let mut mmix = MMix::new();
        // ADD $1, $2, $3
        mmix.write_tetra(0, 0x20010203);
        mmix.set_register(2, (-50i64) as u64);
        mmix.set_register(3, (-30i64) as u64);

        mmix.execute_instruction();
        assert_eq!(mmix.get_register(1) as i64, -80);
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_addu_wrapping() {
        let mut mmix = MMix::new();
        // ADDU $1, $2, $3 (already tested as LDA, but verify here)
        mmix.write_tetra(0, 0x22010203);
        mmix.set_register(2, u64::MAX);
        mmix.set_register(3, 1);

        mmix.execute_instruction();
        assert_eq!(mmix.get_register(1), 0); // Wraps around
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_addu_immediate() {
        let mut mmix = MMix::new();
        // ADDU $1, $2, 100
        mmix.write_tetra(0, 0x23010264);
        mmix.set_register(2, 50);

        mmix.execute_instruction();
        assert_eq!(mmix.get_register(1), 150);
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_2addu_register() {
        let mut mmix = MMix::new();
        // 2ADDU $1, $2, $3
        mmix.write_tetra(0, 0x28010203);
        mmix.set_register(2, 10);
        mmix.set_register(3, 5);

        mmix.execute_instruction();
        assert_eq!(mmix.get_register(1), 25); // 2*10 + 5 = 25
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_2addu_immediate() {
        let mut mmix = MMix::new();
        // 2ADDU $1, $2, 7
        mmix.write_tetra(0, 0x29010207);
        mmix.set_register(2, 12);

        mmix.execute_instruction();
        assert_eq!(mmix.get_register(1), 31); // 2*12 + 7 = 31
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_4addu_register() {
        let mut mmix = MMix::new();
        // 4ADDU $1, $2, $3
        mmix.write_tetra(0, 0x2A010203);
        mmix.set_register(2, 10);
        mmix.set_register(3, 5);

        mmix.execute_instruction();
        assert_eq!(mmix.get_register(1), 45); // 4*10 + 5 = 45
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_4addu_immediate() {
        let mut mmix = MMix::new();
        // 4ADDU $1, $2, 8
        mmix.write_tetra(0, 0x2B010208);
        mmix.set_register(2, 10);

        mmix.execute_instruction();
        assert_eq!(mmix.get_register(1), 48); // 4*10 + 8 = 48
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_8addu_register() {
        let mut mmix = MMix::new();
        // 8ADDU $1, $2, $3
        mmix.write_tetra(0, 0x2C010203);
        mmix.set_register(2, 10);
        mmix.set_register(3, 5);

        mmix.execute_instruction();
        assert_eq!(mmix.get_register(1), 85); // 8*10 + 5 = 85
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_8addu_immediate() {
        let mut mmix = MMix::new();
        // 8ADDU $1, $2, 15
        mmix.write_tetra(0, 0x2D01020F);
        mmix.set_register(2, 10);

        mmix.execute_instruction();
        assert_eq!(mmix.get_register(1), 95); // 8*10 + 15 = 95
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_16addu_register() {
        let mut mmix = MMix::new();
        // 16ADDU $1, $2, $3
        mmix.write_tetra(0, 0x2E010203);
        mmix.set_register(2, 10);
        mmix.set_register(3, 5);

        mmix.execute_instruction();
        assert_eq!(mmix.get_register(1), 165); // 16*10 + 5 = 165
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_16addu_immediate() {
        let mut mmix = MMix::new();
        // 16ADDU $1, $2, 20
        mmix.write_tetra(0, 0x2F010214);
        mmix.set_register(2, 10);

        mmix.execute_instruction();
        assert_eq!(mmix.get_register(1), 180); // 16*10 + 20 = 180
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_sub_positive_result() {
        let mut mmix = MMix::new();
        // SUB $1, $2, $3
        mmix.write_tetra(0, 0x24010203);
        mmix.set_register(2, 100);
        mmix.set_register(3, 30);

        mmix.execute_instruction();
        assert_eq!(mmix.get_register(1), 70);
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_sub_negative_result() {
        let mut mmix = MMix::new();
        // SUB $1, $2, $3
        mmix.write_tetra(0, 0x24010203);
        mmix.set_register(2, 30);
        mmix.set_register(3, 100);

        mmix.execute_instruction();
        assert_eq!(mmix.get_register(1) as i64, -70);
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_sub_immediate() {
        let mut mmix = MMix::new();
        // SUB $1, $2, 25
        mmix.write_tetra(0, 0x25010219);
        mmix.set_register(2, 100);

        mmix.execute_instruction();
        assert_eq!(mmix.get_register(1), 75);
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_subu_wrapping() {
        let mut mmix = MMix::new();
        // SUBU $1, $2, $3
        mmix.write_tetra(0, 0x26010203);
        mmix.set_register(2, 10);
        mmix.set_register(3, 20);

        mmix.execute_instruction();
        assert_eq!(mmix.get_register(1), u64::MAX - 9); // 10 - 20 wraps
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_subu_immediate() {
        let mut mmix = MMix::new();
        // SUBU $1, $2, 30
        mmix.write_tetra(0, 0x2701021E);
        mmix.set_register(2, 100);

        mmix.execute_instruction();
        assert_eq!(mmix.get_register(1), 70);
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_neg_zero_minus_value() {
        let mut mmix = MMix::new();
        // NEG $1, 0, $3 - effectively 0 - $3
        mmix.write_tetra(0, 0x34010003);
        mmix.set_register(3, 50);

        mmix.execute_instruction();
        assert_eq!(mmix.get_register(1) as i64, -50);
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_neg_immediate_both() {
        let mut mmix = MMix::new();
        // NEG $1, 10, 3 - effectively 10 - 3
        mmix.write_tetra(0, 0x35010A03);

        mmix.execute_instruction();
        assert_eq!(mmix.get_register(1), 7);
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_neg_one_minus_two() {
        let mut mmix = MMix::new();
        // NEG $1, 1, 2 - effectively 1 - 2 = -1
        mmix.write_tetra(0, 0x35010102);

        mmix.execute_instruction();
        assert_eq!(mmix.get_register(1) as i64, -1);
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_negu_register() {
        let mut mmix = MMix::new();
        // NEGU $1, 0, $3
        mmix.write_tetra(0, 0x36010003);
        mmix.set_register(3, 50);

        mmix.execute_instruction();
        assert_eq!(mmix.get_register(1), u64::MAX - 49); // 0 - 50 wraps
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_negu_immediate() {
        let mut mmix = MMix::new();
        // NEGU $1, 100, 30
        mmix.write_tetra(0, 0x3701641E);

        mmix.execute_instruction();
        assert_eq!(mmix.get_register(1), 70); // 100 - 30 = 70
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_multiply_add_for_array_indexing() {
        let mut mmix = MMix::new();
        // Common pattern: 8ADDU for array of 64-bit values
        // base_addr + index * 8
        mmix.write_tetra(0, 0x2C010203);
        mmix.set_register(2, 5); // index
        mmix.set_register(3, 1000); // base address

        mmix.execute_instruction();
        assert_eq!(mmix.get_register(1), 1040); // 1000 + 5*8
    }

    #[test]
    fn test_all_arithmetic_instructions_have_tests() {
        let mut mmix = MMix::new();

        // ADD $1, $2, $3
        mmix.write_tetra(0, 0x20010203);
        mmix.set_register(2, 200);
        mmix.set_register(3, 44);
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 244);

        // ADDU $1, $2, $3
        mmix.set_pc(0);
        mmix.write_tetra(0, 0x22010203);
        mmix.set_register(2, 900);
        mmix.set_register(3, 33);
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 933);

        // 2ADDU $1, $2, $3
        mmix.set_pc(0);
        mmix.write_tetra(0, 0x28010203);
        mmix.set_register(2, 7);
        mmix.set_register(3, 9);
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 23); // 2*7 + 9 = 23

        // 4ADDU $1, $2, $3
        mmix.set_pc(0);
        mmix.write_tetra(0, 0x2A010203);
        mmix.set_register(2, 7);
        mmix.set_register(3, 9);
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 37); // 4*7 + 9 = 37

        // 8ADDU $1, $2, $3
        mmix.set_pc(0);
        mmix.write_tetra(0, 0x2C010203);
        mmix.set_register(2, 7);
        mmix.set_register(3, 9);
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 65); // 8*7 + 9 = 65

        // 16ADDU $1, $2, $3
        mmix.set_pc(0);
        mmix.write_tetra(0, 0x2E010203);
        mmix.set_register(2, 7);
        mmix.set_register(3, 9);
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 121); // 16*7 + 9 = 121

        // SUB $1, $2, $3
        mmix.set_pc(0);
        mmix.write_tetra(0, 0x24010203);
        mmix.set_register(2, 500);
        mmix.set_register(3, 120);
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 380);

        // SUBU $1, $2, $3
        mmix.set_pc(0);
        mmix.write_tetra(0, 0x26010203);
        mmix.set_register(2, 5);
        mmix.set_register(3, 8);
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), u64::MAX - 2); // 5 - 8 wraps

        // NEG $1, 0, $3 - effectively 0 - $3
        mmix.set_pc(0);
        mmix.write_tetra(0, 0x34010003);
        mmix.set_register(3, 77);
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1) as i64, -77);

        // NEGU $1, 0, $3
        mmix.set_pc(0);
        mmix.write_tetra(0, 0x36010003);
        mmix.set_register(3, 77);
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), u64::MAX - 76); // 0 - 77 wraps
    }

    #[test]
    fn test_bitwise_operations() {
        let mut mmix = MMix::new();

        // AND: 0xFF & 0x0F = 0x0F
        mmix.set_register(1, 0xFF);
        mmix.set_register(2, 0x0F);
        mmix.write_tetra(0, 0xC8030102); // AND $3,$1,$2
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(3), 0x0F);

        // ANDI: 0xFF & 0x0F = 0x0F
        mmix.set_pc(0);
        mmix.set_register(1, 0xFF);
        mmix.write_tetra(0, 0xC903010F); // ANDI $3,$1,0x0F
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(3), 0x0F);

        // OR: 0xF0 | 0x0F = 0xFF
        mmix.set_pc(0);
        mmix.set_register(1, 0xF0);
        mmix.set_register(2, 0x0F);
        mmix.write_tetra(0, 0xC0030102); // OR $3,$1,$2
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(3), 0xFF);

        // ORI: 0xF0 | 0x0F = 0xFF
        mmix.set_pc(0);
        mmix.set_register(1, 0xF0);
        mmix.write_tetra(0, 0xC103010F); // ORI $3,$1,0x0F
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(3), 0xFF);

        // XOR: 0xFF ^ 0xAA = 0x55
        mmix.set_pc(0);
        mmix.set_register(1, 0xFF);
        mmix.set_register(2, 0xAA);
        mmix.write_tetra(0, 0xC6030102); // XOR $3,$1,$2
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(3), 0x55);

        // XORI: 0xFF ^ 0xAA = 0x55
        mmix.set_pc(0);
        mmix.set_register(1, 0xFF);
        mmix.write_tetra(0, 0xC70301AA); // XORI $3,$1,0xAA
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(3), 0x55);

        // ANDN: 0xFF & !0x0F = 0xF0
        mmix.set_pc(0);
        mmix.set_register(1, 0xFF);
        mmix.set_register(2, 0x0F);
        mmix.write_tetra(0, 0xCA030102); // ANDN $3,$1,$2
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(3), 0xF0);

        // ANDNI: 0xFF & !0x0F = 0xF0
        mmix.set_pc(0);
        mmix.set_register(1, 0xFF);
        mmix.write_tetra(0, 0xCB03010F); // ANDNI $3,$1,0x0F
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(3), 0xF0);

        // ORN: 0x00 | !0x0F = 0xFFFFFFFFFFFFFFF0
        mmix.set_pc(0);
        mmix.set_register(1, 0x00);
        mmix.set_register(2, 0x0F);
        mmix.write_tetra(0, 0xC2030102); // ORN $3,$1,$2
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(3), 0xFFFFFFFFFFFFFFF0);

        // ORNI: 0x00 | !0x0F = 0xFFFFFFFFFFFFFFF0
        mmix.set_pc(0);
        mmix.set_register(1, 0x00);
        mmix.write_tetra(0, 0xC303010F); // ORNI $3,$1,0x0F
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(3), 0xFFFFFFFFFFFFFFF0);

        // NAND: !(0xFF & 0xFF) = 0xFFFFFFFFFFFFFF00
        mmix.set_pc(0);
        mmix.set_register(1, 0xFF);
        mmix.set_register(2, 0xFF);
        mmix.write_tetra(0, 0xCC030102); // NAND $3,$1,$2
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(3), 0xFFFFFFFFFFFFFF00);

        // NANDI: !(0xFF & 0xFF) = 0xFFFFFFFFFFFFFF00
        mmix.set_pc(0);
        mmix.set_register(1, 0xFF);
        mmix.write_tetra(0, 0xCD0301FF); // NANDI $3,$1,0xFF
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(3), 0xFFFFFFFFFFFFFF00);

        // NOR: !(0x00 | 0x00) = 0xFFFFFFFFFFFFFFFF
        mmix.set_pc(0);
        mmix.set_register(1, 0x00);
        mmix.set_register(2, 0x00);
        mmix.write_tetra(0, 0xC4030102); // NOR $3,$1,$2
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(3), 0xFFFFFFFFFFFFFFFF);

        // NORI: !(0x00 | 0x00) = 0xFFFFFFFFFFFFFFFF
        mmix.set_pc(0);
        mmix.set_register(1, 0x00);
        mmix.write_tetra(0, 0xC5030100); // NORI $3,$1,0x00
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(3), 0xFFFFFFFFFFFFFFFF);

        // NXOR: !(0xFF ^ 0xFF) = 0xFFFFFFFFFFFFFFFF
        mmix.set_pc(0);
        mmix.set_register(1, 0xFF);
        mmix.set_register(2, 0xFF);
        mmix.write_tetra(0, 0xCE030102); // NXOR $3,$1,$2
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(3), 0xFFFFFFFFFFFFFFFF);

        // NXORI: !(0xFF ^ 0xFF) = 0xFFFFFFFFFFFFFFFF
        mmix.set_pc(0);
        mmix.set_register(1, 0xFF);
        mmix.write_tetra(0, 0xCF0301FF); // NXORI $3,$1,0xFF
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(3), 0xFFFFFFFFFFFFFFFF);

        // MUX: mask=0xF0, Y=0xFF, Z=0x00 -> (0xFF & 0xF0) | (0x00 & !0xF0) = 0xF0
        mmix.set_pc(0);
        mmix.set_special(SpecialReg::RM, 0xF0);
        mmix.set_register(1, 0xFF);
        mmix.set_register(2, 0x00);
        mmix.write_tetra(0, 0xD8030102); // MUX $3,$1,$2
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(3), 0xF0);

        // MUXI: mask=0xAA, Y=0xFF, Z=0x55 -> (0xFF & 0xAA) | (0x55 & !0xAA) = 0xFF
        mmix.set_pc(0);
        mmix.set_special(SpecialReg::RM, 0xAA);
        mmix.set_register(1, 0xFF);
        mmix.write_tetra(0, 0xD9030155); // MUXI $3,$1,0x55
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(3), 0xFF);
    }

    #[test]
    fn test_bdif() {
        let mut mmix = MMix::new();
        // BDIF: byte difference - each byte independently
        mmix.set_register(1, 0xFF20_3040_5060_7080);
        mmix.set_register(2, 0x1010_1010_1010_1010);
        mmix.write_tetra(0, 0xD0030102); // BDIF $3,$1,$2
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(3), 0xEF10_2030_4050_6070);
    }

    #[test]
    fn test_bdifi() {
        let mut mmix = MMix::new();
        // BDIFI: byte difference immediate
        mmix.set_register(1, 0x2020_2020_2020_2020);
        mmix.write_tetra(0, 0xD1030110); // BDIFI $3,$1,0x10
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(3), 0x1010_1010_1010_1010);
    }

    #[test]
    fn test_wdif() {
        let mut mmix = MMix::new();
        // WDIF: wyde difference
        mmix.set_register(1, 0xFFFF_2000_3000_4000);
        mmix.set_register(2, 0x1000_1000_1000_1000);
        mmix.write_tetra(0, 0xD2030102); // WDIF $3,$1,$2
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(3), 0xEFFF_1000_2000_3000);
    }

    #[test]
    fn test_wdifi() {
        let mut mmix = MMix::new();
        // WDIFI: wyde difference immediate
        mmix.set_register(1, 0x1000_2000_3000_4000);
        mmix.write_tetra(0, 0xD3030105); // WDIFI $3,$1,5
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(3), 0x0FFB_1FFB_2FFB_3FFB);
    }

    #[test]
    fn test_tdif() {
        let mut mmix = MMix::new();
        // TDIF: tetra difference
        mmix.set_register(1, 0xFFFFFFFF_20000000);
        mmix.set_register(2, 0x10000000_10000000);
        mmix.write_tetra(0, 0xD4030102); // TDIF $3,$1,$2
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(3), 0xEFFFFFFF_10000000);
    }

    #[test]
    fn test_tdifi() {
        let mut mmix = MMix::new();
        // TDIFI: tetra difference immediate
        mmix.set_register(1, 0x10000000_20000000);
        mmix.write_tetra(0, 0xD503010A); // TDIFI $3,$1,10
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(3), 0x0FFFFFF6_1FFFFFF6);
    }

    #[test]
    fn test_odif() {
        let mut mmix = MMix::new();
        // ODIF: octa difference (unsigned)
        mmix.set_register(1, 1000);
        mmix.set_register(2, 300);
        mmix.write_tetra(0, 0xD6030102); // ODIF $3,$1,$2
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(3), 700);

        // Test clipping to zero
        mmix.set_pc(0);
        mmix.set_register(1, 100);
        mmix.set_register(2, 500);
        mmix.write_tetra(0, 0xD6030102); // ODIF $3,$1,$2
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(3), 0);
    }

    #[test]
    fn test_odifi() {
        let mut mmix = MMix::new();
        // ODIFI: octa difference immediate
        mmix.set_register(1, 255);
        mmix.write_tetra(0, 0xD70301FF); // ODIFI $3,$1,255
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(3), 0);
    }

    #[test]
    fn test_sadd() {
        let mut mmix = MMix::new();
        // SADD: sideways add (population count of Y \ Z)
        mmix.set_register(1, 0xFF);
        mmix.set_register(2, 0x0F);
        mmix.write_tetra(0, 0xDA030102); // SADD $3,$1,$2
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(3), 4); // 0xFF & !0x0F = 0xF0 has 4 ones
    }

    #[test]
    fn test_saddi_population_count() {
        let mut mmix = MMix::new();
        // SADDI with Z=0 gives population count
        mmix.set_register(1, 0b10101010);
        mmix.write_tetra(0, 0xDB030100); // SADDI $3,$1,0
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(3), 4); // 4 ones in 10101010
    }

    #[test]
    fn test_mor() {
        let mut mmix = MMix::new();
        // MOR: multiple or (Boolean matrix multiplication)
        // Example: byte reversal with Z = 0x0102040810204080
        mmix.set_register(1, 0x0123456789ABCDEF);
        mmix.set_register(2, 0x0102040810204080);
        mmix.write_tetra(0, 0xDC030102); // MOR $3,$1,$2
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(3), 0xEFCDAB8967452301); // byte-reversed
    }

    #[test]
    fn test_mori() {
        let mut mmix = MMix::new();
        // MORI: multiple or immediate
        mmix.set_register(1, 0xFF00FF00FF00FF00);
        mmix.write_tetra(0, 0xDD0301FF); // MORI $3,$1,255
        assert!(mmix.execute_instruction());
        // Result should be in bottom byte
        assert_eq!(mmix.get_register(3) & 0xFF, 0xFF);
    }

    #[test]
    fn test_mxor() {
        let mut mmix = MMix::new();
        // MXOR: multiple exclusive-or (matrix product over GF(2))
        // Simple test: identity matrix behavior
        mmix.set_register(1, 0x00);
        mmix.set_register(2, 0x00);
        mmix.write_tetra(0, 0xDE030102); // MXOR $3,$1,$2
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(3), 0);
    }

    #[test]
    fn test_mxori() {
        let mut mmix = MMix::new();
        // MXORI: multiple exclusive-or immediate
        mmix.set_register(1, 0x00);
        mmix.write_tetra(0, 0xDF030100); // MXORI $3,$1,0
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(3), 0);
    }

    // Shift instruction tests
    #[test]
    fn test_sl() {
        let mut mmix = MMix::new();
        // SL: shift left - 0xFF << 4 = 0xFF0
        mmix.set_register(1, 0xFF);
        mmix.set_register(2, 4);
        mmix.write_tetra(0, 0x38030102); // SL $3,$1,$2
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(3), 0xFF0);
    }

    #[test]
    fn test_sli() {
        let mut mmix = MMix::new();
        // SLI: shift left immediate - 0x123 << 8 = 0x12300
        mmix.set_register(1, 0x123);
        mmix.write_tetra(0, 0x39030108); // SLI $3,$1,8
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(3), 0x12300);
    }

    #[test]
    fn test_sl_overflow() {
        let mut mmix = MMix::new();
        // SL with overflow: shifting out non-sign bits sets overflow
        mmix.set_register(1, 0x8000_0000_0000_0000);
        mmix.set_register(2, 1);
        mmix.write_tetra(0, 0x38030102); // SL $3,$1,$2
        assert!(mmix.execute_instruction());
        // Check that overflow bit is set in rA
        assert!((mmix.get_special(SpecialReg::RA) & RA_V) != 0);
    }

    #[test]
    fn test_sl_large_shift() {
        let mut mmix = MMix::new();
        // SL with shift >= 64 results in 0
        mmix.set_register(1, 0xFFFF_FFFF_FFFF_FFFF);
        mmix.set_register(2, 64);
        mmix.write_tetra(0, 0x38030102); // SL $3,$1,$2
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(3), 0);
    }

    #[test]
    fn test_slu() {
        let mut mmix = MMix::new();
        // SLU: shift left unsigned - no overflow check
        mmix.set_register(1, 0xFFFF_FFFF_FFFF_FFFF);
        mmix.set_register(2, 8);
        mmix.write_tetra(0, 0x3A030102); // SLU $3,$1,$2
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(3), 0xFFFF_FFFF_FFFF_FF00);
    }

    #[test]
    fn test_slui() {
        let mut mmix = MMix::new();
        // SLUI: shift left unsigned immediate
        mmix.set_register(1, 0x1);
        mmix.write_tetra(0, 0x3B030110); // SLUI $3,$1,16
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(3), 0x10000);
    }

    #[test]
    fn test_sr() {
        let mut mmix = MMix::new();
        // SR: arithmetic shift right - negative number stays negative
        mmix.set_register(1, 0xFFFF_FFFF_FFFF_FFF0u64); // -16 as u64
        mmix.set_register(2, 4);
        mmix.write_tetra(0, 0x3C030102); // SR $3,$1,$2
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(3), 0xFFFF_FFFF_FFFF_FFFFu64); // -1 as u64
    }

    #[test]
    fn test_sri() {
        let mut mmix = MMix::new();
        // SRI: arithmetic shift right immediate - positive number
        mmix.set_register(1, 0x1000);
        mmix.write_tetra(0, 0x3D030104); // SRI $3,$1,4
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(3), 0x100);
    }

    #[test]
    fn test_sr_large_shift_negative() {
        let mut mmix = MMix::new();
        // SR with large shift on negative number results in -1
        mmix.set_register(1, 0x8000_0000_0000_0000);
        mmix.set_register(2, 100);
        mmix.write_tetra(0, 0x3C030102); // SR $3,$1,$2
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(3), 0xFFFF_FFFF_FFFF_FFFF);
    }

    #[test]
    fn test_sr_large_shift_positive() {
        let mut mmix = MMix::new();
        // SR with large shift on positive number results in 0
        mmix.set_register(1, 0x7FFF_FFFF_FFFF_FFFF);
        mmix.set_register(2, 100);
        mmix.write_tetra(0, 0x3C030102); // SR $3,$1,$2
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(3), 0);
    }

    #[test]
    fn test_sru() {
        let mut mmix = MMix::new();
        // SRU: logical shift right - fills with zeros
        mmix.set_register(1, 0xFFFF_FFFF_FFFF_FFFF);
        mmix.set_register(2, 4);
        mmix.write_tetra(0, 0x3E030102); // SRU $3,$1,$2
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(3), 0x0FFF_FFFF_FFFF_FFFF);
    }

    #[test]
    fn test_srui() {
        let mut mmix = MMix::new();
        // SRUI: logical shift right immediate
        mmix.set_register(1, 0x8000_0000_0000_0000);
        mmix.write_tetra(0, 0x3F030101); // SRUI $3,$1,1
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(3), 0x4000_0000_0000_0000);
    }

    #[test]
    fn test_sru_large_shift() {
        let mut mmix = MMix::new();
        // SRU with shift >= 64 results in 0
        mmix.set_register(1, 0xFFFF_FFFF_FFFF_FFFF);
        mmix.set_register(2, 64);
        mmix.write_tetra(0, 0x3E030102); // SRU $3,$1,$2
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(3), 0);
    }

    #[test]
    fn test_bn_taken() {
        let mut mmix = MMix::new();
        // BN $1, 0, 5 - Branch if $1 is negative, offset = 5
        mmix.set_register(1, (-42i64) as u64);
        mmix.write_tetra(0, 0x40010005); // BN $1,0,5
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 20); // PC = 0 + 5*4 = 20
    }

    #[test]
    fn test_bn_not_taken() {
        let mut mmix = MMix::new();
        // BN $1, 0, 5 - Branch if $1 is negative, offset = 5
        mmix.set_register(1, 42);
        mmix.write_tetra(0, 0x40010005); // BN $1,0,5
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 4); // PC advances normally
    }

    #[test]
    fn test_bnb_taken() {
        let mut mmix = MMix::new();
        // BNB $1, 0xFFFD - Branch backward if $1 is negative (-3 tetras)
        mmix.set_pc(100);
        mmix.set_register(1, (-42i64) as u64);
        mmix.write_tetra(100, 0x4101FFFD); // BNB $1,0,0xFFFD (offset -3)
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 88); // PC = 100 + 4*(-3) = 88
    }

    #[test]
    fn test_bz_taken() {
        let mut mmix = MMix::new();
        // BZ $1, 0, 10 - Branch if $1 is zero
        mmix.set_register(1, 0);
        mmix.write_tetra(0, 0x4201000A); // BZ $1,0,10
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 40); // PC = 0 + 10*4 = 40
    }

    #[test]
    fn test_bz_not_taken() {
        let mut mmix = MMix::new();
        // BZ $1, 0, 10 - Branch if $1 is zero
        mmix.set_register(1, 1);
        mmix.write_tetra(0, 0x4201000A); // BZ $1,0,10
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_bzb_taken() {
        let mut mmix = MMix::new();
        // BZB $1, 0xFFFB - Branch backward if $1 is zero (-5 tetras)
        mmix.set_pc(100);
        mmix.set_register(1, 0);
        mmix.write_tetra(100, 0x4301FFFB); // BZB $1,0,0xFFFB (offset -5)
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 80); // PC = 100 + 4*(-5) = 80
    }

    #[test]
    fn test_bp_taken() {
        let mut mmix = MMix::new();
        // BP $1, 0, 8 - Branch if $1 is positive
        mmix.set_register(1, 42);
        mmix.write_tetra(0, 0x44010008); // BP $1,0,8
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 32); // PC = 0 + 8*4 = 32
    }

    #[test]
    fn test_bp_not_taken_zero() {
        let mut mmix = MMix::new();
        // BP $1, 0, 8 - Branch if $1 is positive (zero is not positive)
        mmix.set_register(1, 0);
        mmix.write_tetra(0, 0x44010008); // BP $1,0,8
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_bp_not_taken_negative() {
        let mut mmix = MMix::new();
        // BP $1, 0, 8 - Branch if $1 is positive
        mmix.set_register(1, (-1i64) as u64);
        mmix.write_tetra(0, 0x44010008); // BP $1,0,8
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_bpb_taken() {
        let mut mmix = MMix::new();
        // BPB $1, 0xFFFE - Branch backward if $1 is positive (-2 tetras)
        mmix.set_pc(200);
        mmix.set_register(1, 100);
        mmix.write_tetra(200, 0x4501FFFE); // BPB $1,0,0xFFFE (offset -2)
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 192); // PC = 200 + 4*(-2) = 192
    }

    #[test]
    fn test_bod_taken() {
        let mut mmix = MMix::new();
        // BOD $1, 0, 3 - Branch if $1 is odd
        mmix.set_register(1, 7);
        mmix.write_tetra(0, 0x46010003); // BOD $1,0,3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 12); // PC = 0 + 3*4 = 12
    }

    #[test]
    fn test_bod_not_taken() {
        let mut mmix = MMix::new();
        // BOD $1, 0, 3 - Branch if $1 is odd
        mmix.set_register(1, 8);
        mmix.write_tetra(0, 0x46010003); // BOD $1,0,3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_bodb_taken() {
        let mut mmix = MMix::new();
        // BODB $1, 0xFFFC - Branch backward if $1 is odd (-4 tetras)
        mmix.set_pc(100);
        mmix.set_register(1, 15);
        mmix.write_tetra(100, 0x4701FFFC); // BODB $1,0,0xFFFC (offset -4)
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 84); // PC = 100 + 4*(-4) = 84
    }

    #[test]
    fn test_bnn_taken_positive() {
        let mut mmix = MMix::new();
        // BNN $1, 0, 6 - Branch if $1 is non-negative (>= 0)
        mmix.set_register(1, 42);
        mmix.write_tetra(0, 0x48010006); // BNN $1,0,6
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 24); // PC = 0 + 6*4 = 24
    }

    #[test]
    fn test_bnn_taken_zero() {
        let mut mmix = MMix::new();
        // BNN $1, 0, 6 - Branch if $1 is non-negative (includes zero)
        mmix.set_register(1, 0);
        mmix.write_tetra(0, 0x48010006); // BNN $1,0,6
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 24);
    }

    #[test]
    fn test_bnn_not_taken() {
        let mut mmix = MMix::new();
        // BNN $1, 0, 6 - Branch if $1 is non-negative
        mmix.set_register(1, (-1i64) as u64);
        mmix.write_tetra(0, 0x48010006); // BNN $1,0,6
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_bnnb_taken() {
        let mut mmix = MMix::new();
        // BNNB $1, 0xFFFD - Branch backward if $1 is non-negative (-3 tetras)
        mmix.set_pc(100);
        mmix.set_register(1, 0);
        mmix.write_tetra(100, 0x4901FFFD); // BNNB $1,0,0xFFFD (offset -3)
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 88); // PC = 100 + 4*(-3) = 88
    }

    #[test]
    fn test_bnz_taken() {
        let mut mmix = MMix::new();
        // BNZ $1, 0, 7 - Branch if $1 is non-zero
        mmix.set_register(1, 1);
        mmix.write_tetra(0, 0x4A010007); // BNZ $1,0,7
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 28); // PC = 0 + 7*4 = 28
    }

    #[test]
    fn test_bnz_not_taken() {
        let mut mmix = MMix::new();
        // BNZ $1, 0, 7 - Branch if $1 is non-zero
        mmix.set_register(1, 0);
        mmix.write_tetra(0, 0x4A010007); // BNZ $1,0,7
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_bnzb_taken() {
        let mut mmix = MMix::new();
        // BNZB $1, 0xFFF6 - Branch backward if $1 is non-zero (-10 tetras)
        mmix.set_pc(200);
        mmix.set_register(1, 99);
        mmix.write_tetra(200, 0x4B01FFF6); // BNZB $1,0,0xFFF6 (offset -10)
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 160); // PC = 200 + 4*(-10) = 160
    }

    #[test]
    fn test_bnp_taken_negative() {
        let mut mmix = MMix::new();
        // BNP $1, 0, 4 - Branch if $1 is non-positive (<= 0)
        mmix.set_register(1, (-5i64) as u64);
        mmix.write_tetra(0, 0x4C010004); // BNP $1,0,4
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 16); // PC = 0 + 4*4 = 16
    }

    #[test]
    fn test_bnp_taken_zero() {
        let mut mmix = MMix::new();
        // BNP $1, 0, 4 - Branch if $1 is non-positive (includes zero)
        mmix.set_register(1, 0);
        mmix.write_tetra(0, 0x4C010004); // BNP $1,0,4
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 16);
    }

    #[test]
    fn test_bnp_not_taken() {
        let mut mmix = MMix::new();
        // BNP $1, 0, 4 - Branch if $1 is non-positive
        mmix.set_register(1, 1);
        mmix.write_tetra(0, 0x4C010004); // BNP $1,0,4
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_bnpb_taken() {
        let mut mmix = MMix::new();
        // BNPB $1, 0xFFFF - Branch backward if $1 is non-positive (-1 tetra)
        mmix.set_pc(100);
        mmix.set_register(1, 0);
        mmix.write_tetra(100, 0x4D01FFFF); // BNPB $1,0,0xFFFF (offset -1)
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 96); // PC = 100 + 4*(-1) = 96
    }

    #[test]
    fn test_bev_taken() {
        let mut mmix = MMix::new();
        // BEV $1, 0, 12 - Branch if $1 is even
        mmix.set_register(1, 8);
        mmix.write_tetra(0, 0x4E01000C); // BEV $1,0,12
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 48); // PC = 0 + 12*4 = 48
    }

    #[test]
    fn test_bev_not_taken() {
        let mut mmix = MMix::new();
        // BEV $1, 0, 12 - Branch if $1 is even
        mmix.set_register(1, 7);
        mmix.write_tetra(0, 0x4E01000C); // BEV $1,0,12
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_bevb_taken() {
        let mut mmix = MMix::new();
        // BEVB $1, 0xFFFE - Branch backward if $1 is even (-2 tetras)
        mmix.set_pc(100);
        mmix.set_register(1, 0);
        mmix.write_tetra(100, 0x4F01FFFE); // BEVB $1,0,0xFFFE (offset -2)
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 92); // PC = 100 + 4*(-2) = 92
    }

    #[test]
    fn test_pbn_taken() {
        let mut mmix = MMix::new();
        // PBN $1, 0, 5 - Probable branch if $1 is negative
        mmix.set_register(1, (-10i64) as u64);
        mmix.write_tetra(0, 0x50010005); // PBN $1,0,5
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 20); // PC = 0 + 5*4 = 20
    }

    #[test]
    fn test_pbnb_taken() {
        let mut mmix = MMix::new();
        // PBNB $1, 0xFFFD - Probable branch backward if $1 is negative (-3 tetras)
        mmix.set_pc(100);
        mmix.set_register(1, (-1i64) as u64);
        mmix.write_tetra(100, 0x5101FFFD); // PBNB $1,0,0xFFFD (offset -3)
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 88); // PC = 100 + 4*(-3) = 88
    }

    #[test]
    fn test_pbz_taken() {
        let mut mmix = MMix::new();
        // PBZ $1, 0, 6 - Probable branch if $1 is zero
        mmix.set_register(1, 0);
        mmix.write_tetra(0, 0x52010006); // PBZ $1,0,6
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 24); // PC = 0 + 6*4 = 24
    }

    #[test]
    fn test_pbzb_taken() {
        let mut mmix = MMix::new();
        // PBZB $1, 0xFFFC - Probable branch backward if $1 is zero (-4 tetras)
        mmix.set_pc(100);
        mmix.set_register(1, 0);
        mmix.write_tetra(100, 0x5301FFFC); // PBZB $1,0,0xFFFC (offset -4)
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 84); // PC = 100 + 4*(-4) = 84
    }

    #[test]
    fn test_pbp_taken() {
        let mut mmix = MMix::new();
        // PBP $1, 0, 8 - Probable branch if $1 is positive
        mmix.set_register(1, 50);
        mmix.write_tetra(0, 0x54010008); // PBP $1,0,8
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 32); // PC = 0 + 8*4 = 32
    }

    #[test]
    fn test_pbpb_taken() {
        let mut mmix = MMix::new();
        // PBPB $1, 0xFFFE - Probable branch backward if $1 is positive (-2 tetras)
        mmix.set_pc(100);
        mmix.set_register(1, 1);
        mmix.write_tetra(100, 0x5501FFFE); // PBPB $1,0,0xFFFE (offset -2)
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 92); // PC = 100 + 4*(-2) = 92
    }

    #[test]
    fn test_pbod_taken() {
        let mut mmix = MMix::new();
        // PBOD $1, 0, 3 - Probable branch if $1 is odd
        mmix.set_register(1, 11);
        mmix.write_tetra(0, 0x56010003); // PBOD $1,0,3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 12); // PC = 0 + 3*4 = 12
    }

    #[test]
    fn test_pbodb_taken() {
        let mut mmix = MMix::new();
        // PBODB $1, 0xFFFB - Probable branch backward if $1 is odd (-5 tetras)
        mmix.set_pc(100);
        mmix.set_register(1, 99);
        mmix.write_tetra(100, 0x5701FFFB); // PBODB $1,0,0xFFFB (offset -5)
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 80); // PC = 100 + 4*(-5) = 80
    }

    #[test]
    fn test_pbnn_taken() {
        let mut mmix = MMix::new();
        // PBNN $1, 0, 7 - Probable branch if $1 is non-negative
        mmix.set_register(1, 100);
        mmix.write_tetra(0, 0x58010007); // PBNN $1,0,7
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 28); // PC = 0 + 7*4 = 28
    }

    #[test]
    fn test_pbnnb_taken() {
        let mut mmix = MMix::new();
        // PBNNB $1, 0xFFFF - Probable branch backward if $1 is non-negative (-1 tetra)
        mmix.set_pc(100);
        mmix.set_register(1, 0);
        mmix.write_tetra(100, 0x5901FFFF); // PBNNB $1,0,0xFFFF (offset -1)
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 96); // PC = 100 + 4*(-1) = 96
    }

    #[test]
    fn test_pbnz_taken() {
        let mut mmix = MMix::new();
        // PBNZ $1, 0, 9 - Probable branch if $1 is non-zero
        mmix.set_register(1, 42);
        mmix.write_tetra(0, 0x5A010009); // PBNZ $1,0,9
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 36); // PC = 0 + 9*4 = 36
    }

    #[test]
    fn test_pbnzb_taken() {
        let mut mmix = MMix::new();
        // PBNZB $1, 0xFFFA - Probable branch backward if $1 is non-zero (-6 tetras)
        mmix.set_pc(200);
        mmix.set_register(1, 1);
        mmix.write_tetra(200, 0x5B01FFFA); // PBNZB $1,0,0xFFFA (offset -6)
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 176); // PC = 200 + 4*(-6) = 176
    }

    #[test]
    fn test_pbnp_taken() {
        let mut mmix = MMix::new();
        // PBNP $1, 0, 4 - Probable branch if $1 is non-positive
        mmix.set_register(1, (-100i64) as u64);
        mmix.write_tetra(0, 0x5C010004); // PBNP $1,0,4
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 16); // PC = 0 + 4*4 = 16
    }

    #[test]
    fn test_pbnpb_taken() {
        let mut mmix = MMix::new();
        // PBNPB $1, 0xFFF8 - Probable branch backward if $1 is non-positive (-8 tetras)
        mmix.set_pc(100);
        mmix.set_register(1, 0);
        mmix.write_tetra(100, 0x5D01FFF8); // PBNPB $1,0,0xFFF8 (offset -8)
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 68); // PC = 100 + 4*(-8) = 68
    }

    #[test]
    fn test_pbev_taken() {
        let mut mmix = MMix::new();
        // PBEV $1, 0, 10 - Probable branch if $1 is even
        mmix.set_register(1, 100);
        mmix.write_tetra(0, 0x5E01000A); // PBEV $1,0,10
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 40); // PC = 0 + 10*4 = 40
    }

    #[test]
    fn test_pbevb_taken() {
        let mut mmix = MMix::new();
        // PBEVB $1, 0xFFF9 - Probable branch backward if $1 is even (-7 tetras)
        mmix.set_pc(100);
        mmix.set_register(1, 0);
        mmix.write_tetra(100, 0x5F01FFF9); // PBEVB $1,0,0xFFF9 (offset -7)
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 72); // PC = 100 + 4*(-7) = 72
    }

    #[test]
    fn test_jmp_forward() {
        let mut mmix = MMix::new();
        // JMP +10 (offset = 10)
        mmix.write_tetra(0, 0xF000000A); // JMP 0,0,10
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 40); // PC = 0 + 10*4 = 40
    }

    #[test]
    fn test_jmp_large_forward_offset() {
        let mut mmix = MMix::new();
        mmix.set_pc(100);
        // XYZ is unsigned in the forward opcode: 0xFFFFFB is 16777211, not -5.
        mmix.write_tetra(100, 0xF0FFFFFB); // JMP 0xFFFFFB
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 100 + 16777211 * 4); // PC = 100 + 16777211*4
    }

    #[test]
    fn test_jmpb() {
        let mut mmix = MMix::new();
        mmix.set_pc(100);
        // XYZ = 0xFFFFFB is 0xFFFFFB - 2^24 = -5 tetras.
        mmix.write_tetra(100, 0xF1FFFFFB); // JMPB 0xFFFFFB
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 80); // PC = 100 - 5*4 = 80
    }

    #[test]
    fn test_pushj() {
        let mut mmix = MMix::new();
        // PUSHJ $0, 0, 10 - Push and jump to relative offset 10
        mmix.write_tetra(0, 0xF200000A); // PUSHJ $0,0,10
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 40); // PC = 0 + 10*4 = 40
        assert_eq!(mmix.get_special(SpecialReg::RJ), 4); // Return address saved
    }

    #[test]
    fn test_pushjb() {
        let mut mmix = MMix::new();
        mmix.set_pc(100);
        // YZ = 0xFFFB is 0xFFFB - 65536 = -5 tetras.
        mmix.write_tetra(100, 0xF300FFFB); // PUSHJB $0,0xFFFB
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 80); // PC = 100 - 5*4 = 80
        assert_eq!(mmix.get_special(SpecialReg::RJ), 104); // Return address saved
    }

    #[test]
    fn test_geta() {
        let mut mmix = MMix::new();
        mmix.set_pc(100);
        // GETA $1, 0, 10 - Get address at relative offset 10
        mmix.write_tetra(100, 0xF401000A); // GETA $1,0,10
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 140); // Addr = 100 + 10*4 = 140
        assert_eq!(mmix.get_pc(), 104); // PC advances normally
    }

    #[test]
    fn test_getab() {
        let mut mmix = MMix::new();
        mmix.set_pc(100);
        // YZ = 0xFFFB is 0xFFFB - 65536 = -5 tetras.
        mmix.write_tetra(100, 0xF501FFFB); // GETAB $1,0xFFFB
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 80); // Addr = 100 - 5*4 = 80
        assert_eq!(mmix.get_pc(), 104);
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

    #[test]
    fn pop_returns_a_callee_computed_remainder_to_the_hole() {
        // Each case: dividend, and its Euclidean remainder mod 100.
        let cases = [
            (42, 42),
            (142, 42),
            (-58, 42),
            (-194, 6),
            (0, 0),
            (100, 0),
            (-100, 0),
            (-1, 99),
        ];
        for (dividend, expected) in cases {
            let source = format!(
                "\
\tLOC\t#100
Main\tSETI\t$1,{dividend}
\tSET\t$2,100
\tPUSHJ\t$0,RemEuclid
\tSET\t$255,$0
\tTRAP\t0,Halt,0
RemEuclid\tDIV\t$2,$0,$1
\tMUL\t$3,$2,$1
\tSUB\t$0,$0,$3
\tBNN\t$0,Done
\tADDU\t$0,$0,$1
Done\tPOP\t1,0
"
            );
            assert_eq!(run_to_halt(&source), expected, "{dividend} mod 100");
        }
    }

    #[test]
    fn pushjb_reaches_a_backward_callee() {
        // The PUSHJB sits ten tetras past AddFunc, so YZ is 65536 - 10.
        // Read as a magnitude that lands in zeroed memory and halts with 0.
        let source = "\
\tLOC\t#100
AddFunc\tADDU\t$0,$0,$1
\tPOP\t1,0
Main\tSETI\t$1,40
\tSETI\t$2,2
\tPUSHJB\t$0,AddFunc
\tSET\t$255,$0
\tTRAP\t0,Halt,0
";
        assert_eq!(run_to_halt(source), 42);
    }

    #[test]
    fn forward_branch_past_half_the_field_still_goes_forward() {
        // BZ's YZ is 32768; sign extension reads it as -32768.
        let source = "\
\tLOC\t#100
Start\tSETI\t$1,0
\tBZ\t$1,Far
\tSETI\t$255,1
\tTRAP\t0,Halt,0
\tLOC\t#20110
Far\tSETI\t$255,42
\tTRAP\t0,Halt,0
";
        assert_eq!(run_to_halt(source), 42);
    }

    #[test]
    fn backward_branch_past_half_the_field_still_goes_backward() {
        // BZB's YZ is 32764, which is 32764 - 65536 = -32772 tetras.
        // Sign extension reads it as +32764.
        let source = "\
\tLOC\t#100
Back\tSETI\t$255,42
\tTRAP\t0,Halt,0
\tLOC\t#20100
Main\tSETI\t$1,0
\tBZB\t$1,Back
\tSETI\t$255,1
\tTRAP\t0,Halt,0
";
        assert_eq!(run_to_halt(source), 42);
    }

    #[test]
    fn pushj_forward_past_half_the_field_still_goes_forward() {
        // PUSHJ's YZ is 32772; sign extension reads it as -32764.
        let source = "\
\tLOC\t#100
Main\tSETI\t$1,40
\tPUSHJ\t$0,Far
\tSET\t$255,$0
\tTRAP\t0,Halt,0
\tLOC\t#20120
Far\tSETI\t$0,42
\tPOP\t1,0
";
        assert_eq!(run_to_halt(source), 42);
    }

    #[test]
    fn incl_adds_its_immediate() {
        let source = "\
\tLOC\t#100
Main\tSETI\t$1,100
\tINCL\t$1,#203
\tSET\t$255,$1
\tTRAP\t0,Halt,0
";
        assert_eq!(run_to_halt(source), 100 + 0x203);
    }

    #[test]
    fn test_geta_forward_field_above_half_the_range() {
        let mut mmix = MMix::new();
        mmix.set_pc(0x100);
        // YZ = 0xC000 is 49152 tetras forward, not -16384.
        mmix.write_tetra(0x100, 0xF401C000); // GETA $1,0xC000
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 0x100 + 49152 * 4);
    }

    #[test]
    fn test_jmpb_small_field_is_a_far_backward_jump() {
        let mut mmix = MMix::new();
        mmix.set_pc(0x4000000);
        // XYZ = 5 is 5 - 2^24 tetras, the far end of JMPB's reach.
        mmix.write_tetra(0x4000000, 0xF1000005); // JMPB 5
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 0x4000000 - (16777216 - 5) * 4);
    }

    #[test]
    fn test_getab_small_field_is_a_far_backward_address() {
        let mut mmix = MMix::new();
        mmix.set_pc(0x40000);
        // YZ = 5 is 5 - 65536 tetras, the far end of GETAB's reach.
        mmix.write_tetra(0x40000, 0xF5010005); // GETAB $1,5
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 0x40000 - (65536 - 5) * 4);
    }

    #[test]
    fn test_pop_frame_yz_above_half_the_field_resumes_forward() {
        let mut mmix = MMix::new();
        mmix.set_pc(0x100);
        mmix.write_tetra(0x100, 0xF2000004); // PUSHJ $0,4 -> 0x110, rJ = 0x104
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_special(SpecialReg::RJ), 0x104);
        // POP resumes at rJ + 4*YZ, unsigned: 0x8000 is forward, not -32768.
        mmix.write_tetra(0x110, 0xF8008000); // POP 0,0x8000
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 0x104 + 32768 * 4);
    }

    #[test]
    fn test_pop_without_a_frame_resumes_forward_from_rj() {
        let mut mmix = MMix::new();
        mmix.set_pc(0x100);
        mmix.set_special(SpecialReg::RJ, 0x200);
        mmix.write_tetra(0x100, 0xF8008000); // POP 0,0x8000
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 0x200 + 32768 * 4);
    }

    #[test]
    fn test_put_get() {
        let mut mmix = MMix::new();
        // PUT rR, $1 - Put value from $1 into rR (special register 6)
        mmix.set_register(1, 0x123456789ABCDEF0);
        mmix.write_tetra(0, 0xF6060001); // PUT X=6 (rR), Y=0, Z=1 ($1)
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_special(SpecialReg::RR), 0x123456789ABCDEF0);

        // GET $2, rR - Get value from rR into $2
        mmix.write_tetra(4, 0xFE020006); // GET X=2 ($2), Y=0, Z=6 (rR)
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(2), 0x123456789ABCDEF0);
    }

    #[test]
    fn test_puti_ignores_y_and_stores_z_alone() {
        let mut mmix = MMix::new();
        // PUTI rH, YZ=0x1234 - Y is ignored; only Z=0x34 reaches rH.
        mmix.write_tetra(0, 0xF7031234); // PUTI X=3 (rH), Y=0x12, Z=0x34
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_special(SpecialReg::RH), 0x34);
    }

    #[test]
    fn test_put_x_at_32_names_no_special_register() {
        let (host, handle) = CaptureHost::new();
        let mut mmix = MMix::with_host(host);
        mmix.set_register(1, 42);
        mmix.write_tetra(0, 0xF6200001); // PUT X=32,$1 -- no register above 31
        assert!(!mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 0, "the PC stays on the rejected instruction");
        assert_eq!(handle.diagnostics().len(), 1);
    }

    #[test]
    fn test_put_rc_is_rejected_with_a_privileged_operation_interrupt() {
        let (host, handle) = CaptureHost::new();
        let mut mmix = MMix::with_host(host);
        mmix.set_register(1, 222);
        mmix.write_tetra(0, 0xF6080001); // PUT X=8 (rC), $1
        assert!(!mmix.execute_instruction());
        assert_eq!(
            mmix.get_special(SpecialReg::RC),
            0,
            "the write did not land"
        );
        assert_eq!(mmix.get_pc(), 0, "the PC stays on the rejected instruction");
        assert_eq!(handle.diagnostics().len(), 1);
        assert!(handle.diagnostics()[0].contains("rC"));
        assert!(handle.diagnostics()[0].contains("privileged"));
    }

    #[test]
    fn test_put_rn_is_rejected_with_an_illegal_instruction_interrupt() {
        let (host, handle) = CaptureHost::new();
        let mut mmix = MMix::with_host(host);
        mmix.set_special(SpecialReg::RN, 111);
        mmix.set_register(1, 222);
        mmix.write_tetra(0, 0xF6090001); // PUT X=9 (rN), $1
        assert!(!mmix.execute_instruction());
        assert_eq!(
            mmix.get_special(SpecialReg::RN),
            111,
            "the write did not land"
        );
        assert_eq!(mmix.get_pc(), 0, "the PC stays on the rejected instruction");
        assert_eq!(handle.diagnostics().len(), 1);
        assert!(handle.diagnostics()[0].contains("rN"));
    }

    #[test]
    fn test_put_ro_is_rejected_with_an_illegal_instruction_interrupt() {
        let (host, handle) = CaptureHost::new();
        let mut mmix = MMix::with_host(host);
        let ro = mmix.get_special(SpecialReg::RO);
        mmix.set_register(1, 222);
        mmix.write_tetra(0, 0xF60A0001); // PUT X=10 (rO), $1
        assert!(!mmix.execute_instruction());
        assert_eq!(
            mmix.get_special(SpecialReg::RO),
            ro,
            "the write did not land"
        );
        assert_eq!(mmix.get_pc(), 0, "the PC stays on the rejected instruction");
        assert_eq!(handle.diagnostics().len(), 1);
        assert!(handle.diagnostics()[0].contains("rO"));
    }

    #[test]
    fn test_put_rs_is_rejected_with_an_illegal_instruction_interrupt() {
        let (host, handle) = CaptureHost::new();
        let mut mmix = MMix::with_host(host);
        let rs = mmix.get_special(SpecialReg::RS);
        mmix.set_register(1, 222);
        mmix.write_tetra(0, 0xF60B0001); // PUT X=11 (rS), $1
        assert!(!mmix.execute_instruction());
        assert_eq!(
            mmix.get_special(SpecialReg::RS),
            rs,
            "the write did not land"
        );
        assert_eq!(mmix.get_pc(), 0, "the PC stays on the rejected instruction");
        assert_eq!(handle.diagnostics().len(), 1);
        assert!(handle.diagnostics()[0].contains("rS"));
    }

    #[test]
    fn test_put_rg_below_32_is_rejected() {
        let (host, handle) = CaptureHost::new();
        let mut mmix = MMix::with_host(host);
        mmix.set_register(1, 31);
        mmix.write_tetra(0, 0xF6130001); // PUT X=19 (rG), $1
        assert!(!mmix.execute_instruction());
        assert_eq!(mmix.get_special(SpecialReg::RG), 32, "rG unchanged");
        assert_eq!(mmix.get_pc(), 0, "the PC stays on the rejected instruction");
        assert_eq!(handle.diagnostics().len(), 1);
        assert!(handle.diagnostics()[0].contains("rG"));
    }

    #[test]
    fn test_put_rg_below_rl_is_rejected() {
        let (host, handle) = CaptureHost::new();
        let mut mmix = MMix::with_host(host);
        mmix.set_special(SpecialReg::RL, 40);
        mmix.set_register(1, 35); // >= 32, but < rL
        mmix.write_tetra(0, 0xF6130001); // PUT X=19 (rG), $1
        assert!(!mmix.execute_instruction());
        assert_eq!(mmix.get_special(SpecialReg::RG), 32, "rG unchanged");
        assert_eq!(mmix.get_pc(), 0, "the PC stays on the rejected instruction");
        assert_eq!(handle.diagnostics().len(), 1);
    }

    #[test]
    fn test_put_rg_above_255_is_rejected() {
        let (host, handle) = CaptureHost::new();
        let mut mmix = MMix::with_host(host);
        mmix.set_register(1, 256);
        mmix.write_tetra(0, 0xF6130001); // PUT X=19 (rG), $1
        assert!(!mmix.execute_instruction());
        assert_eq!(mmix.get_special(SpecialReg::RG), 32, "rG unchanged");
        assert_eq!(mmix.get_pc(), 0, "the PC stays on the rejected instruction");
        assert_eq!(handle.diagnostics().len(), 1);
    }

    #[test]
    fn test_put_rg_accepts_the_low_boundary_32() {
        let mut mmix = MMix::new();
        mmix.set_special(SpecialReg::RG, 60);
        mmix.set_register(1, 32);
        mmix.write_tetra(0, 0xF6130001); // PUT X=19 (rG), $1
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_special(SpecialReg::RG), 32);
    }

    #[test]
    fn test_put_rg_accepts_the_high_boundary_255() {
        let mut mmix = MMix::new();
        mmix.set_register(1, 255);
        mmix.write_tetra(0, 0xF6130001); // PUT X=19 (rG), $1
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_special(SpecialReg::RG), 255);
    }

    #[test]
    fn test_put_rg_accepts_a_value_equal_to_rl() {
        let mut mmix = MMix::new();
        mmix.set_special(SpecialReg::RL, 40);
        mmix.set_register(1, 40);
        mmix.write_tetra(0, 0xF6130001); // PUT X=19 (rG), $1
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_special(SpecialReg::RG), 40);
    }

    #[test]
    fn test_put_rg_raising_zeroes_the_newly_local_span() {
        let mut mmix = MMix::new();
        mmix.set_register(40, 0xDEAD); // global while rG = 32
        mmix.set_register(70, 777); // outside the raised span; stays global

        mmix.write_tetra(0, 0xF713003C); // PUTI rG,60
        assert!(mmix.execute_instruction());

        assert_eq!(mmix.get_special(SpecialReg::RG), 60);
        assert_eq!(
            mmix.get_register(40),
            0,
            "reclassified into the local/marginal range reads zero"
        );
        assert_eq!(
            mmix.get_register(70),
            777,
            "outside the span keeps its value"
        );
    }

    #[test]
    fn test_put_rg_lowering_zeroes_the_newly_global_span() {
        let mut mmix = MMix::new();
        mmix.set_register(40, 0xDEAD); // global while rG = 32
        mmix.set_special(SpecialReg::RG, 60); // raw raise: $40 goes stale-marginal
        mmix.set_register(70, 777); // outside the lowered span; stays global

        mmix.write_tetra(0, 0xF7130020); // PUTI rG,32
        assert!(mmix.execute_instruction());

        assert_eq!(mmix.get_special(SpecialReg::RG), 32);
        assert_eq!(
            mmix.get_register(40),
            0,
            "reclassified back into the global range reads zero"
        );
        assert_eq!(
            mmix.get_register(70),
            777,
            "outside the span keeps its value"
        );
    }

    #[test]
    fn test_put_rg_zeroes_a_reclassified_register_end_to_end() {
        // SET $40,#DEAD / PUT rG,60 / PUT rG,32 / ADD $5,$40,$0 -- C6's
        // adversarial review (2026-09-17) found this left $5 = #DEAD;
        // MMIXware gives 0.
        let mut mmix = MMix::new();
        mmix.set_register(40, 0xDEAD); // global while rG = 32

        mmix.write_tetra(0, 0xF713003C); // PUTI rG,60
        assert!(mmix.execute_instruction());
        mmix.write_tetra(4, 0xF7130020); // PUTI rG,32
        assert!(mmix.execute_instruction());
        mmix.write_tetra(8, 0x20052800); // ADD $5,$40,$0
        assert!(mmix.execute_instruction());

        assert_eq!(mmix.get_register(5), 0);
    }

    #[test]
    fn test_set_register_zeros_a_stale_marginal_gap() {
        // Raising rG is a raw write with no rL rules of its own, so it can
        // leave a stale value in a register that becomes marginal. Writing
        // a higher register must still zero it out when the write claims
        // the range.
        let mut mmix = MMix::new();
        mmix.set_register(40, 0xDEAD); // global while rG = 32
        mmix.set_special(SpecialReg::RG, 50); // $40 is now marginal, still 0xDEAD

        mmix.set_register(45, 123);

        assert_eq!(mmix.get_register(40), 0, "the gap zeros out");
        assert_eq!(mmix.get_register(45), 123);
        assert_eq!(mmix.get_special(SpecialReg::RL), 46);
    }

    #[test]
    fn test_put_rl_with_a_larger_z_leaves_rl_unchanged() {
        let mut mmix = MMix::new();
        mmix.set_special(SpecialReg::RL, 5);

        // PUTI rL,10 - z > rL, so rL stays at min(10, 5) = 5.
        mmix.write_tetra(0, 0xF714000A);
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_special(SpecialReg::RL), 5);
    }

    #[test]
    fn test_put_rl_zeros_the_registers_it_drops() {
        let mut mmix = MMix::new();
        mmix.set_special(SpecialReg::RL, 6);
        mmix.set_register(4, 777);

        // PUTI rL,3 - drops $3..$5 out of the local range.
        mmix.write_tetra(0, 0xF7140003);
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_special(SpecialReg::RL), 3);

        // ADD $1,$4,$0 reads the dropped $4 with no intervening write: zero.
        // $1 is already local (1 < 3), so its own destination rise is a
        // no-op and does not confound this read.
        mmix.write_tetra(4, 0x20010400);
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 0);
    }

    #[test]
    fn test_put_rl_survives_an_rl_beyond_the_register_file() {
        let mut mmix = MMix::new();
        mmix.set_special(SpecialReg::RL, u64::MAX);
        mmix.set_register(200, 42); // global at rG = 32

        // PUTI rL,3
        mmix.write_tetra(0, 0xF7140003);
        assert!(mmix.execute_instruction());

        assert_eq!(mmix.get_special(SpecialReg::RL), 3);
        assert_eq!(mmix.get_register(200), 42, "a global is not a local");
    }

    /// `UNSAVE` can no longer produce an rG/rL pair this far out of range —
    /// its own validation now refuses a packed rG above 255 or a local
    /// count above the packed rG — so this state is planted directly
    /// through `set_special`, which stays raw by design.
    #[test]
    fn test_put_rl_after_an_out_of_range_state_naming_more_registers_than_the_file_holds() {
        let mut mmix = MMix::new();
        mmix.set_special(SpecialReg::RG, 1000); // beyond the register file
        mmix.set_special(SpecialReg::RL, 500);

        // PUTI rL,3
        mmix.write_tetra(0, 0xF7140003);
        assert!(mmix.execute_instruction());

        assert_eq!(mmix.get_special(SpecialReg::RL), 3);
    }

    /// UNSAVE restores rG from guest memory, so rG can name a register the
    /// file does not have. `claim_local` must compare against it at full
    /// width: a byte-narrowed comparison turns a real local, sitting below
    /// the true rG but above its truncated low byte, into a global that is
    /// never claimed. `PUT rG` carries no validation yet, so this state is
    /// reachable today; C7's validation may close it later.
    #[test]
    fn test_register_claims_local_when_rg_names_no_real_register() {
        let mut mmix = MMix::new();
        mmix.set_register(150, 0xDEAD); // global while rG = 32 (the default)
        mmix.set_special(SpecialReg::RG, 300); // beyond the register file
        mmix.set_special(SpecialReg::RL, 3);

        // $150 is now marginal (3 <= 150 < 300); claiming $200 sweeps it.
        mmix.set_register(200, 777);

        assert_eq!(mmix.get_special(SpecialReg::RL), 201);
        assert_eq!(
            mmix.get_register(150),
            0,
            "the claim zeroed the marginal range"
        );
        assert_eq!(mmix.get_register(200), 777);
    }

    /// `SAVE`'s own X < rG rejection, and proof that `writes_general_register_x`'s
    /// pre-claim (real for every other destination-writing opcode) is
    /// skipped for `SAVE`: were it not, claiming a marginal $45 here would
    /// raise rL and zero $40 before this arm ever ran.
    #[test]
    fn test_save_rejects_a_destination_below_rg_leaving_the_machine_unchanged() {
        let (host, handle) = CaptureHost::new();
        let mut mmix = MMix::with_host(host);
        mmix.set_register(40, 0xDEAD); // global while rG = 32
        mmix.set_special(SpecialReg::RG, 50);
        mmix.set_special(SpecialReg::RL, 3);

        // SAVE $45,0 - $45 < rG (50): a local, rejected.
        mmix.write_tetra(0, 0xFA2D0000);
        assert!(!mmix.execute_instruction());

        assert_eq!(mmix.get_pc(), 0, "the PC stays on the rejected instruction");
        assert_eq!(mmix.get_special(SpecialReg::RL), 3, "rL is untouched");
        assert_eq!(
            mmix.get_register(40),
            0xDEAD,
            "the destination-rise pre-claim never ran"
        );
        assert_eq!(mmix.get_register(45), 0, "the rejected SAVE wrote nothing");
        assert_eq!(handle.diagnostics().len(), 1);
        assert!(handle.diagnostics()[0].contains("SAVE"));
    }

    /// `writes_general_register_x` decides, for every opcode byte, whether
    /// the instruction's X field becomes a live local before the
    /// instruction runs. This table is derived independently from
    /// `MMIX.md` and from `Opcode`, never from the predicate itself, so a
    /// single misclassified byte fails as itself rather than surviving in
    /// an aggregate comparison.
    #[test]
    fn test_destination_predicate_matches_an_independent_table_at_every_byte() {
        use crate::mmixal::Opcode;

        // (byte, mnemonic, is a general-register destination before the
        // instruction runs). PRELD, PREGO, PREST, SYNCD and SYNCID parse X
        // as a register in checksmix but take an immediate byte count in
        // the specification, so all ten forms are false. PUSHJ and PUSHGO
        // write nothing to $X directly, but $X is the call hole and must be
        // local first, so both are true. PUT's X names a special register,
        // POP's X is a count, UNSAVE's X is fixed at 0, and TRAP/TRIP/SYNC/
        // SWYM/RESUME/JMP/JMPB/HALT carry no general-register operand in X.
        let table: [(u8, Opcode, bool); 256] = [
            (0x00, Opcode::TRAP, false),
            (0x01, Opcode::FCMP, true),
            (0x02, Opcode::FUN, true),
            (0x03, Opcode::FEQL, true),
            (0x04, Opcode::FADD, true),
            (0x05, Opcode::FIX, true),
            (0x06, Opcode::FSUB, true),
            (0x07, Opcode::FIXU, true),
            (0x08, Opcode::FLOT, true),
            (0x09, Opcode::FLOTI, true),
            (0x0A, Opcode::FLOTU, true),
            (0x0B, Opcode::FLOTUI, true),
            (0x0C, Opcode::SFLOT, true),
            (0x0D, Opcode::SFLOTI, true),
            (0x0E, Opcode::SFLOTU, true),
            (0x0F, Opcode::SFLOTUI, true),
            (0x10, Opcode::FMUL, true),
            (0x11, Opcode::FCMPE, true),
            (0x12, Opcode::FUNE, true),
            (0x13, Opcode::FEQLE, true),
            (0x14, Opcode::FDIV, true),
            (0x15, Opcode::FSQRT, true),
            (0x16, Opcode::FREM, true),
            (0x17, Opcode::FINT, true),
            (0x18, Opcode::MUL, true),
            (0x19, Opcode::MULI, true),
            (0x1A, Opcode::MULU, true),
            (0x1B, Opcode::MULUI, true),
            (0x1C, Opcode::DIV, true),
            (0x1D, Opcode::DIVI, true),
            (0x1E, Opcode::DIVU, true),
            (0x1F, Opcode::DIVUI, true),
            (0x20, Opcode::ADD, true),
            (0x21, Opcode::ADDI, true),
            (0x22, Opcode::ADDU, true),
            (0x23, Opcode::ADDUI, true),
            (0x24, Opcode::SUB, true),
            (0x25, Opcode::SUBI, true),
            (0x26, Opcode::SUBU, true),
            (0x27, Opcode::SUBUI, true),
            (0x28, Opcode::ADDU2, true),
            (0x29, Opcode::ADDU2I, true),
            (0x2A, Opcode::ADDU4, true),
            (0x2B, Opcode::ADDU4I, true),
            (0x2C, Opcode::ADDU8, true),
            (0x2D, Opcode::ADDU8I, true),
            (0x2E, Opcode::ADDU16, true),
            (0x2F, Opcode::ADDU16I, true),
            (0x30, Opcode::CMP, true),
            (0x31, Opcode::CMPI, true),
            (0x32, Opcode::CMPU, true),
            (0x33, Opcode::CMPUI, true),
            (0x34, Opcode::NEG, true),
            (0x35, Opcode::NEGI, true),
            (0x36, Opcode::NEGU, true),
            (0x37, Opcode::NEGUI, true),
            (0x38, Opcode::SL, true),
            (0x39, Opcode::SLI, true),
            (0x3A, Opcode::SLU, true),
            (0x3B, Opcode::SLUI, true),
            (0x3C, Opcode::SR, true),
            (0x3D, Opcode::SRI, true),
            (0x3E, Opcode::SRU, true),
            (0x3F, Opcode::SRUI, true),
            (0x40, Opcode::BN, false),
            (0x41, Opcode::BNB, false),
            (0x42, Opcode::BZ, false),
            (0x43, Opcode::BZB, false),
            (0x44, Opcode::BP, false),
            (0x45, Opcode::BPB, false),
            (0x46, Opcode::BOD, false),
            (0x47, Opcode::BODB, false),
            (0x48, Opcode::BNN, false),
            (0x49, Opcode::BNNB, false),
            (0x4A, Opcode::BNZ, false),
            (0x4B, Opcode::BNZB, false),
            (0x4C, Opcode::BNP, false),
            (0x4D, Opcode::BNPB, false),
            (0x4E, Opcode::BEV, false),
            (0x4F, Opcode::BEVB, false),
            (0x50, Opcode::PBN, false),
            (0x51, Opcode::PBNB, false),
            (0x52, Opcode::PBZ, false),
            (0x53, Opcode::PBZB, false),
            (0x54, Opcode::PBP, false),
            (0x55, Opcode::PBPB, false),
            (0x56, Opcode::PBOD, false),
            (0x57, Opcode::PBODB, false),
            (0x58, Opcode::PBNN, false),
            (0x59, Opcode::PBNNB, false),
            (0x5A, Opcode::PBNZ, false),
            (0x5B, Opcode::PBNZB, false),
            (0x5C, Opcode::PBNP, false),
            (0x5D, Opcode::PBNPB, false),
            (0x5E, Opcode::PBEV, false),
            (0x5F, Opcode::PBEVB, false),
            (0x60, Opcode::CSN, true),
            (0x61, Opcode::CSNI, true),
            (0x62, Opcode::CSZ, true),
            (0x63, Opcode::CSZI, true),
            (0x64, Opcode::CSP, true),
            (0x65, Opcode::CSPI, true),
            (0x66, Opcode::CSOD, true),
            (0x67, Opcode::CSODI, true),
            (0x68, Opcode::CSNN, true),
            (0x69, Opcode::CSNNI, true),
            (0x6A, Opcode::CSNZ, true),
            (0x6B, Opcode::CSNZI, true),
            (0x6C, Opcode::CSNP, true),
            (0x6D, Opcode::CSNPI, true),
            (0x6E, Opcode::CSEV, true),
            (0x6F, Opcode::CSEVI, true),
            (0x70, Opcode::ZSN, true),
            (0x71, Opcode::ZSNI, true),
            (0x72, Opcode::ZSZ, true),
            (0x73, Opcode::ZSZI, true),
            (0x74, Opcode::ZSP, true),
            (0x75, Opcode::ZSPI, true),
            (0x76, Opcode::ZSOD, true),
            (0x77, Opcode::ZSODI, true),
            (0x78, Opcode::ZSNN, true),
            (0x79, Opcode::ZSNNI, true),
            (0x7A, Opcode::ZSNZ, true),
            (0x7B, Opcode::ZSNZI, true),
            (0x7C, Opcode::ZSNP, true),
            (0x7D, Opcode::ZSNPI, true),
            (0x7E, Opcode::ZSEV, true),
            (0x7F, Opcode::ZSEVI, true),
            (0x80, Opcode::LDB, true),
            (0x81, Opcode::LDBI, true),
            (0x82, Opcode::LDBU, true),
            (0x83, Opcode::LDBUI, true),
            (0x84, Opcode::LDW, true),
            (0x85, Opcode::LDWI, true),
            (0x86, Opcode::LDWU, true),
            (0x87, Opcode::LDWUI, true),
            (0x88, Opcode::LDT, true),
            (0x89, Opcode::LDTI, true),
            (0x8A, Opcode::LDTU, true),
            (0x8B, Opcode::LDTUI, true),
            (0x8C, Opcode::LDO, true),
            (0x8D, Opcode::LDOI, true),
            (0x8E, Opcode::LDOU, true),
            (0x8F, Opcode::LDOUI, true),
            (0x90, Opcode::LDSF, true),
            (0x91, Opcode::LDSFI, true),
            (0x92, Opcode::LDHT, true),
            (0x93, Opcode::LDHTI, true),
            (0x94, Opcode::CSWAP, true),
            (0x95, Opcode::CSWAPI, true),
            (0x96, Opcode::LDUNC, true),
            (0x97, Opcode::LDUNCI, true),
            (0x98, Opcode::LDVTS, true),
            (0x99, Opcode::LDVTSI, true),
            (0x9A, Opcode::PRELD, false),
            (0x9B, Opcode::PRELDI, false),
            (0x9C, Opcode::PREGO, false),
            (0x9D, Opcode::PREGOI, false),
            (0x9E, Opcode::GO, true),
            (0x9F, Opcode::GOI, true),
            (0xA0, Opcode::STB, false),
            (0xA1, Opcode::STBI, false),
            (0xA2, Opcode::STBU, false),
            (0xA3, Opcode::STBUI, false),
            (0xA4, Opcode::STW, false),
            (0xA5, Opcode::STWI, false),
            (0xA6, Opcode::STWU, false),
            (0xA7, Opcode::STWUI, false),
            (0xA8, Opcode::STT, false),
            (0xA9, Opcode::STTI, false),
            (0xAA, Opcode::STTU, false),
            (0xAB, Opcode::STTUI, false),
            (0xAC, Opcode::STO, false),
            (0xAD, Opcode::STOI, false),
            (0xAE, Opcode::STOU, false),
            (0xAF, Opcode::STOUI, false),
            (0xB0, Opcode::STSF, false),
            (0xB1, Opcode::STSFI, false),
            (0xB2, Opcode::STHT, false),
            (0xB3, Opcode::STHTI, false),
            (0xB4, Opcode::STCO, false),
            (0xB5, Opcode::STCOI, false),
            (0xB6, Opcode::STUNC, false),
            (0xB7, Opcode::STUNCI, false),
            (0xB8, Opcode::SYNCD, false),
            (0xB9, Opcode::SYNCDI, false),
            (0xBA, Opcode::PREST, false),
            (0xBB, Opcode::PRESTI, false),
            (0xBC, Opcode::SYNCID, false),
            (0xBD, Opcode::SYNCIDI, false),
            (0xBE, Opcode::PUSHGO, true),
            (0xBF, Opcode::PUSHGOI, true),
            (0xC0, Opcode::OR, true),
            (0xC1, Opcode::ORI, true),
            (0xC2, Opcode::ORN, true),
            (0xC3, Opcode::ORNI, true),
            (0xC4, Opcode::NOR, true),
            (0xC5, Opcode::NORI, true),
            (0xC6, Opcode::XOR, true),
            (0xC7, Opcode::XORI, true),
            (0xC8, Opcode::AND, true),
            (0xC9, Opcode::ANDI, true),
            (0xCA, Opcode::ANDN, true),
            (0xCB, Opcode::ANDNI, true),
            (0xCC, Opcode::NAND, true),
            (0xCD, Opcode::NANDI, true),
            (0xCE, Opcode::NXOR, true),
            (0xCF, Opcode::NXORI, true),
            (0xD0, Opcode::BDIF, true),
            (0xD1, Opcode::BDIFI, true),
            (0xD2, Opcode::WDIF, true),
            (0xD3, Opcode::WDIFI, true),
            (0xD4, Opcode::TDIF, true),
            (0xD5, Opcode::TDIFI, true),
            (0xD6, Opcode::ODIF, true),
            (0xD7, Opcode::ODIFI, true),
            (0xD8, Opcode::MUX, true),
            (0xD9, Opcode::MUXI, true),
            (0xDA, Opcode::SADD, true),
            (0xDB, Opcode::SADDI, true),
            (0xDC, Opcode::MOR, true),
            (0xDD, Opcode::MORI, true),
            (0xDE, Opcode::MXOR, true),
            (0xDF, Opcode::MXORI, true),
            (0xE0, Opcode::SETH, true),
            (0xE1, Opcode::SETMH, true),
            (0xE2, Opcode::SETML, true),
            (0xE3, Opcode::SETL, true),
            (0xE4, Opcode::INCH, true),
            (0xE5, Opcode::INCMH, true),
            (0xE6, Opcode::INCML, true),
            (0xE7, Opcode::INCL, true),
            (0xE8, Opcode::ORH, true),
            (0xE9, Opcode::ORMH, true),
            (0xEA, Opcode::ORML, true),
            (0xEB, Opcode::ORL, true),
            (0xEC, Opcode::ANDNH, true),
            (0xED, Opcode::ANDNMH, true),
            (0xEE, Opcode::ANDNML, true),
            (0xEF, Opcode::ANDNL, true),
            (0xF0, Opcode::JMP, false),
            (0xF1, Opcode::JMPB, false),
            (0xF2, Opcode::PUSHJ, true),
            (0xF3, Opcode::PUSHJB, true),
            (0xF4, Opcode::GETA, true),
            (0xF5, Opcode::GETAB, true),
            (0xF6, Opcode::PUT, false),
            (0xF7, Opcode::PUTI, false),
            (0xF8, Opcode::POP, false),
            (0xF9, Opcode::RESUME, false),
            (0xFA, Opcode::SAVE, true),
            (0xFB, Opcode::UNSAVE, false),
            (0xFC, Opcode::SYNC, false),
            (0xFD, Opcode::SWYM, false),
            (0xFE, Opcode::GET, true),
            (0xFF, Opcode::TRIP, false),
        ];

        assert_eq!(table.len(), 256, "the table must cover every opcode byte");

        for (i, (byte, mnemonic, expected)) in table.iter().enumerate() {
            assert_eq!(
                *byte, i as u8,
                "row {i} is out of place: carries byte {byte:#04X}"
            );
            assert_eq!(
                Opcode::try_from(*byte).unwrap(),
                *mnemonic,
                "row {byte:#04X} names {mnemonic:?}, but Opcode disagrees"
            );
            let actual = MMix::writes_general_register_x(*byte);
            assert_eq!(
                actual, *expected,
                "byte {byte:#04X} ({mnemonic:?}): predicate says {actual}, table says {expected}"
            );
        }
    }

    #[test]
    fn test_put_rl_keeps_the_globals_in_a_state_mmix_rejects() {
        let mut mmix = MMix::new();
        mmix.set_special(SpecialReg::RG, 60);
        mmix.set_register(55, 777); // local while rG = 60, so rL rises to 56
        mmix.set_special(SpecialReg::RG, 32); // $55 is global now

        // PUTI rL,3
        mmix.write_tetra(0, 0xF7140003);
        assert!(mmix.execute_instruction());

        assert_eq!(mmix.get_special(SpecialReg::RL), 3);
        assert_eq!(mmix.get_register(55), 777, "a global keeps its value");
    }

    #[test]
    fn test_get_of_rl_into_a_marginal_destination_sees_the_raised_value() {
        let mut mmix = MMix::new();
        mmix.set_special(SpecialReg::RG, 32);
        mmix.set_special(SpecialReg::RL, 5);

        // GET $10,rL
        mmix.write_tetra(0, 0xFE0A0014);
        assert!(mmix.execute_instruction());

        assert_eq!(mmix.get_register(10), 11);
        assert_eq!(mmix.get_special(SpecialReg::RL), 11);
    }

    #[test]
    fn test_arithmetic_destination_rise_leaves_a_marginal_source_reading_zero() {
        let mut mmix = MMix::new();
        mmix.set_register(40, 0xDEAD); // global while rG = 32
        mmix.set_special(SpecialReg::RG, 50);
        mmix.set_special(SpecialReg::RL, 3);

        // ADD $45,$40,$0
        mmix.write_tetra(0, 0x202D2800);
        assert!(mmix.execute_instruction());

        assert_eq!(mmix.get_register(45), 0);
        assert_eq!(mmix.get_register(40), 0);
        assert_eq!(mmix.get_special(SpecialReg::RL), 46);
    }

    #[test]
    fn test_a_conditional_set_that_stores_nothing_still_raises_rl() {
        let mut mmix = MMix::new();
        mmix.set_special(SpecialReg::RL, 3);

        // CSN $8,$6,$7 - $6 is zero, so nothing is stored.
        mmix.write_tetra(0, 0x60080607);
        assert!(mmix.execute_instruction());

        assert_eq!(mmix.get_special(SpecialReg::RL), 9);
    }

    #[test]
    fn test_a_read_modify_write_destination_reads_its_own_rise_as_zero() {
        let mut mmix = MMix::new();
        mmix.set_register(40, 777); // global while rG = 32
        mmix.set_special(SpecialReg::RG, 50);
        mmix.set_special(SpecialReg::RL, 3);

        // INCL $40,5
        mmix.write_tetra(0, 0xE7280005);
        assert!(mmix.execute_instruction());

        assert_eq!(mmix.get_register(40), 5);
        assert_eq!(mmix.get_special(SpecialReg::RL), 41);
    }

    #[test]
    fn test_go_claims_its_destination_before_reading_its_address() {
        let mut mmix = MMix::new();
        mmix.set_register(40, 0xDEAD); // global while rG = 32
        mmix.set_special(SpecialReg::RG, 50);
        mmix.set_special(SpecialReg::RL, 3);

        // GO $45,$40,4
        mmix.write_tetra(0, 0x9F2D2804);
        assert!(mmix.execute_instruction());

        assert_eq!(mmix.get_pc(), 4, "$40 reads zero, so the target is 0+4");
        assert_eq!(mmix.get_register(45), 4, "the tetra after the GO");
        assert_eq!(mmix.get_special(SpecialReg::RL), 46);
    }

    #[test]
    fn test_a_store_does_not_claim_its_x_register() {
        let mut mmix = MMix::new();
        mmix.set_special(SpecialReg::RL, 3);

        // STO $8,$0,0
        mmix.write_tetra(0, 0xAD080000);
        assert!(mmix.execute_instruction());

        assert_eq!(mmix.get_special(SpecialReg::RL), 3);
    }

    #[test]
    fn test_pop_rg_guard_preserves_a_global_set_after_a_mid_call_shrink() {
        // Caller has rG=40, rL=38 and calls PUSHJ $36; the callee shrinks
        // rG to 32 with PUT rG, then sets the newly-global $34 before
        // POP 0,0. Without the rG guard, POP would restore the caller's
        // stale $34 from the spilled frame and clobber the callee's global
        // write.
        let mut mmix = MMix::new();
        mmix.set_pc(0x100);
        mmix.set_special(SpecialReg::RG, 40);
        mmix.set_special(SpecialReg::RL, 38);

        // PUSHJ $36, +1
        mmix.write_tetra(0x100, 0xF2240001);
        assert!(mmix.execute_instruction());

        // PUTI rG,32
        mmix.write_tetra(0x104, 0xF7130020);
        assert!(mmix.execute_instruction());

        // SETL $34,999
        mmix.write_tetra(0x108, 0xE32203E7);
        assert!(mmix.execute_instruction());

        // POP 0,0
        mmix.write_tetra(0x10C, 0xF8000000);
        assert!(mmix.execute_instruction());

        assert_eq!(mmix.get_register(34), 999);
        assert_eq!(mmix.get_special(SpecialReg::RG), 32);
        assert_eq!(mmix.get_special(SpecialReg::RL), 32);
    }

    #[test]
    fn test_pop() {
        let mut mmix = MMix::new();
        // Set return address in rJ
        mmix.set_special(SpecialReg::RJ, 200);
        // POP 0, 0 - Return to address in rJ
        mmix.write_tetra(0, 0xF8000000); // POP 0,0,0
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 200); // PC = rJ value
    }

    #[test]
    fn test_swym() {
        let mut mmix = MMix::new();
        // SWYM - no-op
        mmix.write_tetra(0, 0xFD000000); // SWYM 0,0,0
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 4); // PC advances normally
    }

    /// Write `value`'s four bytes as loaded, the way `write_image` does, so a
    /// trip landing here does not read the address as an unloaded vector.
    fn load_tetra(mmix: &mut MMix, addr: u64, value: u32) {
        for i in 0..4 {
            let shift = 24 - 8 * i;
            mmix.write_loaded_byte(addr + i as u64, (value >> shift) as u8);
        }
    }

    #[test]
    fn test_trip() {
        // TRIP X,Y,Z sets rX/rY/rZ/rB/$255/rW (§1 rule 3) and lands at #00.
        let mut mmix = MMix::new();
        load_tetra(&mut mmix, 0x00, 0xFD000000); // vector loaded: SWYM
        mmix.set_pc(0x100);
        mmix.set_register(255, 0xAAAA);
        mmix.set_register(2, 0x2222);
        mmix.set_register(3, 0x3333);
        mmix.set_special(SpecialReg::RJ, 0xBEEF);
        mmix.write_tetra(0x100, 0xFF110203); // TRIP X=0x11, Y=2, Z=3

        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 0x00);
        assert_eq!(mmix.get_special(SpecialReg::RW), 0x104);
        assert_eq!(mmix.get_special(SpecialReg::RX), 0x80000000FF110203);
        assert_eq!(mmix.get_special(SpecialReg::RY), 0x2222);
        assert_eq!(mmix.get_special(SpecialReg::RZ), 0x3333);
        assert_eq!(mmix.get_special(SpecialReg::RB), 0xAAAA);
        assert_eq!(mmix.get_register(255), 0xBEEF);
    }

    #[test]
    fn test_sync() {
        let mut mmix = MMix::new();
        // SYNC - memory barrier (no-op in simulator)
        mmix.write_tetra(0, 0xFC000000); // SYNC 0,0,0
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_resume_with_negative_rx_continues_at_rw() {
        let mut mmix = MMix::new();
        mmix.set_pc(0x100);
        mmix.set_special(SpecialReg::RX, 0x8000000000000000); // negative
        mmix.set_special(SpecialReg::RW, 0x200);
        mmix.write_tetra(0x100, 0xF9000000); // RESUME 0

        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 0x200);
    }

    #[test]
    fn test_resume_ropcode_0_runs_rxs_instruction_and_continues_at_rw() {
        let mut mmix = MMix::new();
        mmix.set_pc(0x100);
        mmix.set_register(2, 10);
        mmix.set_register(3, 20);
        // rX (nonnegative, ropcode 0): ADD $1,$2,$3.
        mmix.set_special(SpecialReg::RX, 0x20010203);
        mmix.set_special(SpecialReg::RW, 0x300);
        mmix.write_tetra(0x100, 0xF9000000); // RESUME 0

        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 30);
        assert_eq!(mmix.get_pc(), 0x300);
    }

    #[test]
    fn test_resume_ropcodes_1_to_3_halt() {
        for ropcode in 1u64..=3 {
            let (host, handle) = CaptureHost::new();
            let mut mmix = MMix::with_host(host);
            mmix.set_pc(0x100);
            mmix.set_special(SpecialReg::RX, ropcode << 56);
            mmix.set_special(SpecialReg::RW, 0x300);
            mmix.write_tetra(0x100, 0xF9000000); // RESUME 0

            assert!(!mmix.execute_instruction(), "ropcode {ropcode}");
            assert_eq!(mmix.get_pc(), 0x100, "PC unmoved for ropcode {ropcode}");
            assert_eq!(handle.diagnostics().len(), 1);
        }
    }

    #[test]
    fn test_resume_with_nonzero_z_halts() {
        let (host, handle) = CaptureHost::new();
        let mut mmix = MMix::with_host(host);
        mmix.set_pc(0x100);
        mmix.write_tetra(0x100, 0xF9000001); // RESUME 1 - privileged

        assert!(!mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 0x100, "PC stays on the rejected instruction");
        assert_eq!(handle.diagnostics().len(), 1);
    }

    #[test]
    fn test_divide_check_trips_when_enabled() {
        let mut mmix = MMix::new();
        load_tetra(&mut mmix, 0x10, 0xFD000000); // D's vector loaded
        mmix.set_special(SpecialReg::RA, RA_D << 8); // enable D only
        mmix.set_pc(0x100);
        mmix.set_register(2, 7);
        mmix.set_register(3, 0);
        mmix.write_tetra(0x100, 0x1C010203); // DIV $1,$2,$3

        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 0x10, "trips to D's vector");
        assert_eq!(
            mmix.get_register(1),
            0,
            "DIV's own zero result is written first"
        );
        assert_eq!(mmix.get_special(SpecialReg::RY), 7);
        assert_eq!(
            mmix.get_special(SpecialReg::RA) & RA_D,
            0,
            "a tripped exception's own event bit stays clear"
        );
    }

    #[test]
    fn test_integer_overflow_trips_and_the_handler_sees_the_pre_write_operand() {
        let mut mmix = MMix::new();
        load_tetra(&mut mmix, 0x20, 0xFD000000); // V's vector loaded
        mmix.set_special(SpecialReg::RA, RA_V << 8); // enable V only
        mmix.set_pc(0x100);
        mmix.set_register(5, i64::MAX as u64);
        mmix.set_register(3, 1);
        mmix.write_tetra(0x100, 0x20050503); // ADD $5,$5,$3 - destination aliases a source

        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 0x20);
        assert_eq!(
            mmix.get_register(5),
            i64::MIN as u64,
            "the wrapped result is written before the trip"
        );
        assert_eq!(
            mmix.get_special(SpecialReg::RY),
            i64::MAX as u64,
            "rY holds $5's value from before ADD overwrote it"
        );
        assert_eq!(mmix.get_special(SpecialReg::RA) & RA_V, 0);
    }

    #[test]
    fn test_immediate_overflow_trip_reports_the_literal_z() {
        let mut mmix = MMix::new();
        load_tetra(&mut mmix, 0x20, 0xFD000000); // V's vector loaded
        mmix.set_special(SpecialReg::RA, RA_V << 8); // enable V only
        mmix.set_pc(0x100);
        mmix.set_register(2, i64::MAX as u64);
        // $5 holds a value distinct from the literal Z=5: a revert that reads
        // get_register(5) instead of the literal would report this instead.
        mmix.set_register(5, 0xDEAD_BEEF);
        mmix.write_tetra(0x100, 0x21030205); // ADDI $3,$2,5

        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 0x20);
        assert_eq!(
            mmix.get_special(SpecialReg::RZ),
            5,
            "rZ holds the literal Z, not $5's contents"
        );
    }

    #[test]
    fn test_store_overflow_trip_reports_address_and_stored_value() {
        let mut mmix = MMix::new();
        load_tetra(&mut mmix, 0x20, 0xFD000000); // V's vector loaded
        mmix.set_special(SpecialReg::RA, RA_V << 8); // enable V only
        mmix.set_pc(0x100);
        mmix.set_register(1, 200); // out of signed byte range: overflows
        mmix.set_register(2, 0x4000);
        mmix.set_register(3, 8);
        // A nonzero neighbouring byte in the target octabyte: rZ must be the
        // merged octabyte memory now holds, not the raw stored byte alone.
        mmix.write_byte(0x4009, 0xAB);
        mmix.write_tetra(0x100, 0xA0010203); // STB $1,$2,$3

        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 0x20);
        assert_eq!(
            mmix.get_special(SpecialReg::RY),
            0x4008,
            "rY holds the computed address, not raw $Y"
        );
        assert_eq!(
            mmix.get_special(SpecialReg::RZ),
            0xC8AB_0000_0000_0000,
            "rZ holds the aligned octabyte after the store: the stored byte \
             (200 = 0xC8) in place, the neighbouring byte and the rest of \
             memory unchanged"
        );
    }

    #[test]
    fn test_stsf_overflow_trip_reports_address_and_merged_octa() {
        let mut mmix = MMix::new();
        load_tetra(&mut mmix, 0x50, 0xFD000000); // O's vector loaded
        mmix.set_special(SpecialReg::RA, RA_O << 8); // enable O only
        mmix.set_pc(0x100);
        mmix.set_register(1, f64::MAX.to_bits()); // narrows to +inf: overflow
        mmix.set_register(2, 0x4000);
        mmix.set_register(3, 8);
        // A nonzero byte past the stored tetra, in the same octabyte: rZ
        // must be the merged octabyte, not the plain $Y/$Z operands.
        mmix.write_byte(0x400C, 0xAB);
        mmix.write_tetra(0x100, 0xB0010203); // STSF $1,$2,$3

        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 0x50);
        assert!(f32::from_bits(mmix.read_tetra(0x4008)).is_infinite());
        assert_eq!(
            mmix.get_special(SpecialReg::RY),
            0x4008,
            "rY holds the computed address, not raw $Y — catches a revert \
             to the plain $Y/$Z operands"
        );
        assert_eq!(
            mmix.get_special(SpecialReg::RZ),
            0x7F80_0000_AB00_0000,
            "rZ holds the aligned octabyte after the store: the stored \
             +inf tetra in place, the neighbouring byte unchanged — catches \
             a revert to the plain $Y/$Z operands"
        );
        assert_eq!(mmix.get_special(SpecialReg::RA) & RA_O, 0);
    }

    #[test]
    fn test_float_to_fix_overflow_trips_when_enabled() {
        let mut mmix = MMix::new();
        load_tetra(&mut mmix, 0x30, 0xFD000000); // W's vector loaded
        mmix.set_special(SpecialReg::RA, RA_W << 8); // enable W only
        mmix.set_pc(0x100);
        mmix.set_register(2, f64::INFINITY.to_bits());
        mmix.write_tetra(0x100, 0x05010002); // FIX $1,0,$2

        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 0x30);
        assert_eq!(mmix.get_special(SpecialReg::RA) & RA_W, 0);
    }

    #[test]
    fn test_invalid_operation_trips_when_enabled() {
        let mut mmix = MMix::new();
        load_tetra(&mut mmix, 0x40, 0xFD000000); // I's vector loaded
        mmix.set_special(SpecialReg::RA, RA_I << 8); // enable I only
        mmix.set_pc(0x100);
        mmix.set_register(2, f64::NAN.to_bits());
        mmix.set_register(3, 0);
        mmix.write_tetra(0x100, 0x01010203); // FCMP $1,$2,$3 - $2 is NaN

        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 0x40);
        assert_eq!(mmix.get_special(SpecialReg::RA) & RA_I, 0);
    }

    #[test]
    fn test_floating_overflow_trips_and_the_unenabled_inexact_still_sets_its_event_bit() {
        let mut mmix = MMix::new();
        load_tetra(&mut mmix, 0x50, 0xFD000000); // O's vector loaded
        mmix.set_special(SpecialReg::RA, RA_O << 8); // enable O only, not X
        mmix.set_pc(0x100);
        mmix.set_register(2, f64::MAX.to_bits());
        mmix.set_register(3, f64::MAX.to_bits());
        mmix.write_tetra(0x100, 0x10010203); // FMUL $1,$2,$3 overflows (raises O and X)

        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 0x50);
        let ra = mmix.get_special(SpecialReg::RA);
        assert_eq!(ra & RA_O, 0, "O tripped: its own event bit stays clear");
        assert_eq!(
            ra & RA_X,
            RA_X,
            "X was raised but not enabled: it still sets its event bit"
        );
    }

    #[test]
    fn test_two_enabled_exceptions_trip_to_the_leftmost_and_drop_the_other_silently() {
        let mut mmix = MMix::new();
        load_tetra(&mut mmix, 0x50, 0xFD000000); // O's vector loaded
        mmix.set_special(SpecialReg::RA, (RA_O << 8) | (RA_X << 8)); // both enabled
        mmix.set_pc(0x100);
        mmix.set_register(2, f64::MAX.to_bits());
        mmix.set_register(3, f64::MAX.to_bits());
        mmix.write_tetra(0x100, 0x10010203); // FMUL $1,$2,$3 overflows (raises O and X)

        assert!(mmix.execute_instruction());
        assert_eq!(
            mmix.get_pc(),
            0x50,
            "trips to O, the leftmost enabled exception"
        );
        assert_eq!(
            mmix.get_special(SpecialReg::RA) & 0xFF,
            0,
            "O tripped and X was enabled but not leftmost: neither sets an event bit"
        );
    }

    #[test]
    fn test_floating_underflow_trips_when_enabled() {
        let mut mmix = MMix::new();
        load_tetra(&mut mmix, 0x60, 0xFD000000); // U's vector loaded
        mmix.set_special(SpecialReg::RA, RA_U << 8); // enable U only
        mmix.set_pc(0x100);
        mmix.set_register(2, f64::MIN_POSITIVE.to_bits());
        mmix.set_register(3, f64::MIN_POSITIVE.to_bits());
        mmix.write_tetra(0x100, 0x10010203); // FMUL $1,$2,$3 underflows to zero

        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 0x60);
        assert_eq!(mmix.get_special(SpecialReg::RA) & RA_U, 0);
    }

    #[test]
    fn test_floating_divide_by_zero_trips_when_enabled() {
        let mut mmix = MMix::new();
        load_tetra(&mut mmix, 0x70, 0xFD000000); // Z's vector loaded
        mmix.set_special(SpecialReg::RA, RA_Z << 8); // enable Z only
        mmix.set_pc(0x100);
        mmix.set_register(2, 1.0f64.to_bits());
        mmix.set_register(3, 0.0f64.to_bits());
        mmix.write_tetra(0x100, 0x14010203); // FDIV $1,$2,$3

        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 0x70);
        assert_eq!(mmix.get_special(SpecialReg::RA) & RA_Z, 0);
    }

    #[test]
    fn test_floating_inexact_trips_when_enabled() {
        let mut mmix = MMix::new();
        load_tetra(&mut mmix, 0x80, 0xFD000000); // X's vector loaded
        mmix.set_special(SpecialReg::RA, RA_X << 8); // enable X only
        mmix.set_pc(0x100);
        mmix.set_register(2, 1.0f64.to_bits());
        mmix.set_register(3, 1e-30f64.to_bits());
        mmix.write_tetra(0x100, 0x04010203); // FADD $1,$2,$3 rounds 1e-30 away

        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 0x80);
        assert_eq!(mmix.get_special(SpecialReg::RA) & RA_X, 0);
    }

    #[test]
    fn test_save() {
        let mut mmix = MMix::new();
        // SAVE $40,0 - $40 is global while rG = 32.
        mmix.write_tetra(0, 0xFA280000); // SAVE $40,0
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_unsave() {
        let mut mmix = MMix::new();
        // UNSAVE 0,$1 - $1 holds 0, so the packed octa read from address 0
        // is all zero: a packed rG of 0 is outside 32..=255 and rejects.
        mmix.write_tetra(0, 0xFB000001); // UNSAVE 0,$1
        assert!(!mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 0);
    }

    /// After `SAVE`, the register stack holds — lowest address to highest
    /// — the locals, a marker octa of their count, the globals, the
    /// twelve specials in `SAVE_SPECIALS` order, and a packed octa of
    /// `rG << 56 | rA`. `$X`, `rO`, `rS` and `rL` land where §1 says.
    /// Reverting `SAVE` to the old fixed-address format turns every
    /// assertion here red.
    #[test]
    fn test_save_writes_the_documented_layout() {
        let mut mmix = MMix::new();
        mmix.set_register(0, 0x1111);
        mmix.set_register(1, 0x2222);
        mmix.set_register(2, 0x3333); // rL becomes 3
        mmix.set_special(SpecialReg::RG, 250); // six globals: $250..$255
        for i in 250u8..=255 {
            mmix.set_register(i, 0x9000 + i as u64);
        }
        for (i, reg) in SAVE_SPECIALS.iter().enumerate() {
            mmix.set_special(*reg, 0x7000 + i as u64);
        }
        mmix.set_special(SpecialReg::RA, 0x2A);
        let ro_before = mmix.get_special(SpecialReg::RO);

        // SAVE $250,0 - $250 is global (== rG).
        mmix.write_tetra(0, 0xFAFA0000);
        assert!(mmix.execute_instruction());

        assert_eq!(mmix.read_octa(ro_before), 0x1111);
        assert_eq!(mmix.read_octa(ro_before + 8), 0x2222);
        assert_eq!(mmix.read_octa(ro_before + 16), 0x3333);
        let marker_addr = ro_before + 24;
        assert_eq!(mmix.read_octa(marker_addr), 3, "the marker holds rL");

        let globals_base = marker_addr + 8;
        for i in 0u64..6 {
            assert_eq!(
                mmix.read_octa(globals_base + i * 8),
                0x9000 + 250 + i,
                "global {}",
                250 + i
            );
        }

        let specials_base = globals_base + 6 * 8;
        for i in 0..SAVE_SPECIALS.len() as u64 {
            assert_eq!(
                mmix.read_octa(specials_base + i * 8),
                0x7000 + i,
                "special at index {i}"
            );
        }

        let packed_addr = specials_base + (SAVE_SPECIALS.len() as u64) * 8;
        assert_eq!(mmix.read_octa(packed_addr), (250u64 << 56) | 0x2A);

        assert_eq!(
            mmix.get_register(250),
            packed_addr,
            "$X holds the packed octa's address"
        );
        assert_eq!(mmix.get_special(SpecialReg::RO), packed_addr + 8);
        assert_eq!(mmix.get_special(SpecialReg::RS), packed_addr + 8);
        assert_eq!(mmix.get_special(SpecialReg::RL), 0);
    }

    /// `SAVE`, clobber every register class, `UNSAVE`: everything lands
    /// back exactly where it was, and `rO = rS` returns to where `SAVE`
    /// found them. Dropping any restoration in `UNSAVE`'s arm turns one of
    /// these assertions red.
    #[test]
    fn test_save_unsave_round_trips_every_register_class() {
        let mut mmix = MMix::new();
        mmix.set_register(0, 0x1111); // local
        mmix.set_register(1, 0x2222); // local
        mmix.set_register(60, 0x3333); // global
        mmix.set_special(SpecialReg::RJ, 0x4444);
        mmix.set_special(SpecialReg::RM, 0x5555);
        mmix.set_special(SpecialReg::RA, 0x2A);
        let rl_before = mmix.get_special(SpecialReg::RL);
        let ro_before = mmix.get_special(SpecialReg::RO);

        // SAVE $70,0 - $70 is global.
        mmix.write_tetra(0, 0xFA460000);
        assert!(mmix.execute_instruction());

        // Clobber every register class SAVE just captured.
        mmix.set_register(0, 0);
        mmix.set_register(1, 0);
        mmix.set_register(60, 0);
        mmix.set_special(SpecialReg::RJ, 0);
        mmix.set_special(SpecialReg::RM, 0);
        mmix.set_special(SpecialReg::RA, 0);
        mmix.set_special(SpecialReg::RG, 200);
        mmix.set_special(SpecialReg::RL, 0);

        // UNSAVE 0,$70.
        mmix.write_tetra(4, 0xFB000046);
        assert!(mmix.execute_instruction());

        assert_eq!(mmix.get_register(0), 0x1111);
        assert_eq!(mmix.get_register(1), 0x2222);
        assert_eq!(mmix.get_register(60), 0x3333);
        assert_eq!(mmix.get_special(SpecialReg::RJ), 0x4444);
        assert_eq!(mmix.get_special(SpecialReg::RM), 0x5555);
        assert_eq!(mmix.get_special(SpecialReg::RA), 0x2A);
        assert_eq!(mmix.get_special(SpecialReg::RG), 32);
        assert_eq!(mmix.get_special(SpecialReg::RL), rl_before);
        assert_eq!(mmix.get_special(SpecialReg::RO), ro_before);
        assert_eq!(mmix.get_special(SpecialReg::RS), ro_before);
    }

    /// A `SAVE`/`UNSAVE` pair inside a call is transparent to the call
    /// itself: the enclosing `PUSHJ`/`POP` retract exactly as if the pair
    /// had never run.
    #[test]
    fn test_save_unsave_inside_a_call_leaves_the_caller_frame_intact() {
        let mut mmix = MMix::new();
        mmix.set_pc(0x100);
        mmix.set_register(0, 888); // caller's own local, must survive the call
        mmix.set_special(SpecialReg::RL, 3); // $0..$2 real locals; $2 is PUSHJ's hole
        let ro_before_call = mmix.get_special(SpecialReg::RO);

        // PUSHJ $2,+1 pushes $0,$1 and the hole ($2); the callee starts
        // with rL = 0.
        mmix.write_tetra(0x100, 0xF2020001);
        assert!(mmix.execute_instruction());
        let ro_in_callee = mmix.get_special(SpecialReg::RO);
        assert_eq!(mmix.get_special(SpecialReg::RL), 0);

        // Callee's own local, then SAVE $40,0 ($40 is global).
        mmix.set_register(0, 0xABC);
        mmix.write_tetra(0x104, 0xFA280000);
        assert!(mmix.execute_instruction());

        // Clobber everything SAVE just captured.
        mmix.set_register(0, 0xDEAD);
        mmix.set_special(SpecialReg::RM, 0xBAD);

        // UNSAVE 0,$40.
        mmix.write_tetra(0x108, 0xFB000028);
        assert!(mmix.execute_instruction());

        assert_eq!(
            mmix.get_register(0),
            0xABC,
            "the callee's own local round-trips"
        );
        assert_eq!(
            mmix.get_special(SpecialReg::RM),
            0,
            "rM round-trips to its pre-SAVE value"
        );
        assert_eq!(mmix.get_special(SpecialReg::RO), ro_in_callee);
        assert_eq!(mmix.get_special(SpecialReg::RS), ro_in_callee);
        assert_eq!(
            mmix.get_special(SpecialReg::RL),
            1,
            "the callee's own single local"
        );

        // POP 0,0 returns to the caller.
        mmix.write_tetra(0x10C, 0xF8000000);
        assert!(mmix.execute_instruction());

        assert_eq!(
            mmix.get_register(0),
            888,
            "caller's $0, pushed by PUSHJ, survives the call"
        );
        assert_eq!(mmix.get_special(SpecialReg::RO), ro_before_call);
        assert_eq!(mmix.get_special(SpecialReg::RS), ro_before_call);
        assert_eq!(mmix.call_depth(), 0);
    }

    /// The register-stack review's reproduction: a `SAVE` before a call,
    /// a nested `PUSHJ`, then an `UNSAVE` inside the callee that rewinds
    /// `rO` clear past the frame the `PUSHJ` just opened. Reintroducing a
    /// counter `POP` trusts over `rO` turns this test red: it would still
    /// think the frame was open and read a hole from dead memory instead
    /// of taking the fallback `rO` now calls for.
    #[test]
    fn test_call_depth_and_pop_follow_ro_through_save_pushj_unsave() {
        let mut mmix = MMix::new();
        let base = mmix.get_special(SpecialReg::RO);
        assert_eq!(mmix.call_depth(), 0);

        // SAVE $40,0 at top level -- a context, not a frame.
        let pc = mmix.get_pc();
        mmix.write_tetra(pc, 0xFA280000);
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.call_depth(), 0, "a SAVE context is not a frame");

        // PUSHJ $0,+1 opens a frame inside what becomes the callee.
        let pc = mmix.get_pc();
        mmix.write_tetra(pc, 0xF2000001);
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.call_depth(), 1, "PUSHJ opened one frame");

        // UNSAVE 0,$40 inside the callee rewinds rO past that frame, back
        // to where SAVE found it -- the frame PUSHJ opened is now dead
        // memory below rO.
        let pc = mmix.get_pc();
        mmix.write_tetra(pc, 0xFB000028);
        assert!(mmix.execute_instruction());
        assert_eq!(
            mmix.get_special(SpecialReg::RO),
            base,
            "UNSAVE rewinds rO to where SAVE found it"
        );
        assert_eq!(
            mmix.call_depth(),
            0,
            "rO names the top level again; the dead frame does not count"
        );

        // POP 1,0 acts on what rO now names: the top level, so it takes
        // the no-frame fallback, branching via rJ -- restored by UNSAVE
        // to 0, its value when SAVE captured it. The fallback touches no
        // register or memory, so rO and rL, unlike a real pop, do not move.
        let rl_before_pop = mmix.get_special(SpecialReg::RL);
        let pc = mmix.get_pc();
        mmix.write_tetra(pc, 0xF8010000);
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 0, "POP took the no-frame fallback");
        assert_eq!(
            mmix.get_special(SpecialReg::RO),
            base,
            "the fallback leaves rO alone"
        );
        assert_eq!(
            mmix.get_special(SpecialReg::RL),
            rl_before_pop,
            "the fallback leaves rL alone"
        );
        assert_eq!(mmix.call_depth(), 0);
    }

    /// A `SAVE`/`UNSAVE` pair inside a call leaves `call_depth` at the
    /// call's own count throughout: the context never counts as a second
    /// frame. Making `call_depth` count a `SAVE` context as a frame turns
    /// the middle assertion here red (2, not 1).
    #[test]
    fn test_call_depth_reads_one_across_a_save_inside_a_call() {
        let mut mmix = MMix::new();
        assert_eq!(mmix.call_depth(), 0);

        let pc = mmix.get_pc();
        mmix.write_tetra(pc, 0xF2000001); // PUSHJ $0,+1
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.call_depth(), 1);

        let pc = mmix.get_pc();
        mmix.write_tetra(pc, 0xFA280000); // SAVE $40,0
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.call_depth(), 1, "the SAVE context does not count");

        let pc = mmix.get_pc();
        mmix.write_tetra(pc, 0xFB000028); // UNSAVE 0,$40
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.call_depth(), 1, "still just the one open call");
    }

    /// Three nested `PUSHJ`s deep and back, `call_depth` 0→3→0, with a
    /// `SAVE`/`UNSAVE` pair at depth 2 in between.
    #[test]
    fn test_call_depth_nests_three_deep_with_a_save_unsave_pair_at_depth_two() {
        let mut mmix = MMix::new();
        assert_eq!(mmix.call_depth(), 0);

        for depth in 1..=2u64 {
            let pc = mmix.get_pc();
            mmix.write_tetra(pc, 0xF2000001); // PUSHJ $0,+1
            assert!(mmix.execute_instruction());
            assert_eq!(mmix.call_depth(), depth as usize);
        }

        let pc = mmix.get_pc();
        mmix.write_tetra(pc, 0xFA280000); // SAVE $40,0
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.call_depth(), 2);

        let pc = mmix.get_pc();
        mmix.write_tetra(pc, 0xFB000028); // UNSAVE 0,$40
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.call_depth(), 2);

        let pc = mmix.get_pc();
        mmix.write_tetra(pc, 0xF2000001); // PUSHJ $0,+1 -- depth 3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.call_depth(), 3);

        for depth in (0..=2u64).rev() {
            let pc = mmix.get_pc();
            mmix.write_tetra(pc, 0xF8000000); // POP 0,0
            assert!(mmix.execute_instruction());
            assert_eq!(mmix.call_depth(), depth as usize);
        }
    }

    /// `POP` does not recognize a `SAVE` context: at top level after a
    /// bare `SAVE`, it reads the packed octa's low byte as a real hole
    /// count and retracts `rO` accordingly, rather than taking the
    /// no-frame fallback (which would leave `rO` untouched).
    #[test]
    fn test_pop_after_a_bare_save_reads_the_packed_octas_low_byte_as_a_hole() {
        let mut mmix = MMix::new();
        mmix.set_special(SpecialReg::RA, 5); // packed octa's low byte becomes 5

        let pc = mmix.get_pc();
        mmix.write_tetra(pc, 0xFA280000); // SAVE $40,0
        assert!(mmix.execute_instruction());
        let ro_after_save = mmix.get_special(SpecialReg::RO);
        assert!(ro_after_save > STACK_SEGMENT_START);

        let pc = mmix.get_pc();
        mmix.write_tetra(pc, 0xF8000000); // POP 0,0
        assert!(mmix.execute_instruction());
        assert_eq!(
            mmix.get_special(SpecialReg::RO),
            ro_after_save - 48,
            "rO retracted by 8*(5+1), the packed octa's low byte read as a hole"
        );
    }

    /// `rO` at or below the stack base — the fresh-machine case, and a
    /// forged one further below, only reachable through `set_special` —
    /// both take `POP`'s no-frame fallback, and `call_depth` reads 0
    /// without looping either way.
    #[test]
    fn test_pop_and_call_depth_at_or_below_the_base_take_the_fallback() {
        let mut mmix = MMix::new();
        assert_eq!(mmix.get_special(SpecialReg::RO), STACK_SEGMENT_START);
        assert_eq!(mmix.call_depth(), 0);

        mmix.set_special(SpecialReg::RJ, 0x200);
        let pc = mmix.get_pc();
        mmix.write_tetra(pc, 0xF8008000); // POP 0,0x8000
        assert!(mmix.execute_instruction());
        assert_eq!(
            mmix.get_pc(),
            0x200 + 32768 * 4,
            "at the base: the fallback fired"
        );

        mmix.set_special(SpecialReg::RO, STACK_SEGMENT_START - 8);
        assert_eq!(mmix.call_depth(), 0);
        mmix.set_special(SpecialReg::RJ, 0x300);
        mmix.set_pc(0);
        mmix.write_tetra(0, 0xF8000000); // POP 0,0
        assert!(mmix.execute_instruction());
        assert_eq!(
            mmix.get_pc(),
            0x300,
            "below the base: the fallback fired too"
        );
    }

    /// A misaligned `rO` — only reachable through a forged `set_special`,
    /// since every legal instruction keeps it a multiple of 8 — halts
    /// `POP` with a diagnostic and leaves the machine unchanged, since the
    /// reference would raise a protection fault checksmix has no vector
    /// for. `call_depth` stops at 0 without looping.
    #[test]
    fn test_pop_halts_on_a_misaligned_ro() {
        let (host, handle) = CaptureHost::new();
        let mut mmix = MMix::with_host(host);
        mmix.set_special(SpecialReg::RO, STACK_SEGMENT_START + 3);
        mmix.set_register(0, 0xFEED);

        assert_eq!(
            mmix.call_depth(),
            0,
            "the walk stops at once on a misaligned rO"
        );

        mmix.write_tetra(0, 0xF8000000); // POP 0,0
        assert!(!mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 0, "no PC advance on a halt");
        assert_eq!(mmix.get_register(0), 0xFEED, "no register change on a halt");
        assert_eq!(handle.diagnostics().len(), 1);
        assert!(handle.diagnostics()[0].contains("rO"));
    }

    /// An `rO` above the register-stack segment — only reachable through a
    /// forged `set_special` — halts `POP` the same way, and `call_depth`
    /// again stops at once without looping.
    #[test]
    fn test_pop_halts_on_an_ro_above_the_stack_segment() {
        let (host, handle) = CaptureHost::new();
        let mut mmix = MMix::with_host(host);
        mmix.set_special(SpecialReg::RO, 0x8000000000000000);
        mmix.set_register(0, 0xFEED);

        assert_eq!(
            mmix.call_depth(),
            0,
            "the walk stops at once outside the segment"
        );

        mmix.write_tetra(0, 0xF8000000); // POP 0,0
        assert!(!mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 0);
        assert_eq!(mmix.get_register(0), 0xFEED);
        assert_eq!(handle.diagnostics().len(), 1);
        assert!(handle.diagnostics()[0].contains("rO"));
    }

    /// The review's non-forged reproduction: a real top-level `SAVE $40,0`,
    /// then an ordinary `write_octa` overwrites the marker octa `SAVE`
    /// wrote with a local count chosen so the walk's own arithmetic maps
    /// `locals_base` back to `ro` itself -- a fixed point, no forged `rO`
    /// anywhere. Without the saved-local-count bound in
    /// `save_context_layout`, `call_depth` recomputes this same address
    /// forever and the test never returns; with it, the count (far above
    /// the saved `rG`) is rejected and the walk stops at once.
    #[test]
    fn test_call_depth_does_not_loop_on_a_marker_mapping_back_to_itself() {
        let mut mmix = MMix::new();

        // SAVE $40,0 at top level.
        mmix.write_tetra(0, 0xFA280000);
        assert!(mmix.execute_instruction());
        let packed_addr = mmix.get_register(40);
        let ro = mmix.get_special(SpecialReg::RO);
        assert_eq!(ro, packed_addr + 8);

        let global_count = 256u64 - 32; // rG = 32 at SAVE time
        let marker_addr = packed_addr - (SAVE_SPECIALS.len() as u64) * 8 - global_count * 8 - 8;

        // Solve for a count with locals_base == marker_addr - count*8 == ro.
        let diff = marker_addr.wrapping_sub(ro);
        assert_eq!(diff % 8, 0, "sanity: layout offsets are all multiples of 8");
        mmix.write_octa(marker_addr, diff / 8);

        assert_eq!(
            mmix.call_depth(),
            0,
            "the corrupted local count exceeds the saved rG; the walk stops \
             instead of looping back to where it started"
        );
    }

    /// A `SAVE` executed with a nonzero `rL` and `rG != 32` -- the review's
    /// coverage gap. `call_depth` must step over the whole context, globals
    /// and locals alike, using the saved (not the machine's current) `rG`.
    #[test]
    fn test_call_depth_walks_a_save_context_with_nonzero_rl_and_rg_ne_32() {
        let mut mmix = MMix::new();
        mmix.set_special(SpecialReg::RG, 50);
        mmix.set_special(SpecialReg::RL, 3);
        mmix.set_register(0, 0x111);
        mmix.set_register(1, 0x222);
        mmix.set_register(2, 0x333);

        // SAVE $60,0 -- $60 is global (rG=50).
        mmix.write_tetra(0, 0xFA3C0000);
        assert!(mmix.execute_instruction());
        assert_eq!(
            mmix.call_depth(),
            0,
            "the context alone, locals and all, is not a frame"
        );

        // PUSHJ $0,+1 opens a frame above the context.
        let pc = mmix.get_pc();
        mmix.write_tetra(pc, 0xF2000001);
        assert!(mmix.execute_instruction());
        assert_eq!(
            mmix.call_depth(),
            1,
            "the walk steps over the saved rL=3, rG=50 context uncounted \
             and still finds the one PUSHJ frame beneath it"
        );
    }

    /// `SAVE`'s Y and Z, and `UNSAVE`'s X and Y, are must-be-zero fields the
    /// machine never reads. A nonzero value there behaves exactly as zero.
    #[test]
    fn test_save_and_unsave_ignore_their_must_be_zero_fields() {
        let mut mmix = MMix::new();
        mmix.set_register(0, 0x1234); // a local
        mmix.set_register(50, 0xABCD); // a global

        // SAVE $60,255,255 - Y and Z both nonzero.
        mmix.write_tetra(0, 0xFA3CFFFF);
        assert!(mmix.execute_instruction());

        mmix.set_register(0, 0);
        mmix.set_register(50, 0);

        // UNSAVE 255,255,$60 - X and Y both nonzero; the address still
        // comes from $Z alone.
        mmix.write_tetra(4, 0xFBFFFF3C);
        assert!(mmix.execute_instruction());

        assert_eq!(mmix.get_register(0), 0x1234);
        assert_eq!(mmix.get_register(50), 0xABCD);
    }

    /// Two independent machines run the same `SAVE`; `$X` lands on the same
    /// address in both. The old fixed-address format shared one process-
    /// global counter across every `MMix`, so two machines (or two tests
    /// running in parallel) handed out interleaved addresses instead.
    #[test]
    fn test_save_is_deterministic_across_instances() {
        let mut a = MMix::new();
        let mut b = MMix::new();

        a.write_tetra(0, 0xFA280000); // SAVE $40,0
        b.write_tetra(0, 0xFA280000);
        assert!(a.execute_instruction());
        assert!(b.execute_instruction());

        assert_eq!(a.get_register(40), b.get_register(40));
    }

    /// `UNSAVE` rejects a packed rG below 32 (above 255 cannot occur — it
    /// is one byte), before touching any register or memory.
    #[test]
    fn test_unsave_rejects_a_packed_rg_below_32() {
        let (host, handle) = CaptureHost::new();
        let mut mmix = MMix::with_host(host);
        mmix.set_register(0, 0xFEED); // survives iff UNSAVE never runs

        let context = 0x2000u64;
        mmix.write_octa(context, 31u64 << 56); // packed rG = 31, rA = 0
        mmix.set_register(60, context);

        // UNSAVE 0,$60
        mmix.write_tetra(0, 0xFB00003C);
        assert!(!mmix.execute_instruction());

        assert_eq!(mmix.get_pc(), 0);
        assert_eq!(mmix.get_register(0), 0xFEED);
        assert_eq!(mmix.get_special(SpecialReg::RG), 32);
        assert_eq!(handle.diagnostics().len(), 1);
        assert!(handle.diagnostics()[0].contains("rG"));
    }

    /// `UNSAVE` rejects a packed rA above `RA_MAX`.
    #[test]
    fn test_unsave_rejects_a_packed_ra_above_ra_max() {
        let (host, handle) = CaptureHost::new();
        let mut mmix = MMix::with_host(host);
        mmix.set_register(0, 0xFEED);

        let context = 0x2000u64;
        mmix.write_octa(context, (32u64 << 56) | (RA_MAX + 1));
        mmix.set_register(60, context);

        // UNSAVE 0,$60
        mmix.write_tetra(0, 0xFB00003C);
        assert!(!mmix.execute_instruction());

        assert_eq!(mmix.get_pc(), 0);
        assert_eq!(mmix.get_register(0), 0xFEED);
        assert_eq!(mmix.get_special(SpecialReg::RA), 0);
        assert_eq!(handle.diagnostics().len(), 1);
        assert!(handle.diagnostics()[0].contains("rA"));
    }

    /// `UNSAVE` rejects a saved local count greater than the packed rG. A
    /// real `SAVE` gives a well-formed context; only its marker is
    /// corrupted.
    #[test]
    fn test_unsave_rejects_a_saved_local_count_above_the_packed_rg() {
        let (host, handle) = CaptureHost::new();
        let mut mmix = MMix::with_host(host);
        mmix.set_special(SpecialReg::RL, 0);

        // SAVE $40,0 with no locals: the marker holds 0.
        mmix.write_tetra(0, 0xFA280000);
        assert!(mmix.execute_instruction());
        let context = mmix.get_register(40);

        let global_count = 256u64 - 32; // rg = 32
        let marker_addr = context - (SAVE_SPECIALS.len() as u64) * 8 - global_count * 8 - 8;
        assert_eq!(
            mmix.read_octa(marker_addr),
            0,
            "sanity: 0 locals were saved"
        );
        mmix.write_octa(marker_addr, 33); // 33 > rG (32)

        mmix.set_register(0, 0xFEED); // survives iff UNSAVE never runs

        // UNSAVE 0,$40
        mmix.write_tetra(4, 0xFB000028);
        assert!(!mmix.execute_instruction());

        assert_eq!(mmix.get_pc(), 4);
        assert_eq!(mmix.get_register(0), 0xFEED);
        assert_eq!(handle.diagnostics().len(), 1);
        assert!(handle.diagnostics()[0].contains("local count"));
    }

    #[test]
    fn test_csn_condition_true() {
        let mut mmix = MMix::new();
        // CSN $1, $2, $3 - If $2 < 0, set $1 = $3, else $1 = $2
        mmix.set_register(2, (-10i64) as u64);
        mmix.set_register(3, 50);
        mmix.write_tetra(0, 0x60010203); // CSN $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 50); // Condition true: $3
    }

    #[test]
    fn test_csn_condition_false() {
        let mut mmix = MMix::new();
        // CSN $1, $2, $3 - If $2 >= 0, do nothing
        mmix.set_register(1, 99); // Initial value
        mmix.set_register(2, 100);
        mmix.set_register(3, 50);
        mmix.write_tetra(0, 0x60010203); // CSN $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 99); // Condition false: unchanged
    }

    #[test]
    fn test_csni() {
        let mut mmix = MMix::new();
        // CSNI $1, $2, 50 - If $2 < 0, set $1 = 50, else $1 = $2
        mmix.set_register(2, (-1i64) as u64);
        mmix.write_tetra(0, 0x61010232); // CSNI $1,$2,50
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 50); // Condition true: 50
    }

    #[test]
    fn test_csz_condition_true() {
        let mut mmix = MMix::new();
        // CSZ $1, $2, $3 - If $2 == 0, set $1 = $3, else $1 = $2
        mmix.set_register(2, 0);
        mmix.set_register(3, 20);
        mmix.write_tetra(0, 0x62010203); // CSZ $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 20); // Condition true: $3
    }

    #[test]
    fn test_csz_condition_false() {
        let mut mmix = MMix::new();
        // CSZ $1, $2, $3 - If $2 != 0, do nothing
        mmix.set_register(1, 88); // Initial value
        mmix.set_register(2, 10);
        mmix.set_register(3, 20);
        mmix.write_tetra(0, 0x62010203); // CSZ $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 88); // Condition false: unchanged
    }

    #[test]
    fn test_cszi() {
        let mut mmix = MMix::new();
        // CSZI $1, $2, 15 - If $2 == 0, set $1 = 15, else $1 = $2
        mmix.set_register(2, 0);
        mmix.write_tetra(0, 0x6301020F); // CSZI $1,$2,15
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 15); // Condition true: 15
    }

    #[test]
    fn test_csp_condition_true() {
        let mut mmix = MMix::new();
        // CSP $1, $2, $3 - If $2 > 0, set $1 = $3, else $1 = $2
        mmix.set_register(2, 5);
        mmix.set_register(3, 7);
        mmix.write_tetra(0, 0x64010203); // CSP $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 7); // Condition true: $3
    }

    #[test]
    fn test_csp_condition_false_zero() {
        let mut mmix = MMix::new();
        // CSP $1, $2, $3 - If $2 <= 0, do nothing
        mmix.set_register(1, 77); // Initial value
        mmix.set_register(2, 0);
        mmix.set_register(3, 7);
        mmix.write_tetra(0, 0x64010203); // CSP $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 77); // Condition false: unchanged
    }

    #[test]
    fn test_cspi() {
        let mut mmix = MMix::new();
        // CSPI $1, $2, 25 - If $2 > 0, set $1 = 25, else $1 = $2
        mmix.set_register(2, 50);
        mmix.write_tetra(0, 0x65010219); // CSPI $1,$2,25
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 25); // Condition true: 25
    }

    #[test]
    fn test_csod_condition_true() {
        let mut mmix = MMix::new();
        // CSOD $1, $2, $3 - If $2 is odd, set $1 = $3, else $1 = $2
        mmix.set_register(2, 7);
        mmix.set_register(3, 15);
        mmix.write_tetra(0, 0x66010203); // CSOD $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 15); // Condition true: $3
    }

    #[test]
    fn test_csod_condition_false() {
        let mut mmix = MMix::new();
        // CSOD $1, $2, $3 - If $2 is even, do nothing
        mmix.set_register(1, 66); // Initial value
        mmix.set_register(2, 8);
        mmix.set_register(3, 15);
        mmix.write_tetra(0, 0x66010203); // CSOD $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 66); // Condition false: unchanged
    }

    #[test]
    fn test_csodi() {
        let mut mmix = MMix::new();
        // CSODI $1, $2, 11 - If $2 is odd, set $1 = 11, else $1 = $2
        mmix.set_register(2, 99);
        mmix.write_tetra(0, 0x6701020B); // CSODI $1,$2,11
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 11); // Condition true: 11
    }

    #[test]
    fn test_csnn_condition_true_positive() {
        let mut mmix = MMix::new();
        // CSNN $1, $2, $3 - If $2 >= 0, set $1 = $3, else $1 = $2
        mmix.set_register(2, 30);
        mmix.set_register(3, 40);
        mmix.write_tetra(0, 0x68010203); // CSNN $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 40); // Condition true: $3
    }

    #[test]
    fn test_csnn_condition_true_zero() {
        let mut mmix = MMix::new();
        // CSNN $1, $2, $3 - If $2 >= 0, set $1 = $3, else $1 = $2
        mmix.set_register(2, 0);
        mmix.set_register(3, 40);
        mmix.write_tetra(0, 0x68010203); // CSNN $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 40); // Condition true: $3
    }

    #[test]
    fn test_csnn_condition_false() {
        let mut mmix = MMix::new();
        // CSNN $1, $2, $3 - If $2 < 0, do nothing
        mmix.set_register(1, 55); // Initial value
        mmix.set_register(2, (-5i64) as u64);
        mmix.set_register(3, 40);
        mmix.write_tetra(0, 0x68010203); // CSNN $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 55); // Condition false: unchanged
    }

    #[test]
    fn test_csnni() {
        let mut mmix = MMix::new();
        // CSNNI $1, $2, 8 - If $2 >= 0, set $1 = 8, else $1 = $2
        mmix.set_register(2, 92);
        mmix.write_tetra(0, 0x69010208); // CSNNI $1,$2,8
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 8); // Condition true: 8
    }

    #[test]
    fn test_csnz_condition_true() {
        let mut mmix = MMix::new();
        // CSNZ $1, $2, $3 - If $2 != 0, set $1 = $3, else $1 = $2
        mmix.set_register(2, 100);
        mmix.set_register(3, 200);
        mmix.write_tetra(0, 0x6A010203); // CSNZ $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 200); // Condition true: $3
    }

    #[test]
    fn test_csnz_condition_false() {
        let mut mmix = MMix::new();
        // CSNZ $1, $2, $3 - If $2 == 0, do nothing
        mmix.set_register(1, 44); // Initial value
        mmix.set_register(2, 0);
        mmix.set_register(3, 200);
        mmix.write_tetra(0, 0x6A010203); // CSNZ $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 44); // Condition false: unchanged
    }

    #[test]
    fn test_csnzi() {
        let mut mmix = MMix::new();
        // CSNZI $1, $2, 33 - If $2 != 0, set $1 = 33, else $1 = $2
        mmix.set_register(2, 67);
        mmix.write_tetra(0, 0x6B010221); // CSNZI $1,$2,33
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 33); // Condition true: 33
    }

    #[test]
    fn test_csnp_condition_true_negative() {
        let mut mmix = MMix::new();
        // CSNP $1, $2, $3 - If $2 <= 0, set $1 = $3, else $1 = $2
        mmix.set_register(2, (-100i64) as u64);
        mmix.set_register(3, 25);
        mmix.write_tetra(0, 0x6C010203); // CSNP $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 25); // Condition true: $3
    }

    #[test]
    fn test_csnp_condition_true_zero() {
        let mut mmix = MMix::new();
        // CSNP $1, $2, $3 - If $2 == 0, set $1 = $3, else $1 = $2
        mmix.set_register(2, 0);
        mmix.set_register(3, 25);
        mmix.write_tetra(0, 0x6C010203); // CSNP $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 25); // Condition true: $3
    }

    #[test]
    fn test_csnp_condition_false() {
        let mut mmix = MMix::new();
        // CSNP $1, $2, $3 - If $2 > 0, do nothing
        mmix.set_register(1, 33); // Initial value
        mmix.set_register(2, 50);
        mmix.set_register(3, 25);
        mmix.write_tetra(0, 0x6C010203); // CSNP $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 33); // Condition false: unchanged
    }

    #[test]
    fn test_csnpi() {
        let mut mmix = MMix::new();
        // CSNPI $1, $2, 44 - If $2 <= 0, set $1 = 44, else $1 = $2
        mmix.set_register(2, 0);
        mmix.write_tetra(0, 0x6D01022C); // CSNPI $1,$2,44
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 44); // Condition true: 44
    }

    #[test]
    fn test_csev_condition_true() {
        let mut mmix = MMix::new();
        // CSEV $1, $2, $3 - If $2 is even, set $1 = $3, else $1 = $2
        mmix.set_register(2, 80);
        mmix.set_register(3, 20);
        mmix.write_tetra(0, 0x6E010203); // CSEV $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 20); // Condition true: $3
    }

    #[test]
    fn test_csev_condition_false() {
        let mut mmix = MMix::new();
        // CSEV $1, $2, $3 - If $2 is odd, do nothing
        mmix.set_register(1, 22); // Initial value
        mmix.set_register(2, 7);
        mmix.set_register(3, 20);
        mmix.write_tetra(0, 0x6E010203); // CSEV $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 22); // Condition false: unchanged
    }

    #[test]
    fn test_csevi() {
        let mut mmix = MMix::new();
        // CSEVI $1, $2, 12 - If $2 is even, set $1 = 12, else $1 = $2
        mmix.set_register(2, 88);
        mmix.write_tetra(0, 0x6F01020C); // CSEVI $1,$2,12
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 12); // Condition true: 12
    }

    // ========== Floating Point Tests ==========

    #[test]
    fn test_fcmp_less_than() {
        let mut mmix = MMix::new();
        // FCMP $1, $2, $3 - Compare 2.5 < 5.0
        mmix.set_register(2, 2.5f64.to_bits());
        mmix.set_register(3, 5.0f64.to_bits());
        mmix.write_tetra(0, 0x01010203); // FCMP $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1) as i64, -1); // Less than
    }

    #[test]
    fn test_fcmp_greater_than() {
        let mut mmix = MMix::new();
        // FCMP $1, $2, $3 - Compare 10.0 > 3.0
        mmix.set_register(2, 10.0f64.to_bits());
        mmix.set_register(3, 3.0f64.to_bits());
        mmix.write_tetra(0, 0x01010203); // FCMP $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 1); // Greater than
    }

    #[test]
    fn test_fcmp_equal() {
        let mut mmix = MMix::new();
        // FCMP $1, $2, $3 - Compare 7.5 == 7.5
        mmix.set_register(2, 7.5f64.to_bits());
        mmix.set_register(3, 7.5f64.to_bits());
        mmix.write_tetra(0, 0x01010203); // FCMP $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 0); // Equal
    }

    #[test]
    fn test_fcmp_unordered() {
        let mut mmix = MMix::new();
        // FCMP computes $X = [$Y > $Z] − [$Y < $Z]; with a NaN operand
        // both brackets are 0, so $X = 0, and I reports the NaN.
        mmix.set_register(2, f64::NAN.to_bits());
        mmix.set_register(3, 5.0f64.to_bits());
        mmix.write_tetra(0, 0x01010203); // FCMP $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 0);
        assert!((mmix.get_special(SpecialReg::RA) & RA_I) != 0);
    }

    #[test]
    fn test_cmp_cmpu_signed_unsigned_divergence() {
        let mut mmix = MMix::new();
        // $2 has its top bit set: negative as i64, huge as u64. $3 is a
        // small positive value. Signed and unsigned 3-way compare disagree.
        mmix.set_register(2, 0x8000000000000000);
        mmix.set_register(3, 1);
        mmix.write_tetra(0, 0x30010203); // CMP $1,$2,$3
        mmix.write_tetra(4, 0x32040203); // CMPU $4,$2,$3
        assert!(mmix.execute_instruction());
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1) as i64, -1); // signed: $2 < $3
        assert_eq!(mmix.get_register(4) as i64, 1); // unsigned: $2 > $3
    }

    #[test]
    fn test_feql() {
        let mut mmix = MMix::new();
        // FEQL $1, $2, $3 - Test 4.0 == 4.0
        mmix.set_register(2, 4.0f64.to_bits());
        mmix.set_register(3, 4.0f64.to_bits());
        mmix.write_tetra(0, 0x03010203); // FEQL $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 1); // Equal
    }

    #[test]
    fn test_feql_not_equal() {
        let mut mmix = MMix::new();
        // FEQL $1, $2, $3 - Test 4.0 != 5.0
        mmix.set_register(2, 4.0f64.to_bits());
        mmix.set_register(3, 5.0f64.to_bits());
        mmix.write_tetra(0, 0x03010203); // FEQL $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 0); // Not equal
    }

    #[test]
    fn test_fun() {
        let mut mmix = MMix::new();
        // FUN $1, $2, $3 - Test if unordered
        mmix.set_register(2, f64::NAN.to_bits());
        mmix.set_register(3, 1.0f64.to_bits());
        mmix.write_tetra(0, 0x02010203); // FUN $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 1); // Unordered
    }

    #[test]
    fn test_fun_ordered() {
        let mut mmix = MMix::new();
        // FUN $1, $2, $3 - Test if unordered (both normal)
        mmix.set_register(2, 2.0f64.to_bits());
        mmix.set_register(3, 3.0f64.to_bits());
        mmix.write_tetra(0, 0x02010203); // FUN $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 0); // Ordered
    }

    #[test]
    fn test_fcmpe() {
        let mut mmix = MMix::new();
        // FCMPE $1, $2, $3 - Compare 5.0 and 5.001 with epsilon 0.01
        mmix.set_special(SpecialReg::RE, 0.01f64.to_bits());
        mmix.set_register(2, 5.0f64.to_bits());
        mmix.set_register(3, 5.001f64.to_bits());
        mmix.write_tetra(0, 0x11010203); // FCMPE $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 0); // Equal within epsilon
    }

    #[test]
    fn test_feqle() {
        let mut mmix = MMix::new();
        // FEQLE $1, $2, $3 - Test equivalence with epsilon
        mmix.set_special(SpecialReg::RE, 0.1f64.to_bits());
        mmix.set_register(2, 10.0f64.to_bits());
        mmix.set_register(3, 10.05f64.to_bits());
        mmix.write_tetra(0, 0x13010203); // FEQLE $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 1); // Equivalent
    }

    #[test]
    fn test_fune() {
        let mut mmix = MMix::new();
        // FUNE $1, $2, $3 - neither operand nor rE is exceptional (no NaN,
        // rE not negative), so FUNE reports 0: it says nothing about
        // proximity, only whether the inputs are exceptional.
        mmix.set_special(SpecialReg::RE, 0.5f64.to_bits());
        mmix.set_register(2, 7.0f64.to_bits());
        mmix.set_register(3, 7.3f64.to_bits());
        mmix.write_tetra(0, 0x12010203); // FUNE $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 0);
    }

    #[test]
    fn test_fadd() {
        let mut mmix = MMix::new();
        // FADD $1, $2, $3 - Add 2.5 + 3.7
        mmix.set_register(2, 2.5f64.to_bits());
        mmix.set_register(3, 3.7f64.to_bits());
        mmix.write_tetra(0, 0x04010203); // FADD $1,$2,$3
        assert!(mmix.execute_instruction());
        let result = f64::from_bits(mmix.get_register(1));
        assert!((result - 6.2).abs() < 1e-10);
    }

    #[test]
    fn test_fsub() {
        let mut mmix = MMix::new();
        // FSUB $1, $2, $3 - Subtract 10.0 - 3.5
        mmix.set_register(2, 10.0f64.to_bits());
        mmix.set_register(3, 3.5f64.to_bits());
        mmix.write_tetra(0, 0x06010203); // FSUB $1,$2,$3
        assert!(mmix.execute_instruction());
        let result = f64::from_bits(mmix.get_register(1));
        assert!((result - 6.5).abs() < 1e-10);
    }

    #[test]
    fn test_fmul() {
        let mut mmix = MMix::new();
        // FMUL $1, $2, $3 - Multiply 4.0 * 2.5
        mmix.set_register(2, 4.0f64.to_bits());
        mmix.set_register(3, 2.5f64.to_bits());
        mmix.write_tetra(0, 0x10010203); // FMUL $1,$2,$3
        assert!(mmix.execute_instruction());
        let result = f64::from_bits(mmix.get_register(1));
        assert!((result - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_fdiv() {
        let mut mmix = MMix::new();
        // FDIV $1, $2, $3 - Divide 15.0 / 3.0
        mmix.set_register(2, 15.0f64.to_bits());
        mmix.set_register(3, 3.0f64.to_bits());
        mmix.write_tetra(0, 0x14010203); // FDIV $1,$2,$3
        assert!(mmix.execute_instruction());
        let result = f64::from_bits(mmix.get_register(1));
        assert!((result - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_frem() {
        let mut mmix = MMix::new();
        // IEEE 754 remainder of 7.5 by 2.0: 7.5/2 = 3.75 → round-half-even = 4
        // → r = 7.5 - 4·2 = -0.5. (Rust's `%` would give 1.5.)
        mmix.set_register(2, 7.5f64.to_bits());
        mmix.set_register(3, 2.0f64.to_bits());
        mmix.write_tetra(0, 0x16010203);
        assert!(mmix.execute_instruction());
        let result = f64::from_bits(mmix.get_register(1));
        assert!((result + 0.5).abs() < 1e-10, "got {}", result);
    }

    #[test]
    fn test_frem_zero_divisor_nan() {
        let mut mmix = MMix::new();
        mmix.set_register(2, 5.0f64.to_bits());
        mmix.set_register(3, 0.0f64.to_bits());
        mmix.write_tetra(0, 0x16010203);
        assert!(mmix.execute_instruction());
        assert!(f64::from_bits(mmix.get_register(1)).is_nan());
        assert!((mmix.get_special(SpecialReg::RA) & RA_I) != 0);
    }

    #[test]
    fn test_fsqrt() {
        let mut mmix = MMix::new();
        // FSQRT $1, $3 - Square root of 16.0
        mmix.set_register(3, 16.0f64.to_bits());
        mmix.write_tetra(0, 0x15010003); // FSQRT $1,$0,$3
        assert!(mmix.execute_instruction());
        let result = f64::from_bits(mmix.get_register(1));
        assert!((result - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_fint() {
        let mut mmix = MMix::new();
        // FINT $1, $3 - Round 3.7 to nearest integer
        mmix.set_register(3, 3.7f64.to_bits());
        mmix.write_tetra(0, 0x17010003); // FINT $1,$0,$3
        assert!(mmix.execute_instruction());
        let result = f64::from_bits(mmix.get_register(1));
        assert!((result - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_fix() {
        let mut mmix = MMix::new();
        // Default rA mode 0 = ROUND_NEAR: 42.9 → 43.
        mmix.set_register(3, 42.9f64.to_bits());
        mmix.write_tetra(0, 0x05010003); // FIX $1,$0,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 43);
        assert!((mmix.get_special(SpecialReg::RA) & RA_X) != 0);
    }

    #[test]
    fn test_fix_trunc_mode() {
        let mut mmix = MMix::new();
        // rA mode 1 = ROUND_OFF (toward zero): 42.9 → 42.
        mmix.set_special(SpecialReg::RA, 1 << RA_ROUND_SHIFT);
        mmix.set_register(3, 42.9f64.to_bits());
        mmix.write_tetra(0, 0x05010003);
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 42);
    }

    #[test]
    fn test_fix_negative() {
        let mut mmix = MMix::new();
        // Mode 0 = NEAR rounds -17.8 → -18.
        mmix.set_register(3, (-17.8f64).to_bits());
        mmix.write_tetra(0, 0x05010003);
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1) as i64, -18);
    }

    #[test]
    fn test_fix_nan_sets_w_and_i() {
        let mut mmix = MMix::new();
        mmix.set_register(3, f64::NAN.to_bits());
        mmix.write_tetra(0, 0x05010003);
        assert!(mmix.execute_instruction());
        let ra = mmix.get_special(SpecialReg::RA);
        assert!((ra & RA_W) != 0);
        assert!((ra & RA_I) != 0);
    }

    #[test]
    fn test_fixu() {
        let mut mmix = MMix::new();
        // Mode 0 = NEAR with round-half-to-even: 99.5 → 100 (100 is even).
        mmix.set_register(3, 99.5f64.to_bits());
        mmix.write_tetra(0, 0x07010003);
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 100);
        assert!((mmix.get_special(SpecialReg::RA) & RA_X) != 0);
    }

    #[test]
    fn test_flot() {
        let mut mmix = MMix::new();
        // FLOT $1, $3 - Convert signed integer 42 to float
        mmix.set_register(3, 42);
        mmix.write_tetra(0, 0x08010003); // FLOT $1,$0,$3
        assert!(mmix.execute_instruction());
        let result = f64::from_bits(mmix.get_register(1));
        assert!((result - 42.0).abs() < 1e-10);
    }

    #[test]
    fn test_flot_negative() {
        let mut mmix = MMix::new();
        // FLOT $1, $3 - Convert signed integer -100 to float
        mmix.set_register(3, (-100i64) as u64);
        mmix.write_tetra(0, 0x08010003); // FLOT $1,$0,$3
        assert!(mmix.execute_instruction());
        let result = f64::from_bits(mmix.get_register(1));
        assert!((result - (-100.0)).abs() < 1e-10);
    }

    #[test]
    fn test_floti() {
        let mut mmix = MMix::new();
        // FLOTI $1, $0, 100 - Convert immediate signed 100 to float (Y=$0 is rounding mode, Z=100 is value)
        mmix.write_tetra(0, 0x09010064); // FLOTI $1,$0,100 (X=01, Y=00, Z=64)
        assert!(mmix.execute_instruction());
        let result = f64::from_bits(mmix.get_register(1));
        assert!((result - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_floti_negative() {
        let mut mmix = MMix::new();
        // FLOTI $1, $0, -1 - Convert immediate signed -1 to float (Y=$0, Z=0xFF=-1 as signed byte)
        mmix.write_tetra(0, 0x090100FF); // FLOTI $1,$0,255 (X=01, Y=00, Z=FF which is -1 signed)
        assert!(mmix.execute_instruction());
        let result = f64::from_bits(mmix.get_register(1));
        assert!((result - (-1.0)).abs() < 1e-10);
    }

    #[test]
    fn test_flotu() {
        let mut mmix = MMix::new();
        // FLOTU $1, $3 - Convert unsigned integer to float
        mmix.set_register(3, 1000);
        mmix.write_tetra(0, 0x0A010003); // FLOTU $1,$0,$3
        assert!(mmix.execute_instruction());
        let result = f64::from_bits(mmix.get_register(1));
        assert!((result - 1000.0).abs() < 1e-10);
    }

    #[test]
    fn test_flotui() {
        let mut mmix = MMix::new();
        // FLOTUI $1, $0, 244 - Convert immediate unsigned 244 to float (Y=$0, Z=244)
        mmix.write_tetra(0, 0x0B0100F4); // FLOTUI $1,$0,244 (X=01, Y=00, Z=F4=244)
        assert!(mmix.execute_instruction());
        let result = f64::from_bits(mmix.get_register(1));
        assert!((result - 244.0).abs() < 1e-10);
    }

    #[test]
    fn test_sflot() {
        let mut mmix = MMix::new();
        // SFLOT $1, $3 - Convert signed to short float (f32 precision)
        mmix.set_register(3, 123);
        mmix.write_tetra(0, 0x0C010003); // SFLOT $1,$0,$3
        assert!(mmix.execute_instruction());
        let result = f64::from_bits(mmix.get_register(1));
        assert!((result - 123.0).abs() < 1e-5);
    }

    #[test]
    fn test_sfloti() {
        let mut mmix = MMix::new();
        // SFLOTI $1, 64 - Convert immediate signed to short float
        mmix.write_tetra(0, 0x0D010040); // SFLOTI $1,64 (YZ=0x0040)
        assert!(mmix.execute_instruction());
        let result = f64::from_bits(mmix.get_register(1));
        assert!((result - 64.0).abs() < 1e-5);
    }

    #[test]
    fn test_sflotu() {
        let mut mmix = MMix::new();
        // SFLOTU $1, $3 - Convert unsigned to short float
        mmix.set_register(3, 777);
        mmix.write_tetra(0, 0x0E010003); // SFLOTU $1,$0,$3
        assert!(mmix.execute_instruction());
        let result = f64::from_bits(mmix.get_register(1));
        assert!((result - 777.0).abs() < 1e-5);
    }

    #[test]
    fn test_sflotui() {
        let mut mmix = MMix::new();
        // SFLOTUI $1, 255 - Convert immediate unsigned to short float
        mmix.write_tetra(0, 0x0F0100FF); // SFLOTUI $1,255 (YZ=0x00FF)
        assert!(mmix.execute_instruction());
        let result = f64::from_bits(mmix.get_register(1));
        assert!((result - 255.0).abs() < 1e-5);
    }

    #[test]
    fn test_fint_round_near() {
        let mut mmix = MMix::new();
        // FINT $1, $0, $3 - Integerize with ROUND_NEAR mode
        mmix.set_special(SpecialReg::RA, 0 << RA_ROUND_SHIFT); // Round mode 0 = ROUND_NEAR
        mmix.set_register(3, 3.7f64.to_bits());
        mmix.write_tetra(0, 0x17010003); // FINT $1,$0,$3
        assert!(mmix.execute_instruction());
        let result = f64::from_bits(mmix.get_register(1));
        assert!((result - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_fint_round_off() {
        let mut mmix = MMix::new();
        // Mode 1 = ROUND_OFF (toward zero / truncate).
        mmix.set_special(SpecialReg::RA, 1 << RA_ROUND_SHIFT);
        mmix.set_register(3, 3.7f64.to_bits());
        mmix.write_tetra(0, 0x17010003); // FINT $1,$0,$3
        assert!(mmix.execute_instruction());
        let result = f64::from_bits(mmix.get_register(1));
        assert!((result - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_fint_round_up() {
        let mut mmix = MMix::new();
        // Mode 2 = ROUND_UP (toward +∞).
        mmix.set_special(SpecialReg::RA, 2 << RA_ROUND_SHIFT);
        mmix.set_register(3, 3.2f64.to_bits());
        mmix.write_tetra(0, 0x17010003); // FINT $1,$0,$3
        assert!(mmix.execute_instruction());
        let result = f64::from_bits(mmix.get_register(1));
        assert!((result - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_fint_round_down() {
        let mut mmix = MMix::new();
        // Mode 3 = ROUND_DOWN (toward -∞).
        mmix.set_special(SpecialReg::RA, 3 << RA_ROUND_SHIFT);
        mmix.set_register(3, 3.9f64.to_bits());
        mmix.write_tetra(0, 0x17010003); // FINT $1,$0,$3
        assert!(mmix.execute_instruction());
        let result = f64::from_bits(mmix.get_register(1));
        assert!((result - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_fint_round_down_negative() {
        let mut mmix = MMix::new();
        // Mode 3 floors -3.2 → -4.0 (toward -∞).
        mmix.set_special(SpecialReg::RA, 3 << RA_ROUND_SHIFT);
        mmix.set_register(3, (-3.2f64).to_bits());
        mmix.write_tetra(0, 0x17010003);
        assert!(mmix.execute_instruction());
        let result = f64::from_bits(mmix.get_register(1));
        assert!((result + 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_fint_inexact_flag() {
        let mut mmix = MMix::new();
        mmix.set_special(SpecialReg::RA, 0 << RA_ROUND_SHIFT);
        mmix.set_register(3, 3.5f64.to_bits());
        mmix.write_tetra(0, 0x17010003);
        assert!(mmix.execute_instruction());
        assert!((mmix.get_special(SpecialReg::RA) & RA_X) != 0);
    }

    // ========== Zero or Set Tests ==========

    #[test]
    fn test_zsn_condition_true() {
        let mut mmix = MMix::new();
        // ZSN $1, $2, $3 - If $2 < 0, set $1 = $3, else $1 = 0
        mmix.set_register(2, (-10i64) as u64);
        mmix.set_register(3, 50);
        mmix.write_tetra(0, 0x70010203); // ZSN $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 50); // Condition true: $3
    }

    #[test]
    fn test_zsn_condition_false() {
        let mut mmix = MMix::new();
        // ZSN $1, $2, $3 - If $2 >= 0, set $1 = 0
        mmix.set_register(2, 100);
        mmix.set_register(3, 50);
        mmix.write_tetra(0, 0x70010203); // ZSN $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 0); // Condition false: 0
    }

    #[test]
    fn test_zsni() {
        let mut mmix = MMix::new();
        // ZSNI $1, $2, 50 - If $2 < 0, set $1 = 50, else $1 = 0
        mmix.set_register(2, (-1i64) as u64);
        mmix.write_tetra(0, 0x71010232); // ZSNI $1,$2,50
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 50); // Condition true: 50
    }

    #[test]
    fn test_zsz_condition_true() {
        let mut mmix = MMix::new();
        // ZSZ $1, $2, $3 - If $2 == 0, set $1 = $3, else $1 = 0
        mmix.set_register(2, 0);
        mmix.set_register(3, 20);
        mmix.write_tetra(0, 0x72010203); // ZSZ $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 20); // Condition true: $3
    }

    #[test]
    fn test_zsz_condition_false() {
        let mut mmix = MMix::new();
        // ZSZ $1, $2, $3 - Set $1 = 0 if $1 is not zero
        mmix.set_register(1, 1);
        mmix.set_register(2, 10);
        mmix.set_register(3, 20);
        mmix.write_tetra(0, 0x72010203); // ZSZ $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 0); // Condition false: 0
    }

    #[test]
    fn test_zszi() {
        let mut mmix = MMix::new();
        // ZSZI $1, $2, 15 - If $2 == 0, set $1 = 15, else $1 = 0
        mmix.set_register(2, 0);
        mmix.write_tetra(0, 0x7301020F); // ZSZI $1,$2,15
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 15); // Condition true: 15
    }

    #[test]
    fn test_zsp_condition_true() {
        let mut mmix = MMix::new();
        // ZSP $1, $2, $3 - If $2 > 0, set $1 = $3, else $1 = 0
        mmix.set_register(2, 5);
        mmix.set_register(3, 7);
        mmix.write_tetra(0, 0x74010203); // ZSP $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 7); // Condition true: $3
    }

    #[test]
    fn test_zsp_condition_false_zero() {
        let mut mmix = MMix::new();
        // ZSP $1, $2, $3 - If $2 <= 0, set $1 = 0
        mmix.set_register(2, 0);
        mmix.set_register(3, 7);
        mmix.write_tetra(0, 0x74010203); // ZSP $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 0); // Condition false: 0
    }

    #[test]
    fn test_zspi() {
        let mut mmix = MMix::new();
        // ZSPI $1, $2, 25 - If $2 > 0, set $1 = 25, else $1 = 0
        mmix.set_register(2, 50);
        mmix.write_tetra(0, 0x75010219); // ZSPI $1,$2,25
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 25); // Condition true: 25
    }

    #[test]
    fn test_zsod_condition_true() {
        let mut mmix = MMix::new();
        // ZSOD $1, $2, $3 - If $2 is odd, set $1 = $3, else $1 = 0
        mmix.set_register(2, 7);
        mmix.set_register(3, 15);
        mmix.write_tetra(0, 0x76010203); // ZSOD $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 15); // Condition true: $3
    }

    #[test]
    fn test_zsod_condition_false() {
        let mut mmix = MMix::new();
        // ZSOD $1, $2, $3 - Set $1 = 0 if $1 is even
        mmix.set_register(1, 8);
        mmix.set_register(2, 10);
        mmix.set_register(3, 15);
        mmix.write_tetra(0, 0x76010203); // ZSOD $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 0); // Condition false: 0
    }

    #[test]
    fn test_zsodi() {
        let mut mmix = MMix::new();
        // ZSODI $1, $2, 11 - If $2 is odd, set $1 = 11, else $1 = 0
        mmix.set_register(2, 99);
        mmix.write_tetra(0, 0x7701020B); // ZSODI $1,$2,11
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 11); // Condition true: 11
    }

    #[test]
    fn test_zsnn_condition_true_positive() {
        let mut mmix = MMix::new();
        // ZSNN $1, $2, $3 - If $2 >= 0, set $1 = $3, else $1 = 0
        mmix.set_register(2, 30);
        mmix.set_register(3, 40);
        mmix.write_tetra(0, 0x78010203); // ZSNN $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 40); // Condition true: $3
    }

    #[test]
    fn test_zsnn_condition_true_zero() {
        let mut mmix = MMix::new();
        // ZSNN $1, $2, $3 - If $2 >= 0, set $1 = $3, else $1 = 0
        mmix.set_register(2, 0);
        mmix.set_register(3, 40);
        mmix.write_tetra(0, 0x78010203); // ZSNN $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 40); // Condition true: $3
    }

    #[test]
    fn test_zsnn_condition_false() {
        let mut mmix = MMix::new();
        // ZSNN $1, $2, $3 - If $2 < 0, set $1 = 0
        mmix.set_register(2, (-5i64) as u64);
        mmix.set_register(3, 40);
        mmix.write_tetra(0, 0x78010203); // ZSNN $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 0); // Condition false: 0
    }

    #[test]
    fn test_zsnni() {
        let mut mmix = MMix::new();
        // ZSNNI $1, $2, 8 - If $2 >= 0, set $1 = 8, else $1 = 0
        mmix.set_register(2, 92);
        mmix.write_tetra(0, 0x79010208); // ZSNNI $1,$2,8
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 8); // Condition true: 8
    }

    #[test]
    fn test_zsnz_condition_true() {
        let mut mmix = MMix::new();
        // ZSNZ $1, $2, $3 - If $2 != 0, set $1 = $3, else $1 = 0
        mmix.set_register(2, 100);
        mmix.set_register(3, 200);
        mmix.write_tetra(0, 0x7A010203); // ZSNZ $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 200); // Condition true: $3
    }

    #[test]
    fn test_zsnz_condition_false() {
        let mut mmix = MMix::new();
        // ZSNZ $1, $2, $3 - If $2 == 0, set $1 = 0
        mmix.set_register(2, 0);
        mmix.set_register(3, 200);
        mmix.write_tetra(0, 0x7A010203); // ZSNZ $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 0); // Condition false: 0
    }

    #[test]
    fn test_zsnzi() {
        let mut mmix = MMix::new();
        // ZSNZI $1, $2, 33 - If $2 != 0, set $1 = 33, else $1 = 0
        mmix.set_register(2, 67);
        mmix.write_tetra(0, 0x7B010221); // ZSNZI $1,$2,33
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 33); // Condition true: 33
    }

    #[test]
    fn test_zsnp_condition_true_negative() {
        let mut mmix = MMix::new();
        // ZSNP $1, $2, $3 - If $2 <= 0, set $1 = $3, else $1 = 0
        mmix.set_register(2, (-100i64) as u64);
        mmix.set_register(3, 25);
        mmix.write_tetra(0, 0x7C010203); // ZSNP $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 25); // Condition true: $3
    }

    #[test]
    fn test_zsnp_condition_true_zero() {
        let mut mmix = MMix::new();
        // ZSNP $1, $2, $3 - If $2 == 0, set $1 = $3, else $1 = 0
        mmix.set_register(2, 0);
        mmix.set_register(3, 25);
        mmix.write_tetra(0, 0x7C010203); // ZSNP $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 25); // Condition true: $3
    }

    #[test]
    fn test_zsnp_condition_false() {
        let mut mmix = MMix::new();
        // ZSNP $1, $2, $3 - Set $1 = 0 if $1 is positive
        mmix.set_register(1, 1);
        mmix.set_register(2, 50);
        mmix.set_register(3, 25);
        mmix.write_tetra(0, 0x7C010203); // ZSNP $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 0); // Condition false: 0
    }

    #[test]
    fn test_zsnpi() {
        let mut mmix = MMix::new();
        // ZSNPI $1, $2, 44 - If $2 <= 0, set $1 = 44, else $1 = 0
        mmix.set_register(2, 0);
        mmix.write_tetra(0, 0x7D01022C); // ZSNPI $1,$2,44
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 44); // Condition true: 44
    }

    #[test]
    fn test_zsev_condition_true() {
        let mut mmix = MMix::new();
        // ZSEV $1, $2, $3 - If $2 is even, set $1 = $3, else $1 = 0
        mmix.set_register(2, 80);
        mmix.set_register(3, 20);
        mmix.write_tetra(0, 0x7E010203); // ZSEV $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 20); // Condition true: $3
    }

    #[test]
    fn test_zsev_condition_false() {
        let mut mmix = MMix::new();
        // ZSEV $1, $2, $3 - If $2 is odd, set $1 = 0
        mmix.set_register(2, 7);
        mmix.set_register(3, 20);
        mmix.write_tetra(0, 0x7E010203); // ZSEV $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 0); // Condition false: 0
    }

    #[test]
    fn test_zsevi() {
        let mut mmix = MMix::new();
        // ZSEVI $1, $2, 12 - If $2 is even, set $1 = 12, else $1 = 0
        mmix.set_register(2, 88);
        mmix.write_tetra(0, 0x7F01020C); // ZSEVI $1,$2,12
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 12); // Condition true: 12
    }

    // ========== Special Load/Store Tests ==========

    #[test]
    fn test_ldht() {
        let mut mmix = MMix::new();
        // LDHT $1, $2, $3 - Load high tetra
        mmix.set_register(2, 100);
        mmix.set_register(3, 4);
        mmix.write_tetra(104, 0x12345678);
        mmix.write_tetra(0, 0x92010203); // LDHT $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 0x1234567800000000);
    }

    #[test]
    fn test_ldhti() {
        let mut mmix = MMix::new();
        // LDHTI $1, $2, 8 - Load high tetra immediate
        mmix.set_register(2, 100);
        mmix.write_tetra(108, 0xABCDEF01);
        mmix.write_tetra(0, 0x93010208); // LDHTI $1,$2,8
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 0xABCDEF0100000000);
    }

    #[test]
    fn test_cswap_success() {
        let mut mmix = MMix::new();
        // CSWAP $1, $2, $3 - Compare and swap (successful)
        let addr = 1000u64;
        let old_value = 0x123456789ABCDEF0u64;
        let new_value = 0xFEDCBA9876543210u64;

        mmix.write_octa(addr, old_value);
        mmix.set_special(SpecialReg::RP, old_value); // Set compare value
        mmix.set_register(1, new_value); // New value to write
        mmix.set_register(2, addr);
        mmix.set_register(3, 0);

        mmix.write_tetra(0, 0x94010203); // CSWAP $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 1); // Success
        assert_eq!(mmix.read_octa(addr), new_value); // Memory updated
    }

    #[test]
    fn test_cswap_failure() {
        let mut mmix = MMix::new();
        // CSWAP $1, $2, $3 - Compare and swap (failed)
        let addr = 1000u64;
        let mem_value = 0x123456789ABCDEF0u64;
        let compare_value = 0x1111111111111111u64;
        let new_value = 0xFEDCBA9876543210u64;

        mmix.write_octa(addr, mem_value);
        mmix.set_special(SpecialReg::RP, compare_value); // Different compare value
        mmix.set_register(1, new_value);
        mmix.set_register(2, addr);
        mmix.set_register(3, 0);

        mmix.write_tetra(0, 0x94010203); // CSWAP $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 0); // Failure
        assert_eq!(mmix.read_octa(addr), mem_value); // Memory unchanged
        assert_eq!(mmix.get_special(SpecialReg::RP), mem_value); // rP <- M8[$Y+$Z]
    }

    #[test]
    fn test_cswapi() {
        let mut mmix = MMix::new();
        // CSWAPI $1, $2, 16 - Compare and swap immediate
        let addr = 2000u64;
        let old_value = 0xAAAAAAAAAAAAAAAAu64;
        let new_value = 0xBBBBBBBBBBBBBBBBu64;

        mmix.write_octa(addr + 16, old_value);
        mmix.set_special(SpecialReg::RP, old_value);
        mmix.set_register(1, new_value);
        mmix.set_register(2, addr);

        mmix.write_tetra(0, 0x95010210); // CSWAPI $1,$2,16
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 1); // Success
        assert_eq!(mmix.read_octa(addr + 16), new_value);
    }

    #[test]
    fn test_cswapi_failure() {
        let mut mmix = MMix::new();
        // CSWAPI $1, $2, 16 - Compare and swap immediate (failed)
        let addr = 2000u64;
        let mem_value = 0x123456789ABCDEF0u64;
        let compare_value = 0x1111111111111111u64;
        let new_value = 0xFEDCBA9876543210u64;

        mmix.write_octa(addr + 16, mem_value);
        mmix.set_special(SpecialReg::RP, compare_value); // Different compare value
        mmix.set_register(1, new_value);
        mmix.set_register(2, addr);

        mmix.write_tetra(0, 0x95010210); // CSWAPI $1,$2,16
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 0); // Failure
        assert_eq!(mmix.read_octa(addr + 16), mem_value); // Memory unchanged
        assert_eq!(mmix.get_special(SpecialReg::RP), mem_value); // rP <- M8[$Y+Z]
    }

    #[test]
    fn octa_load_reads_the_aligned_base_from_any_address_in_the_block() {
        // LDO $1,$2,$3 with $3 sweeping the octabyte's own aligned block:
        // every one of the 8 addresses must resolve to the same value:
        // M8[A] = M8[8*floor(A/8)].
        let base = 800u64;
        let value = 0x1122334455667788u64;
        for offset in 0u64..8 {
            let mut mmix = MMix::new();
            mmix.write_octa(base, value);
            mmix.set_register(2, base);
            mmix.set_register(3, offset);
            mmix.write_tetra(0, 0x8C010203); // LDO $1,$2,$3
            assert!(mmix.execute_instruction());
            assert_eq!(mmix.get_register(1), value, "offset {offset}");
        }
    }

    #[test]
    fn octa_store_at_misaligned_address_lands_at_aligned_base() {
        // STO $1,$2,$3 at base+5: the write must land at the aligned base,
        // not straddle base and base+8. Reverting write_octa's mask alone
        // fails this, since the value would then land 5 bytes high.
        let mut mmix = MMix::new();
        let base = 800u64;
        let value = 0x99AABBCCDDEEFF00u64;
        mmix.set_register(1, value);
        mmix.set_register(2, base);
        mmix.set_register(3, 5);
        mmix.write_tetra(0, 0xAC010203); // STO $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.read_octa(base), value);
    }

    #[test]
    fn tetra_load_reads_the_aligned_base_from_any_address_in_the_block() {
        // LDTU $1,$2,$3 with $3 sweeping the tetra's own aligned block.
        let base = 400u64;
        let value = 0x11223344u32;
        for offset in 0u64..4 {
            let mut mmix = MMix::new();
            mmix.write_tetra(base, value);
            mmix.set_register(2, base);
            mmix.set_register(3, offset);
            mmix.write_tetra(0, 0x8A010203); // LDTU $1,$2,$3
            assert!(mmix.execute_instruction());
            assert_eq!(mmix.get_register(1), value as u64, "offset {offset}");
        }
    }

    #[test]
    fn tetra_store_at_misaligned_address_lands_at_aligned_base() {
        // STT $1,$2,$3 at base+3: reverting write_tetra's mask alone fails
        // this.
        let mut mmix = MMix::new();
        let base = 400u64;
        let value = 0xAABBCCDDu32;
        mmix.set_register(1, value as u64);
        mmix.set_register(2, base);
        mmix.set_register(3, 3);
        mmix.write_tetra(0, 0xA8010203); // STT $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.read_tetra(base), value);
    }

    #[test]
    fn wyde_load_reads_the_aligned_base_from_any_address_in_the_block() {
        // LDWU $1,$2,$3 with $3 sweeping the wyde's own aligned block.
        let base = 200u64;
        let value = 0xBEEFu16;
        for offset in 0u64..2 {
            let mut mmix = MMix::new();
            mmix.write_wyde(base, value);
            mmix.set_register(2, base);
            mmix.set_register(3, offset);
            mmix.write_tetra(0, 0x86010203); // LDWU $1,$2,$3
            assert!(mmix.execute_instruction());
            assert_eq!(mmix.get_register(1), value as u64, "offset {offset}");
        }
    }

    #[test]
    fn wyde_store_at_misaligned_address_lands_at_aligned_base() {
        // STW $1,$2,$3 at base+1: reverting write_wyde's mask alone fails
        // this.
        let mut mmix = MMix::new();
        let base = 200u64;
        let value = 0xCAFEu16;
        mmix.set_register(1, value as u64);
        mmix.set_register(2, base);
        mmix.set_register(3, 1);
        mmix.write_tetra(0, 0xA4010203); // STW $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.read_wyde(base), value);
    }

    #[test]
    fn byte_access_at_an_odd_address_still_reads_that_byte() {
        // A byte is its own alignment: read_byte/write_byte take no mask,
        // unlike the wider accessors above. There is no fix to revert here —
        // this guards against someone later "helpfully" masking read_byte to
        // match its wider siblings.
        let mut mmix = MMix::new();
        mmix.write_byte(801, 0x42);
        assert_eq!(mmix.read_byte(801), 0x42);
        assert_eq!(mmix.read_byte(800), 0);
    }

    #[test]
    fn test_ldunc() {
        let mut mmix = MMix::new();
        // LDUNC $1, $2, $3 - Load uncached
        mmix.set_register(2, 500);
        mmix.set_register(3, 24);
        mmix.write_octa(524, 0x0123456789ABCDEFu64);
        mmix.write_tetra(0, 0x96010203); // LDUNC $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 0x0123456789ABCDEFu64);
    }

    #[test]
    fn test_ldunci() {
        let mut mmix = MMix::new();
        // LDUNCI $1, $2, 32 - Load uncached immediate
        mmix.set_register(2, 600);
        mmix.write_octa(632, 0xFEDCBA9876543210u64);
        mmix.write_tetra(0, 0x97010220); // LDUNCI $1,$2,32
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 0xFEDCBA9876543210u64);
    }

    #[test]
    fn test_ldvts() {
        let mut mmix = MMix::new();
        // LDVTS $1, $2, $3 - Load virtual translation status
        mmix.set_register(2, 0x1000);
        mmix.set_register(3, 0);
        mmix.write_tetra(0, 0x98010203); // LDVTS $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 0); // Returns 0 in simulation
    }

    #[test]
    fn test_ldvtsi() {
        let mut mmix = MMix::new();
        // LDVTSI $1, $2, 0 - Load virtual translation status immediate
        mmix.set_register(2, 0x2000);
        mmix.write_tetra(0, 0x99010200); // LDVTSI $1,$2,0
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 0); // Returns 0 in simulation
    }

    #[test]
    fn test_preld() {
        let mut mmix = MMix::new();
        // PRELD $1, $2, $3 - Preload data (no-op)
        mmix.write_tetra(0, 0x9A010203); // PRELD $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 4); // PC advanced
    }

    #[test]
    fn test_preldi() {
        let mut mmix = MMix::new();
        // PRELDI $1, $2, 64 - Preload data immediate (no-op)
        mmix.write_tetra(0, 0x9B010240); // PRELDI $1,$2,64
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 4); // PC advanced
    }

    #[test]
    fn test_prego() {
        let mut mmix = MMix::new();
        // PREGO $1, $2, $3 - Preload to go (no-op)
        mmix.write_tetra(0, 0x9C010203); // PREGO $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 4); // PC advanced
    }

    #[test]
    fn test_pregoi() {
        let mut mmix = MMix::new();
        // PREGOI $1, $2, 128 - Preload to go immediate (no-op)
        mmix.write_tetra(0, 0x9D010280); // PREGOI $1,$2,128
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 4); // PC advanced
    }

    #[test]
    fn test_go() {
        let mut mmix = MMix::new();
        // GO $1, $2, $3 - Go to location
        mmix.set_register(2, 1000);
        mmix.set_register(3, 24);
        mmix.write_tetra(0, 0x9E010203); // GO $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 4); // Return address
        assert_eq!(mmix.get_pc(), 1024); // Jump to 1000 + 24
    }

    #[test]
    fn test_goi() {
        let mut mmix = MMix::new();
        // GOI $1, $2, 200 - Go to location immediate
        mmix.set_register(2, 5000);
        mmix.write_tetra(0, 0x9F0102C8); // GOI $1,$2,200
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 4); // Return address
        assert_eq!(mmix.get_pc(), 5200); // Jump to 5000 + 200
    }

    // ========== Stack/Sync/Store Tests ==========

    #[test]
    fn test_stsf() {
        let mut mmix = MMix::new();
        // STSF $1, $2, $3 - Store short float
        let f64_value = std::f64::consts::PI;
        mmix.set_register(1, f64_value.to_bits());
        mmix.set_register(2, 1000);
        mmix.set_register(3, 8);
        mmix.write_tetra(0, 0xB0010203); // STSF $1,$2,$3
        assert!(mmix.execute_instruction());

        let stored_tetra = mmix.read_tetra(1008);
        let f32_value = f32::from_bits(stored_tetra);
        assert!((f32_value - std::f32::consts::PI).abs() < 1e-5);
    }

    #[test]
    fn test_stsfi() {
        let mut mmix = MMix::new();
        // STSFI $1, $2, 16 - Store short float immediate
        let f64_value = std::f64::consts::E;
        mmix.set_register(1, f64_value.to_bits());
        mmix.set_register(2, 2000);
        mmix.write_tetra(0, 0xB1010210); // STSFI $1,$2,16
        assert!(mmix.execute_instruction());

        let stored_tetra = mmix.read_tetra(2016);
        let f32_value = f32::from_bits(stored_tetra);
        assert!((f32_value - std::f32::consts::E).abs() < 1e-5);
    }

    #[test]
    fn test_stht() {
        let mut mmix = MMix::new();
        // STHT $1, $2, $3 - Store high tetra
        mmix.set_register(1, 0x1234567890ABCDEFu64);
        mmix.set_register(2, 500);
        mmix.set_register(3, 12);
        mmix.write_tetra(0, 0xB2010203); // STHT $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.read_tetra(512), 0x12345678);
    }

    #[test]
    fn test_sthti() {
        let mut mmix = MMix::new();
        // STHTI $1, $2, 24 - Store high tetra immediate
        mmix.set_register(1, 0xFEDCBA9876543210u64);
        mmix.set_register(2, 600);
        mmix.write_tetra(0, 0xB7010218); // STHTI $1,$2,24
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.read_tetra(624), 0xFEDCBA98);
    }

    #[test]
    fn test_stco() {
        let mut mmix = MMix::new();
        // STCO $X, $Y, $Z - Store constant octabyte (X=42)
        mmix.set_register(2, 1500);
        mmix.set_register(3, 8);
        mmix.write_tetra(0, 0xB42A0203); // STCO $42,$2,$3 (X=0x2A=42)
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.read_octa(1508), 42);
    }

    #[test]
    fn test_stcoi() {
        let mut mmix = MMix::new();
        // STCOI $X, $Y, Z - Store constant octabyte immediate (X=100)
        mmix.set_register(2, 2500);
        mmix.write_tetra(0, 0xB5640220); // STCOI $100,$2,32 (X=0x64=100)
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.read_octa(2532), 100);
    }

    #[test]
    fn test_stunc() {
        let mut mmix = MMix::new();
        // STUNC $1, $2, $3 - Store uncached
        mmix.set_register(1, 0xABCDEF0123456789u64);
        mmix.set_register(2, 3000);
        mmix.set_register(3, 16);
        mmix.write_tetra(0, 0xB6010203); // STUNC $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.read_octa(3016), 0xABCDEF0123456789u64);
    }

    #[test]
    fn test_stunci() {
        let mut mmix = MMix::new();
        // STUNCI $1, $2, 40 - Store uncached immediate
        mmix.set_register(1, 0x123456789ABCDEFu64);
        mmix.set_register(2, 4000);
        mmix.write_tetra(0, 0xB7010228); // STUNCI $1,$2,40
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.read_octa(4040), 0x123456789ABCDEFu64);
    }

    #[test]
    fn test_syncd() {
        let mut mmix = MMix::new();
        // SYNCD $1, $2, $3 - Synchronize data (no-op)
        mmix.write_tetra(0, 0xB8010203); // SYNCD $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 4); // PC advanced
    }

    #[test]
    fn test_syncdi() {
        let mut mmix = MMix::new();
        // SYNCDI $1, $2, 64 - Synchronize data immediate (no-op)
        mmix.write_tetra(0, 0xB9010203); // SYNCDI $1,$2,64
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 4); // PC advanced
    }

    #[test]
    fn test_prest() {
        let mut mmix = MMix::new();
        // PREST $1, $2, $3 - Prestore (no-op)
        mmix.write_tetra(0, 0xBA010203); // PREST $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 4); // PC advanced
    }

    #[test]
    fn test_presti() {
        let mut mmix = MMix::new();
        // PRESTI $1, $2, 128 - Prestore immediate (no-op)
        mmix.write_tetra(0, 0xBB010280); // PRESTI $1,$2,128
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 4); // PC advanced
    }

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

    // ==================== Floating-point: rA flag coverage ====================

    #[test]
    fn test_fadd_overflow_sets_o() {
        let mut mmix = MMix::new();
        mmix.set_register(2, f64::MAX.to_bits());
        mmix.set_register(3, f64::MAX.to_bits());
        mmix.write_tetra(0, 0x04010203); // FADD $1,$2,$3
        assert!(mmix.execute_instruction());
        assert!(f64::from_bits(mmix.get_register(1)).is_infinite());
        assert!((mmix.get_special(SpecialReg::RA) & RA_O) != 0);
    }

    #[test]
    fn test_fsub_inf_minus_inf_sets_i() {
        let mut mmix = MMix::new();
        mmix.set_register(2, f64::INFINITY.to_bits());
        mmix.set_register(3, f64::INFINITY.to_bits());
        mmix.write_tetra(0, 0x06010203); // FSUB $1,$2,$3
        assert!(mmix.execute_instruction());
        assert!(f64::from_bits(mmix.get_register(1)).is_nan());
        assert!((mmix.get_special(SpecialReg::RA) & RA_I) != 0);
    }

    #[test]
    fn test_fmul_zero_times_inf_sets_i() {
        let mut mmix = MMix::new();
        mmix.set_register(2, 0.0f64.to_bits());
        mmix.set_register(3, f64::INFINITY.to_bits());
        mmix.write_tetra(0, 0x10010203); // FMUL $1,$2,$3
        assert!(mmix.execute_instruction());
        assert!(f64::from_bits(mmix.get_register(1)).is_nan());
        assert!((mmix.get_special(SpecialReg::RA) & RA_I) != 0);
    }

    #[test]
    fn test_fdiv_by_zero_sets_z() {
        let mut mmix = MMix::new();
        mmix.set_register(2, 1.0f64.to_bits());
        mmix.set_register(3, 0.0f64.to_bits());
        mmix.write_tetra(0, 0x14010203); // FDIV $1,$2,$3
        assert!(mmix.execute_instruction());
        assert!(f64::from_bits(mmix.get_register(1)).is_infinite());
        let ra = mmix.get_special(SpecialReg::RA);
        assert!((ra & RA_Z) != 0, "rA={:#x}", ra);
    }

    #[test]
    fn test_fdiv_zero_by_zero_sets_i() {
        let mut mmix = MMix::new();
        mmix.set_register(2, 0.0f64.to_bits());
        mmix.set_register(3, 0.0f64.to_bits());
        mmix.write_tetra(0, 0x14010203);
        assert!(mmix.execute_instruction());
        assert!(f64::from_bits(mmix.get_register(1)).is_nan());
        assert!((mmix.get_special(SpecialReg::RA) & RA_I) != 0);
    }

    #[test]
    fn test_fdiv_underflow_sets_u() {
        let mut mmix = MMix::new();
        mmix.set_register(2, f64::MIN_POSITIVE.to_bits());
        mmix.set_register(3, 1e16f64.to_bits());
        mmix.write_tetra(0, 0x14010203);
        assert!(mmix.execute_instruction());
        let r = f64::from_bits(mmix.get_register(1));
        assert_eq!(r, 0.0, "expected complete underflow to zero, got {:?}", r);
        assert!((mmix.get_special(SpecialReg::RA) & RA_U) != 0);
    }

    #[test]
    fn test_fsqrt_negative_sets_i() {
        let mut mmix = MMix::new();
        mmix.set_register(3, (-4.0f64).to_bits());
        mmix.write_tetra(0, 0x15010003); // FSQRT $1,$0,$3
        assert!(mmix.execute_instruction());
        assert!(f64::from_bits(mmix.get_register(1)).is_nan());
        assert!((mmix.get_special(SpecialReg::RA) & RA_I) != 0);
    }

    #[test]
    fn test_fsqrt_normal_no_flag() {
        let mut mmix = MMix::new();
        mmix.set_register(3, 9.0f64.to_bits());
        mmix.write_tetra(0, 0x15010003);
        assert!(mmix.execute_instruction());
        assert!((f64::from_bits(mmix.get_register(1)) - 3.0).abs() < 1e-10);
        assert_eq!(mmix.get_special(SpecialReg::RA) & RA_I, 0);
    }

    #[test]
    fn test_fcmp_nan_sets_i() {
        let mut mmix = MMix::new();
        mmix.set_register(2, f64::NAN.to_bits());
        mmix.set_register(3, 1.0f64.to_bits());
        mmix.write_tetra(0, 0x01010203); // FCMP
        assert!(mmix.execute_instruction());
        assert!((mmix.get_special(SpecialReg::RA) & RA_I) != 0);
    }

    #[test]
    fn test_feql_nan_no_flag() {
        let mut mmix = MMix::new();
        // FEQL is "quiet" for NaN: returns 0, no I flag.
        mmix.set_register(2, f64::NAN.to_bits());
        mmix.set_register(3, 1.0f64.to_bits());
        mmix.write_tetra(0, 0x03010203);
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 0);
        assert_eq!(mmix.get_special(SpecialReg::RA) & RA_I, 0);
    }

    #[test]
    fn test_fune_includes_nan() {
        let mut mmix = MMix::new();
        mmix.set_special(SpecialReg::RE, 0.001f64.to_bits());
        mmix.set_register(2, f64::NAN.to_bits());
        mmix.set_register(3, 5.0f64.to_bits());
        mmix.write_tetra(0, 0x12010203); // FUNE $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 1);
    }

    #[test]
    fn test_fcmpe_outside_epsilon() {
        let mut mmix = MMix::new();
        mmix.set_special(SpecialReg::RE, 0.001f64.to_bits());
        mmix.set_register(2, 1.0f64.to_bits());
        mmix.set_register(3, 2.0f64.to_bits());
        mmix.write_tetra(0, 0x11010203); // FCMPE $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1) as i64, -1);
    }

    #[test]
    fn test_fcmpe_binade_scaled_radius() {
        // A flat |y-z|<=epsilon test gives -1 (16 > 0.25). 1024's raw
        // exponent field is 1033, so Nε's radius is 0.25 * 2^11 = 512,
        // which covers 1040 (diff 16) and FCMPE reports 0.
        let mut mmix = MMix::new();
        mmix.set_special(SpecialReg::RE, 0.25f64.to_bits());
        mmix.set_register(2, 1024.0f64.to_bits());
        mmix.set_register(3, 1040.0f64.to_bits());
        mmix.write_tetra(0, 0x11010203); // FCMPE $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 0);
    }

    #[test]
    fn test_fcmpe_radius_uses_e_minus_1022_not_1023() {
        // 1.0's raw exponent field is 1023, so the radius is
        // 0.5 * 2^(1023-1022) = 1.0, covering the 0.75 gap to 1.75. An
        // off-by-one-binade radius (2^(e-1023) = 0.5) would not.
        let mut mmix = MMix::new();
        mmix.set_special(SpecialReg::RE, 0.5f64.to_bits());
        mmix.set_register(2, 1.0f64.to_bits());
        mmix.set_register(3, 1.75f64.to_bits());
        mmix.write_tetra(0, 0x11010203); // FCMPE $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 0);
    }

    #[test]
    fn test_fcmpe_denormal_neighborhood_uses_fixed_radius() {
        // $3 is subnormal (raw exponent field 0): Nε's denormal case uses
        // the fixed radius 2^-1021 * ε, not a per-value binade scale, and
        // that radius is far smaller than the gap to $2. A flat
        // |y-z|<=epsilon check would call this pair close (1e-300 <=
        // 1e-10) and report 0; the correct radius does not, and $2 > $3
        // gives +1.
        let mut mmix = MMix::new();
        mmix.set_special(SpecialReg::RE, 1e-10f64.to_bits());
        mmix.set_register(2, 1e-300f64.to_bits());
        mmix.set_register(3, f64::from_bits(1).to_bits()); // smallest subnormal
        mmix.write_tetra(0, 0x11010203); // FCMPE $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1) as i64, 1);
    }

    #[test]
    fn test_fcmpe_infinite_neighborhood_epsilon_at_least_two() {
        // Nε(+∞) is everything when ε≥2, so 5.0 ∈ Nε(+∞) and FCMPE
        // reports 0. A flat |y-z|<=epsilon check sees an infinite
        // difference, never within any finite epsilon, and falls back to
        // the sign compare (5.0 < ∞ ⇒ -1).
        let mut mmix = MMix::new();
        mmix.set_special(SpecialReg::RE, 3.0f64.to_bits());
        mmix.set_register(2, 5.0f64.to_bits());
        mmix.set_register(3, f64::INFINITY.to_bits());
        mmix.write_tetra(0, 0x11010203); // FCMPE $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 0);
    }

    #[test]
    fn test_fcmpe_nan_operand_forces_zero_and_raises_invalid() {
        let mut mmix = MMix::new();
        mmix.set_special(SpecialReg::RE, 0.1f64.to_bits());
        mmix.set_register(2, f64::NAN.to_bits());
        mmix.set_register(3, 5.0f64.to_bits());
        mmix.write_tetra(0, 0x11010203); // FCMPE $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 0);
        assert!((mmix.get_special(SpecialReg::RA) & RA_I) != 0);
    }

    #[test]
    fn test_fcmpe_negative_epsilon_forces_zero_and_raises_invalid() {
        let mut mmix = MMix::new();
        mmix.set_special(SpecialReg::RE, (-1.0f64).to_bits());
        mmix.set_register(2, 5.0f64.to_bits());
        mmix.set_register(3, 6.0f64.to_bits());
        mmix.write_tetra(0, 0x11010203); // FCMPE $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 0);
        assert!((mmix.get_special(SpecialReg::RA) & RA_I) != 0);
    }

    #[test]
    fn test_fune_negative_epsilon_is_exceptional() {
        let mut mmix = MMix::new();
        mmix.set_special(SpecialReg::RE, (-1.0f64).to_bits());
        mmix.set_register(2, 5.0f64.to_bits());
        mmix.set_register(3, 5.0f64.to_bits());
        mmix.write_tetra(0, 0x12010203); // FUNE $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 1);
        assert_eq!(mmix.get_special(SpecialReg::RA) & RA_I, 0); // FUNE raises nothing
    }

    #[test]
    fn test_fix_y_override_forces_mode_regardless_of_ra() {
        // rA's persistent mode is ROUND_UP (2); Y=1 (ROUND_OFF) must
        // override it: trunc(2.5)=2, not rA's ceil(2.5)=3.
        let mut mmix = MMix::new();
        mmix.set_special(SpecialReg::RA, 2 << RA_ROUND_SHIFT);
        mmix.set_register(2, 2.5f64.to_bits());
        mmix.write_tetra(0, 0x05010102); // FIX $1,1,$2 (Y=ROUND_OFF)
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1) as i64, 2);
    }

    #[test]
    fn test_fix_two_operand_form_still_honors_ra_mode() {
        let mut mmix = MMix::new();
        mmix.set_special(SpecialReg::RA, 2 << RA_ROUND_SHIFT); // ROUND_UP
        mmix.set_register(2, 2.5f64.to_bits());
        mmix.write_tetra(0, 0x05010002); // FIX $1,0,$2 (Y=0, no override)
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1) as i64, 3); // ceil(2.5) per rA's mode
    }

    #[test]
    fn test_y_greater_than_four_halts_with_diagnostic() {
        let (host, handle) = CaptureHost::new();
        let mut mmix = MMix::with_host(host);
        mmix.write_tetra(0, 0x0901050A); // FLOTI $1,5,10 (Y=5, illegal)
        let should_continue = mmix.execute_instruction();
        assert!(!should_continue);
        assert_eq!(handle.diagnostics().len(), 1);
        assert!(handle.diagnostics()[0].contains("FLOTI"));
        assert!(handle.diagnostics()[0].contains("Y=5"));
    }

    #[test]
    fn test_fsqrt_y_greater_than_four_halts_with_diagnostic() {
        // The Y>4 halt is wired at every rounding-mode read site; FLOTI
        // above pins the FLOT/i2f_conv_ri! site, this pins FSQRT's separate
        // finalize_fp_unop site.
        let (host, handle) = CaptureHost::new();
        let mut mmix = MMix::with_host(host);
        mmix.write_tetra(0, 0x15010502); // FSQRT $1,5,$2 (Y=5, illegal)
        let should_continue = mmix.execute_instruction();
        assert!(!should_continue);
        assert_eq!(handle.diagnostics().len(), 1);
        assert!(handle.diagnostics()[0].contains("FSQRT"));
        assert!(handle.diagnostics()[0].contains("Y=5"));
    }

    #[test]
    fn test_fix_y_two_selects_round_up_not_round_off() {
        // Y=2 (ROUND_UP) must map to rA mode 2, not `Y-1`'s mode 1
        // (ROUND_OFF). rA holds ROUND_OFF already, so the two mappings
        // coincide unless Y's own value (2) is honored: ceil(2.5)=3 under
        // the correct mapping's ROUND_UP, trunc(2.5)=2 under the
        // forbidden Y-1 mapping (indistinguishable from rA's own
        // ROUND_OFF, i.e. Y ignored).
        let mut mmix = MMix::new();
        mmix.set_special(SpecialReg::RA, 1 << RA_ROUND_SHIFT); // ROUND_OFF
        mmix.set_register(2, 2.5f64.to_bits());
        mmix.write_tetra(0, 0x05010202); // FIX $1,2,$2 (Y=ROUND_UP)
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1) as i64, 3);
    }

    #[test]
    fn test_fcmpe_denormal_radius_pins_exact_constant() {
        // Both operands are denormal (raw exponent field 0), placing the
        // gap strictly between the off-by-one radius 2^-1022*ε and the
        // correct radius 2^-1021*ε — only the correct constant reports 0.
        let mut mmix = MMix::new();
        mmix.set_special(SpecialReg::RE, 0.1f64.to_bits());
        mmix.set_register(2, 1.5e-308f64.to_bits());
        mmix.set_register(3, 1.83e-308f64.to_bits());
        mmix.write_tetra(0, 0x11010203); // FCMPE $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 0);
    }

    #[test]
    fn test_fcmpe_denormal_radius_pins_exact_constant_upper_side() {
        // The sibling test above pins the -1021 -> -1022 boundary; this
        // pins the other side. The gap sits strictly between the correct
        // radius 2^-1021*ε and the off-by-one radius 2^-1020*ε, so only
        // the correct constant reports non-zero (not close).
        let mut mmix = MMix::new();
        mmix.set_special(SpecialReg::RE, 0.1f64.to_bits());
        mmix.set_register(2, 1.5e-308f64.to_bits());
        mmix.set_register(3, 2.16752215755216e-308f64.to_bits());
        mmix.write_tetra(0, 0x11010203); // FCMPE $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1) as i64, -1);
    }

    #[test]
    fn test_fcmpe_infinite_neighborhood_epsilon_below_one() {
        // Nε(+∞) = {+∞} only when ε < 1: a finite value is never close to
        // +∞ and the ordinary sign compare applies.
        let mut mmix = MMix::new();
        mmix.set_special(SpecialReg::RE, 0.5f64.to_bits());
        mmix.set_register(2, 5.0f64.to_bits());
        mmix.set_register(3, f64::INFINITY.to_bits());
        mmix.write_tetra(0, 0x11010203); // FCMPE $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1) as i64, -1);
    }

    #[test]
    fn test_fcmpe_infinite_neighborhood_epsilon_below_one_opposite_infinities() {
        // Nε(+∞) = {+∞} when ε < 1: the entry condition is `u == v`, exact
        // equality, not `u.is_infinite()` — opposite infinities are each
        // infinite but never equal, so they must compare -1, not 0. A
        // mutant widening the entry test to `u.is_infinite()` would wrongly
        // place -∞ in Nε(+∞) here and report 0.
        let mut mmix = MMix::new();
        mmix.set_special(SpecialReg::RE, 0.5f64.to_bits());
        mmix.set_register(2, f64::NEG_INFINITY.to_bits());
        mmix.set_register(3, f64::INFINITY.to_bits());
        mmix.write_tetra(0, 0x11010203); // FCMPE $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1) as i64, -1);
    }

    #[test]
    fn test_fcmpe_infinite_neighborhood_epsilon_one_to_two() {
        // Nε(+∞) = everything except -∞ when 1 ≤ ε < 2: a finite value is
        // close to +∞, but -∞ itself is not — the "except" half, which a
        // mutation collapsing this branch to unconditional "everything"
        // (ε ≥ 2's behavior) would not catch on the finite vector alone.
        let mut mmix = MMix::new();
        mmix.set_special(SpecialReg::RE, 1.5f64.to_bits());
        mmix.set_register(2, 5.0f64.to_bits());
        mmix.set_register(3, f64::INFINITY.to_bits());
        mmix.write_tetra(0, 0x11010203); // FCMPE $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 0);

        let mut opposite = MMix::new();
        opposite.set_special(SpecialReg::RE, 1.5f64.to_bits());
        opposite.set_register(2, f64::NEG_INFINITY.to_bits());
        opposite.set_register(3, f64::INFINITY.to_bits());
        opposite.write_tetra(0, 0x11010203); // FCMPE $1,$2,$3
        assert!(opposite.execute_instruction());
        assert_eq!(opposite.get_register(1) as i64, -1);
    }

    #[test]
    fn test_feqle_stronger_than_fcmpe_asymmetric_binade() {
        // 4.0 sits at the start of a binade twice as wide as 3.99's; the
        // gap fits inside 4.0's radius but not inside 3.99's, so
        // 3.99 ∈ Nε(4.0) while 4.0 ∉ Nε(3.99) — FCMPE's OR is satisfied
        // (∼ holds) but FEQLE's AND (≈) is not. An &&-to-|| mutation in
        // FEQLE's arm would survive without this test.
        let mut fcmpe = MMix::new();
        fcmpe.set_special(SpecialReg::RE, 0.002f64.to_bits());
        fcmpe.set_register(2, 3.99f64.to_bits());
        fcmpe.set_register(3, 4.0f64.to_bits());
        fcmpe.write_tetra(0, 0x11010203); // FCMPE $1,$2,$3
        assert!(fcmpe.execute_instruction());
        assert_eq!(fcmpe.get_register(1), 0);

        let mut feqle = MMix::new();
        feqle.set_special(SpecialReg::RE, 0.002f64.to_bits());
        feqle.set_register(2, 3.99f64.to_bits());
        feqle.set_register(3, 4.0f64.to_bits());
        feqle.write_tetra(0, 0x13010203); // FEQLE $1,$2,$3
        assert!(feqle.execute_instruction());
        assert_eq!(feqle.get_register(1), 0);
    }

    #[test]
    fn test_feqle_nan_operand_forces_zero_and_raises_invalid() {
        let mut mmix = MMix::new();
        mmix.set_special(SpecialReg::RE, 0.1f64.to_bits());
        mmix.set_register(2, f64::NAN.to_bits());
        mmix.set_register(3, 5.0f64.to_bits());
        mmix.write_tetra(0, 0x13010203); // FEQLE $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 0);
        assert!((mmix.get_special(SpecialReg::RA) & RA_I) != 0);
    }

    #[test]
    fn test_fcmpe_reflexive_at_top_binade_zero_epsilon() {
        // f64::MAX's raw exponent field is 2046, the top binade, where
        // `2^(e-1022)` is `2^1024` — unrepresentable, and `0.0 * inf` is
        // NaN. Reflexivity must still hold: a value is always in its own
        // Nε-neighborhood, so FCMPE(v, v) is 0 even with ε = 0.0.
        let mut mmix = MMix::new();
        mmix.set_special(SpecialReg::RE, 0.0f64.to_bits());
        mmix.set_register(2, f64::MAX.to_bits());
        mmix.set_register(3, f64::MAX.to_bits());
        mmix.write_tetra(0, 0x11010203); // FCMPE $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1) as i64, 0);
    }

    #[test]
    fn test_feqle_reflexive_at_top_binade_zero_epsilon() {
        let mut mmix = MMix::new();
        mmix.set_special(SpecialReg::RE, 0.0f64.to_bits());
        mmix.set_register(2, f64::MAX.to_bits());
        mmix.set_register(3, f64::MAX.to_bits());
        mmix.write_tetra(0, 0x13010203); // FEQLE $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 1);
    }

    #[test]
    fn test_fcmpe_zero_neighborhood_reflexive() {
        // Nε(0) = {0}: zero is always in its own neighborhood, so
        // FCMPE(0.0, 0.0) is 0 for any ε >= 0.
        let mut mmix = MMix::new();
        mmix.set_special(SpecialReg::RE, 0.0f64.to_bits());
        mmix.set_register(2, 0.0f64.to_bits());
        mmix.set_register(3, 0.0f64.to_bits());
        mmix.write_tetra(0, 0x11010203); // FCMPE $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 0);
    }

    #[test]
    fn test_fcmpe_zero_neighborhood_excludes_nonzero() {
        // Nε(0) = {0}, the single point, not "anything within ε of 0" —
        // a nonzero value is never in it, however small, and no ε widens
        // that set.
        let mut mmix = MMix::new();
        mmix.set_special(SpecialReg::RE, 0.1f64.to_bits());
        mmix.set_register(2, 0.0f64.to_bits());
        mmix.set_register(3, 1.0f64.to_bits());
        mmix.write_tetra(0, 0x11010203); // FCMPE $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1) as i64, -1);
    }

    #[test]
    fn test_fcmpe_top_binade_radius_stays_finite_not_infinite() {
        // Both operands are huge and finite (top binade), but far enough
        // apart that the correct, finite radius (ε * 2^1024, computed
        // without materializing the unrepresentable literal `2^1024`)
        // does not cover the gap — an `inf`-radius implementation would
        // wrongly call this pair close (0) for any ε > 0.
        let mut mmix = MMix::new();
        mmix.set_special(SpecialReg::RE, 1e-300f64.to_bits());
        mmix.set_register(2, (f64::MAX / 2.0).to_bits());
        mmix.set_register(3, f64::MAX.to_bits());
        mmix.write_tetra(0, 0x11010203); // FCMPE $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1) as i64, -1);
    }

    #[test]
    fn test_stsf_rounding_default_near() {
        let mut mmix = MMix::new();
        let v: f64 = 1.0f64 + f32::EPSILON as f64 / 2.0; // exactly half-ULP above 1.0 in f32
        mmix.set_register(1, v.to_bits()); // value to store
        mmix.set_register(2, 0x100); // base address
        mmix.set_register(3, 0); // offset
        // STSF $1,$2,$3
        mmix.write_tetra(0, 0xB0010203);
        assert!(mmix.execute_instruction());
        let stored_bits = mmix.read_tetra(0x100);
        let round_trip = f32::from_bits(stored_bits) as f64;
        // Default round-to-nearest-even rounds to 1.0 exactly (1.0 is even).
        assert_eq!(round_trip, 1.0);
        assert!((mmix.get_special(SpecialReg::RA) & RA_X) != 0);
    }

    #[test]
    fn test_stsf_rounding_up_mode() {
        let mut mmix = MMix::new();
        // Pick a value strictly between two adjacent f32 values.
        let v: f64 = 1.0f64 + (f32::EPSILON as f64) * 0.25;
        mmix.set_special(SpecialReg::RA, 2 << RA_ROUND_SHIFT); // ROUND_UP
        mmix.set_register(1, v.to_bits());
        mmix.set_register(2, 0x100);
        mmix.set_register(3, 0);
        mmix.write_tetra(0, 0xB0010203);
        assert!(mmix.execute_instruction());
        let stored = f32::from_bits(mmix.read_tetra(0x100));
        // Result must be >= input.
        assert!((stored as f64) >= v, "{} >= {}", stored, v);
        // And strictly above 1.0 since exact value > 1.0.
        assert!(stored > 1.0);
    }

    #[test]
    fn test_stsf_rounding_down_mode() {
        let mut mmix = MMix::new();
        let v: f64 = 1.0f64 + (f32::EPSILON as f64) * 0.25;
        mmix.set_special(SpecialReg::RA, 3 << RA_ROUND_SHIFT); // ROUND_DOWN
        mmix.set_register(1, v.to_bits());
        mmix.set_register(2, 0x100);
        mmix.set_register(3, 0);
        mmix.write_tetra(0, 0xB0010203);
        assert!(mmix.execute_instruction());
        let stored = f32::from_bits(mmix.read_tetra(0x100));
        // Result must be <= input.
        assert!((stored as f64) <= v);
        // And exactly 1.0 (largest f32 ≤ v).
        assert_eq!(stored, 1.0);
    }

    #[test]
    fn test_stsf_overflow_to_inf_sets_o() {
        let mut mmix = MMix::new();
        // 1e40 overflows f32.
        mmix.set_register(1, 1e40f64.to_bits());
        mmix.set_register(2, 0x100);
        mmix.set_register(3, 0);
        mmix.write_tetra(0, 0xB0010203);
        assert!(mmix.execute_instruction());
        let stored = f32::from_bits(mmix.read_tetra(0x100));
        assert!(stored.is_infinite() && stored.is_sign_positive());
        assert!((mmix.get_special(SpecialReg::RA) & RA_O) != 0);
    }

    #[test]
    fn test_ieee_remainder_helper() {
        // Direct check of the helper for clarity.
        assert_eq!(MMix::ieee_remainder(7.5, 2.0), -0.5);
        assert_eq!(MMix::ieee_remainder(10.0, 3.0), 1.0);
        assert!(MMix::ieee_remainder(1.0, 0.0).is_nan());
        assert!(MMix::ieee_remainder(f64::INFINITY, 1.0).is_nan());
        assert_eq!(MMix::ieee_remainder(3.0, f64::INFINITY), 3.0);
    }

    // ==================== Assembler integration: FCMPE/FUNE/FEQLE ====================

    #[test]
    fn test_assembler_emits_fcmpe() {
        use crate::encode::encode_instruction_bytes;
        use crate::mmixal::MMixInstruction;
        let bytes = encode_instruction_bytes(&MMixInstruction::FCMPE(1, 2, 3));
        assert_eq!(bytes, vec![0x11, 1, 2, 3]);
    }

    #[test]
    fn test_assembler_emits_fune() {
        use crate::encode::encode_instruction_bytes;
        use crate::mmixal::MMixInstruction;
        let bytes = encode_instruction_bytes(&MMixInstruction::FUNE(1, 2, 3));
        assert_eq!(bytes, vec![0x12, 1, 2, 3]);
    }

    #[test]
    fn test_assembler_emits_feqle() {
        use crate::encode::encode_instruction_bytes;
        use crate::mmixal::MMixInstruction;
        let bytes = encode_instruction_bytes(&MMixInstruction::FEQLE(1, 2, 3));
        assert_eq!(bytes, vec![0x13, 1, 2, 3]);
    }

    // ==================== sNaN handling ====================

    /// IEEE 754 binary64 sNaN: exponent all 1s, mantissa nonzero, bit 51 clear.
    const SNAN_BITS: u64 = 0x7FF0_0000_0000_0001;
    const QNAN_BITS: u64 = 0x7FF8_0000_0000_0001;

    #[test]
    fn test_is_signaling_nan_classifies_correctly() {
        assert!(MMix::is_signaling_nan(f64::from_bits(SNAN_BITS)));
        assert!(!MMix::is_signaling_nan(f64::from_bits(QNAN_BITS)));
        assert!(!MMix::is_signaling_nan(f64::NAN));
        assert!(!MMix::is_signaling_nan(1.0));
        assert!(!MMix::is_signaling_nan(f64::INFINITY));
    }

    #[test]
    fn test_quiet_nan_sets_high_mantissa_bit() {
        let q = MMix::quiet_nan(f64::from_bits(SNAN_BITS));
        assert!(q.is_nan());
        assert_eq!(q.to_bits() & (1 << 51), 1 << 51);
        // Non-NaN passes through unchanged.
        assert_eq!(MMix::quiet_nan(1.5).to_bits(), 1.5f64.to_bits());
    }

    #[test]
    fn test_fadd_snan_raises_i_and_quiets() {
        let mut mmix = MMix::new();
        mmix.set_register(2, SNAN_BITS);
        mmix.set_register(3, 1.0f64.to_bits());
        mmix.write_tetra(0, 0x04010203); // FADD $1,$2,$3
        assert!(mmix.execute_instruction());
        assert!((mmix.get_special(SpecialReg::RA) & RA_I) != 0);
        let result = mmix.get_register(1);
        // Result must be a NaN, and must be quiet.
        assert!(f64::from_bits(result).is_nan());
        assert_eq!(result & (1 << 51), 1 << 51);
    }

    #[test]
    fn test_fadd_qnan_does_not_raise_i() {
        let mut mmix = MMix::new();
        mmix.set_register(2, QNAN_BITS);
        mmix.set_register(3, 1.0f64.to_bits());
        mmix.write_tetra(0, 0x04010203);
        assert!(mmix.execute_instruction());
        // qNaN propagation is silent — I must not be raised.
        assert_eq!(mmix.get_special(SpecialReg::RA) & RA_I, 0);
        assert!(f64::from_bits(mmix.get_register(1)).is_nan());
    }

    #[test]
    fn test_fmul_snan_raises_i() {
        let mut mmix = MMix::new();
        mmix.set_register(2, 2.0f64.to_bits());
        mmix.set_register(3, SNAN_BITS);
        mmix.write_tetra(0, 0x10010203); // FMUL
        assert!(mmix.execute_instruction());
        assert!((mmix.get_special(SpecialReg::RA) & RA_I) != 0);
    }

    #[test]
    fn test_fsqrt_snan_raises_i() {
        let mut mmix = MMix::new();
        mmix.set_register(3, SNAN_BITS);
        mmix.write_tetra(0, 0x15010003); // FSQRT
        assert!(mmix.execute_instruction());
        assert!((mmix.get_special(SpecialReg::RA) & RA_I) != 0);
    }

    // ==================== Directed rounding for FP arithmetic ====================

    /// Build an operand pair whose true sum lies strictly between two adjacent
    /// f64 values: 1.0 + 2^-53. Round-to-nearest-even gives 1.0 (mantissa LSB
    /// of 1.0 is even); ROUND_UP must produce 1.0 + 2^-52.
    fn one_plus_half_ulp() -> (f64, f64) {
        let a = 1.0f64;
        let half_ulp = f64::from_bits(0x3CA0_0000_0000_0000); // 2^-53
        (a, half_ulp)
    }

    #[test]
    fn test_fadd_round_near_default() {
        let mut mmix = MMix::new();
        let (a, b) = one_plus_half_ulp();
        mmix.set_register(2, a.to_bits());
        mmix.set_register(3, b.to_bits());
        mmix.write_tetra(0, 0x04010203);
        assert!(mmix.execute_instruction());
        assert_eq!(f64::from_bits(mmix.get_register(1)), 1.0);
        // Inexact must be raised because the sum was rounded.
        assert!((mmix.get_special(SpecialReg::RA) & RA_X) != 0);
    }

    #[test]
    fn test_fadd_round_up_bumps_to_next_f64() {
        let mut mmix = MMix::new();
        let (a, b) = one_plus_half_ulp();
        mmix.set_register(2, a.to_bits());
        mmix.set_register(3, b.to_bits());
        mmix.set_special(SpecialReg::RA, 2 << RA_ROUND_SHIFT); // ROUND_UP
        mmix.write_tetra(0, 0x04010203);
        assert!(mmix.execute_instruction());
        let r = f64::from_bits(mmix.get_register(1));
        assert!(r > 1.0, "expected r > 1.0, got {}", r);
        assert_eq!(r, 1.0f64.next_up());
    }

    #[test]
    fn test_fadd_round_down_keeps_below() {
        let mut mmix = MMix::new();
        let (a, b) = one_plus_half_ulp();
        mmix.set_register(2, a.to_bits());
        mmix.set_register(3, b.to_bits());
        mmix.set_special(SpecialReg::RA, 3 << RA_ROUND_SHIFT); // ROUND_DOWN
        mmix.write_tetra(0, 0x04010203);
        assert!(mmix.execute_instruction());
        assert_eq!(f64::from_bits(mmix.get_register(1)), 1.0);
    }

    #[test]
    fn test_fadd_round_off_toward_zero_negative() {
        let mut mmix = MMix::new();
        // -(1 + 2^-53) under round-up should keep -1.0 (less negative).
        // Under ROUND_OFF (toward 0), also -1.0.
        let (a, b) = one_plus_half_ulp();
        mmix.set_register(2, (-a).to_bits());
        mmix.set_register(3, (-b).to_bits());
        mmix.set_special(SpecialReg::RA, 1 << RA_ROUND_SHIFT); // ROUND_OFF
        mmix.write_tetra(0, 0x04010203);
        assert!(mmix.execute_instruction());
        assert_eq!(f64::from_bits(mmix.get_register(1)), -1.0);
    }

    #[test]
    fn test_fmul_directed_rounding() {
        let mut mmix = MMix::new();
        // 1/3 in f64 is inexact; (1/3) * 3 ≠ 1 exactly, gives a nearby f64.
        // The exact product 0.333…·3 = 0.999… so result is just below 1.
        let third = 1.0f64 / 3.0;
        mmix.set_register(2, third.to_bits());
        mmix.set_register(3, 3.0f64.to_bits());
        // Default: round-to-nearest gives 1.0.
        mmix.write_tetra(0, 0x10010203); // FMUL
        assert!(mmix.execute_instruction());
        let near_result = f64::from_bits(mmix.get_register(1));

        // Reset and try ROUND_DOWN (toward -∞): result must be ≤ near_result
        // and strictly less than 1.
        let mut mmix = MMix::new();
        mmix.set_register(2, third.to_bits());
        mmix.set_register(3, 3.0f64.to_bits());
        mmix.set_special(SpecialReg::RA, 3 << RA_ROUND_SHIFT);
        mmix.write_tetra(0, 0x10010203);
        assert!(mmix.execute_instruction());
        let down_result = f64::from_bits(mmix.get_register(1));
        assert!(down_result < 1.0, "ROUND_DOWN gave {}", down_result);
        assert!(down_result <= near_result);
        assert!((mmix.get_special(SpecialReg::RA) & RA_X) != 0);
    }

    #[test]
    fn test_fdiv_directed_rounding() {
        let mut mmix = MMix::new();
        // 1.0 / 3.0 is inexact. Default → nearest. ROUND_UP must give a value
        // strictly greater than the nearest result.
        mmix.set_register(2, 1.0f64.to_bits());
        mmix.set_register(3, 3.0f64.to_bits());
        mmix.write_tetra(0, 0x14010203); // FDIV
        assert!(mmix.execute_instruction());
        let near = f64::from_bits(mmix.get_register(1));

        let mut mmix = MMix::new();
        mmix.set_register(2, 1.0f64.to_bits());
        mmix.set_register(3, 3.0f64.to_bits());
        mmix.set_special(SpecialReg::RA, 2 << RA_ROUND_SHIFT); // ROUND_UP
        mmix.write_tetra(0, 0x14010203);
        assert!(mmix.execute_instruction());
        let up = f64::from_bits(mmix.get_register(1));
        assert!(up > near, "ROUND_UP={} should exceed nearest={}", up, near);
        assert!((mmix.get_special(SpecialReg::RA) & RA_X) != 0);
    }

    #[test]
    fn test_fdiv_directed_rounding_negative_divisor() {
        let mut mmix = MMix::new();
        // 1.0 / -3.0 — verify sign-of-divisor handling in residual.
        mmix.set_register(2, 1.0f64.to_bits());
        mmix.set_register(3, (-3.0f64).to_bits());
        mmix.set_special(SpecialReg::RA, 3 << RA_ROUND_SHIFT); // ROUND_DOWN
        mmix.write_tetra(0, 0x14010203);
        assert!(mmix.execute_instruction());
        let r = f64::from_bits(mmix.get_register(1));
        // True 1/-3 ≈ -0.333…; ROUND_DOWN must give a value ≤ true (more negative).
        assert!(r < -0.333, "ROUND_DOWN of 1/-3 was {}", r);
    }

    #[test]
    fn test_fsqrt_directed_rounding() {
        let mut mmix = MMix::new();
        // sqrt(2) is irrational. ROUND_UP and ROUND_DOWN must straddle the
        // nearest result.
        mmix.set_register(3, 2.0f64.to_bits());
        mmix.write_tetra(0, 0x15010003); // FSQRT
        assert!(mmix.execute_instruction());
        let near = f64::from_bits(mmix.get_register(1));

        let mut mmix = MMix::new();
        mmix.set_register(3, 2.0f64.to_bits());
        mmix.set_special(SpecialReg::RA, 2 << RA_ROUND_SHIFT); // UP
        mmix.write_tetra(0, 0x15010003);
        assert!(mmix.execute_instruction());
        let up = f64::from_bits(mmix.get_register(1));

        let mut mmix = MMix::new();
        mmix.set_register(3, 2.0f64.to_bits());
        mmix.set_special(SpecialReg::RA, 3 << RA_ROUND_SHIFT); // DOWN
        mmix.write_tetra(0, 0x15010003);
        assert!(mmix.execute_instruction());
        let down = f64::from_bits(mmix.get_register(1));

        assert!(down <= near && near <= up);
        assert!(down < up);
    }

    // ==================== Inexact (X) flag for arithmetic ====================

    #[test]
    fn test_fadd_exact_no_x_flag() {
        let mut mmix = MMix::new();
        // 1.0 + 2.0 is exact; X must not be raised.
        mmix.set_register(2, 1.0f64.to_bits());
        mmix.set_register(3, 2.0f64.to_bits());
        mmix.write_tetra(0, 0x04010203);
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_special(SpecialReg::RA) & RA_X, 0);
    }

    #[test]
    fn test_fmul_exact_no_x_flag() {
        let mut mmix = MMix::new();
        // 1.5 × 4.0 = 6.0 exactly.
        mmix.set_register(2, 1.5f64.to_bits());
        mmix.set_register(3, 4.0f64.to_bits());
        mmix.write_tetra(0, 0x10010203);
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_special(SpecialReg::RA) & RA_X, 0);
    }

    #[test]
    fn test_fdiv_exact_no_x_flag() {
        let mut mmix = MMix::new();
        // 12.0 / 4.0 = 3.0 exactly.
        mmix.set_register(2, 12.0f64.to_bits());
        mmix.set_register(3, 4.0f64.to_bits());
        mmix.write_tetra(0, 0x14010203);
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_special(SpecialReg::RA) & RA_X, 0);
    }

    // ============== Subnormal operands and results ==============

    #[test]
    fn test_subnormal_operand_raises_no_divide_check() {
        let mut mmix = MMix::new();
        // MMIX has no denormalized-operand event; D is the integer divide check.
        mmix.set_register(2, f64::MIN_POSITIVE.to_bits() >> 4); // subnormal
        mmix.set_register(3, 1.0f64.to_bits());
        mmix.write_tetra(0, 0x04010203); // FADD
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_special(SpecialReg::RA) & RA_D, 0);
    }

    #[test]
    fn test_subnormal_result_raises_underflow() {
        let mut mmix = MMix::new();
        // MIN_POSITIVE / 2.0 underflows to a subnormal; U reports it.
        mmix.set_register(2, f64::MIN_POSITIVE.to_bits());
        mmix.set_register(3, 2.0f64.to_bits());
        mmix.write_tetra(0, 0x14010203); // FDIV
        assert!(mmix.execute_instruction());
        let ra = mmix.get_special(SpecialReg::RA);
        let r = f64::from_bits(mmix.get_register(1));
        assert!(r.is_subnormal(), "expected subnormal result, got {}", r);
        assert!((ra & RA_U) != 0, "U should be set on underflow");
    }

    #[test]
    fn test_subnormal_plus_zero_raises_no_underflow() {
        let mut mmix = MMix::new();
        // A subnormal result that merely reproduces a subnormal operand lost
        // nothing; the nonzero-operand guard keeps U off it.
        mmix.set_register(2, 1u64); // smallest positive subnormal
        mmix.set_register(3, 0.0f64.to_bits());
        mmix.write_tetra(0, 0x04010203); // FADD
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 1u64);
        assert_eq!(mmix.get_special(SpecialReg::RA) & RA_U, 0);
    }

    #[test]
    fn test_exact_cancellation_fadd_raises_no_underflow() {
        let mut mmix = MMix::new();
        // 1.0 + -1.0 is exactly zero: nothing was too small to represent.
        mmix.set_register(2, 1.0f64.to_bits());
        mmix.set_register(3, (-1.0f64).to_bits());
        mmix.write_tetra(0, 0x04010203); // FADD
        assert!(mmix.execute_instruction());
        assert_eq!(f64::from_bits(mmix.get_register(1)), 0.0);
        assert_eq!(
            mmix.get_special(SpecialReg::RA) & RA_U,
            0,
            "exact cancellation is not an underflow"
        );
    }

    #[test]
    fn test_exact_cancellation_fsub_raises_no_underflow() {
        let mut mmix = MMix::new();
        mmix.set_register(2, 1.0f64.to_bits());
        mmix.set_register(3, 1.0f64.to_bits());
        mmix.write_tetra(0, 0x06010203); // FSUB
        assert!(mmix.execute_instruction());
        assert_eq!(f64::from_bits(mmix.get_register(1)), 0.0);
        assert_eq!(
            mmix.get_special(SpecialReg::RA) & RA_U,
            0,
            "exact cancellation is not an underflow"
        );
    }

    #[test]
    fn test_frem_exact_zero_raises_no_underflow() {
        let mut mmix = MMix::new();
        // The IEEE remainder is exact by definition, so FREM cannot underflow.
        mmix.set_register(2, (-3.0f64).to_bits());
        mmix.set_register(3, 1.5f64.to_bits());
        mmix.write_tetra(0, 0x16010203); // FREM
        assert!(mmix.execute_instruction());
        assert_eq!(
            mmix.get_special(SpecialReg::RA) & RA_U,
            0,
            "an exact remainder is not an underflow"
        );
    }

    #[test]
    fn test_fdiv_to_zero_raises_underflow() {
        let mut mmix = MMix::new();
        // The true quotient is nonzero but far below the subnormal range.
        mmix.set_register(2, f64::MIN_POSITIVE.to_bits());
        mmix.set_register(3, 1e16f64.to_bits());
        mmix.write_tetra(0, 0x14010203); // FDIV
        assert!(mmix.execute_instruction());
        assert_eq!(f64::from_bits(mmix.get_register(1)), 0.0);
        assert!((mmix.get_special(SpecialReg::RA) & RA_U) != 0);
    }

    #[test]
    fn test_fmul_to_zero_raises_underflow() {
        let mut mmix = MMix::new();
        // MIN_POSITIVE^2 is 2^-2044: nonzero, and below the subnormal range.
        // The FMA residual rounds to zero here, so it cannot witness the
        // underflow — only the operands can.
        mmix.set_register(2, f64::MIN_POSITIVE.to_bits());
        mmix.set_register(3, f64::MIN_POSITIVE.to_bits());
        mmix.write_tetra(0, 0x10010203); // FMUL
        assert!(mmix.execute_instruction());
        assert_eq!(f64::from_bits(mmix.get_register(1)), 0.0);
        assert!((mmix.get_special(SpecialReg::RA) & RA_U) != 0);
    }

    #[test]
    fn test_fdiv_by_infinity_is_exact_zero() {
        let mut mmix = MMix::new();
        // finite / inf is exactly +0.0: neither inexact nor an underflow. The
        // residual is NaN here, so an unguarded `err != 0.0` would raise X.
        mmix.set_register(2, 1.0f64.to_bits());
        mmix.set_register(3, f64::INFINITY.to_bits());
        mmix.write_tetra(0, 0x14010203); // FDIV
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 0.0f64.to_bits());
        assert_eq!(mmix.get_special(SpecialReg::RA), 0);
    }

    // ==================== FREM / FIXU / FCMP / conversions ====================

    #[test]
    fn test_frem_zero_takes_dividend_sign() {
        let mut mmix = MMix::new();
        // IEEE 754 gives a zero remainder the sign of the dividend; the
        // divisor's sign does not enter. Compare bit patterns — -0.0 == 0.0.
        mmix.set_register(2, (-3.0f64).to_bits());
        mmix.set_register(3, 1.5f64.to_bits());
        mmix.write_tetra(0, 0x16010203); // FREM $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), (-0.0f64).to_bits());

        let mut mmix = MMix::new();
        mmix.set_register(2, 3.0f64.to_bits());
        mmix.set_register(3, (-1.5f64).to_bits());
        mmix.write_tetra(0, 0x16010203);
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 0.0f64.to_bits());
    }

    #[test]
    fn test_fixu_wraps_mod_two_to_the_64() {
        // u($X) <- int(f($Z)) mod 2^64. A value whose ulp reaches 2^64
        // therefore yields zero rather than saturating.
        let cases: [(f64, u64); 10] = [
            (-1.0, 0xFFFF_FFFF_FFFF_FFFF),
            (-0.5, 0), // rounds to -0.0 under NEAR
            (3.7, 4),
            (9223372036854775808.0, 0x8000_0000_0000_0000), // 2^63
            (-9223372036854775808.0, 0x8000_0000_0000_0000), // -2^63
            (18446744073709551616.0, 0),                    // 2^64
            (1e300, 0),
            (18446744073709549568.0, 0xFFFF_FFFF_FFFF_F800),
            // The exponent at which every low bit has shifted out. An odd
            // significand distinguishes the two sides: at 2^115 the value is
            // 2^115 + 2^63, whose low octabyte is 2^63; one exponent higher
            // every surviving bit sits above the octabyte.
            (
                f64::from_bits(((115 + 1023) << 52) | 1),
                0x8000_0000_0000_0000,
            ),
            (f64::from_bits(((116 + 1023) << 52) | 1), 0),
        ];
        for (operand, expected) in cases {
            let mut mmix = MMix::new();
            mmix.set_register(3, operand.to_bits());
            mmix.write_tetra(0, 0x07010003); // FIXU $1,$0,$3
            assert!(mmix.execute_instruction());
            assert_eq!(
                mmix.get_register(1),
                expected,
                "FIXU {operand} should be {expected:#X}"
            );
        }
    }

    #[test]
    fn test_fix_negative_is_still_signed() {
        let mut mmix = MMix::new();
        // FIX keeps its signed conversion; only FIXU wraps.
        mmix.set_register(3, (-1.0f64).to_bits());
        mmix.write_tetra(0, 0x05010003); // FIX $1,$0,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1) as i64, -1);
    }

    #[test]
    fn test_fcmp_nan_answers_zero_in_either_position() {
        for (y, z) in [(f64::NAN, 1.0f64), (1.0f64, f64::NAN), (f64::NAN, f64::NAN)] {
            let mut mmix = MMix::new();
            mmix.set_register(2, y.to_bits());
            mmix.set_register(3, z.to_bits());
            mmix.write_tetra(0, 0x01010203); // FCMP $1,$2,$3
            assert!(mmix.execute_instruction());
            assert_eq!(mmix.get_register(1), 0, "FCMP {y},{z}");
            assert!((mmix.get_special(SpecialReg::RA) & RA_I) != 0);
        }
    }

    #[test]
    fn test_fcmp_ordered_still_three_way() {
        for (y, z, expected) in [(1.0f64, 2.0f64, -1i64), (2.0, 2.0, 0), (3.0, 2.0, 1)] {
            let mut mmix = MMix::new();
            mmix.set_register(2, y.to_bits());
            mmix.set_register(3, z.to_bits());
            mmix.write_tetra(0, 0x01010203); // FCMP $1,$2,$3
            assert!(mmix.execute_instruction());
            assert_eq!(mmix.get_register(1) as i64, expected, "FCMP {y},{z}");
            assert_eq!(mmix.get_special(SpecialReg::RA) & RA_I, 0);
        }
    }

    #[test]
    fn test_flot_inexact_conversion_raises_x() {
        let mut mmix = MMix::new();
        // 2^53+1 has no f64 image; it rounds to 2^53 and X reports the loss.
        mmix.set_register(3, (1u64 << 53) + 1);
        mmix.write_tetra(0, 0x08010003); // FLOT $1,$0,$3
        assert!(mmix.execute_instruction());
        assert_eq!(f64::from_bits(mmix.get_register(1)), 9007199254740992.0);
        assert!((mmix.get_special(SpecialReg::RA) & RA_X) != 0);
    }

    #[test]
    fn test_flot_exact_conversion_raises_nothing() {
        let mut mmix = MMix::new();
        mmix.set_register(3, 42);
        mmix.write_tetra(0, 0x08010003); // FLOT $1,$0,$3
        assert!(mmix.execute_instruction());
        assert_eq!(f64::from_bits(mmix.get_register(1)), 42.0);
        assert_eq!(mmix.get_special(SpecialReg::RA), 0);
    }

    #[test]
    fn test_flot_honors_directed_rounding_mode() {
        let mut up = MMix::new();
        up.set_special(SpecialReg::RA, 2 << RA_ROUND_SHIFT); // ROUND_UP
        up.set_register(3, (1u64 << 53) + 1);
        up.write_tetra(0, 0x08010003); // FLOT $1,$0,$3
        assert!(up.execute_instruction());
        let up_result = f64::from_bits(up.get_register(1));

        let mut down = MMix::new();
        down.set_special(SpecialReg::RA, 3 << RA_ROUND_SHIFT); // ROUND_DOWN
        down.set_register(3, (1u64 << 53) + 1);
        down.write_tetra(0, 0x08010003);
        assert!(down.execute_instruction());
        let down_result = f64::from_bits(down.get_register(1));

        assert_eq!(down_result, 9007199254740992.0);
        assert_eq!(up_result, 9007199254740994.0);
        assert!(up_result > down_result);

        // The negative arm negates both the magnitude and the residual, so a
        // directed mode must still round toward its own infinity.
        let mut neg_up = MMix::new();
        neg_up.set_special(SpecialReg::RA, 2 << RA_ROUND_SHIFT); // ROUND_UP
        neg_up.set_register(3, (-((1i64 << 53) + 1)) as u64);
        neg_up.write_tetra(0, 0x08010003);
        assert!(neg_up.execute_instruction());
        assert_eq!(f64::from_bits(neg_up.get_register(1)), -9007199254740992.0);

        let mut neg_down = MMix::new();
        neg_down.set_special(SpecialReg::RA, 3 << RA_ROUND_SHIFT); // ROUND_DOWN
        neg_down.set_register(3, (-((1i64 << 53) + 1)) as u64);
        neg_down.write_tetra(0, 0x08010003);
        assert!(neg_down.execute_instruction());
        assert_eq!(
            f64::from_bits(neg_down.get_register(1)),
            -9007199254740994.0
        );
    }

    #[test]
    fn test_flotu_max_raises_x() {
        let mut mmix = MMix::new();
        // u64::MAX rounds up to 2^64, losing eleven bits. Recovering the
        // residual by casting back saturates to u64::MAX and reports zero, so
        // this vector is the one that catches that mistake.
        mmix.set_register(3, u64::MAX);
        mmix.write_tetra(0, 0x0A010003); // FLOTU $1,$0,$3
        assert!(mmix.execute_instruction());
        assert_eq!(f64::from_bits(mmix.get_register(1)), 18446744073709551616.0);
        assert!((mmix.get_special(SpecialReg::RA) & RA_X) != 0);
    }

    #[test]
    fn test_sflotu_inexact_wide_step_raises_x() {
        let mut mmix = MMix::new();
        // 2^53+1 rounds to 2^53 on the way to f64; 2^53 then narrows to f32
        // exactly, so only the integer-to-f64 step can raise X here.
        mmix.set_register(3, (1u64 << 53) + 1);
        mmix.write_tetra(0, 0x0E010003); // SFLOTU $1,$0,$3
        assert!(mmix.execute_instruction());
        assert_eq!(f64::from_bits(mmix.get_register(1)), 9007199254740992.0);
        assert!((mmix.get_special(SpecialReg::RA) & RA_X) != 0);
    }

    // ==================== Overflow with directed rounding ====================

    #[test]
    fn test_overflow_round_off_clamps_to_max() {
        let mut mmix = MMix::new();
        mmix.set_register(2, f64::MAX.to_bits());
        mmix.set_register(3, f64::MAX.to_bits());
        mmix.set_special(SpecialReg::RA, 1 << RA_ROUND_SHIFT); // ROUND_OFF (toward 0)
        mmix.write_tetra(0, 0x04010203); // FADD
        assert!(mmix.execute_instruction());
        let r = f64::from_bits(mmix.get_register(1));
        assert_eq!(r, f64::MAX);
        assert!((mmix.get_special(SpecialReg::RA) & RA_O) != 0);
    }

    #[test]
    fn test_overflow_round_down_negative_keeps_neg_inf() {
        let mut mmix = MMix::new();
        mmix.set_register(2, f64::MIN.to_bits()); // most-negative finite
        mmix.set_register(3, f64::MIN.to_bits());
        mmix.set_special(SpecialReg::RA, 3 << RA_ROUND_SHIFT); // ROUND_DOWN (toward -∞)
        mmix.write_tetra(0, 0x04010203);
        assert!(mmix.execute_instruction());
        let r = f64::from_bits(mmix.get_register(1));
        assert!(r.is_infinite() && r.is_sign_negative());
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
    fn occupied_yields_nonzero_bytes_ascending_after_a_zero_write() {
        // Six surviving addresses written out of ascending order: with only two
        // survivors, HashMap iteration lands in ascending order by chance often
        // enough that deleting the sort in `occupied` doesn't reliably fail this
        // test. Six addresses drops that accidental-pass rate to roughly 1/720.
        let mut mmix = MMix::new();
        mmix.write_byte(500, 5);
        mmix.write_byte(100, 1);
        mmix.write_byte(700, 7);
        mmix.write_byte(300, 3);
        mmix.write_byte(900, 9);
        mmix.write_byte(400, 4);
        mmix.write_byte(200, 6);
        mmix.write_byte(200, 0); // zero-write: removed, must not appear

        let items: Vec<(u64, u8)> = mmix.occupied().collect();
        assert_eq!(
            items,
            vec![(100, 1), (300, 3), (400, 4), (500, 5), (700, 7), (900, 9)]
        );
    }

    #[test]
    fn loaded_extent_includes_the_hello_world_nul_terminator_that_occupied_omits() {
        // Reproduces examples/hello_world.mms's Text BYTE directive: its
        // trailing NUL is a real loaded byte that write_byte's zero-removal
        // hides from `occupied`, checked by address rather than by a total
        // count (write_image also writes every instruction, so the totals
        // include far more than this one directive).
        use crate::debugger::write_image;
        use crate::mmixal::MMixAssembler;

        const HELLO_WORLD: &str = "\
\tLOC\tData_Segment
\tGREG\t@
Text\tBYTE\t\"Hello world!\",'\\n',0

\tLOC\t#100

Main\tLDA\t$255,Text
\tTRAP\t0,Fputs,StdOut
\tTRAP\t0,Halt,0
";
        let mut asm = MMixAssembler::new(HELLO_WORLD, "hello_world.mms");
        asm.parse().expect("hello_world.mms must assemble");
        let text_addr = *asm.labels.get("Text").expect("Text label");
        // "Hello world!",'\n',0 is 14 bytes; the NUL terminator is the last.
        let nul_addr = text_addr + 13;

        let mut mmix = MMix::new();
        write_image(&mut mmix, &asm);

        let loaded: Vec<(u64, u8)> = mmix.loaded_extent().collect();
        assert!(loaded.contains(&(nul_addr, 0)));

        let occupied: Vec<(u64, u8)> = mmix.occupied().collect();
        assert!(!occupied.iter().any(|&(addr, _)| addr == nul_addr));
    }

    #[test]
    fn loaded_extent_is_unchanged_by_runtime_writes_during_execution() {
        // Mirrors debugger.rs's CALL_PROGRAM fixture: PUSHJ spills the
        // caller's frame to the register stack (addresses at
        // 0x6000000000000000+), a real runtime write that goes through
        // `write_byte`, not `write_loaded_byte`. `loaded_extent` tracks only
        // what `write_image` loaded, so a run must leave it unchanged.
        use crate::debugger::{entry_point, write_image};
        use crate::mmixal::MMixAssembler;

        const CALL_PROGRAM: &str = "\
\tLOC\t#100
Main\tPUSHJ\t$0,Sub
\tSETI\t$1,7
\tTRAP\t0,Halt,0
Sub\tSETI\t$0,3
\tPOP\t0,0
";
        let mut asm = MMixAssembler::new(CALL_PROGRAM, "call.mms");
        asm.parse().expect("call.mms must assemble");

        let mut mmix = MMix::new();
        write_image(&mut mmix, &asm);
        let before: Vec<(u64, u8)> = mmix.loaded_extent().collect();
        assert!(!before.is_empty(), "write_image must have loaded something");

        mmix.set_pc(entry_point(&asm));
        mmix.run(); // PUSHJ spills a frame; POP restores it; TRAP halts.

        let after: Vec<(u64, u8)> = mmix.loaded_extent().collect();
        assert_eq!(
            before, after,
            "a runtime write (PUSHJ's register-stack spill) must not appear \
             in loaded_extent"
        );
    }

    #[test]
    fn journal_records_writes_only_while_enabled_including_a_zero_write() {
        let mut mmix = MMix::new();
        mmix.write_byte(10, 1); // before enabling: not recorded

        mmix.set_journal(true);
        mmix.write_byte(20, 2);
        // A zero-write to a fresh address (never written before) is still a
        // recorded state change, distinct from the nonzero write above.
        mmix.write_byte(25, 0);
        mmix.write_byte(30, 3);
        mmix.set_journal(false);
        mmix.write_byte(40, 4); // journal off again: not recorded

        assert_eq!(mmix.take_journal(), vec![20, 25, 30]);
        // Drained: the next call is empty without another write.
        assert!(mmix.take_journal().is_empty());
    }

    #[test]
    fn journal_enabled_flag_survives_reset_but_the_buffer_does_not() {
        let mut mmix = MMix::new();
        mmix.set_journal(true);
        mmix.write_byte(50, 7);
        assert_eq!(mmix.take_journal(), vec![50]);

        mmix.reset();
        assert!(mmix.take_journal().is_empty());

        // The flag itself survived the reset.
        mmix.write_byte(60, 8);
        assert_eq!(mmix.take_journal(), vec![60]);
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

    /// The dump labels each slot with the name of the register that slot
    /// holds, so a value never appears under a neighbour's name.
    #[test]
    fn display_labels_each_special_register_with_its_own_name() {
        let mut mmix = MMix::new();
        let expected = [
            (SpecialReg::RN, "rN", 0x1111_u64),
            (SpecialReg::RO, "rO", 0x2222),
            (SpecialReg::RG, "rG", 0x3333),
            (SpecialReg::RL, "rL", 0x4444),
        ];
        for (reg, _, value) in expected {
            mmix.set_special(reg, value);
        }

        let dump = mmix.to_string();
        for (_, name, value) in expected {
            let line = format!("{:<4} = {:#018x}", name, value);
            assert!(
                dump.contains(&line),
                "expected `{line}` in the dump, got:\n{dump}"
            );
        }
    }
    // ============== rA layout and the integer events ==============

    #[test]
    fn test_ra_bits_match_knuth_predefs() {
        // The predefined symbols: D_BIT=#80 V_BIT=#40 W_BIT=#20 I_BIT=#10
        //                         O_BIT=#08 U_BIT=#04 Z_BIT=#02 X_BIT=#01
        assert_eq!(RA_D, 0x80);
        assert_eq!(RA_V, 0x40);
        assert_eq!(RA_W, 0x20);
        assert_eq!(RA_I, 0x10);
        assert_eq!(RA_O, 0x08);
        assert_eq!(RA_U, 0x04);
        assert_eq!(RA_Z, 0x02);
        assert_eq!(RA_X, 0x01);
        assert_eq!(RA_ROUND_SHIFT, 16);
        assert_eq!(RA_MAX, 0x3FFFF);
    }

    /// Run the signed division `word` encodes over $1 and the divisor,
    /// returning quotient, remainder and rA. The register form reads the
    /// divisor from $2, the immediate form from the instruction's Z field.
    fn run_signed_div(word: u32, dividend: i64, divisor: i64) -> (i64, i64, u64) {
        let mut mmix = MMix::new();
        mmix.set_register(1, dividend as u64);
        mmix.set_register(2, divisor as u64);
        mmix.write_tetra(0, word);
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 4, "division must advance the PC");
        (
            mmix.get_register(3) as i64,
            mmix.get_special(SpecialReg::RR) as i64,
            mmix.get_special(SpecialReg::RA),
        )
    }

    /// Run `DIV $3,$1,$2` on the given operands.
    fn run_div(dividend: i64, divisor: i64) -> (i64, i64, u64) {
        run_signed_div(0x1C030102, dividend, divisor)
    }

    /// Run `DIVI $3,$1,Z`; Z is a byte, so the divisor is in `0..=255`.
    fn run_divi(dividend: i64, divisor: u8) -> (i64, i64, u64) {
        run_signed_div(0x1D030100 | divisor as u32, dividend, 0)
    }

    #[test]
    fn test_div_floors_toward_negative_infinity() {
        // Truncation would give (-3, -1), (-3, 1) and (3, 1).
        assert_eq!(run_div(-7, 2), (-4, 1, 0));
        assert_eq!(run_div(7, -2), (-4, -1, 0));
        assert_eq!(run_div(-7, -2), (3, -1, 0));
        // Exact division is unaffected by the floor adjustment.
        assert_eq!(run_div(-8, 2), (-4, 0, 0));
        assert_eq!(run_div(7, 2), (3, 1, 0));
    }

    #[test]
    fn test_div_min_by_minus_one_wraps_and_raises_v() {
        let (quotient, remainder, ra) = run_div(i64::MIN, -1);
        assert_eq!(quotient as u64, 0x8000000000000000);
        assert_eq!(remainder, 0);
        assert_eq!(ra & RA_V, RA_V);
    }

    #[test]
    fn test_divi_floors_toward_negative_infinity() {
        // Only positive divisors are encodable; truncation would give (-3, -1),
        // (-2, -1) and (3, 1).
        assert_eq!(run_divi(-7, 2), (-4, 1, 0));
        assert_eq!(run_divi(-7, 3), (-3, 2, 0));
        // Exact division is unaffected by the floor adjustment.
        assert_eq!(run_divi(-8, 2), (-4, 0, 0));
        assert_eq!(run_divi(7, 2), (3, 1, 0));
    }

    #[test]
    fn test_divi_by_zero_raises_divide_check() {
        let (quotient, remainder, ra) = run_divi(42, 0);
        assert_eq!(quotient, 0);
        assert_eq!(remainder, 42);
        assert_eq!(ra & RA_D, RA_D, "D is the integer divide check");
        assert_eq!(ra & RA_V, 0, "divide by zero is not an overflow");
    }

    #[test]
    fn test_signed_divide_by_zero_raises_divide_check() {
        let mut mmix = MMix::new();
        mmix.set_register(1, 42);
        mmix.set_register(2, 0);
        mmix.write_tetra(0, 0x1C030102); // DIV $3,$1,$2
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(3), 0);
        assert_eq!(mmix.get_special(SpecialReg::RR), 42);
        let ra = mmix.get_special(SpecialReg::RA);
        assert_eq!(ra & RA_D, RA_D, "D is the integer divide check");
        assert_eq!(ra & RA_V, 0, "divide by zero is not an overflow");
    }

    #[test]
    fn test_divu_uses_rd_when_divisor_is_not_greater() {
        let mut mmix = MMix::new();
        mmix.set_special(SpecialReg::RD, 5);
        mmix.set_register(1, 0x1234);
        mmix.set_register(2, 3);
        mmix.write_tetra(0, 0x1E030102); // DIVU $3,$1,$2
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(3), 5);
        assert_eq!(mmix.get_special(SpecialReg::RR), 0x1234);
        assert_eq!(mmix.get_special(SpecialReg::RA), 0);
    }

    #[test]
    fn test_divu_by_zero_is_the_rd_rule_and_raises_nothing() {
        let mut mmix = MMix::new();
        mmix.set_special(SpecialReg::RD, 7);
        mmix.set_register(1, 0xABCD);
        mmix.set_register(2, 0);
        mmix.write_tetra(0, 0x1E030102); // DIVU $3,$1,$2
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(3), 7);
        assert_eq!(mmix.get_special(SpecialReg::RR), 0xABCD);
        assert_eq!(mmix.get_special(SpecialReg::RA), 0, "DIVU has no D");
    }

    #[test]
    fn test_divu_divides_the_full_128_bit_dividend() {
        let mut mmix = MMix::new();
        mmix.set_special(SpecialReg::RD, 1);
        mmix.set_register(1, 0);
        mmix.set_register(2, 2);
        mmix.write_tetra(0, 0x1E030102); // DIVU $3,$1,$2
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(3), 0x8000000000000000); // 2^64 / 2
        assert_eq!(mmix.get_special(SpecialReg::RR), 0);
    }

    #[test]
    fn test_divui_uses_rd_when_divisor_is_not_greater() {
        let mut mmix = MMix::new();
        mmix.set_special(SpecialReg::RD, 5);
        mmix.set_register(1, 0x1234);
        mmix.write_tetra(0, 0x1F030103); // DIVUI $3,$1,3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(3), 5);
        assert_eq!(mmix.get_special(SpecialReg::RR), 0x1234);
        assert_eq!(mmix.get_special(SpecialReg::RA), 0);
    }

    #[test]
    fn test_divui_by_zero_is_the_rd_rule_and_raises_nothing() {
        let mut mmix = MMix::new();
        mmix.set_special(SpecialReg::RD, 7);
        mmix.set_register(1, 0xABCD);
        mmix.write_tetra(0, 0x1F030100); // DIVUI $3,$1,0
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(3), 7);
        assert_eq!(mmix.get_special(SpecialReg::RR), 0xABCD);
        assert_eq!(mmix.get_special(SpecialReg::RA), 0, "DIVUI has no D");
    }

    #[test]
    fn test_divui_divides_the_full_128_bit_dividend() {
        let mut mmix = MMix::new();
        mmix.set_special(SpecialReg::RD, 1);
        mmix.set_register(1, 0);
        mmix.write_tetra(0, 0x1F030102); // DIVUI $3,$1,2
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(3), 0x8000000000000000); // 2^64 / 2
        assert_eq!(mmix.get_special(SpecialReg::RR), 0);
    }

    #[test]
    fn test_mul_leaves_rh_to_mulu() {
        let mut mmix = MMix::new();
        mmix.set_register(1, 1 << 32);
        mmix.set_register(2, 1 << 32);
        mmix.write_tetra(0, 0x1A030102); // MULU $3,$1,$2
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(3), 0);
        assert_eq!(mmix.get_special(SpecialReg::RH), 1);

        mmix.set_register(5, 6);
        mmix.set_register(6, 7);
        mmix.write_tetra(4, 0x18040506); // MUL $4,$5,$6
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(4), 42);
        assert_eq!(mmix.get_special(SpecialReg::RH), 1, "rH is MULU's output");
    }

    #[test]
    fn test_mul_overflow_raises_v() {
        let mut mmix = MMix::new();
        mmix.set_register(1, i64::MAX as u64);
        mmix.set_register(2, 2);
        mmix.write_tetra(0, 0x18030102); // MUL $3,$1,$2
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_special(SpecialReg::RA) & RA_V, RA_V);
        assert_eq!(mmix.get_special(SpecialReg::RH), 0);
    }

    #[test]
    fn test_add_overflow_raises_v_not_u() {
        let mut mmix = MMix::new();
        mmix.set_register(1, i64::MAX as u64);
        mmix.set_register(2, 1);
        mmix.write_tetra(0, 0x20030102); // ADD $3,$1,$2
        assert!(mmix.execute_instruction());
        let ra = mmix.get_special(SpecialReg::RA);
        assert_eq!(ra & RA_V, RA_V, "integer overflow is V");
        assert_eq!(ra & RA_U, 0, "U is floating underflow");
    }

    /// Run the signed left shift `word` encodes over $1, returning the result
    /// and rA. The register form reads the count from $2, the immediate form
    /// from the instruction's Z field.
    fn run_shift_left(word: u32, value: u64, shift: u64) -> (u64, u64) {
        let mut mmix = MMix::new();
        mmix.set_register(1, value);
        mmix.set_register(2, shift);
        mmix.write_tetra(0, word);
        assert!(mmix.execute_instruction());
        (mmix.get_register(3), mmix.get_special(SpecialReg::RA))
    }

    /// Run `SL $3,$1,$2`.
    fn run_sl(value: u64, shift: u64) -> (u64, u64) {
        run_shift_left(0x38030102, value, shift)
    }

    /// Run `SLI $3,$1,Z`.
    fn run_sli(value: u64, shift: u8) -> (u64, u64) {
        run_shift_left(0x39030100 | shift as u32, value, 0)
    }

    #[test]
    fn test_sl_overflows_when_the_product_leaves_the_signed_range() {
        // A set bit leaves the top: 2^62 · 4 = 2^64.
        let (result, ra) = run_sl(1 << 62, 2);
        assert_eq!(result, 0);
        assert_eq!(ra & RA_V, RA_V);

        // Nothing is shifted out, yet 2^62 · 2 = 2^63 exceeds the signed range.
        let (result, ra) = run_sl(0x4000000000000000, 1);
        assert_eq!(result, 0x8000000000000000);
        assert_eq!(ra & RA_V, RA_V, "the sign flip is an overflow");

        // 2^60 · 4 = 2^62 fits, so nothing is raised.
        let (result, ra) = run_sl(1 << 60, 2);
        assert_eq!(result, 1 << 62);
        assert_eq!(ra, 0, "a representable product raises nothing");

        // -1 · 2^63 = -2^63 is the most negative octabyte, and fits.
        let (result, ra) = run_sl(u64::MAX, 63);
        assert_eq!(result, 0x8000000000000000);
        assert_eq!(ra, 0);
    }

    #[test]
    fn test_sli_overflows_when_the_product_leaves_the_signed_range() {
        // A set bit leaves the top: 2^62 · 4 = 2^64.
        let (result, ra) = run_sli(1 << 62, 2);
        assert_eq!(result, 0);
        assert_eq!(ra & RA_V, RA_V);

        // Nothing is shifted out, yet 2^62 · 2 = 2^63 exceeds the signed range.
        let (result, ra) = run_sli(0x4000000000000000, 1);
        assert_eq!(result, 0x8000000000000000);
        assert_eq!(ra & RA_V, RA_V, "the sign flip is an overflow");

        // 2^60 · 4 = 2^62 fits, so nothing is raised.
        let (result, ra) = run_sli(1 << 60, 2);
        assert_eq!(result, 1 << 62);
        assert_eq!(ra, 0, "a representable product raises nothing");

        // A shift of zero is the identity, whatever the sign bits hold.
        let (result, ra) = run_sli(0x8000000000000000, 0);
        assert_eq!(result, 0x8000000000000000);
        assert_eq!(ra, 0);
    }

    #[test]
    fn test_slu_never_overflows() {
        let mut mmix = MMix::new();
        mmix.set_register(1, 0x4000000000000000);
        mmix.set_register(2, 1);
        mmix.write_tetra(0, 0x3A030102); // SLU $3,$1,$2
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(3), 0x8000000000000000);
        assert_eq!(mmix.get_special(SpecialReg::RA), 0);
    }

    #[test]
    fn test_neg_overflow_raises_v() {
        // NEG $3,Y,$1 computes Y - s($1); every such overflow reports V.
        for y in [0u32, 5, 255] {
            let mut mmix = MMix::new();
            mmix.set_register(1, i64::MIN as u64);
            mmix.write_tetra(0, 0x34030001 | (y << 8)); // NEG $3,y,$1
            assert!(mmix.execute_instruction());
            assert_eq!(
                mmix.get_special(SpecialReg::RA) & RA_V,
                RA_V,
                "NEG $3,{y},$1 overflows"
            );
            assert_eq!(
                mmix.get_register(3),
                (y as i64).wrapping_sub(i64::MIN) as u64
            );
        }
    }

    #[test]
    fn test_negu_has_no_overflow() {
        let mut mmix = MMix::new();
        mmix.set_register(1, i64::MIN as u64);
        mmix.write_tetra(0, 0x36030001); // NEGU $3,0,$1
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(3), 0x8000000000000000);
        assert_eq!(mmix.get_special(SpecialReg::RA), 0);
    }

    /// Store `value` from $1 to address 0 with the given store opcode word,
    /// returning rA.
    fn run_store(word: u32, value: u64) -> u64 {
        let mut mmix = MMix::new();
        mmix.set_register(1, value);
        mmix.write_tetra(0, word);
        assert!(mmix.execute_instruction());
        mmix.get_special(SpecialReg::RA)
    }

    #[test]
    fn test_signed_stores_raise_v_when_the_value_does_not_fit() {
        // Register form: ST* $1,$2,$3 with $2 = $3 = 0. Immediate form: Z = 0.
        for (word, wide, narrow) in [
            (0xA0010203u32, 200u64, 127u64),   // STB
            (0xA1010200, 200, 127),            // STBI
            (0xA4010203, 40000, 32767),        // STW
            (0xA5010200, 40000, 32767),        // STWI
            (0xA8010203, 1 << 32, 0x7FFFFFFF), // STT
            (0xA9010200, 1 << 32, 0x7FFFFFFF), // STTI
        ] {
            assert_eq!(run_store(word, wide) & RA_V, RA_V, "{word:#010X} overflows");
            assert_eq!(run_store(word, narrow), 0, "{word:#010X} in range");
        }
    }

    #[test]
    fn test_put_ra_writes_the_rounding_mode_at_bits_17_16() {
        let mut mmix = MMix::new();
        mmix.set_register(1, 1 << RA_ROUND_SHIFT); // ROUND_OFF
        mmix.write_tetra(0, 0xF6150001); // PUT rA,$1
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_special(SpecialReg::RA), 1 << RA_ROUND_SHIFT);

        // The mode is read from those bits: ROUND_OFF truncates 42.9 to 42.
        mmix.set_register(3, 42.9f64.to_bits());
        mmix.write_tetra(4, 0x05020003); // FIX $2,$0,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(2), 42);

        // Raising an event leaves the mode field alone.
        let ra = mmix.get_special(SpecialReg::RA);
        assert_eq!(ra & RA_X, RA_X, "the conversion was inexact");
        assert_eq!(ra >> RA_ROUND_SHIFT, 1, "the mode survives an event");
    }

    #[test]
    fn test_put_ra_rejects_a_value_wider_than_18_bits() {
        let (host, handle) = CaptureHost::new();
        let mut mmix = MMix::with_host(host);
        mmix.set_special(SpecialReg::RA, 2 << RA_ROUND_SHIFT);
        mmix.set_register(1, RA_MAX + 1);
        mmix.write_tetra(0, 0xF6150001); // PUT rA,$1
        assert!(!mmix.execute_instruction());
        assert_eq!(
            mmix.get_special(SpecialReg::RA),
            2 << RA_ROUND_SHIFT,
            "a rejected write leaves rA unchanged"
        );
        assert_eq!(mmix.get_pc(), 0, "the PC stays on the rejected instruction");
        assert_eq!(handle.diagnostics().len(), 1);
        assert!(handle.diagnostics()[0].contains("rA"));
    }

    #[test]
    fn test_put_ra_accepts_the_widest_legal_value() {
        let mut mmix = MMix::new();
        mmix.set_register(1, RA_MAX);
        mmix.write_tetra(0, 0xF6150001); // PUT rA,$1
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_special(SpecialReg::RA), RA_MAX);
    }

    #[test]
    fn test_put_other_specials_is_not_capped() {
        let mut mmix = MMix::new();
        mmix.set_register(1, u64::MAX);
        mmix.write_tetra(0, 0xF6010001); // PUT rD,$1
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_special(SpecialReg::RD), u64::MAX);
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
        let source = "\tLOC\t#100\nMain\tdebug \"hi\"\n; nothing else follows\n";
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
}
