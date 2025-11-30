use std::fmt;
use tracing::{debug, instrument, trace};

/// Macro for register-register binary operations
macro_rules! binop_rr {
    ($cpu:expr, $x:expr, $y:expr, $z:expr, $f:expr) => {{
        let a = $cpu.get_register($y);
        let b = $cpu.get_register($z);
        $cpu.set_register($x, $f(a, b));
        $cpu.advance_pc();
        true
    }};
}

/// Macro for register-immediate binary operations
macro_rules! binop_ri {
    ($cpu:expr, $x:expr, $y:expr, $z:expr, $f:expr) => {{
        let a = $cpu.get_register($y);
        let b = $z as u64;
        $cpu.set_register($x, $f(a, b));
        $cpu.advance_pc();
        true
    }};
}

/// Macro for comparison operations (register-register)
macro_rules! cmp_rr {
    ($cpu:expr, $x:expr, $y:expr, $z:expr, $conv:expr) => {{
        let a = $conv($cpu.get_register($y));
        let b = $conv($cpu.get_register($z));
        let result = if a < b {
            (-1i64) as u64
        } else if a == b {
            0
        } else {
            1
        };
        $cpu.set_register($x, result);
        $cpu.advance_pc();
        true
    }};
}

/// Macro for comparison operations (register-immediate)
macro_rules! cmp_ri {
    ($cpu:expr, $x:expr, $y:expr, $z:expr, $conv_y:expr, $conv_z:expr) => {{
        let a = $conv_y($cpu.get_register($y));
        let b = $conv_z($z);
        let result = if a < b {
            (-1i64) as u64
        } else if a == b {
            0
        } else {
            1
        };
        $cpu.set_register($x, result);
        $cpu.advance_pc();
        true
    }};
}

/// Macro for floating-point binary operations (register-register)
macro_rules! fbinop_rr {
    ($cpu:expr, $x:expr, $y:expr, $z:expr, $op:expr) => {{
        let y_val = MMix::u64_to_f64($cpu.get_register($y));
        let z_val = MMix::u64_to_f64($cpu.get_register($z));
        let result = $op(y_val, z_val);
        $cpu.set_register($x, MMix::f64_to_u64(result));
        $cpu.advance_pc();
        true
    }};
}

/// Macro for floating-point unary operations
macro_rules! funop {
    ($cpu:expr, $x:expr, $z:expr, $op:expr) => {{
        let z_val = MMix::u64_to_f64($cpu.get_register($z));
        let result = $op(z_val);
        $cpu.set_register($x, MMix::f64_to_u64(result));
        $cpu.advance_pc();
        true
    }};
}

/// Macro for multiply-add operations: $X = $Y * N + $Z (register-register)
macro_rules! muladd_rr {
    ($cpu:expr, $x:expr, $y:expr, $z:expr, $n:expr) => {{
        let sum = $cpu
            .get_register($y)
            .wrapping_mul($n)
            .wrapping_add($cpu.get_register($z));
        $cpu.set_register($x, sum);
        $cpu.advance_pc();
        true
    }};
}

/// Macro for multiply-add operations: $X = $Y * N + Z (register-immediate)
macro_rules! muladd_ri {
    ($cpu:expr, $x:expr, $y:expr, $z:expr, $n:expr) => {{
        let sum = $cpu
            .get_register($y)
            .wrapping_mul($n)
            .wrapping_add($z as u64);
        $cpu.set_register($x, sum);
        $cpu.advance_pc();
        true
    }};
}

/// Macro for float-to-int conversions
macro_rules! f2i_conv {
    ($cpu:expr, $x:expr, $z:expr, $conv:expr) => {{
        let z_val = MMix::u64_to_f64($cpu.get_register($z));
        let result = $conv(z_val);
        $cpu.set_register($x, result);
        $cpu.advance_pc();
        true
    }};
}

/// Macro for int-to-float conversions (register)
macro_rules! i2f_conv_rr {
    ($cpu:expr, $x:expr, $z:expr, $conv:expr) => {{
        let z_val = $cpu.get_register($z);
        let result = $conv(z_val);
        $cpu.set_register($x, MMix::f64_to_u64(result));
        $cpu.advance_pc();
        true
    }};
}

/// Macro for int-to-float conversions (immediate)
macro_rules! i2f_conv_ri {
    ($cpu:expr, $x:expr, $y:expr, $z:expr, $conv:expr) => {{
        let yz = ($y as u16) << 8 | $z as u16;
        let result = $conv(yz);
        $cpu.set_register($x, MMix::f64_to_u64(result));
        $cpu.advance_pc();
        true
    }};
}

/// Macro for floating point comparison/test operations
macro_rules! fcmp_rr {
    ($cpu:expr, $x:expr, $y:expr, $z:expr, $test:expr) => {{
        let y_val = MMix::u64_to_f64($cpu.get_register($y));
        let z_val = MMix::u64_to_f64($cpu.get_register($z));
        let result = $test(y_val, z_val);
        $cpu.set_register($x, result);
        $cpu.advance_pc();
        true
    }};
}

/// Macro for signed multiplication with overflow detection (register-register)
macro_rules! mul_rr {
    ($cpu:expr, $x:expr, $y:expr, $z:expr) => {{
        let a = $cpu.get_register($y) as i64;
        let b = $cpu.get_register($z) as i64;
        let product = (a as i128) * (b as i128);
        $cpu.set_register($x, product as u64);
        $cpu.set_special(SpecialReg::RH, (product >> 64) as u64);
        let sign_ext = if (product as u64) as i64 >= 0 {
            0i64
        } else {
            -1i64
        };
        if (product >> 64) as i64 != sign_ext {
            $cpu.set_special(SpecialReg::RA, $cpu.get_special(SpecialReg::RA) | 0x04);
        }
        $cpu.advance_pc();
        true
    }};
}

/// Macro for signed multiplication with overflow detection (register-immediate)
macro_rules! mul_ri {
    ($cpu:expr, $x:expr, $y:expr, $z:expr) => {{
        let a = $cpu.get_register($y) as i64;
        let b = $z as i64;
        let product = (a as i128) * (b as i128);
        $cpu.set_register($x, product as u64);
        $cpu.set_special(SpecialReg::RH, (product >> 64) as u64);
        let sign_ext = if (product as u64) as i64 >= 0 {
            0i64
        } else {
            -1i64
        };
        if (product >> 64) as i64 != sign_ext {
            $cpu.set_special(SpecialReg::RA, $cpu.get_special(SpecialReg::RA) | 0x04);
        }
        $cpu.advance_pc();
        true
    }};
}

/// Macro for unsigned multiplication (register-register)
macro_rules! mulu_rr {
    ($cpu:expr, $x:expr, $y:expr, $z:expr) => {{
        let a = $cpu.get_register($y) as u128;
        let b = $cpu.get_register($z) as u128;
        let product = a * b;
        $cpu.set_register($x, product as u64);
        $cpu.set_special(SpecialReg::RH, (product >> 64) as u64);
        $cpu.advance_pc();
        true
    }};
}

/// Macro for unsigned multiplication (register-immediate)
macro_rules! mulu_ri {
    ($cpu:expr, $x:expr, $y:expr, $z:expr) => {{
        let a = $cpu.get_register($y) as u128;
        let b = $z as u128;
        let product = a * b;
        $cpu.set_register($x, product as u64);
        $cpu.set_special(SpecialReg::RH, (product >> 64) as u64);
        $cpu.advance_pc();
        true
    }};
}

/// Macro for signed division (register-register)
macro_rules! div_rr {
    ($cpu:expr, $x:expr, $y:expr, $z:expr) => {{
        let dividend = $cpu.get_register($y) as i64;
        let divisor = $cpu.get_register($z) as i64;
        if divisor == 0 {
            $cpu.set_register($x, 0);
            $cpu.set_special(SpecialReg::RR, $cpu.get_register($y));
        } else {
            let quotient = dividend / divisor;
            let remainder = dividend % divisor;
            $cpu.set_register($x, quotient as u64);
            $cpu.set_special(SpecialReg::RR, remainder as u64);
        }
        $cpu.advance_pc();
        true
    }};
}

/// Macro for signed division (register-immediate)
macro_rules! div_ri {
    ($cpu:expr, $x:expr, $y:expr, $z:expr) => {{
        let dividend = $cpu.get_register($y) as i64;
        let divisor = $z as i64;
        if divisor == 0 {
            $cpu.set_register($x, 0);
            $cpu.set_special(SpecialReg::RR, $cpu.get_register($y));
        } else {
            let quotient = dividend / divisor;
            let remainder = dividend % divisor;
            $cpu.set_register($x, quotient as u64);
            $cpu.set_special(SpecialReg::RR, remainder as u64);
        }
        $cpu.advance_pc();
        true
    }};
}

/// Macro for unsigned division (register-register)
macro_rules! divu_rr {
    ($cpu:expr, $x:expr, $y:expr, $z:expr) => {{
        let dividend_low = $cpu.get_register($y);
        let dividend_high = $cpu.get_special(SpecialReg::RD);
        let dividend = ((dividend_high as u128) << 64) | (dividend_low as u128);
        let divisor = $cpu.get_register($z) as u128;
        if divisor == 0 {
            $cpu.set_register($x, 0);
            $cpu.set_special(SpecialReg::RR, dividend_low);
        } else {
            let quotient = dividend / divisor;
            let remainder = dividend % divisor;
            $cpu.set_register($x, quotient as u64);
            $cpu.set_special(SpecialReg::RR, remainder as u64);
        }
        $cpu.advance_pc();
        true
    }};
}

/// Macro for unsigned division (register-immediate)
macro_rules! divu_ri {
    ($cpu:expr, $x:expr, $y:expr, $z:expr) => {{
        let dividend_low = $cpu.get_register($y);
        let dividend_high = $cpu.get_special(SpecialReg::RD);
        let dividend = ((dividend_high as u128) << 64) | (dividend_low as u128);
        let divisor = $z as u128;
        if divisor == 0 {
            $cpu.set_register($x, 0);
            $cpu.set_special(SpecialReg::RR, dividend_low);
        } else {
            let quotient = dividend / divisor;
            let remainder = dividend % divisor;
            $cpu.set_register($x, quotient as u64);
            $cpu.set_special(SpecialReg::RR, remainder as u64);
        }
        $cpu.advance_pc();
        true
    }};
}

/// Macro for signed addition with overflow detection (register-register)
macro_rules! add_rr {
    ($cpu:expr, $x:expr, $y:expr, $z:expr) => {{
        let a = $cpu.get_register($y) as i64;
        let b = $cpu.get_register($z) as i64;
        match a.checked_add(b) {
            Some(result) => {
                $cpu.set_register($x, result as u64);
            }
            None => {
                $cpu.set_register($x, a.wrapping_add(b) as u64);
                $cpu.set_special(SpecialReg::RA, $cpu.get_special(SpecialReg::RA) | 0x04);
            }
        }
        $cpu.advance_pc();
        true
    }};
}

/// Macro for signed addition with overflow detection (register-immediate)
macro_rules! add_ri {
    ($cpu:expr, $x:expr, $y:expr, $z:expr) => {{
        let a = $cpu.get_register($y) as i64;
        let b = $z as i64;
        match a.checked_add(b) {
            Some(result) => {
                $cpu.set_register($x, result as u64);
            }
            None => {
                $cpu.set_register($x, a.wrapping_add(b) as u64);
                $cpu.set_special(SpecialReg::RA, $cpu.get_special(SpecialReg::RA) | 0x04);
            }
        }
        $cpu.advance_pc();
        true
    }};
}

/// Macro for signed subtraction with overflow detection (register-register)
macro_rules! sub_rr {
    ($cpu:expr, $x:expr, $y:expr, $z:expr) => {{
        let a = $cpu.get_register($y) as i64;
        let b = $cpu.get_register($z) as i64;
        match a.checked_sub(b) {
            Some(result) => {
                $cpu.set_register($x, result as u64);
            }
            None => {
                $cpu.set_register($x, a.wrapping_sub(b) as u64);
                $cpu.set_special(SpecialReg::RA, $cpu.get_special(SpecialReg::RA) | 0x04);
            }
        }
        $cpu.advance_pc();
        true
    }};
}

/// Macro for signed subtraction with overflow detection (register-immediate)
macro_rules! sub_ri {
    ($cpu:expr, $x:expr, $y:expr, $z:expr) => {{
        let a = $cpu.get_register($y) as i64;
        let b = $z as i64;
        match a.checked_sub(b) {
            Some(result) => {
                $cpu.set_register($x, result as u64);
            }
            None => {
                $cpu.set_register($x, a.wrapping_sub(b) as u64);
                $cpu.set_special(SpecialReg::RA, $cpu.get_special(SpecialReg::RA) | 0x04);
            }
        }
        $cpu.advance_pc();
        true
    }};
}

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

/// The MMIX computer architecture.
///
/// MMIX has:
/// - 256 general-purpose registers ($0-$255), each holding 64 bits (an octabyte)
/// - 32 special-purpose registers (rA-rZ, rBB, rTT, rWW, rXX, rYY, rZZ)
/// - 2^64 bytes of virtual memory
///
/// Instructions are tetrybytes (4 bytes) with format: OP X Y Z
/// where OP is the opcode and X, Y, Z are operands.
pub struct MMix {
    /// 256 general-purpose registers, each 64 bits
    /// Register $255 is special: its value is always zero
    general_regs: [u64; 256],

    /// 32 special-purpose registers, each 64 bits
    /// Indexed by SpecialReg enum values
    special_regs: [u64; 32],

    /// Virtual memory (simplified as an indexmap for sparse storage)
    /// In a real implementation, this would use paging/segmentation
    /// Key is the memory address, value is the byte
    /// Using IndexMap for deterministic iteration order
    memory: indexmap::IndexMap<u64, u8>,

    /// Program counter (location of next instruction)
    pc: u64,
}

impl Default for MMix {
    fn default() -> Self {
        Self::new()
    }
}

impl MMix {
    /// Create a new MMIX computer with all registers and memory initialized to zero.
    pub fn new() -> Self {
        Self {
            general_regs: [0; 256],
            special_regs: [0; 32],
            memory: indexmap::IndexMap::new(),
            pc: 0,
        }
    }

    /// Get the value of a general-purpose register.
    /// Register $255 always returns 0.
    pub fn get_register(&self, reg: u8) -> u64 {
        if reg == 255 {
            0 // $255 is always zero
        } else {
            self.general_regs[reg as usize]
        }
    }

    /// Set the value of a general-purpose register.
    /// Writes to $255 are ignored (it remains zero).
    pub fn set_register(&mut self, reg: u8, value: u64) {
        if reg != 255 {
            self.general_regs[reg as usize] = value;
        }
    }

    /// Get the value of a special-purpose register.
    pub fn get_special(&self, reg: SpecialReg) -> u64 {
        self.special_regs[reg as usize]
    }

    /// Set the value of a special-purpose register.
    pub fn set_special(&mut self, reg: SpecialReg, value: u64) {
        self.special_regs[reg as usize] = value;
    }

    /// Read a byte from memory at the given address.
    /// Uninitialized memory reads as zero.
    #[instrument(skip(self), level = "trace")]
    pub fn read_byte(&self, addr: u64) -> u8 {
        let value = *self.memory.get(&addr).unwrap_or(&0);
        trace!(
            addr = format!("0x{:X}", addr),
            value, "Read byte from memory"
        );
        value
    }

