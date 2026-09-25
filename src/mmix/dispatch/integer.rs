//! The integer family: MUL/DIV, ADD/SUB and variants, CMP, NEG, shifts,
//! conditional set, zero-or-set, bitwise and bit-fiddling operations, and
//! the SETH/INC/OR/ANDN wyde family.

use super::super::{MMix, RA_D, RA_V, SpecialReg};
use crate::mmixal::Opcode;

impl MMix {
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

    /// Dispatches the integer family; `dispatch` routes only these opcodes
    /// here.
    pub(super) fn dispatch_integer(
        &mut self,
        opcode: Opcode,
        op_byte: u8,
        x: u8,
        y: u8,
        z: u8,
    ) -> bool {
        match opcode {
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
            Opcode::ADDU => {
                // ADDU $X, $Y, $Z
                binop_rr!(self, x, y, z, u64::wrapping_add)
            }
            Opcode::ADDUI => {
                // ADDUI $X, $Y, Z
                binop_ri!(self, x, y, z, u64::wrapping_add)
            }
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
            // CMP and NEG instructions - opcodes 0x30-0x37
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
            // Bitwise operations - opcodes 0xC0-0xCF
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
                // BDIFI $X, $Y, Z - Byte difference immediate. Z is the
                // octabyte #00...0Z (bdif.html): lane 0 subtracts, every
                // higher lane is $Y's lane unchanged.
                let val_y = self.get_register(y);
                let byte0 = (val_y & 0xFF) as u8;
                let diff = byte0.saturating_sub(z);
                let result = (val_y & !0xFFu64) | diff as u64;
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
                // WDIFI $X, $Y, Z - Wyde difference immediate. Z is the
                // octabyte #00...0Z: lane 0 subtracts, every higher lane
                // is $Y's lane unchanged.
                let val_y = self.get_register(y);
                let wyde0 = (val_y & 0xFFFF) as u16;
                let diff = wyde0.saturating_sub(z as u16);
                let result = (val_y & !0xFFFFu64) | diff as u64;
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
                // TDIFI $X, $Y, Z - Tetra difference immediate. Z is the
                // octabyte #00...0Z: lane 0 subtracts, every higher lane
                // is $Y's lane unchanged.
                let val_y = self.get_register(y);
                let tetra0 = (val_y & 0xFFFF_FFFF) as u32;
                let diff = tetra0.saturating_sub(z as u32);
                let result = (val_y & !0xFFFF_FFFFu64) | diff as u64;
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
            _ => unreachable!("dispatch routes only integer opcodes to dispatch_integer"),
        }
    }
}
