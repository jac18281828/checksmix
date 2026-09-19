//! rA flag bits, `SpecialReg`, `SAVE_SPECIALS`, and the `PUT` register writes.

use super::MMix;

/// rA event-flag bits, matching the predefined symbols `D_BIT` through
/// `X_BIT`. Layout (low → high): X Z U O I W V D.
///
/// `V` and `D` are the integer events; the other six are floating-point.
/// MMIX has no denormalized-operand event — a subnormal operand raises
/// nothing, and `D` means divide check.
pub const RA_X: u64 = 0x01; // floating inexact
pub const RA_Z: u64 = 0x02; // floating divide by zero
pub const RA_U: u64 = 0x04; // floating underflow
pub const RA_O: u64 = 0x08; // floating overflow
pub const RA_I: u64 = 0x10; // floating invalid operation
pub const RA_W: u64 = 0x20; // float-to-fix overflow
pub const RA_V: u64 = 0x40; // integer overflow
pub const RA_D: u64 = 0x80; // integer divide check

/// Bit position of rA's two-bit rounding-mode field, the top of the register.
pub const RA_ROUND_SHIFT: u32 = 16;

/// Widest value `PUT` may write to rA: the register holds 18 bits.
pub const RA_MAX: u64 = 0x3FFFF;

/// Special register identifiers for MMIX.
/// These 32 special registers control various aspects of MMIX operation.
/// Per TAOCP specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SpecialReg {
    RB = 0,   // rB - Bootstrap register
    RD = 1,   // rD - Dividend register
    RE = 2,   // rE - Epsilon register
    RH = 3,   // rH - Himult register
    RJ = 4,   // rJ - Return-jump register
    RM = 5,   // rM - Multiplex mask register
    RR = 6,   // rR - Remainder register
    RBB = 7,  // rBB - Bootstrap register (kernel)
    RC = 8,   // rC - Continuation register
    RN = 9,   // rN - Serial number
    RO = 10,  // rO - Register stack offset
    RS = 11,  // rS - Register stack pointer
    RI = 12,  // rI - Interval counter
    RT = 13,  // rT - Trap address register
    RTT = 14, // rTT - Dynamic trap address register
    RK = 15,  // rK - Interrupt mask register
    RQ = 16,  // rQ - Interrupt request register
    RU = 17,  // rU - Usage counter
    RV = 18,  // rV - Virtual translation register
    RG = 19,  // rG - Global threshold register
    RL = 20,  // rL - Local threshold register
    RA = 21,  // rA - Arithmetic status register
    RF = 22,  // rF - Failure location register
    RP = 23,  // rP - Prediction register
    RW = 24,  // rW - Where-interrupted register (user)
    RX = 25,  // rX - Execution register (user)
    RY = 26,  // rY - Y operand (user)
    RZ = 27,  // rZ - Z operand (user)
    RWW = 28, // rWW - Where-interrupted register (kernel)
    RXX = 29, // rXX - Execution register (kernel)
    RYY = 30, // rYY - Y operand (kernel)
    RZZ = 31, // rZZ - Z operand (kernel)
}

impl SpecialReg {
    /// The MMIXAL spelling of this register, as the assembler predefines it,
    /// the debugger resolves it and the state dump labels it.
    pub fn name(self) -> &'static str {
        match self {
            SpecialReg::RB => "rB",
            SpecialReg::RD => "rD",
            SpecialReg::RE => "rE",
            SpecialReg::RH => "rH",
            SpecialReg::RJ => "rJ",
            SpecialReg::RM => "rM",
            SpecialReg::RR => "rR",
            SpecialReg::RBB => "rBB",
            SpecialReg::RC => "rC",
            SpecialReg::RN => "rN",
            SpecialReg::RO => "rO",
            SpecialReg::RS => "rS",
            SpecialReg::RI => "rI",
            SpecialReg::RT => "rT",
            SpecialReg::RTT => "rTT",
            SpecialReg::RK => "rK",
            SpecialReg::RQ => "rQ",
            SpecialReg::RU => "rU",
            SpecialReg::RV => "rV",
            SpecialReg::RG => "rG",
            SpecialReg::RL => "rL",
            SpecialReg::RA => "rA",
            SpecialReg::RF => "rF",
            SpecialReg::RP => "rP",
            SpecialReg::RW => "rW",
            SpecialReg::RX => "rX",
            SpecialReg::RY => "rY",
            SpecialReg::RZ => "rZ",
            SpecialReg::RWW => "rWW",
            SpecialReg::RXX => "rXX",
            SpecialReg::RYY => "rYY",
            SpecialReg::RZZ => "rZZ",
        }
    }

    /// Convert a u8 register number to a SpecialReg variant
    pub fn from_u8(n: u8) -> Option<Self> {
        match n {
            0 => Some(SpecialReg::RB),
            1 => Some(SpecialReg::RD),
            2 => Some(SpecialReg::RE),
            3 => Some(SpecialReg::RH),
            4 => Some(SpecialReg::RJ),
            5 => Some(SpecialReg::RM),
            6 => Some(SpecialReg::RR),
            7 => Some(SpecialReg::RBB),
            8 => Some(SpecialReg::RC),
            9 => Some(SpecialReg::RN),
            10 => Some(SpecialReg::RO),
            11 => Some(SpecialReg::RS),
            12 => Some(SpecialReg::RI),
            13 => Some(SpecialReg::RT),
            14 => Some(SpecialReg::RTT),
            15 => Some(SpecialReg::RK),
            16 => Some(SpecialReg::RQ),
            17 => Some(SpecialReg::RU),
            18 => Some(SpecialReg::RV),
            19 => Some(SpecialReg::RG),
            20 => Some(SpecialReg::RL),
            21 => Some(SpecialReg::RA),
            22 => Some(SpecialReg::RF),
            23 => Some(SpecialReg::RP),
            24 => Some(SpecialReg::RW),
            25 => Some(SpecialReg::RX),
            26 => Some(SpecialReg::RY),
            27 => Some(SpecialReg::RZ),
            28 => Some(SpecialReg::RWW),
            29 => Some(SpecialReg::RXX),
            30 => Some(SpecialReg::RYY),
            31 => Some(SpecialReg::RZZ),
            _ => None,
        }
    }
}