    /// Write a byte to memory at the given address.
    #[instrument(skip(self), level = "trace")]
    pub fn write_byte(&mut self, addr: u64, value: u8) {
        trace!(
            addr = format!("0x{:X}", addr),
            value, "Writing byte to memory"
        );
        if value == 0 {
            self.memory.shift_remove(&addr); // Don't store zeros (sparse memory)
        } else {
            self.memory.insert(addr, value);
        }
    }

    /// Read a wyde (2 bytes) from memory starting at the given address.
    pub fn read_wyde(&self, addr: u64) -> u16 {
        let b0 = self.read_byte(addr) as u16;
        let b1 = self.read_byte(addr.wrapping_add(1)) as u16;
        (b0 << 8) | b1
    }

    /// Write a wyde (2 bytes) to memory starting at the given address.
    pub fn write_wyde(&mut self, addr: u64, value: u16) {
        self.write_byte(addr, (value >> 8) as u8);
        self.write_byte(addr.wrapping_add(1), value as u8);
    }

    /// Read a tetra (4 bytes) from memory starting at the given address.
    pub fn read_tetra(&self, addr: u64) -> u32 {
        let b0 = self.read_byte(addr) as u32;
        let b1 = self.read_byte(addr.wrapping_add(1)) as u32;
        let b2 = self.read_byte(addr.wrapping_add(2)) as u32;
        let b3 = self.read_byte(addr.wrapping_add(3)) as u32;
        (b0 << 24) | (b1 << 16) | (b2 << 8) | b3
    }

    /// Write a tetra (4 bytes) to memory starting at the given address.
    pub fn write_tetra(&mut self, addr: u64, value: u32) {
        self.write_byte(addr, (value >> 24) as u8);
        self.write_byte(addr.wrapping_add(1), (value >> 16) as u8);
        self.write_byte(addr.wrapping_add(2), (value >> 8) as u8);
        self.write_byte(addr.wrapping_add(3), value as u8);
    }

    /// Read an octa (8 bytes) from memory starting at the given address.
    pub fn read_octa(&self, addr: u64) -> u64 {
        let b0 = self.read_byte(addr) as u64;
        let b1 = self.read_byte(addr.wrapping_add(1)) as u64;
        let b2 = self.read_byte(addr.wrapping_add(2)) as u64;
        let b3 = self.read_byte(addr.wrapping_add(3)) as u64;
        let b4 = self.read_byte(addr.wrapping_add(4)) as u64;
        let b5 = self.read_byte(addr.wrapping_add(5)) as u64;
        let b6 = self.read_byte(addr.wrapping_add(6)) as u64;
        let b7 = self.read_byte(addr.wrapping_add(7)) as u64;
        (b0 << 56) | (b1 << 48) | (b2 << 40) | (b3 << 32) | (b4 << 24) | (b5 << 16) | (b6 << 8) | b7
    }

    /// Write an octa (8 bytes) to memory starting at the given address.
    pub fn write_octa(&mut self, addr: u64, value: u64) {
        self.write_byte(addr, (value >> 56) as u8);
        self.write_byte(addr.wrapping_add(1), (value >> 48) as u8);
        self.write_byte(addr.wrapping_add(2), (value >> 40) as u8);
        self.write_byte(addr.wrapping_add(3), (value >> 32) as u8);
        self.write_byte(addr.wrapping_add(4), (value >> 24) as u8);
        self.write_byte(addr.wrapping_add(5), (value >> 16) as u8);
        self.write_byte(addr.wrapping_add(6), (value >> 8) as u8);
        self.write_byte(addr.wrapping_add(7), value as u8);
    }

    /// Fetch the next instruction from memory and decode it.
    /// Returns (OP, X, Y, Z) where:
    /// - OP is the opcode
    /// - X, Y, Z are the operand bytes
    pub fn fetch_instruction(&self) -> (u8, u8, u8, u8) {
        let instruction = self.read_tetra(self.pc);
        let op = (instruction >> 24) as u8;
        let x = (instruction >> 16) as u8;
        let y = (instruction >> 8) as u8;
        let z = instruction as u8;
        (op, x, y, z)
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

    // ========== Internal Helpers ==========

    /// Conditional branch forward: if cond, PC = (PC + 4) + (Y<<8|Z) * 4
    #[inline]
    fn branch_forward(&mut self, cond: bool, y: u8, z: u8) {
        if cond {
            let offset = ((y as u16) << 8 | z as u16) as i16;
            // Branch is relative to PC+4 (after the branch instruction)
            self.pc = (self.pc + 4).wrapping_add((offset as i64 * 4) as u64);
        } else {
            self.advance_pc();
        }
    }

    /// Conditional branch backward: if cond, PC = (PC + 4) - (Y<<8|Z) * 4
    #[inline]
    fn branch_backward(&mut self, cond: bool, y: u8, z: u8) {
        if cond {
            let offset = (y as u16) << 8 | z as u16;
            // Branch is relative to PC+4 (after the branch instruction)
            self.pc = (self.pc + 4).wrapping_sub((offset as u64) * 4);
        } else {
            self.advance_pc();
        }
    }

    /// Conditional set: if cond($X), $X = $Y + $Z, else $X = $Y
    #[inline]
    fn cond_set_rr(&mut self, x: u8, y: u8, z: u8, cond: bool) {
        let val_y = self.get_register(y);
        let val_z = self.get_register(z);
        let result = if cond {
            val_y.wrapping_add(val_z)
        } else {
            val_y
        };
        self.set_register(x, result);
        self.advance_pc();
    }

    /// Conditional set with immediate: if cond($X), $X = $Y + Z, else $X = $Y
    #[inline]
    fn cond_set_ri(&mut self, x: u8, y: u8, z: u8, cond: bool) {
        let val_y = self.get_register(y);
        let result = if cond {
            val_y.wrapping_add(z as u64)
        } else {
            val_y
        };
        self.set_register(x, result);
        self.advance_pc();
    }

    /// Convert u64 to f64 (reinterpret bits)
    #[inline]
    fn u64_to_f64(value: u64) -> f64 {
        f64::from_bits(value)
    }

    /// Convert f64 to u64 (reinterpret bits)
    #[inline]
    fn f64_to_u64(value: f64) -> u64 {
        value.to_bits()
    }

    /// Floating point comparison: returns -1 if y < z, 0 if y == z, 1 if y > z, 2 if unordered
    #[inline]
    fn fcmp(y: f64, z: f64) -> u64 {
        if y.is_nan() || z.is_nan() {
            2 // Unordered
        } else if y < z {
            (-1i64) as u64
        } else if y > z {
            1
        } else {
            0
        }
    }

    /// Zero or set with register: if cond($X), $X = $Y + $Z, else $X = 0
    #[inline]
    fn zero_set_rr(&mut self, x: u8, y: u8, z: u8, cond: bool) {
        let result = if cond {
            self.get_register(y).wrapping_add(self.get_register(z))
        } else {
            0
        };
        self.set_register(x, result);
        self.advance_pc();
    }

    /// Zero or set with immediate: if cond($X), $X = $Y + Z, else $X = 0
    #[inline]
    fn zero_set_ri(&mut self, x: u8, y: u8, z: u8, cond: bool) {
        let result = if cond {
            self.get_register(y).wrapping_add(z as u64)
        } else {
            0
        };
        self.set_register(x, result);
        self.advance_pc();
    }

    /// Handle TRAP system calls
    /// Returns true if execution should continue, false if halted
    fn handle_trap(&mut self, trap_code: u8, arg: u8) -> bool {
        match trap_code {
            0 => {
                // Halt - stop execution
                debug!(trap_code, arg, "TRAP: Halt");
                self.advance_pc();
                false
            }
            7 => {
                // Fputs - write null-terminated string to file
                // Standard calling convention: $0 contains the string address
                // arg (Z) contains the file descriptor (0=stdin, 1=stdout, 2=stderr)

                let str_addr = self.get_register(0);
                let mut output = String::new();
                let mut addr = str_addr;

                // Read null-terminated string from memory
                loop {
                    let byte = self.read_byte(addr);
                    if byte == 0 {
                        break;
                    }
                    output.push(byte as char);
                    addr += 1;
                    // Safety limit
                    if addr.wrapping_sub(str_addr) > 10000 {
                        eprintln!("Warning: Fputs string too long, truncating");
                        break;
                    }
                }

                // Write to appropriate stream
                match arg {
                    1 => print!("{}", output),  // stdout
                    2 => eprint!("{}", output), // stderr
                    _ => {
                        debug!(trap_code, arg, "Fputs to unsupported file descriptor");
                    }
                }

                debug!(
                    trap_code,
                    arg,
                    str_addr = format!("0x{:X}", str_addr),
                    "TRAP: Fputs"
                );
                self.advance_pc();
                true
            }
            _ => {
                // Unhandled trap - just advance PC and continue
                debug!(trap_code, arg, "TRAP: Unhandled trap code");
                self.advance_pc();
                true
            }
        }
    }

    // ========== Instruction Execution ==========

    /// Execute a single instruction at the current program counter.
    /// Returns true if execution should continue, false if halted.
    #[instrument(skip(self), fields(pc = format!("0x{:X}", self.pc)))]
    pub fn execute_instruction(&mut self) -> bool {
        let (op, x, y, z) = self.fetch_instruction();
        debug!(
            op = format!("0x{:02X}", op),
            x, y, z, "Executing instruction"
        );

        match op {
            // Floating Point instructions
            0x00 => {
                // TRAP X, YZ or TRAP X, Y, Z - Force trap interrupt
                // X = 0 for immediate (YZ), X > 0 for register ($Y, $Z)
                // For immediate form: Y is the trap number, Z is an argument
                if x == 0 {
                    // Immediate TRAP - handle system calls
                    self.handle_trap(y, z)
                } else {
                    // Register form - not commonly used for syscalls
                    let trap_val = {
                        let y_val = self.get_register(y);
                        let z_val = self.get_register(z);
                        (y_val << 32) | z_val
                    };
                    self.set_special(SpecialReg::RBB, trap_val);
                    self.advance_pc();
                    false // Halt by default for unhandled register traps
                }
            }
            0x01 => {
                // FCMP $X, $Y, $Z - Floating compare
                let y_val = Self::u64_to_f64(self.get_register(y));
                let z_val = Self::u64_to_f64(self.get_register(z));
                self.set_register(x, Self::fcmp(y_val, z_val));
                self.advance_pc();
                true
            }
            0x02 => {
                // FUN $X, $Y, $Z - Floating unordered
                fcmp_rr!(
                    self,
                    x,
                    y,
                    z,
                    |y: f64, z: f64| if y.is_nan() || z.is_nan() { 1 } else { 0 }
                )
            }
            0x03 => {
                // FEQL $X, $Y, $Z - Floating equal to
                fcmp_rr!(self, x, y, z, |y: f64, z: f64| if y == z { 1 } else { 0 })
            }
            0x04 => {
                // FADD $X, $Y, $Z
                fbinop_rr!(self, x, y, z, |a, b| a + b)
            }
            0x05 => {
                // FIX $X, $Z - Convert floating to fixed (signed)
                f2i_conv!(self, x, z, |f: f64| f as i64 as u64)
            }
            0x06 => {
                // FSUB $X, $Y, $Z
                fbinop_rr!(self, x, y, z, |a, b| a - b)
            }
            0x07 => {
                // FIXU $X, $Z - Convert floating to fixed unsigned
                f2i_conv!(self, x, z, |f: f64| f as u64)
            }
            0x08 => {
                // FLOT $X, $Z - Convert fixed to floating (signed)
                i2f_conv_rr!(self, x, z, |v: u64| (v as i64) as f64)
            }
            0x09 => {
                // FLOTI $X, YZ - Convert fixed to floating immediate (signed)
                i2f_conv_ri!(self, x, y, z, |yz: u16| (yz as i16 as i64) as f64)
            }
            0x0A => {
                // FLOTU $X, $Z - Convert fixed unsigned to floating
                i2f_conv_rr!(self, x, z, |v: u64| v as f64)
            }
            0x0B => {
                // FLOTUI $X, YZ - Convert fixed unsigned to floating immediate
                i2f_conv_ri!(self, x, y, z, |yz: u16| yz as f64)
            }
            0x0C => {
                // SFLOT $X, $Z - Convert fixed to short float (signed, 32-bit)
                i2f_conv_rr!(self, x, z, |v: u64| ((v as i64) as f32) as f64)
            }
            0x0D => {
                // SFLOTI $X, YZ - Convert fixed to short float immediate (signed)
                i2f_conv_ri!(self, x, y, z, |yz: u16| ((yz as i16 as i64) as f32) as f64)
            }
            0x0E => {
                // SFLOTU $X, $Z - Convert fixed unsigned to short float
                i2f_conv_rr!(self, x, z, |v: u64| (v as f32) as f64)
            }
            0x0F => {
                // SFLOTUI $X, YZ - Convert fixed unsigned to short float immediate
                i2f_conv_ri!(self, x, y, z, |yz: u16| (yz as f32) as f64)
            }
            0x10 => {
                // FMUL $X, $Y, $Z
                fbinop_rr!(self, x, y, z, |a, b| a * b)
            }
            0x11 => {
                // FCMPE $X, $Y, $Z - Floating compare with epsilon
                // Epsilon from rE register
                let y_val = Self::u64_to_f64(self.get_register(y));
                let z_val = Self::u64_to_f64(self.get_register(z));
                let epsilon = Self::u64_to_f64(self.get_special(SpecialReg::RE));
                let diff = (y_val - z_val).abs();
                let result = if diff <= epsilon {
                    0 // Equal within epsilon
                } else if y_val < z_val {
                    (-1i64) as u64
                } else {
                    1
                };
                self.set_register(x, result);
                self.advance_pc();
                true
            }
            0x12 => {
                // FUNE $X, $Y, $Z - Floating unordered with epsilon
                let y_val = Self::u64_to_f64(self.get_register(y));
                let z_val = Self::u64_to_f64(self.get_register(z));
                let epsilon = Self::u64_to_f64(self.get_special(SpecialReg::RE));
                let diff = (y_val - z_val).abs();
                let result = if y_val.is_nan() || z_val.is_nan() || diff <= epsilon {
                    1
                } else {
                    0
                };
                self.set_register(x, result);
                self.advance_pc();
                true
            }
            0x13 => {
                // FEQLE $X, $Y, $Z - Floating equivalent with epsilon
                let y_val = Self::u64_to_f64(self.get_register(y));
                let z_val = Self::u64_to_f64(self.get_register(z));
                let epsilon = Self::u64_to_f64(self.get_special(SpecialReg::RE));
                let diff = (y_val - z_val).abs();
                let result = if diff <= epsilon { 1 } else { 0 };
                self.set_register(x, result);
                self.advance_pc();
                true
            }
            0x14 => {
                // FDIV $X, $Y, $Z
                fbinop_rr!(self, x, y, z, |a, b| a / b)
            }
            0x15 => {
                // FSQRT $X, $Z
                funop!(self, x, z, |v: f64| v.sqrt())
            }
            0x16 => {
                // FREM $X, $Y, $Z
                fbinop_rr!(self, x, y, z, |a, b| a % b)
            }
            0x17 => {
                // FINT $X, $Y, $Z - Floating integerize with rounding mode from rA
                // Y field must be 0, Z field contains operand
                let z_val = Self::u64_to_f64(self.get_register(z));
                // Get rounding mode from rA register (bits 0-15)
                let ra = self.get_special(SpecialReg::RA);
                let round_mode = (ra & 0xFFFF) as u16;

                // Apply rounding based on mode (simplified - use standard rounding)
                // In full implementation, would use round_mode to control rounding
                let result = match round_mode & 0x3 {
                    0 => z_val.round(), // ROUND_NEAR (default)
                    1 => z_val.floor(), // ROUND_DOWN
                    2 => z_val.ceil(),  // ROUND_UP
                    _ => z_val.trunc(), // ROUND_OFF (toward zero)
                };
                self.set_register(x, Self::f64_to_u64(result));
                self.advance_pc();
                true
            }

            // Load instructions
            0x80 => {
                // LDB $X, $Y, $Z - Load byte signed
                // s($X) <- s(M[$Y + $Z])
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let byte = self.read_byte(addr);
                let value = (byte as i8) as i64 as u64; // Sign extend
                self.set_register(x, value);
                self.advance_pc();
                true
            }
            0x81 => {
                // LDB $X, $Y, Z - Load byte signed (immediate)
                let addr = self.get_register(y).wrapping_add(z as u64);
                let byte = self.read_byte(addr);
                let value = (byte as i8) as i64 as u64; // Sign extend
                self.set_register(x, value);
                self.advance_pc();
                true
            }
            0x82 => {
                // LDBU $X, $Y, $Z - Load byte unsigned
                // u($X) <- M[$Y + $Z]
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let byte = self.read_byte(addr);
                self.set_register(x, byte as u64);
                self.advance_pc();
                true
            }
            0x83 => {
                // LDBU $X, $Y, Z - Load byte unsigned (immediate)
                let addr = self.get_register(y).wrapping_add(z as u64);
                let byte = self.read_byte(addr);
                self.set_register(x, byte as u64);
                self.advance_pc();
                true
            }
            0x84 => {
                // LDW $X, $Y, $Z - Load wyde signed
                // s($X) <- s(M2[$Y + $Z])
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let wyde = self.read_wyde(addr);
                let value = (wyde as i16) as i64 as u64; // Sign extend
                self.set_register(x, value);
                self.advance_pc();
                true
            }
            0x85 => {
                // LDW $X, $Y, Z - Load wyde signed (immediate)
                let addr = self.get_register(y).wrapping_add(z as u64);
                let wyde = self.read_wyde(addr);
                let value = (wyde as i16) as i64 as u64; // Sign extend
                self.set_register(x, value);
                self.advance_pc();
                true
            }
            0x86 => {
                // LDWU $X, $Y, $Z - Load wyde unsigned
                // u($X) <- M2[$Y + $Z]
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let wyde = self.read_wyde(addr);
                self.set_register(x, wyde as u64);
                self.advance_pc();
                true
            }
            0x87 => {
                // LDWU $X, $Y, Z - Load wyde unsigned (immediate)
                let addr = self.get_register(y).wrapping_add(z as u64);
                let wyde = self.read_wyde(addr);
                self.set_register(x, wyde as u64);
                self.advance_pc();
                true
            }
            0x88 => {
                // LDT $X, $Y, $Z - Load tetra signed
                // s($X) <- s(M4[$Y + $Z])
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let tetra = self.read_tetra(addr);
                let value = (tetra as i32) as i64 as u64; // Sign extend
                self.set_register(x, value);
                self.advance_pc();
                true
            }
            0x89 => {
                // LDT $X, $Y, Z - Load tetra signed (immediate)
                let addr = self.get_register(y).wrapping_add(z as u64);
                let tetra = self.read_tetra(addr);
                let value = (tetra as i32) as i64 as u64; // Sign extend
                self.set_register(x, value);
                self.advance_pc();
                true
            }
            0x8A => {
                // LDTU $X, $Y, $Z - Load tetra unsigned
                // u($X) <- M4[$Y + $Z]
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let tetra = self.read_tetra(addr);
                self.set_register(x, tetra as u64);
                self.advance_pc();
                true
            }
            0x8B => {
                // LDTU $X, $Y, Z - Load tetra unsigned (immediate)
                let addr = self.get_register(y).wrapping_add(z as u64);
                let tetra = self.read_tetra(addr);
                self.set_register(x, tetra as u64);
                self.advance_pc();
                true
            }
            0x8C => {
                // LDO $X, $Y, $Z - Load octa
                // u($X) <- M8[$Y + $Z]
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let octa = self.read_octa(addr);
                self.set_register(x, octa);
                self.advance_pc();
                true
            }
            0x8D => {
                // LDO $X, $Y, Z - Load octa (immediate)
                let addr = self.get_register(y).wrapping_add(z as u64);
                let octa = self.read_octa(addr);
                self.set_register(x, octa);
                self.advance_pc();
                true
            }
            0x8E => {
                // LDOU $X, $Y, $Z - Load octa unsigned (same as LDO)
                // u($X) <- M8[$Y + $Z]
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let octa = self.read_octa(addr);
                self.set_register(x, octa);
                self.advance_pc();
                true
            }
            0x8F => {
                // LDOU $X, $Y, Z - Load octa unsigned (immediate, same as LDO)
                let addr = self.get_register(y).wrapping_add(z as u64);
                let octa = self.read_octa(addr);
                self.set_register(x, octa);
                self.advance_pc();
                true
            }
            0x90 => {
                // LDSF $X, $Y, $Z - Load short float (32-bit float to 64-bit)
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let tetra = self.read_tetra(addr);
                let short_float = f32::from_bits(tetra);
                let value = short_float as f64;
                self.set_register(x, Self::f64_to_u64(value));
                self.advance_pc();
                true
            }
            0x91 => {
                // LDSFI $X, $Y, Z - Load short float immediate (32-bit float to 64-bit)
                let addr = self.get_register(y).wrapping_add(z as u64);
                let tetra = self.read_tetra(addr);
                let short_float = f32::from_bits(tetra);
                let value = short_float as f64;
                self.set_register(x, Self::f64_to_u64(value));
                self.advance_pc();
                true
            }
            0x22 => {
                // ADDU $X, $Y, $Z
                binop_rr!(self, x, y, z, u64::wrapping_add)
            }
            0x23 => {
                // ADDUI $X, $Y, Z
                binop_ri!(self, x, y, z, u64::wrapping_add)
            }
            // SET instructions
            // SET family instructions - opcodes 0xE0-0xEF
            0xE0 => {
                // SETH $X, YZ - Set high wyde (clears low 48 bits)
                let yz = ((y as u64) << 8) | (z as u64);
                let value = yz << 48;
                self.set_register(x, value);
                self.advance_pc();
                true
            }
            0xE1 => {
                // SETMH $X, YZ - Set medium high wyde (ORs with existing bits)
                let yz = ((y as u64) << 8) | (z as u64);
                let value = yz << 32;
                let current = self.get_register(x);
                self.set_register(x, current | value);
                self.advance_pc();
                true
            }
            0xE2 => {
                // SETML $X, YZ - Set medium low wyde (ORs with existing bits)
                let yz = ((y as u64) << 8) | (z as u64);
                let value = yz << 16;
                let current = self.get_register(x);
                self.set_register(x, current | value);
                self.advance_pc();
                true
            }
            0xE3 => {
                // SETL $X, YZ - Set low wyde (ORs with existing bits)
                let yz = ((y as u64) << 8) | (z as u64);
                let current = self.get_register(x);
                self.set_register(x, current | yz);
                self.advance_pc();
                true
            }
            0xE4 => {
                // INCH $X, YZ - Increase by high wyde
                let yz = ((y as u64) << 8) | (z as u64);
                let value = yz << 48;
                let current = self.get_register(x);
                self.set_register(x, current.wrapping_add(value));
                self.advance_pc();
                true
            }
            0xE5 => {
                // INCMH $X, YZ - Increase by medium high wyde
                let yz = ((y as u64) << 8) | (z as u64);
                let value = yz << 32;
                let current = self.get_register(x);
                self.set_register(x, current.wrapping_add(value));
                self.advance_pc();
                true
            }
            0xE6 => {
                // INCML $X, YZ - Increase by medium low wyde
                let yz = ((y as u64) << 8) | (z as u64);
                let value = yz << 16;
                let current = self.get_register(x);
                self.set_register(x, current.wrapping_add(value));
                self.advance_pc();
                true
            }
            0xE7 => {
                // INCL $X, YZ - Increase by low wyde
                let yz = ((y as u64) << 8) | (z as u64);
                let current = self.get_register(x);
                self.set_register(x, current.wrapping_add(yz));
                self.advance_pc();
                true
            }
            0xE8 => {
                // ORH $X, YZ - OR with high wyde
                let yz = ((y as u64) << 8) | (z as u64);
                let value = yz << 48;
                let current = self.get_register(x);
                self.set_register(x, current | value);
                self.advance_pc();
                true
            }
            0xE9 => {
                // ORMH $X, YZ - OR with medium high wyde
                let yz = ((y as u64) << 8) | (z as u64);
                let value = yz << 32;
                let current = self.get_register(x);
                self.set_register(x, current | value);
                self.advance_pc();
                true
            }
            0xEA => {
                // ORML $X, YZ - OR with medium low wyde
                let yz = ((y as u64) << 8) | (z as u64);
                let value = yz << 16;
                let current = self.get_register(x);
                self.set_register(x, current | value);
                self.advance_pc();
                true
            }
            0xEB => {
                // ORL $X, YZ - OR with low wyde
                let yz = ((y as u64) << 8) | (z as u64);
                let current = self.get_register(x);
                self.set_register(x, current | yz);
                self.advance_pc();
                true
            }
            0xEC => {
                // ANDNH $X, YZ - AND-NOT with high wyde
                let yz = ((y as u64) << 8) | (z as u64);
                let mask = !(yz << 48);
                let current = self.get_register(x);
                self.set_register(x, current & mask);
                self.advance_pc();
                true
            }
            0xED => {
                // ANDNMH $X, YZ - AND-NOT with medium high wyde
                let yz = ((y as u64) << 8) | (z as u64);
                let mask = !(yz << 32);
                let current = self.get_register(x);
                self.set_register(x, current & mask);
                self.advance_pc();
                true
            }
            0xEE => {
                // ANDNML $X, YZ - AND-NOT with medium low wyde
                let yz = ((y as u64) << 8) | (z as u64);
                let mask = !(yz << 16);
                let current = self.get_register(x);
                self.set_register(x, current & mask);
                self.advance_pc();
                true
            }
            0xEF => {
                // ANDNL $X, YZ - AND-NOT with low wyde
                let yz = ((y as u64) << 8) | (z as u64);
                let mask = !yz;
                let current = self.get_register(x);
                self.set_register(x, current & mask);
                self.advance_pc();
                true
            }

            // Special Load/Store instructions (0x92-0x9F)
            0x92 => {
                // LDHT $X, $Y, $Z - Load high tetra
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let tetra = self.read_tetra(addr);
                let value = (tetra as u64) << 32;
                self.set_register(x, value);
                self.advance_pc();
                true
            }
            0x93 => {
                // LDHTI $X, $Y, Z - Load high tetra immediate
                let addr = self.get_register(y).wrapping_add(z as u64);
                let tetra = self.read_tetra(addr);
                let value = (tetra as u64) << 32;
                self.set_register(x, value);
                self.advance_pc();
                true
            }
            0x94 => {
                // CSWAP $X, $Y, $Z - Compare and swap octabytes
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let mem_value = self.read_octa(addr);
                let compare_value = self.get_special(SpecialReg::RP);
                if mem_value == compare_value {
                    // Values match, perform swap
                    self.write_octa(addr, self.get_register(x));
                    self.set_register(x, 1); // Success
                } else {
                    // Values don't match, load current value
                    self.set_register(x, 0); // Failure
                }
                self.advance_pc();
                true
            }
            0x95 => {
                // CSWAPI $X, $Y, Z - Compare and swap octabytes immediate
                let addr = self.get_register(y).wrapping_add(z as u64);
                let mem_value = self.read_octa(addr);
                let compare_value = self.get_special(SpecialReg::RP);
                if mem_value == compare_value {
                    // Values match, perform swap
                    self.write_octa(addr, self.get_register(x));
                    self.set_register(x, 1); // Success
                } else {
                    // Values don't match, load current value
                    self.set_register(x, 0); // Failure
                }
                self.advance_pc();
                true
            }
            0x96 => {
                // LDUNC $X, $Y, $Z - Load uncached (treat as normal load)
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let value = self.read_octa(addr);
                self.set_register(x, value);
                self.advance_pc();
                true
            }
            0x97 => {
                // LDUNCI $X, $Y, Z - Load uncached immediate
                let addr = self.get_register(y).wrapping_add(z as u64);
                let value = self.read_octa(addr);
                self.set_register(x, value);
                self.advance_pc();
                true
            }
            0x98 => {
                // LDVTS $X, $Y, $Z - Load virtual translation status (simplified)
                // In a full implementation, this would interact with virtual memory
                // For now, return 0 (no translation)
                self.set_register(x, 0);
                self.advance_pc();
                true
            }
            0x99 => {
                // LDVTSI $X, $Y, Z - Load virtual translation status immediate
                self.set_register(x, 0);
                self.advance_pc();
                true
            }
            0x9A => {
                // PRELD $X, $Y, $Z - Preload data (hint, no-op in simulation)
                self.advance_pc();
                true
            }
            0x9B => {
                // PRELDI $X, $Y, Z - Preload data immediate (hint, no-op)
                self.advance_pc();
                true
            }
            0x9C => {
                // PREGO $X, $Y, $Z - Preload to go (hint, no-op in simulation)
                self.advance_pc();
                true
            }
            0x9D => {
                // PREGOI $X, $Y, Z - Preload to go immediate (hint, no-op)
                self.advance_pc();
                true
            }
            0x9E => {
                // GO $X, $Y, $Z - Go to location
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                self.set_register(x, self.pc + 4); // Save return address
                self.pc = addr;
                true
            }
            0x9F => {
                // GOI $X, $Y, Z - Go to location immediate
                let addr = self.get_register(y).wrapping_add(z as u64);
                self.set_register(x, self.pc + 4); // Save return address
                self.pc = addr;
                true
            }

            // Store instructions
            0xA0 => {
                // STB $X, $Y, $Z - Store byte (with overflow check)
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let value = self.get_register(x);
                // Check if value fits in signed byte range [-128, 127]
                let signed_value = value as i64;
                if !(-128..=127).contains(&signed_value) {
                    // Set overflow bit in rA (not fully implemented yet)
                    // For now, just store the byte
                }
                self.write_byte(addr, value as u8);
                self.advance_pc();
                true
            }
            0xA1 => {
                // STB $X, $Y, Z - Store byte immediate (with overflow check)
                let addr = self.get_register(y).wrapping_add(z as u64);
                let value = self.get_register(x);
                let signed_value = value as i64;
                if !(-128..=127).contains(&signed_value) {
                    // Set overflow bit in rA
                }
                self.write_byte(addr, value as u8);
                self.advance_pc();
                true
            }
            0xA2 => {
                // STBU $X, $Y, $Z - Store byte unsigned (no overflow check)
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let value = self.get_register(x);
                self.write_byte(addr, value as u8);
                self.advance_pc();
                true
            }
            0xA3 => {
                // STBU $X, $Y, Z - Store byte unsigned immediate
                let addr = self.get_register(y).wrapping_add(z as u64);
                let value = self.get_register(x);
                self.write_byte(addr, value as u8);
                self.advance_pc();
                true
            }
            0xA4 => {
                // STW $X, $Y, $Z - Store wyde (with overflow check)
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let value = self.get_register(x);
                let signed_value = value as i64;
                if !(-32768..=32767).contains(&signed_value) {
                    // Set overflow bit in rA
                }
                self.write_wyde(addr, value as u16);
                self.advance_pc();
                true
            }
            0xA5 => {
                // STW $X, $Y, Z - Store wyde immediate (with overflow check)
                let addr = self.get_register(y).wrapping_add(z as u64);
                let value = self.get_register(x);
                let signed_value = value as i64;
                if !(-32768..=32767).contains(&signed_value) {
                    // Set overflow bit in rA
                }
                self.write_wyde(addr, value as u16);
                self.advance_pc();
                true
            }
            0xA6 => {
                // STWU $X, $Y, $Z - Store wyde unsigned (no overflow check)
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let value = self.get_register(x);
                self.write_wyde(addr, value as u16);
                self.advance_pc();
                true
            }
            0xA7 => {
                // STWU $X, $Y, Z - Store wyde unsigned immediate
                let addr = self.get_register(y).wrapping_add(z as u64);
                let value = self.get_register(x);
                self.write_wyde(addr, value as u16);
                self.advance_pc();
                true
            }
            0xA8 => {
                // STT $X, $Y, $Z - Store tetra (with overflow check)
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let value = self.get_register(x);
                let signed_value = value as i64;
                if !(-2147483648..=2147483647).contains(&signed_value) {
                    // Set overflow bit in rA
                }
                self.write_tetra(addr, value as u32);
                self.advance_pc();
                true
            }
            0xA9 => {
                // STT $X, $Y, Z - Store tetra immediate (with overflow check)
                let addr = self.get_register(y).wrapping_add(z as u64);
                let value = self.get_register(x);
                let signed_value = value as i64;
                if !(-2147483648..=2147483647).contains(&signed_value) {
                    // Set overflow bit in rA
                }
                self.write_tetra(addr, value as u32);
                self.advance_pc();
                true
            }
            0xAA => {
                // STTU $X, $Y, $Z - Store tetra unsigned (no overflow check)
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let value = self.get_register(x);
                self.write_tetra(addr, value as u32);
                self.advance_pc();
                true
            }
            0xAB => {
                // STTU $X, $Y, Z - Store tetra unsigned immediate
                let addr = self.get_register(y).wrapping_add(z as u64);
                let value = self.get_register(x);
                self.write_tetra(addr, value as u32);
                self.advance_pc();
                true
            }
            0xAC => {
                // STO $X, $Y, $Z - Store octa
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let value = self.get_register(x);
                self.write_octa(addr, value);
                self.advance_pc();
                true
            }
            0xAD => {
                // STO $X, $Y, Z - Store octa immediate
                let addr = self.get_register(y).wrapping_add(z as u64);
                let value = self.get_register(x);
                self.write_octa(addr, value);
                self.advance_pc();
                true
            }
            0xAE => {
                // STOU $X, $Y, $Z - Store octa unsigned (same as STO)
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let value = self.get_register(x);
                self.write_octa(addr, value);
                self.advance_pc();
                true
            }
            0xAF => {
                // STOU $X, $Y, Z - Store octa unsigned immediate
                let addr = self.get_register(y).wrapping_add(z as u64);
                let value = self.get_register(x);
                self.write_octa(addr, value);
                self.advance_pc();
                true
            }
            0xB0 => {
                // STSF $X, $Y, $Z - Store short float (32-bit float from 64-bit)
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let value = Self::u64_to_f64(self.get_register(x));
                let short_float = value as f32;
                let tetra = short_float.to_bits();
                self.write_tetra(addr, tetra);
                self.advance_pc();
                true
            }
            0xB1 => {
                // STSFI $X, $Y, Z - Store short float immediate
                let addr = self.get_register(y).wrapping_add(z as u64);
                let value = Self::u64_to_f64(self.get_register(x));
                let short_float = value as f32;
                let tetra = short_float.to_bits();
                self.write_tetra(addr, tetra);
                self.advance_pc();
                true
            }
            0xB2 => {
                // STHT $X, $Y, $Z - Store high tetra
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let value = self.get_register(x);
                let high_tetra = (value >> 32) as u32;
                self.write_tetra(addr, high_tetra);
                self.advance_pc();
                true
            }
            0xB3 => {
                // STHTI $X, $Y, Z - Store high tetra immediate
                let addr = self.get_register(y).wrapping_add(z as u64);
                let value = self.get_register(x);
                let high_tetra = (value >> 32) as u32;
                self.write_tetra(addr, high_tetra);
                self.advance_pc();
                true
            }
            0xB4 => {
                // STCO X, $Y, $Z - Store constant octabyte (X is immediate value)
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                self.write_octa(addr, x as u64);
                self.advance_pc();
                true
            }
            0xB5 => {
                // STCOI X, $Y, Z - Store constant octabyte immediate
                let addr = self.get_register(y).wrapping_add(z as u64);
                self.write_octa(addr, x as u64);
                self.advance_pc();
                true
            }
            0xB6 => {
                // STUNC $X, $Y, $Z - Store uncached (treat as normal store)
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let value = self.get_register(x);
                self.write_octa(addr, value);
                self.advance_pc();
                true
            }
            0xB7 => {
                // STUNCI $X, $Y, Z - Store uncached immediate
                let addr = self.get_register(y).wrapping_add(z as u64);
                let value = self.get_register(x);
                self.write_octa(addr, value);
                self.advance_pc();
                true
            }
            0xB8 => {
                // SYNCD X, $Y, $Z - Synchronize data (no-op in simulation)
                self.advance_pc();
                true
            }
            0xB9 => {
                // SYNCDI X, $Y, Z - Synchronize data immediate (no-op)
                self.advance_pc();
                true
            }
            0xBA => {
                // PREST X, $Y, $Z - Prestore (hint, no-op in simulation)
                self.advance_pc();
                true
            }
            0xBB => {
                // PRESTI X, $Y, Z - Prestore immediate (hint, no-op)
                self.advance_pc();
                true
            }
            0xBC => {
                // SYNCID X, $Y, $Z - Synchronize instruction data (no-op in simulation)
                self.advance_pc();
                true
            }
            0xBD => {
                // SYNCIDI X, $Y, Z - Synchronize instruction data immediate (no-op)
                self.advance_pc();
                true
            }
            0xBE => {
                // PUSHGO $X, $Y, $Z - Push registers and go
                // Push registers onto register stack and jump
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                // Save return address in rJ
                self.set_special(SpecialReg::RJ, self.get_pc() + 4);
                // Set $X to the pushed count (simplified implementation)
                self.set_register(x, 0);
                // Jump to computed address
                self.set_pc(addr);
                true
            }
            0xBF => {
                // PUSHGOI $X, $Y, Z - Push registers and go immediate
                let addr = self.get_register(y).wrapping_add(z as u64);
                self.set_special(SpecialReg::RJ, self.get_pc() + 4);
                self.set_register(x, 0);
                self.set_pc(addr);
                true
            }
            // Arithmetic instructions (§9) - MUL/DIV opcodes 0x18-0x1F
            0x18 => {
                // MUL $X, $Y, $Z - Multiply signed with overflow
                mul_rr!(self, x, y, z)
            }
            0x19 => {
                // MULI $X, $Y, Z - Multiply signed immediate with overflow
                mul_ri!(self, x, y, z)
            }
            0x1A => {
                // MULU $X, $Y, $Z - Multiply unsigned
                mulu_rr!(self, x, y, z)
            }
            0x1B => {
                // MULUI $X, $Y, Z - Multiply unsigned immediate
                mulu_ri!(self, x, y, z)
            }
            0x1C => {
                // DIV $X, $Y, $Z - Divide signed
                div_rr!(self, x, y, z)
            }
            0x1D => {
                // DIVI $X, $Y, Z - Divide signed immediate
                div_ri!(self, x, y, z)
            }
            0x1E => {
                // DIVU $X, $Y, $Z - Divide unsigned
                divu_rr!(self, x, y, z)
            }
            0x1F => {
                // DIVUI $X, $Y, Z - Divide unsigned immediate
                divu_ri!(self, x, y, z)
            }
            // ADD/SUB and variants - opcodes 0x20-0x2F
            0x20 => {
                // ADD $X, $Y, $Z - Add signed with overflow check
                add_rr!(self, x, y, z)
            }
            0x21 => {
                // ADDI $X, $Y, Z - Add signed immediate with overflow check
                add_ri!(self, x, y, z)
            }
            // 0x22 and 0x23 are ADDU/ADDUI, already implemented above
            0x24 => {
                // SUB $X, $Y, $Z - Subtract signed with overflow check
                sub_rr!(self, x, y, z)
            }
            0x25 => {
                // SUBI $X, $Y, Z - Subtract signed immediate with overflow check
                sub_ri!(self, x, y, z)
            }
            0x26 => {
                // SUBU $X, $Y, $Z
                binop_rr!(self, x, y, z, u64::wrapping_sub)
            }
            0x27 => {
                // SUBUI $X, $Y, Z
                binop_ri!(self, x, y, z, u64::wrapping_sub)
            }
            0x28 => {
                // 2ADDU $X, $Y, $Z
                muladd_rr!(self, x, y, z, 2)
            }
            0x29 => {
                // 2ADDUI $X, $Y, Z
                muladd_ri!(self, x, y, z, 2)
            }
            0x2A => {
                // 4ADDU $X, $Y, $Z
                muladd_rr!(self, x, y, z, 4)
            }
            0x2B => {
                // 4ADDUI $X, $Y, Z
                muladd_ri!(self, x, y, z, 4)
            }
            0x2C => {
                // 8ADDU $X, $Y, $Z
                muladd_rr!(self, x, y, z, 8)
            }
            0x2D => {
                // 8ADDUI $X, $Y, Z
                muladd_ri!(self, x, y, z, 8)
            }
            0x2E => {
                // 16ADDU $X, $Y, $Z
                muladd_rr!(self, x, y, z, 16)
            }
            0x2F => {
                // 16ADDUI $X, $Y, Z
                muladd_ri!(self, x, y, z, 16)
            }
            // CMP instructions - opcodes 0x30-0x33
            0x30 => {
                // CMP $X, $Y, $Z
                cmp_rr!(self, x, y, z, |v| v as i64)
            }
            0x31 => {
                // CMPI $X, $Y, Z
                cmp_ri!(self, x, y, z, |v| v as i64, |v| v as i64)
            }
            0x32 => {
                // CMPU $X, $Y, $Z
                cmp_rr!(self, x, y, z, |v| v)
            }
            0x33 => {
                // CMPUI $X, $Y, Z
                cmp_ri!(self, x, y, z, |v| v, |v| v as u64)
            }
            0x34 => {
                // NEG $X, Y, $Z - Negate with overflow check
                // Y is immediate constant, $Z is register
                let a = y as i64;
                let b = self.get_register(z) as i64;
                match a.checked_sub(b) {
                    Some(result) => {
                        self.set_register(x, result as u64);
                    }
                    None => {
                        // Overflow occurred (e.g., 0 - (-2^63))
                        self.set_register(x, a.wrapping_sub(b) as u64);
                    }
                }
                self.advance_pc();
                true
            }
            0x35 => {
                // NEG $X, Y, Z - Negate immediate with overflow check
                // Both Y and Z are immediate constants
                let a = y as i64;
                let b = z as i64;
                match a.checked_sub(b) {
                    Some(result) => {
                        self.set_register(x, result as u64);
                    }
                    None => {
                        // Overflow occurred
                        self.set_register(x, a.wrapping_sub(b) as u64);
                    }
                }
                self.advance_pc();
                true
            }
            0x36 => {
                // NEGU $X, Y, $Z - Negate unsigned
                let a = y as u64;
                let b = self.get_register(z);
                self.set_register(x, a.wrapping_sub(b));
                self.advance_pc();
                true
            }
            0x37 => {
                // NEGU $X, Y, Z - Negate unsigned immediate
                let a = y as u64;
                let b = z as u64;
                self.set_register(x, a.wrapping_sub(b));
                self.advance_pc();
                true
            }
            // Shift instructions (§14) - opcodes 0x38-0x3F
            0x38 => {
                // SL $X, $Y, $Z - Shift left with overflow check
                let val_y = self.get_register(y) as i64;
                let shift = self.get_register(z);
                if shift >= 64 {
                    // Shift by 64 or more: result is 0, overflow unless Y was 0
                    if val_y != 0 {
                        self.set_special(SpecialReg::RA, self.get_special(SpecialReg::RA) | 0x04); // Integer overflow
                    }
                    self.set_register(x, 0);
                } else {
                    let result = (val_y as u64) << shift;
                    // Check for overflow: did we shift out any non-sign bits?
                    let sign_bit = val_y < 0;
                    let expected_high = if sign_bit { !0u64 } else { 0 };
                    let actual_high = result >> (64 - shift);
                    let mask = (1u64 << shift) - 1;
                    if shift > 0 && (actual_high & mask) != (expected_high & mask) {
                        self.set_special(SpecialReg::RA, self.get_special(SpecialReg::RA) | 0x04);
                    }
                    self.set_register(x, result);
                }
                self.advance_pc();
                true
            }
            0x39 => {
                // SLI $X, $Y, Z - Shift left immediate with overflow check
                let val_y = self.get_register(y) as i64;
                let shift = z as u64;
                if shift >= 64 {
                    if val_y != 0 {
                        self.set_special(SpecialReg::RA, self.get_special(SpecialReg::RA) | 0x04);
                    }
                    self.set_register(x, 0);
                } else {
                    let result = (val_y as u64) << shift;
                    let sign_bit = val_y < 0;
                    let expected_high = if sign_bit { !0u64 } else { 0 };
                    let actual_high = result >> (64 - shift);
                    let mask = (1u64 << shift) - 1;
                    if shift > 0 && (actual_high & mask) != (expected_high & mask) {
                        self.set_special(SpecialReg::RA, self.get_special(SpecialReg::RA) | 0x04);
                    }
                    self.set_register(x, result);
                }
                self.advance_pc();
                true
            }
            0x3A => {
                // SLU $X, $Y, $Z - Shift left unsigned (no overflow check)
                let val_y = self.get_register(y);
                let shift = self.get_register(z);
                let result = if shift >= 64 { 0 } else { val_y << shift };
                self.set_register(x, result);
                self.advance_pc();
                true
            }
            0x3B => {
                // SLUI $X, $Y, Z - Shift left unsigned immediate (no overflow check)
                let val_y = self.get_register(y);
                let shift = z as u64;
                let result = if shift >= 64 { 0 } else { val_y << shift };
                self.set_register(x, result);
                self.advance_pc();
                true
            }
            0x3C => {
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
            0x3D => {
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
            0x3E => {
                // SRU $X, $Y, $Z - Shift right unsigned (logical)
                let val_y = self.get_register(y);
                let shift = self.get_register(z);
                let result = if shift >= 64 { 0 } else { val_y >> shift };
                self.set_register(x, result);
                self.advance_pc();
                true
            }
            0x3F => {
                // SRUI $X, $Y, Z - Shift right unsigned immediate (logical)
                let val_y = self.get_register(y);
                let shift = z as u64;
                let result = if shift >= 64 { 0 } else { val_y >> shift };
                self.set_register(x, result);
                self.advance_pc();
                true
            }
            // Branch instructions (§15) - opcodes 0x40-0x5F
            0x40 => {
                // BN $X, $Y, Z - Branch if negative
                let cond = (self.get_register(x) as i64) < 0;
                self.branch_forward(cond, y, z);
                true
            }
            0x41 => {
                // BNB $X, $Y, Z - Branch if negative (backward)
                let cond = (self.get_register(x) as i64) < 0;
                self.branch_backward(cond, y, z);
                true
            }
            0x42 => {
                // BZ $X, $Y, Z - Branch if zero
                let cond = self.get_register(x) == 0;
                self.branch_forward(cond, y, z);
                true
            }
            0x43 => {
                // BZB $X, $Y, Z - Branch if zero (backward)
                let cond = self.get_register(x) == 0;
                self.branch_backward(cond, y, z);
                true
            }
            0x44 => {
                // BP $X, $Y, Z - Branch if positive
                let cond = (self.get_register(x) as i64) > 0;
                self.branch_forward(cond, y, z);
                true
            }
            0x45 => {
                // BPB $X, $Y, Z - Branch if positive (backward)
                let cond = (self.get_register(x) as i64) > 0;
                self.branch_backward(cond, y, z);
                true
            }
            0x46 => {
                // BOD $X, $Y, Z - Branch if odd
                let cond = (self.get_register(x) & 1) != 0;
                self.branch_forward(cond, y, z);
                true
            }
            0x47 => {
                // BODB $X, $Y, Z - Branch if odd (backward)
                let cond = (self.get_register(x) & 1) != 0;
                self.branch_backward(cond, y, z);
                true
            }
            0x48 => {
                // BNN $X, $Y, Z - Branch if non-negative
                let cond = (self.get_register(x) as i64) >= 0;
                self.branch_forward(cond, y, z);
                true
            }
            0x49 => {
                // BNNB $X, $Y, Z - Branch if non-negative (backward)
                let cond = (self.get_register(x) as i64) >= 0;
                self.branch_backward(cond, y, z);
                true
            }
            0x4A => {
                // BNZ $X, $Y, Z - Branch if non-zero
                let cond = self.get_register(x) != 0;
                self.branch_forward(cond, y, z);
                true
            }
            0x4B => {
                // BNZB $X, $Y, Z - Branch if non-zero (backward)
                let cond = self.get_register(x) != 0;
                self.branch_backward(cond, y, z);
                true
            }
            0x4C => {
                // BNP $X, $Y, Z - Branch if non-positive
                let cond = (self.get_register(x) as i64) <= 0;
                self.branch_forward(cond, y, z);
                true
            }
            0x4D => {
                // BNPB $X, $Y, Z - Branch if non-positive (backward)
                let cond = (self.get_register(x) as i64) <= 0;
                self.branch_backward(cond, y, z);
                true
            }
            0x4E => {
                // BEV $X, $Y, Z - Branch if even
                let cond = (self.get_register(x) & 1) == 0;
                self.branch_forward(cond, y, z);
                true
            }
            0x4F => {
                // BEVB $X, $Y, Z - Branch if even (backward)
                let cond = (self.get_register(x) & 1) == 0;
                self.branch_backward(cond, y, z);
                true
            }
            0x50 => {
                // PBN $X, $Y, Z - Probable branch if negative
                let cond = (self.get_register(x) as i64) < 0;
                self.branch_forward(cond, y, z);
                true
            }
            0x51 => {
                // PBNB $X, $Y, Z - Probable branch if negative (backward)
                let cond = (self.get_register(x) as i64) < 0;
                self.branch_backward(cond, y, z);
                true
            }
            0x52 => {
                // PBZ $X, $Y, Z - Probable branch if zero
                let cond = self.get_register(x) == 0;
                self.branch_forward(cond, y, z);
                true
            }
            0x53 => {
                // PBZB $X, $Y, Z - Probable branch if zero (backward)
                let cond = self.get_register(x) == 0;
                self.branch_backward(cond, y, z);
                true
            }
            0x54 => {
                // PBP $X, $Y, Z - Probable branch if positive
                let cond = (self.get_register(x) as i64) > 0;
                self.branch_forward(cond, y, z);
                true
            }
            0x55 => {
                // PBPB $X, $Y, Z - Probable branch if positive (backward)
                let cond = (self.get_register(x) as i64) > 0;
                self.branch_backward(cond, y, z);
                true
            }
            0x56 => {
                // PBOD $X, $Y, Z - Probable branch if odd
                let cond = (self.get_register(x) & 1) != 0;
                self.branch_forward(cond, y, z);
                true
            }
            0x57 => {
                // PBODB $X, $Y, Z - Probable branch if odd (backward)
                let cond = (self.get_register(x) & 1) != 0;
                self.branch_backward(cond, y, z);
                true
            }
            0x58 => {
                // PBNN $X, $Y, Z - Probable branch if non-negative
                let cond = (self.get_register(x) as i64) >= 0;
                self.branch_forward(cond, y, z);
                true
            }
            0x59 => {
                // PBNNB $X, $Y, Z - Probable branch if non-negative (backward)
                let cond = (self.get_register(x) as i64) >= 0;
                self.branch_backward(cond, y, z);
                true
            }
            0x5A => {
                // PBNZ $X, $Y, Z - Probable branch if non-zero
                let cond = self.get_register(x) != 0;
                self.branch_forward(cond, y, z);
                true
            }
            0x5B => {
                // PBNZB $X, $Y, Z - Probable branch if non-zero (backward)
                let cond = self.get_register(x) != 0;
                self.branch_backward(cond, y, z);
                true
            }
            0x5C => {
                // PBNP $X, $Y, Z - Probable branch if non-positive
                let cond = (self.get_register(x) as i64) <= 0;
                self.branch_forward(cond, y, z);
                true
            }
            0x5D => {
                // PBNPB $X, $Y, Z - Probable branch if non-positive (backward)
                let cond = (self.get_register(x) as i64) <= 0;
                self.branch_backward(cond, y, z);
                true
            }
            0x5E => {
                // PBEV $X, $Y, Z - Probable branch if even
                let cond = (self.get_register(x) & 1) == 0;
                self.branch_forward(cond, y, z);
                true
            }
            0x5F => {
                // PBEVB $X, $Y, Z - Probable branch if even (backward)
                let cond = (self.get_register(x) & 1) == 0;
                self.branch_backward(cond, y, z);
                true
            }
            // Conditional Set instructions (§16) - opcodes 0x60-0x6F
            0x60 => {
                // CSN $X, $Y, $Z - Conditional Set if Negative
                let cond = (self.get_register(x) as i64) < 0;
                self.cond_set_rr(x, y, z, cond);
                true
            }
            0x61 => {
                // CSNI $X, $Y, Z - Conditional Set if Negative (immediate)
                let cond = (self.get_register(x) as i64) < 0;
                self.cond_set_ri(x, y, z, cond);
                true
            }
            0x62 => {
                // CSZ $X, $Y, $Z - Conditional Set if Zero
                let cond = self.get_register(x) == 0;
                self.cond_set_rr(x, y, z, cond);
                true
            }
            0x63 => {
                // CSZI $X, $Y, Z - Conditional Set if Zero (immediate)
                let cond = self.get_register(x) == 0;
                self.cond_set_ri(x, y, z, cond);
                true
            }
            0x64 => {
                // CSP $X, $Y, $Z - Conditional Set if Positive
                let cond = (self.get_register(x) as i64) > 0;
                self.cond_set_rr(x, y, z, cond);
                true
            }
            0x65 => {
                // CSPI $X, $Y, Z - Conditional Set if Positive (immediate)
                let cond = (self.get_register(x) as i64) > 0;
                self.cond_set_ri(x, y, z, cond);
                true
            }
            0x66 => {
                // CSOD $X, $Y, $Z - Conditional Set if Odd
                let cond = (self.get_register(x) & 1) != 0;
                self.cond_set_rr(x, y, z, cond);
                true
            }
            0x67 => {
                // CSODI $X, $Y, Z - Conditional Set if Odd (immediate)
                let cond = (self.get_register(x) & 1) != 0;
                self.cond_set_ri(x, y, z, cond);
                true
            }
            0x68 => {
                // CSNN $X, $Y, $Z - Conditional Set if Non-Negative
                let cond = (self.get_register(x) as i64) >= 0;
                self.cond_set_rr(x, y, z, cond);
                true
            }
            0x69 => {
                // CSNNI $X, $Y, Z - Conditional Set if Non-Negative (immediate)
                let cond = (self.get_register(x) as i64) >= 0;
                self.cond_set_ri(x, y, z, cond);
                true
            }
            0x6A => {
                // CSNZ $X, $Y, $Z - Conditional Set if Non-Zero
                let cond = self.get_register(x) != 0;
                self.cond_set_rr(x, y, z, cond);
                true
            }
            0x6B => {
                // CSNZI $X, $Y, Z - Conditional Set if Non-Zero (immediate)
                let cond = self.get_register(x) != 0;
                self.cond_set_ri(x, y, z, cond);
                true
            }
            0x6C => {
                // CSNP $X, $Y, $Z - Conditional Set if Non-Positive
                let cond = (self.get_register(x) as i64) <= 0;
                self.cond_set_rr(x, y, z, cond);
                true
            }
            0x6D => {
                // CSNPI $X, $Y, Z - Conditional Set if Non-Positive (immediate)
                let cond = (self.get_register(x) as i64) <= 0;
                self.cond_set_ri(x, y, z, cond);
                true
            }
            0x6E => {
                // CSEV $X, $Y, $Z - Conditional Set if Even
                let cond = (self.get_register(x) & 1) == 0;
                self.cond_set_rr(x, y, z, cond);
                true
            }
            0x6F => {
                // CSEVI $X, $Y, Z - Conditional Set if Even (immediate)
                let cond = (self.get_register(x) & 1) == 0;
                self.cond_set_ri(x, y, z, cond);
                true
            }

            // Zero or Set instructions (0x70-0x7F)
            0x70 => {
                // ZSN $X, $Y, $Z - Zero or Set if Negative
                let cond = (self.get_register(x) as i64) < 0;
                self.zero_set_rr(x, y, z, cond);
                true
            }
            0x71 => {
                // ZSNI $X, $Y, Z - Zero or Set if Negative (immediate)
                let cond = (self.get_register(x) as i64) < 0;
                self.zero_set_ri(x, y, z, cond);
                true
            }
            0x72 => {
                // ZSZ $X, $Y, $Z - Zero or Set if Zero
                let cond = self.get_register(x) == 0;
                self.zero_set_rr(x, y, z, cond);
                true
            }
            0x73 => {
                // ZSZI $X, $Y, Z - Zero or Set if Zero (immediate)
                let cond = self.get_register(x) == 0;
                self.zero_set_ri(x, y, z, cond);
                true
            }
            0x74 => {
                // ZSP $X, $Y, $Z - Zero or Set if Positive
                let cond = self.get_register(x) != 0 && (self.get_register(x) as i64) > 0;
                self.zero_set_rr(x, y, z, cond);
                true
            }
            0x75 => {
                // ZSPI $X, $Y, Z - Zero or Set if Positive (immediate)
                let cond = self.get_register(x) != 0 && (self.get_register(x) as i64) > 0;
                self.zero_set_ri(x, y, z, cond);
                true
            }
            0x76 => {
                // ZSOD $X, $Y, $Z - Zero or Set if Odd
                let cond = (self.get_register(x) & 1) != 0;
                self.zero_set_rr(x, y, z, cond);
                true
            }
            0x77 => {
                // ZSODI $X, $Y, Z - Zero or Set if Odd (immediate)
                let cond = (self.get_register(x) & 1) != 0;
                self.zero_set_ri(x, y, z, cond);
                true
            }
            0x78 => {
                // ZSNN $X, $Y, $Z - Zero or Set if Non-Negative
                let cond = (self.get_register(x) as i64) >= 0;
                self.zero_set_rr(x, y, z, cond);
                true
            }
            0x79 => {
                // ZSNNI $X, $Y, Z - Zero or Set if Non-Negative (immediate)
                let cond = (self.get_register(x) as i64) >= 0;
                self.zero_set_ri(x, y, z, cond);
                true
            }
            0x7A => {
                // ZSNZ $X, $Y, $Z - Zero or Set if Non-Zero
                let cond = self.get_register(x) != 0;
                self.zero_set_rr(x, y, z, cond);
                true
            }
            0x7B => {
                // ZSNZI $X, $Y, Z - Zero or Set if Non-Zero (immediate)
                let cond = self.get_register(x) != 0;
                self.zero_set_ri(x, y, z, cond);
                true
            }
            0x7C => {
                // ZSNP $X, $Y, $Z - Zero or Set if Non-Positive
                let cond = (self.get_register(x) as i64) <= 0;
                self.zero_set_rr(x, y, z, cond);
                true
            }
            0x7D => {
                // ZSNPI $X, $Y, Z - Zero or Set if Non-Positive (immediate)
                let cond = (self.get_register(x) as i64) <= 0;
                self.zero_set_ri(x, y, z, cond);
                true
            }
            0x7E => {
                // ZSEV $X, $Y, $Z - Zero or Set if Even
                let cond = (self.get_register(x) & 1) == 0;
                self.zero_set_rr(x, y, z, cond);
                true
            }
            0x7F => {
                // ZSEVI $X, $Y, Z - Zero or Set if Even (immediate)
                let cond = (self.get_register(x) & 1) == 0;
                self.zero_set_ri(x, y, z, cond);
                true
            }

            // Bitwise operations (§10) - opcodes 0xC0-0xCF, 0xD8-0xD9
            0xC0 => {
                // OR $X, $Y, $Z
                binop_rr!(self, x, y, z, |a, b| a | b)
            }
            0xC1 => {
                // ORI $X, $Y, Z
                binop_ri!(self, x, y, z, |a, b| a | b)
            }
            0xC2 => {
                // ORN $X, $Y, $Z
                binop_rr!(self, x, y, z, |a: u64, b: u64| a | !b)
            }
            0xC3 => {
                // ORNI $X, $Y, Z
                binop_ri!(self, x, y, z, |a: u64, b: u64| a | !b)
            }
            0xC4 => {
                // NOR $X, $Y, $Z
                binop_rr!(self, x, y, z, |a: u64, b: u64| !(a | b))
            }
            0xC5 => {
                // NORI $X, $Y, Z
                binop_ri!(self, x, y, z, |a: u64, b: u64| !(a | b))
            }
            0xC6 => {
                // XOR $X, $Y, $Z
                binop_rr!(self, x, y, z, |a, b| a ^ b)
            }
            0xC7 => {
                // XORI $X, $Y, Z
                binop_ri!(self, x, y, z, |a, b| a ^ b)
            }
            0xC8 => {
                // AND $X, $Y, $Z
                binop_rr!(self, x, y, z, |a, b| a & b)
            }
            0xC9 => {
                // ANDI $X, $Y, Z
                binop_ri!(self, x, y, z, |a, b| a & b)
            }
            0xCA => {
                // ANDN $X, $Y, $Z
                binop_rr!(self, x, y, z, |a: u64, b: u64| a & !b)
            }
            0xCB => {
                // ANDNI $X, $Y, Z
                binop_ri!(self, x, y, z, |a: u64, b: u64| a & !b)
            }
            0xCC => {
                // NAND $X, $Y, $Z
                binop_rr!(self, x, y, z, |a: u64, b: u64| !(a & b))
            }
            0xCD => {
                // NANDI $X, $Y, Z
                binop_ri!(self, x, y, z, |a: u64, b: u64| !(a & b))
            }
            0xCE => {
                // NXOR $X, $Y, $Z
                binop_rr!(self, x, y, z, |a: u64, b: u64| !(a ^ b))
            }
            0xCF => {
                // NXORI $X, $Y, Z
                binop_ri!(self, x, y, z, |a: u64, b: u64| !(a ^ b))
            }
            // Bit fiddling operations (§11-12) - opcodes 0xD0-0xDF
            0xD0 => {
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
            0xD1 => {
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
            0xD2 => {
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
            0xD3 => {
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
            0xD4 => {
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
            0xD5 => {
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
            0xD6 => {
                // ODIF $X, $Y, $Z
                binop_rr!(self, x, y, z, u64::saturating_sub)
            }
            0xD7 => {
                // ODIFI $X, $Y, Z
                binop_ri!(self, x, y, z, u64::saturating_sub)
            }
            0xDA => {
                // SADD $X, $Y, $Z
                binop_rr!(self, x, y, z, |a: u64, b: u64| (a & !b).count_ones() as u64)
            }
            0xDB => {
                // SADDI $X, $Y, Z
                binop_ri!(self, x, y, z, |a: u64, b: u64| (a & !b).count_ones() as u64)
            }
            0xDC => {
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
            0xDD => {
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
            0xDE => {
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
            0xDF => {
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
            0xD8 => {
                // MUX $X, $Y, $Z - Bitwise multiplex
                let val_y = self.get_register(y);
                let val_z = self.get_register(z);
                let mask = self.special_regs[SpecialReg::RM as usize];
                self.set_register(x, (val_y & mask) | (val_z & !mask));
                self.advance_pc();
                true
            }
            0xD9 => {
                // MUXI $X, $Y, Z - Bitwise multiplex immediate
                let val_y = self.get_register(y);
                let mask = self.special_regs[SpecialReg::RM as usize];
                self.set_register(x, (val_y & mask) | ((z as u64) & !mask));
                self.advance_pc();
                true
            }
            // Jump/Stack/System instructions (§17-19) - opcodes 0xF0-0xFF
            0xF0 => {
                // JMP XYZ - Jump (unconditional)
                let offset = ((x as u32) << 16) | ((y as u32) << 8) | (z as u32);
                let signed_offset = if offset & 0x800000 != 0 {
                    // Sign extend from 24 bits
                    (offset | 0xFF000000) as i32
                } else {
                    offset as i32
                };
                self.pc = self.pc.wrapping_add((signed_offset as i64 * 4) as u64);
                true
            }
            0xF1 => {
                // JMPB XYZ - Jump backward
                let offset = ((x as u32) << 16) | ((y as u32) << 8) | (z as u32);
                self.pc = self.pc.wrapping_sub((offset as u64) * 4);
                true
            }
            0xF2 => {
                // PUSHJ $X, YZ - Push registers and jump
                // Save return address in register rJ
                self.set_special(SpecialReg::RJ, self.pc.wrapping_add(4));
                // Jump to relative address
                let offset = ((y as u16) << 8 | z as u16) as i16;
                self.pc = self.pc.wrapping_add((offset as i64 * 4) as u64);
                // Note: Full implementation should also save local registers
                true
            }
            0xF3 => {
                // PUSHJB $X, YZ - Push registers and jump backward
                self.set_special(SpecialReg::RJ, self.pc.wrapping_add(4));
                let offset = (y as u16) << 8 | z as u16;
                self.pc = self.pc.wrapping_sub((offset as u64) * 4);
                true
            }
            0xF4 => {
                // GETA $X, YZ - Get address relative to PC+4
                let offset = ((y as u16) << 8 | z as u16) as i16;
                let addr = (self.pc + 4).wrapping_add((offset as i64 * 4) as u64);
                self.set_register(x, addr);
                self.advance_pc();
                true
            }
            0xF5 => {
                // GETAB $X, YZ - Get address backward relative to PC+4
                let offset = (y as u16) << 8 | z as u16;
                let addr = (self.pc + 4).wrapping_sub((offset as u64) * 4);
                self.set_register(x, addr);
                self.advance_pc();
                true
            }
            0xF6 => {
                // PUT rX, $Z - Put to special register
                // X field specifies the special register, Z specifies source register
                let special_reg_num = x;
                let value = self.get_register(z);
                // Map register number to SpecialReg enum
                if let Some(special_reg) = SpecialReg::from_u8(special_reg_num) {
                    self.set_special(special_reg, value);
                }
                self.advance_pc();
                true
            }
            0xF7 => {
                // PUTI $X, YZ - Put to special register (immediate)
                let special_reg_num = x;
                let value = ((y as u64) << 8) | (z as u64);
                if let Some(special_reg) = SpecialReg::from_u8(special_reg_num) {
                    self.set_special(special_reg, value);
                }
                self.advance_pc();
                true
            }
            0xF8 => {
                // POP X, YZ - Pop registers and return
                // Return to address in rJ
                self.pc = self.get_special(SpecialReg::RJ);
                // Note: Full implementation should restore local registers
                true
            }
            0xF9 => {
                // RESUME - Resume after interrupt
                // This is a complex instruction that would restore full processor state
                // For now, just continue execution
                self.advance_pc();
                true
            }
            0xFA => {
                // SAVE $X,Z - Save process state
                // Saves local registers and special registers to memory
                // Returns address of saved context in $X

                // Allocate memory for context (256 general registers + 32 special registers)
                // Each register is 8 bytes (octa)
                let context_size = (256 + 32) * 8;

                // For simplicity, allocate context at a fixed high address
                // In a real implementation, this would use a stack or memory allocator
                use std::sync::atomic::{AtomicU64, Ordering};
                static CONTEXT_COUNTER: AtomicU64 = AtomicU64::new(0x8000000000000000);
                let context_addr = CONTEXT_COUNTER.fetch_add(context_size, Ordering::Relaxed);

                // Save all 256 general registers
                for i in 0..256 {
                    let value = self.get_register(i as u8);
                    self.write_octa(context_addr + (i * 8), value);
                }

                // Save special registers
                for i in 0..32 {
                    let value = self.special_regs[i];
                    self.write_octa(context_addr + (256 * 8) + (i as u64 * 8), value);
                }

                // Return context address in $X
                self.set_register(x, context_addr);
                self.advance_pc();
                true
            }
            0xFB => {
                // UNSAVE X,$Z - Restore process state
                // Restores local registers and special registers from memory
                // NOTE: Does NOT restore rJ (return address) - that's managed by PUSHJ/POP
                let context_addr = self.get_register(z);

                // Save current rJ before restoring
                let saved_rj = self.get_special(SpecialReg::RJ);

                // Restore all 256 general registers
                for i in 0..256 {
                    let value = self.read_octa(context_addr + (i * 8));
                    self.set_register(i as u8, value);
                }

                // Restore special registers (excluding rJ)
                for i in 0..32 {
                    if i != SpecialReg::RJ as usize {
                        let value = self.read_octa(context_addr + (256 * 8) + (i as u64 * 8));
                        self.special_regs[i] = value;
                    }
                }

                // Restore rJ
                self.set_special(SpecialReg::RJ, saved_rj);

                self.advance_pc();
                true
            }
            0xFC => {
                // SYNC XYZ - Synchronize
                // Memory synchronization barrier
                // For a simulator, this is typically a no-op
                self.advance_pc();
                true
            }
            0xFD => {
                // SWYM XYZ - Sympathize with your machinery (no-op)
                self.advance_pc();
                true
            }
            0xFE => {
                // GET $X, $Z - Get from special register
                let special_reg_num = z;
                if let Some(special_reg) = SpecialReg::from_u8(special_reg_num) {
                    let value = self.get_special(special_reg);
                    self.set_register(x, value);
                }
                self.advance_pc();
                true
            }
            0xFF => {
                // TRIP XYZ - Software interrupt
                // For now, just halt
                eprintln!("TRIP instruction at PC={:#018x}", self.pc);
                false
            }
        }
    }

    /// Execute instructions starting from the current PC until a halt condition.
    /// Returns the number of instructions executed.
    #[instrument(skip(self))]
    pub fn run(&mut self) -> usize {
        debug!("Starting MMIX execution");
        let mut count = 0;
        while self.execute_instruction() {
            count += 1;
            // Safety limit to prevent infinite loops during development
            if count >= 10000 {
                eprintln!("Warning: Execution limit reached (10000 instructions)");
                break;
            }
        }
        debug!(instruction_count = count, "Execution completed");
        count
    }
}

impl fmt::Display for MMix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "MMIX Computer State:")?;
        writeln!(f, "  PC = {:#018x}", self.pc)?;
        writeln!(f)?;

        // Display non-zero general registers
        writeln!(f, "General Registers:")?;
        let mut any_nonzero = false;
        for (i, &value) in self.general_regs.iter().enumerate() {
            if value != 0 && i != 255 {
                writeln!(f, "  ${:<3} = {:#018x} ({})", i, value, value)?;
                any_nonzero = true;
            }
        }
        if !any_nonzero {
            writeln!(f, "  (all zero)")?;
        }
        writeln!(f)?;

        // Display non-zero special registers
        writeln!(f, "Special Registers:")?;
        let special_names = [
            "rA", "rB", "rC", "rD", "rE", "rF", "rG", "rH", "rI", "rJ", "rK", "rL", "rM", "rN",
            "rO", "rP", "rQ", "rR", "rS", "rT", "rU", "rV", "rW", "rX", "rY", "rZ", "rBB", "rTT",
            "rWW", "rXX", "rYY", "rZZ",
        ];
        any_nonzero = false;
        for (i, &value) in self.special_regs.iter().enumerate() {
            if value != 0 {
                writeln!(f, "  {:<4} = {:#018x} ({})", special_names[i], value, value)?;
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

    #[test]
    fn test_mmix_new() {
        let mmix = MMix::new();
        assert_eq!(mmix.get_register(0), 0);
        assert_eq!(mmix.get_register(255), 0);
        assert_eq!(mmix.get_special(SpecialReg::RA), 0);
        assert_eq!(mmix.get_pc(), 0);
    }

    #[test]
    fn test_register_255_always_zero() {
        let mut mmix = MMix::new();
        mmix.set_register(255, 42);
        assert_eq!(mmix.get_register(255), 0);
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
        // INCL $255, YZ=0x0102 - should not modify $255
        mmix.write_tetra(0, 0xE7FF0102);

        mmix.execute_instruction();
        assert_eq!(mmix.get_register(255), 0); // Still zero
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
    fn test_trip_halts() {
        let mut mmix = MMix::new();
        // TRIP instruction should halt execution
        mmix.write_tetra(0, 0xFF000000);

        let result = mmix.execute_instruction();
        assert!(!result); // Should halt
        assert_eq!(mmix.get_pc(), 0); // PC not advanced
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
        let float_val = 3.14159f32;
        mmix.write_tetra(150, float_val.to_bits());

        mmix.execute_instruction();
        // Should be converted to 64-bit float
        let result_f64 = MMix::u64_to_f64(mmix.get_register(1));
        assert!((result_f64 - 3.14159).abs() < 0.0001);
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
    fn test_all_set_instructions_have_tests() {
        // SETH, SETMH, SETML, SETL are tested via SET instruction tests in mmixal module
        // Verify they exist by executing them
        let mut mmix = MMix::new();

        // SETH $1, 0x0001
        mmix.write_tetra(0, 0xE0010001);
        mmix.execute_instruction();
        assert_eq!(mmix.get_register(1), 0x0001000000000000);

        // SETMH $2, 0x0002
        mmix.set_pc(4);
        mmix.write_tetra(4, 0xE1020002);
        mmix.execute_instruction();
        assert_eq!(mmix.get_register(2), 0x0000000200000000);

        // SETML $3, 0x0003
        mmix.set_pc(8);
        mmix.write_tetra(8, 0xE2030003);
        mmix.execute_instruction();
        assert_eq!(mmix.get_register(3), 0x0000000000030000);

        // SETL $4, 0x0004
        mmix.set_pc(12);
        mmix.write_tetra(12, 0xE3040004);
        mmix.execute_instruction();
        assert_eq!(mmix.get_register(4), 0x0000000000000004);
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

        // STB/STBU - tested
        mmix.write_tetra(0, 0xA0010203);
        assert!(mmix.execute_instruction());

        mmix.set_pc(0);
        mmix.write_tetra(0, 0xA2010203);
        assert!(mmix.execute_instruction());

        // STW/STWU - tested
        mmix.set_pc(0);
        mmix.write_tetra(0, 0xA4010203);
        assert!(mmix.execute_instruction());

        mmix.set_pc(0);
        mmix.write_tetra(0, 0xA6010203);
        assert!(mmix.execute_instruction());

        // STT/STTU - tested
        mmix.set_pc(0);
        mmix.write_tetra(0, 0xA8010203);
        assert!(mmix.execute_instruction());

        mmix.set_pc(0);
        mmix.write_tetra(0, 0xAA010203);
        assert!(mmix.execute_instruction());

        // STO/STOU - tested
        mmix.set_pc(0);
        mmix.write_tetra(0, 0xAC010203);
        assert!(mmix.execute_instruction());

        mmix.set_pc(0);
        mmix.write_tetra(0, 0xAE010203);
        assert!(mmix.execute_instruction());

        // STCO - tested
        mmix.set_pc(0);
        mmix.write_tetra(0, 0xB0010203);
        assert!(mmix.execute_instruction());

        // STHT - tested
        mmix.set_pc(0);
        mmix.write_tetra(0, 0xB2010203);
        assert!(mmix.execute_instruction());
    }

    // Arithmetic instruction tests - Add and Subtract (§9)

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

        // ADD - tested
        mmix.write_tetra(0, 0x20010203);
        assert!(mmix.execute_instruction());

        // ADDU (0x22/0x23) - tested
        mmix.set_pc(0);
        mmix.write_tetra(0, 0x22010203);
        assert!(mmix.execute_instruction());

        // 2ADDU - tested
        mmix.set_pc(0);
        mmix.write_tetra(0, 0x24010203);
        assert!(mmix.execute_instruction());

        // 4ADDU - tested
        mmix.set_pc(0);
        mmix.write_tetra(0, 0x26010203);
        assert!(mmix.execute_instruction());

        // 8ADDU - tested
        mmix.set_pc(0);
        mmix.write_tetra(0, 0x28010203);
        assert!(mmix.execute_instruction());

        // 16ADDU - tested
        mmix.set_pc(0);
        mmix.write_tetra(0, 0x2A010203);
        assert!(mmix.execute_instruction());

        // SUB - tested
        mmix.set_pc(0);
        mmix.write_tetra(0, 0x30010203);
        assert!(mmix.execute_instruction());

        // SUBU - tested
        mmix.set_pc(0);
        mmix.write_tetra(0, 0x32010203);
        assert!(mmix.execute_instruction());

        // NEG - tested
        mmix.set_pc(0);
        mmix.write_tetra(0, 0x34010003);
        assert!(mmix.execute_instruction());

        // NEGU - tested
        mmix.set_pc(0);
        mmix.write_tetra(0, 0x36010003);
        assert!(mmix.execute_instruction());
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

    // Shift instruction tests (§14)
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
        assert!((mmix.get_special(SpecialReg::RA) & 0x04) != 0);
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
        // BNB $1, 0, 3 - Branch backward if $1 is negative
        mmix.set_pc(100);
        mmix.set_register(1, (-42i64) as u64);
        mmix.write_tetra(100, 0x41010003); // BNB $1,0,3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 88); // PC = 100 - 3*4 = 88
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
        // BZB $1, 0, 5 - Branch backward if $1 is zero
        mmix.set_pc(100);
        mmix.set_register(1, 0);
        mmix.write_tetra(100, 0x43010005); // BZB $1,0,5
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 80); // PC = 100 - 5*4 = 80
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
        // BPB $1, 0, 2 - Branch backward if $1 is positive
        mmix.set_pc(200);
        mmix.set_register(1, 100);
        mmix.write_tetra(200, 0x45010002); // BPB $1,0,2
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 192); // PC = 200 - 2*4 = 192
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
        // BODB $1, 0, 4 - Branch backward if $1 is odd
        mmix.set_pc(100);
        mmix.set_register(1, 15);
        mmix.write_tetra(100, 0x47010004); // BODB $1,0,4
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 84); // PC = 100 - 4*4 = 84
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
        // BNNB $1, 0, 3 - Branch backward if $1 is non-negative
        mmix.set_pc(100);
        mmix.set_register(1, 0);
        mmix.write_tetra(100, 0x49010003); // BNNB $1,0,3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 88); // PC = 100 - 3*4 = 88
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
        // BNZB $1, 0, 10 - Branch backward if $1 is non-zero
        mmix.set_pc(200);
        mmix.set_register(1, 99);
        mmix.write_tetra(200, 0x4B01000A); // BNZB $1,0,10
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 160); // PC = 200 - 10*4 = 160
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
        // BNPB $1, 0, 1 - Branch backward if $1 is non-positive
        mmix.set_pc(100);
        mmix.set_register(1, 0);
        mmix.write_tetra(100, 0x4D010001); // BNPB $1,0,1
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 96); // PC = 100 - 1*4 = 96
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
        // BEVB $1, 0, 2 - Branch backward if $1 is even
        mmix.set_pc(100);
        mmix.set_register(1, 0);
        mmix.write_tetra(100, 0x4F010002); // BEVB $1,0,2
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 92); // PC = 100 - 2*4 = 92
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
        // PBNB $1, 0, 3 - Probable branch backward if $1 is negative
        mmix.set_pc(100);
        mmix.set_register(1, (-1i64) as u64);
        mmix.write_tetra(100, 0x51010003); // PBNB $1,0,3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 88); // PC = 100 - 3*4 = 88
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
        // PBZB $1, 0, 4 - Probable branch backward if $1 is zero
        mmix.set_pc(100);
        mmix.set_register(1, 0);
        mmix.write_tetra(100, 0x53010004); // PBZB $1,0,4
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 84); // PC = 100 - 4*4 = 84
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
        // PBPB $1, 0, 2 - Probable branch backward if $1 is positive
        mmix.set_pc(100);
        mmix.set_register(1, 1);
        mmix.write_tetra(100, 0x55010002); // PBPB $1,0,2
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 92); // PC = 100 - 2*4 = 92
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
        // PBODB $1, 0, 5 - Probable branch backward if $1 is odd
        mmix.set_pc(100);
        mmix.set_register(1, 99);
        mmix.write_tetra(100, 0x57010005); // PBODB $1,0,5
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 80); // PC = 100 - 5*4 = 80
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
        // PBNNB $1, 0, 1 - Probable branch backward if $1 is non-negative
        mmix.set_pc(100);
        mmix.set_register(1, 0);
        mmix.write_tetra(100, 0x59010001); // PBNNB $1,0,1
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 96); // PC = 100 - 1*4 = 96
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
        // PBNZB $1, 0, 6 - Probable branch backward if $1 is non-zero
        mmix.set_pc(200);
        mmix.set_register(1, 1);
        mmix.write_tetra(200, 0x5B010006); // PBNZB $1,0,6
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 176); // PC = 200 - 6*4 = 176
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
        // PBNPB $1, 0, 8 - Probable branch backward if $1 is non-positive
        mmix.set_pc(100);
        mmix.set_register(1, 0);
        mmix.write_tetra(100, 0x5D010008); // PBNPB $1,0,8
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 68); // PC = 100 - 8*4 = 68
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
        // PBEVB $1, 0, 7 - Probable branch backward if $1 is even
        mmix.set_pc(100);
        mmix.set_register(1, 0);
        mmix.write_tetra(100, 0x5F010007); // PBEVB $1,0,7
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 72); // PC = 100 - 7*4 = 72
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
    fn test_jmp_negative_offset() {
        let mut mmix = MMix::new();
        mmix.set_pc(100);
        // JMP -5 (offset = -5, encoded as 0xFFFFFB in 24-bit signed)
        mmix.write_tetra(100, 0xF0FFFFFB); // JMP with offset -5
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 80); // PC = 100 + (-5)*4 = 80
    }

    #[test]
    fn test_jmpb() {
        let mut mmix = MMix::new();
        mmix.set_pc(100);
        // JMPB 5 - Jump backward by 5
        mmix.write_tetra(100, 0xF1000005); // JMPB 0,0,5
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
        // PUSHJB $0, 0, 5 - Push and jump backward
        mmix.write_tetra(100, 0xF3000005); // PUSHJB $0,0,5
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
        // GETAB $1, 0, 5 - Get address backward
        mmix.write_tetra(100, 0xF5010005); // GETAB $1,0,5
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 80); // Addr = 100 - 5*4 = 80
        assert_eq!(mmix.get_pc(), 104);
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
    fn test_puti() {
        let mut mmix = MMix::new();
        // PUTI rH, 0x1234 - Put immediate value into rH (special register 3)
        mmix.write_tetra(0, 0xF7031234); // PUTI X=3 (rH), YZ=0x1234
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_special(SpecialReg::RH), 0x1234);
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

    #[test]
    fn test_trip() {
        let mut mmix = MMix::new();
        // TRIP - software interrupt (halts in our implementation)
        mmix.write_tetra(0, 0xFF000000); // TRIP 0,0,0
        assert!(!mmix.execute_instruction()); // Should return false (halt)
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
    fn test_resume() {
        let mut mmix = MMix::new();
        // RESUME - resume after interrupt
        mmix.write_tetra(0, 0xF9000000); // RESUME
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_save() {
        let mut mmix = MMix::new();
        // SAVE $1, 0 - Save process state
        mmix.write_tetra(0, 0xFA010000); // SAVE $1,0
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_unsave() {
        let mut mmix = MMix::new();
        // UNSAVE $1 - Restore process state
        mmix.write_tetra(0, 0xFB000001); // UNSAVE Z=$1
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_csn_condition_true() {
        let mut mmix = MMix::new();
        // CSN $1, $2, $3 - Set $1 = $2 + $3 if $1 is negative
        mmix.set_register(1, (-10i64) as u64);
        mmix.set_register(2, 100);
        mmix.set_register(3, 50);
        mmix.write_tetra(0, 0x60010203); // CSN $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 150); // Condition true: 100 + 50
    }

    #[test]
    fn test_csn_condition_false() {
        let mut mmix = MMix::new();
        // CSN $1, $2, $3 - Set $1 = $2 if $1 is not negative
        mmix.set_register(1, 5);
        mmix.set_register(2, 100);
        mmix.set_register(3, 50);
        mmix.write_tetra(0, 0x60010203); // CSN $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 100); // Condition false: just $2
    }

    #[test]
    fn test_csni() {
        let mut mmix = MMix::new();
        // CSNI $1, $2, 50 - Set $1 = $2 + 50 if $1 is negative
        mmix.set_register(1, (-1i64) as u64);
        mmix.set_register(2, 200);
        mmix.write_tetra(0, 0x61010232); // CSNI $1,$2,50
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 250); // 200 + 50
    }

    #[test]
    fn test_csz_condition_true() {
        let mut mmix = MMix::new();
        // CSZ $1, $2, $3 - Set $1 = $2 + $3 if $1 is zero
        mmix.set_register(1, 0);
        mmix.set_register(2, 10);
        mmix.set_register(3, 20);
        mmix.write_tetra(0, 0x62010203); // CSZ $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 30); // Condition true: 10 + 20
    }

    #[test]
    fn test_csz_condition_false() {
        let mut mmix = MMix::new();
        // CSZ $1, $2, $3 - Set $1 = $2 if $1 is not zero
        mmix.set_register(1, 1);
        mmix.set_register(2, 10);
        mmix.set_register(3, 20);
        mmix.write_tetra(0, 0x62010203); // CSZ $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 10); // Condition false: just $2
    }

    #[test]
    fn test_cszi() {
        let mut mmix = MMix::new();
        // CSZI $1, $2, 15 - Set $1 = $2 + 15 if $1 is zero
        mmix.set_register(1, 0);
        mmix.set_register(2, 100);
        mmix.write_tetra(0, 0x6301020F); // CSZI $1,$2,15
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 115);
    }

    #[test]
    fn test_csp_condition_true() {
        let mut mmix = MMix::new();
        // CSP $1, $2, $3 - Set $1 = $2 + $3 if $1 is positive
        mmix.set_register(1, 42);
        mmix.set_register(2, 5);
        mmix.set_register(3, 7);
        mmix.write_tetra(0, 0x64010203); // CSP $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 12); // Condition true: 5 + 7
    }

    #[test]
    fn test_csp_condition_false_zero() {
        let mut mmix = MMix::new();
        // CSP $1, $2, $3 - Set $1 = $2 if $1 is zero (not positive)
        mmix.set_register(1, 0);
        mmix.set_register(2, 5);
        mmix.set_register(3, 7);
        mmix.write_tetra(0, 0x64010203); // CSP $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 5); // Condition false: just $2
    }

    #[test]
    fn test_cspi() {
        let mut mmix = MMix::new();
        // CSPI $1, $2, 25 - Set $1 = $2 + 25 if $1 is positive
        mmix.set_register(1, 100);
        mmix.set_register(2, 50);
        mmix.write_tetra(0, 0x65010219); // CSPI $1,$2,25
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 75); // 50 + 25
    }

    #[test]
    fn test_csod_condition_true() {
        let mut mmix = MMix::new();
        // CSOD $1, $2, $3 - Set $1 = $2 + $3 if $1 is odd
        mmix.set_register(1, 7);
        mmix.set_register(2, 10);
        mmix.set_register(3, 15);
        mmix.write_tetra(0, 0x66010203); // CSOD $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 25); // Condition true: 10 + 15
    }

    #[test]
    fn test_csod_condition_false() {
        let mut mmix = MMix::new();
        // CSOD $1, $2, $3 - Set $1 = $2 if $1 is even
        mmix.set_register(1, 8);
        mmix.set_register(2, 10);
        mmix.set_register(3, 15);
        mmix.write_tetra(0, 0x66010203); // CSOD $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 10); // Condition false: just $2
    }

    #[test]
    fn test_csodi() {
        let mut mmix = MMix::new();
        // CSODI $1, $2, 11 - Set $1 = $2 + 11 if $1 is odd
        mmix.set_register(1, 99);
        mmix.set_register(2, 20);
        mmix.write_tetra(0, 0x6701020B); // CSODI $1,$2,11
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 31); // 20 + 11
    }

    #[test]
    fn test_csnn_condition_true_positive() {
        let mut mmix = MMix::new();
        // CSNN $1, $2, $3 - Set $1 = $2 + $3 if $1 is non-negative
        mmix.set_register(1, 10);
        mmix.set_register(2, 30);
        mmix.set_register(3, 40);
        mmix.write_tetra(0, 0x68010203); // CSNN $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 70); // Condition true: 30 + 40
    }

    #[test]
    fn test_csnn_condition_true_zero() {
        let mut mmix = MMix::new();
        // CSNN $1, $2, $3 - Set $1 = $2 + $3 if $1 is zero (non-negative)
        mmix.set_register(1, 0);
        mmix.set_register(2, 30);
        mmix.set_register(3, 40);
        mmix.write_tetra(0, 0x68010203); // CSNN $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 70); // Condition true: 30 + 40
    }

    #[test]
    fn test_csnn_condition_false() {
        let mut mmix = MMix::new();
        // CSNN $1, $2, $3 - Set $1 = $2 if $1 is negative
        mmix.set_register(1, (-5i64) as u64);
        mmix.set_register(2, 30);
        mmix.set_register(3, 40);
        mmix.write_tetra(0, 0x68010203); // CSNN $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 30); // Condition false: just $2
    }

    #[test]
    fn test_csnni() {
        let mut mmix = MMix::new();
        // CSNNI $1, $2, 8 - Set $1 = $2 + 8 if $1 is non-negative
        mmix.set_register(1, 0);
        mmix.set_register(2, 92);
        mmix.write_tetra(0, 0x69010208); // CSNNI $1,$2,8
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 100); // 92 + 8
    }