/// The special registers `SAVE` writes to the register stack and `UNSAVE`
/// reads back, in the order the SAVE page's diagram lays them out.
pub(super) const SAVE_SPECIALS: [SpecialReg; 12] = [
    SpecialReg::RB,
    SpecialReg::RD,
    SpecialReg::RE,
    SpecialReg::RH,
    SpecialReg::RJ,
    SpecialReg::RM,
    SpecialReg::RR,
    SpecialReg::RP,
    SpecialReg::RW,
    SpecialReg::RX,
    SpecialReg::RY,
    SpecialReg::RZ,
];

impl MMix {
    /// Apply a `PUT`/`PUTI` write, enforcing every rule MMIX places on the
    /// destination special register
    /// (mmix.cs.hm.edu/doc/instructions/put.html). `X ≥ 32` names no
    /// register; `rC rN rO rS rI rT rTT rK rQ rU rV` (8–18) are read-only in
    /// user mode; `rG` and `rA` each bound the value they accept. A write
    /// the reference calls impermissible halts with a diagnostic naming the
    /// instruction, the register, the value and the interrupt it raises;
    /// the PC does not advance and no register changes. Returns whether the
    /// write succeeded.
    pub(super) fn put_special(&mut self, mnemonic: &str, x: u8, value: u64) -> bool {
        let Some(reg) = SpecialReg::from_u8(x) else {
            return self.reject(&format!(
                "{mnemonic} X={x},{value}: no special register above 31 at PC={:#018x}",
                self.pc
            ));
        };
        match reg {
            SpecialReg::RC
            | SpecialReg::RI
            | SpecialReg::RK
            | SpecialReg::RQ
            | SpecialReg::RT
            | SpecialReg::RU
            | SpecialReg::RV
            | SpecialReg::RTT => self.reject(&format!(
                "{mnemonic} {},{value}: privileged-operation interrupt at PC={:#018x}",
                reg.name(),
                self.pc
            )),
            SpecialReg::RN | SpecialReg::RO | SpecialReg::RS => self.reject(&format!(
                "{mnemonic} {},{value}: illegal-instruction interrupt at PC={:#018x}",
                reg.name(),
                self.pc
            )),
            SpecialReg::RG => {
                let rl = self.get_special(SpecialReg::RL);
                if value < 32 || value < rl || value > 255 {
                    return self.reject(&format!(
                        "{mnemonic} rG,{value}: illegal-instruction interrupt \
                         (rG must be 32-255 and >= rL={rl}) at PC={:#018x}",
                        self.pc
                    ));
                }
                self.put_rg(value);
                true
            }
            SpecialReg::RL => {
                self.put_rl(value);
                true
            }
            SpecialReg::RA if value > RA_MAX => self.reject(&format!(
                "{mnemonic} rA,{value:#x}: illegal-instruction interrupt \
                 (rA holds at most 18 bits, max {RA_MAX:#x}) at PC={:#018x}",
                self.pc
            )),
            _ => {
                self.set_special(reg, value);
                true
            }
        }
    }

    /// `PUT rL,z` only ever lowers rL, to `min(z, rL)`. The registers the
    /// drop excludes from the local range become marginal and must read
    /// zero.
    fn put_rl(&mut self, z: u64) {
        let rl_old = self.get_special(SpecialReg::RL);
        let rl_new = z.min(rl_old);
        // Only the local range goes marginal: the globals $rG.. keep their
        // values, and an rL restored from guest memory can name a register
        // the file does not have.
        let rg = self.get_special(SpecialReg::RG);
        let drop_end = rl_old.min(rg).min(self.general_regs.len() as u64);
        for marginal in rl_new..drop_end {
            self.general_regs[marginal as usize] = 0;
        }
        self.set_special(SpecialReg::RL, rl_new);
    }

    /// `PUT rG,z` moves the boundary between the local and global register
    /// ranges. Every register between the old and new `rG` changes class —
    /// global to local/marginal when raising, local/marginal to global when
    /// lowering — and must read zero afterward; registers outside that span
    /// keep their values. A stale `rG` beyond the register file (reachable
    /// only through an unvalidated `UNSAVE`) is clamped so the sweep never
    /// indexes past it.
    fn put_rg(&mut self, z: u64) {
        let rg_old = self.get_special(SpecialReg::RG);
        let (lo, hi) = if z > rg_old { (rg_old, z) } else { (z, rg_old) };
        let hi = hi.min(self.general_regs.len() as u64);
        for reg in lo..hi {
            self.general_regs[reg as usize] = 0;
        }
        self.set_special(SpecialReg::RG, z);
    }
}