    #[test]
    fn test_csnz_condition_true() {
        let mut mmix = MMix::new();
        // CSNZ $1, $2, $3 - Set $1 = $2 + $3 if $1 is non-zero
        mmix.set_register(1, 1);
        mmix.set_register(2, 100);
        mmix.set_register(3, 200);
        mmix.write_tetra(0, 0x6A010203); // CSNZ $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 300); // Condition true: 100 + 200
    }

    #[test]
    fn test_csnz_condition_false() {
        let mut mmix = MMix::new();
        // CSNZ $1, $2, $3 - Set $1 = $2 if $1 is zero
        mmix.set_register(1, 0);
        mmix.set_register(2, 100);
        mmix.set_register(3, 200);
        mmix.write_tetra(0, 0x6A010203); // CSNZ $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 100); // Condition false: just $2
    }

    #[test]
    fn test_csnzi() {
        let mut mmix = MMix::new();
        // CSNZI $1, $2, 33 - Set $1 = $2 + 33 if $1 is non-zero
        mmix.set_register(1, 42);
        mmix.set_register(2, 67);
        mmix.write_tetra(0, 0x6B010221); // CSNZI $1,$2,33
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 100); // 67 + 33
    }

    #[test]
    fn test_csnp_condition_true_negative() {
        let mut mmix = MMix::new();
        // CSNP $1, $2, $3 - Set $1 = $2 + $3 if $1 is non-positive
        mmix.set_register(1, (-100i64) as u64);
        mmix.set_register(2, 50);
        mmix.set_register(3, 25);
        mmix.write_tetra(0, 0x6C010203); // CSNP $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 75); // Condition true: 50 + 25
    }

    #[test]
    fn test_csnp_condition_true_zero() {
        let mut mmix = MMix::new();
        // CSNP $1, $2, $3 - Set $1 = $2 + $3 if $1 is zero (non-positive)
        mmix.set_register(1, 0);
        mmix.set_register(2, 50);
        mmix.set_register(3, 25);
        mmix.write_tetra(0, 0x6C010203); // CSNP $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 75); // Condition true: 50 + 25
    }

    #[test]
    fn test_csnp_condition_false() {
        let mut mmix = MMix::new();
        // CSNP $1, $2, $3 - Set $1 = $2 if $1 is positive
        mmix.set_register(1, 1);
        mmix.set_register(2, 50);
        mmix.set_register(3, 25);
        mmix.write_tetra(0, 0x6C010203); // CSNP $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 50); // Condition false: just $2
    }

    #[test]
    fn test_csnpi() {
        let mut mmix = MMix::new();
        // CSNPI $1, $2, 44 - Set $1 = $2 + 44 if $1 is non-positive
        mmix.set_register(1, 0);
        mmix.set_register(2, 56);
        mmix.write_tetra(0, 0x6D01022C); // CSNPI $1,$2,44
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 100); // 56 + 44
    }

    #[test]
    fn test_csev_condition_true() {
        let mut mmix = MMix::new();
        // CSEV $1, $2, $3 - Set $1 = $2 + $3 if $1 is even
        mmix.set_register(1, 100);
        mmix.set_register(2, 80);
        mmix.set_register(3, 20);
        mmix.write_tetra(0, 0x6E010203); // CSEV $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 100); // Condition true: 80 + 20
    }

    #[test]
    fn test_csev_condition_false() {
        let mut mmix = MMix::new();
        // CSEV $1, $2, $3 - Set $1 = $2 if $1 is odd
        mmix.set_register(1, 7);
        mmix.set_register(2, 80);
        mmix.set_register(3, 20);
        mmix.write_tetra(0, 0x6E010203); // CSEV $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 80); // Condition false: just $2
    }

    #[test]
    fn test_csevi() {
        let mut mmix = MMix::new();
        // CSEVI $1, $2, 12 - Set $1 = $2 + 12 if $1 is even
        mmix.set_register(1, 0);
        mmix.set_register(2, 88);
        mmix.write_tetra(0, 0x6F01020C); // CSEVI $1,$2,12
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 100); // 88 + 12
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
        // FCMP $1, $2, $3 - Compare with NaN
        mmix.set_register(2, f64::NAN.to_bits());
        mmix.set_register(3, 5.0f64.to_bits());
        mmix.write_tetra(0, 0x01010203); // FCMP $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 2); // Unordered
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
        // FUNE $1, $2, $3 - Test unordered or equivalent with epsilon
        mmix.set_special(SpecialReg::RE, 0.5f64.to_bits());
        mmix.set_register(2, 7.0f64.to_bits());
        mmix.set_register(3, 7.3f64.to_bits());
        mmix.write_tetra(0, 0x12010203); // FUNE $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 1); // Within epsilon
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
        // FREM $1, $2, $3 - Remainder 7.5 % 2.0
        mmix.set_register(2, 7.5f64.to_bits());
        mmix.set_register(3, 2.0f64.to_bits());
        mmix.write_tetra(0, 0x16010203); // FREM $1,$2,$3
        assert!(mmix.execute_instruction());
        let result = f64::from_bits(mmix.get_register(1));
        assert!((result - 1.5).abs() < 1e-10);
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
        // FIX $1, $3 - Convert 42.9 to signed integer
        mmix.set_register(3, 42.9f64.to_bits());
        mmix.write_tetra(0, 0x05010003); // FIX $1,$0,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 42);
    }

    #[test]
    fn test_fix_negative() {
        let mut mmix = MMix::new();
        // FIX $1, $3 - Convert -17.8 to signed integer
        mmix.set_register(3, (-17.8f64).to_bits());
        mmix.write_tetra(0, 0x05010003); // FIX $1,$0,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1) as i64, -17);
    }

    #[test]
    fn test_fixu() {
        let mut mmix = MMix::new();
        // FIXU $1, $3 - Convert 99.5 to unsigned integer
        mmix.set_register(3, 99.5f64.to_bits());
        mmix.write_tetra(0, 0x07010003); // FIXU $1,$0,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 99);
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
        // FLOTI $1, 256 - Convert immediate signed 256 to float
        mmix.write_tetra(0, 0x09010100); // FLOTI $1,256 (YZ=0x0100)
        assert!(mmix.execute_instruction());
        let result = f64::from_bits(mmix.get_register(1));
        assert!((result - 256.0).abs() < 1e-10);
    }

    #[test]
    fn test_floti_negative() {
        let mut mmix = MMix::new();
        // FLOTI $1, -1 - Convert immediate signed -1 to float
        mmix.write_tetra(0, 0x0901FFFF); // FLOTI $1,-1 (YZ=0xFFFF)
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
        // FLOTUI $1, 500 - Convert immediate unsigned 500 to float
        mmix.write_tetra(0, 0x0B0101F4); // FLOTUI $1,500 (YZ=0x01F4)
        assert!(mmix.execute_instruction());
        let result = f64::from_bits(mmix.get_register(1));
        assert!((result - 500.0).abs() < 1e-10);
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
        mmix.set_special(SpecialReg::RA, 0); // Round mode 0 = ROUND_NEAR
        mmix.set_register(3, 3.7f64.to_bits());
        mmix.write_tetra(0, 0x17010003); // FINT $1,$0,$3
        assert!(mmix.execute_instruction());
        let result = f64::from_bits(mmix.get_register(1));
        assert!((result - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_fint_round_down() {
        let mut mmix = MMix::new();
        // FINT $1, $0, $3 - Integerize with ROUND_DOWN mode
        mmix.set_special(SpecialReg::RA, 1); // Round mode 1 = ROUND_DOWN (floor)
        mmix.set_register(3, 3.7f64.to_bits());
        mmix.write_tetra(0, 0x17010003); // FINT $1,$0,$3
        assert!(mmix.execute_instruction());
        let result = f64::from_bits(mmix.get_register(1));
        assert!((result - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_fint_round_up() {
        let mut mmix = MMix::new();
        // FINT $1, $0, $3 - Integerize with ROUND_UP mode
        mmix.set_special(SpecialReg::RA, 2); // Round mode 2 = ROUND_UP (ceil)
        mmix.set_register(3, 3.2f64.to_bits());
        mmix.write_tetra(0, 0x17010003); // FINT $1,$0,$3
        assert!(mmix.execute_instruction());
        let result = f64::from_bits(mmix.get_register(1));
        assert!((result - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_fint_round_off() {
        let mut mmix = MMix::new();
        // FINT $1, $0, $3 - Integerize with ROUND_OFF mode (toward zero)
        mmix.set_special(SpecialReg::RA, 3); // Round mode 3 = ROUND_OFF (trunc)
        mmix.set_register(3, 3.9f64.to_bits());
        mmix.write_tetra(0, 0x17010003); // FINT $1,$0,$3
        assert!(mmix.execute_instruction());
        let result = f64::from_bits(mmix.get_register(1));
        assert!((result - 3.0).abs() < 1e-10);
    }

    // ========== Zero or Set Tests ==========

    #[test]
    fn test_zsn_condition_true() {
        let mut mmix = MMix::new();
        // ZSN $1, $2, $3 - Set $1 = $2 + $3 if $1 is negative
        mmix.set_register(1, (-10i64) as u64);
        mmix.set_register(2, 100);
        mmix.set_register(3, 50);
        mmix.write_tetra(0, 0x70010203); // ZSN $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 150); // Condition true: 100 + 50
    }

    #[test]
    fn test_zsn_condition_false() {
        let mut mmix = MMix::new();
        // ZSN $1, $2, $3 - Set $1 = 0 if $1 is not negative
        mmix.set_register(1, 5);
        mmix.set_register(2, 100);
        mmix.set_register(3, 50);
        mmix.write_tetra(0, 0x70010203); // ZSN $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 0); // Condition false: 0
    }

    #[test]
    fn test_zsni() {
        let mut mmix = MMix::new();
        // ZSNI $1, $2, 50 - Set $1 = $2 + 50 if $1 is negative
        mmix.set_register(1, (-1i64) as u64);
        mmix.set_register(2, 200);
        mmix.write_tetra(0, 0x71010232); // ZSNI $1,$2,50
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 250); // 200 + 50
    }

    #[test]
    fn test_zsz_condition_true() {
        let mut mmix = MMix::new();
        // ZSZ $1, $2, $3 - Set $1 = $2 + $3 if $1 is zero
        mmix.set_register(1, 0);
        mmix.set_register(2, 10);
        mmix.set_register(3, 20);
        mmix.write_tetra(0, 0x72010203); // ZSZ $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 30); // Condition true: 10 + 20
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
        // ZSZI $1, $2, 15 - Set $1 = $2 + 15 if $1 is zero
        mmix.set_register(1, 0);
        mmix.set_register(2, 100);
        mmix.write_tetra(0, 0x7301020F); // ZSZI $1,$2,15
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 115);
    }

    #[test]
    fn test_zsp_condition_true() {
        let mut mmix = MMix::new();
        // ZSP $1, $2, $3 - Set $1 = $2 + $3 if $1 is positive
        mmix.set_register(1, 42);
        mmix.set_register(2, 5);
        mmix.set_register(3, 7);
        mmix.write_tetra(0, 0x74010203); // ZSP $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 12); // Condition true: 5 + 7
    }

    #[test]
    fn test_zsp_condition_false_zero() {
        let mut mmix = MMix::new();
        // ZSP $1, $2, $3 - Set $1 = 0 if $1 is zero (not positive)
        mmix.set_register(1, 0);
        mmix.set_register(2, 5);
        mmix.set_register(3, 7);
        mmix.write_tetra(0, 0x74010203); // ZSP $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 0); // Condition false: 0
    }

    #[test]
    fn test_zspi() {
        let mut mmix = MMix::new();
        // ZSPI $1, $2, 25 - Set $1 = $2 + 25 if $1 is positive
        mmix.set_register(1, 100);
        mmix.set_register(2, 50);
        mmix.write_tetra(0, 0x75010219); // ZSPI $1,$2,25
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 75); // 50 + 25
    }

    #[test]
    fn test_zsod_condition_true() {
        let mut mmix = MMix::new();
        // ZSOD $1, $2, $3 - Set $1 = $2 + $3 if $1 is odd
        mmix.set_register(1, 7);
        mmix.set_register(2, 10);
        mmix.set_register(3, 15);
        mmix.write_tetra(0, 0x76010203); // ZSOD $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 25); // Condition true: 10 + 15
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
        // ZSODI $1, $2, 11 - Set $1 = $2 + 11 if $1 is odd
        mmix.set_register(1, 99);
        mmix.set_register(2, 20);
        mmix.write_tetra(0, 0x7701020B); // ZSODI $1,$2,11
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 31); // 20 + 11
    }

    #[test]
    fn test_zsnn_condition_true_positive() {
        let mut mmix = MMix::new();
        // ZSNN $1, $2, $3 - Set $1 = $2 + $3 if $1 is non-negative
        mmix.set_register(1, 10);
        mmix.set_register(2, 30);
        mmix.set_register(3, 40);
        mmix.write_tetra(0, 0x78010203); // ZSNN $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 70); // Condition true: 30 + 40
    }

    #[test]
    fn test_zsnn_condition_true_zero() {
        let mut mmix = MMix::new();
        // ZSNN $1, $2, $3 - Set $1 = $2 + $3 if $1 is zero (non-negative)
        mmix.set_register(1, 0);
        mmix.set_register(2, 30);
        mmix.set_register(3, 40);
        mmix.write_tetra(0, 0x78010203); // ZSNN $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 70); // Condition true: 30 + 40
    }

    #[test]
    fn test_zsnn_condition_false() {
        let mut mmix = MMix::new();
        // ZSNN $1, $2, $3 - Set $1 = 0 if $1 is negative
        mmix.set_register(1, (-5i64) as u64);
        mmix.set_register(2, 30);
        mmix.set_register(3, 40);
        mmix.write_tetra(0, 0x78010203); // ZSNN $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 0); // Condition false: 0
    }

    #[test]
    fn test_zsnni() {
        let mut mmix = MMix::new();
        // ZSNNI $1, $2, 8 - Set $1 = $2 + 8 if $1 is non-negative
        mmix.set_register(1, 0);
        mmix.set_register(2, 92);
        mmix.write_tetra(0, 0x79010208); // ZSNNI $1,$2,8
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 100); // 92 + 8
    }

    #[test]
    fn test_zsnz_condition_true() {
        let mut mmix = MMix::new();
        // ZSNZ $1, $2, $3 - Set $1 = $2 + $3 if $1 is non-zero
        mmix.set_register(1, 1);
        mmix.set_register(2, 100);
        mmix.set_register(3, 200);
        mmix.write_tetra(0, 0x7A010203); // ZSNZ $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 300); // Condition true: 100 + 200
    }

    #[test]
    fn test_zsnz_condition_false() {
        let mut mmix = MMix::new();
        // ZSNZ $1, $2, $3 - Set $1 = 0 if $1 is zero
        mmix.set_register(1, 0);
        mmix.set_register(2, 100);
        mmix.set_register(3, 200);
        mmix.write_tetra(0, 0x7A010203); // ZSNZ $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 0); // Condition false: 0
    }

    #[test]
    fn test_zsnzi() {
        let mut mmix = MMix::new();
        // ZSNZI $1, $2, 33 - Set $1 = $2 + 33 if $1 is non-zero
        mmix.set_register(1, 42);
        mmix.set_register(2, 67);
        mmix.write_tetra(0, 0x7B010221); // ZSNZI $1,$2,33
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 100); // 67 + 33
    }

    #[test]
    fn test_zsnp_condition_true_negative() {
        let mut mmix = MMix::new();
        // ZSNP $1, $2, $3 - Set $1 = $2 + $3 if $1 is non-positive
        mmix.set_register(1, (-100i64) as u64);
        mmix.set_register(2, 50);
        mmix.set_register(3, 25);
        mmix.write_tetra(0, 0x7C010203); // ZSNP $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 75); // Condition true: 50 + 25
    }

    #[test]
    fn test_zsnp_condition_true_zero() {
        let mut mmix = MMix::new();
        // ZSNP $1, $2, $3 - Set $1 = $2 + $3 if $1 is zero (non-positive)
        mmix.set_register(1, 0);
        mmix.set_register(2, 50);
        mmix.set_register(3, 25);
        mmix.write_tetra(0, 0x7C010203); // ZSNP $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 75); // Condition true: 50 + 25
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
        // ZSNPI $1, $2, 44 - Set $1 = $2 + 44 if $1 is non-positive
        mmix.set_register(1, 0);
        mmix.set_register(2, 56);
        mmix.write_tetra(0, 0x7D01022C); // ZSNPI $1,$2,44
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 100); // 56 + 44
    }

    #[test]
    fn test_zsev_condition_true() {
        let mut mmix = MMix::new();
        // ZSEV $1, $2, $3 - Set $1 = $2 + $3 if $1 is even
        mmix.set_register(1, 100);
        mmix.set_register(2, 80);
        mmix.set_register(3, 20);
        mmix.write_tetra(0, 0x7E010203); // ZSEV $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 100); // Condition true: 80 + 20
    }

    #[test]
    fn test_zsev_condition_false() {
        let mut mmix = MMix::new();
        // ZSEV $1, $2, $3 - Set $1 = 0 if $1 is odd
        mmix.set_register(1, 7);
        mmix.set_register(2, 80);
        mmix.set_register(3, 20);
        mmix.write_tetra(0, 0x7E010203); // ZSEV $1,$2,$3
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 0); // Condition false: 0
    }

    #[test]
    fn test_zsevi() {
        let mut mmix = MMix::new();
        // ZSEVI $1, $2, 12 - Set $1 = $2 + 12 if $1 is even
        mmix.set_register(1, 0);
        mmix.set_register(2, 88);
        mmix.write_tetra(0, 0x7F01020C); // ZSEVI $1,$2,12
        assert!(mmix.execute_instruction());
        assert_eq!(mmix.get_register(1), 100); // 88 + 12
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
        let f64_value = 3.14159265358979f64;
        mmix.set_register(1, f64_value.to_bits());
        mmix.set_register(2, 1000);
        mmix.set_register(3, 8);
        mmix.write_tetra(0, 0xB0010203); // STSF $1,$2,$3
        assert!(mmix.execute_instruction());

        let stored_tetra = mmix.read_tetra(1008);
        let f32_value = f32::from_bits(stored_tetra);
        assert!((f32_value - 3.14159265f32).abs() < 1e-5);
    }

    #[test]
    fn test_stsfi() {
        let mut mmix = MMix::new();
        // STSFI $1, $2, 16 - Store short float immediate
        let f64_value = 2.71828f64;
        mmix.set_register(1, f64_value.to_bits());
        mmix.set_register(2, 2000);
        mmix.write_tetra(0, 0xB1010210); // STSFI $1,$2,16
        assert!(mmix.execute_instruction());

        let stored_tetra = mmix.read_tetra(2016);
        let f32_value = f32::from_bits(stored_tetra);
        assert!((f32_value - 2.71828f32).abs() < 1e-5);
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
}
