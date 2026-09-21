//! The 21 register-register/register-immediate opcode implementation macros.

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

/// Macro for int-to-float conversions (register). `$signed` reinterprets `$Z`
/// as `i64`. `$mnem` names the instruction in the `Y > 4` diagnostic.
macro_rules! i2f_conv_rr {
    ($cpu:expr, $op:expr, $x:expr, $y:expr, $z:expr, $signed:expr, $mnem:expr) => {{
        let mode = match $cpu.resolved_round_mode($y) {
            Ok(m) => m,
            Err(()) => return $cpu.illegal_round_mode($mnem, $y),
        };
        // Y is the rounding-mode field, not a register operand.
        let y_val = $y as u64;
        let z_val = $cpu.get_register($z);
        let negative = $signed && (z_val as i64) < 0;
        let magnitude = if negative {
            (z_val as i64).unsigned_abs()
        } else {
            z_val
        };
        let (result, flags) = $cpu.int_to_f64_rounded(negative, magnitude, mode);
        $cpu.set_register($x, MMix::f64_to_u64(result));
        return $cpu.raise_exceptions(flags, $op, $x, $y, $z, y_val, z_val);
    }};
}

/// Macro for int-to-float conversions (immediate). `Z` is an unsigned
/// byte, 0-255, like any immediate operand — `FLOTI` and `FLOTUI` agree on
/// it, so there is no signed/unsigned choice to make here. Every such value
/// converts exactly, so the rounding mode can never change the result — `Y`
/// is still checked, since `Y > 4` halts regardless.
macro_rules! i2f_conv_ri {
    ($cpu:expr, $op:expr, $x:expr, $y:expr, $z:expr, $mnem:expr) => {{
        let mode = match $cpu.resolved_round_mode($y) {
            Ok(m) => m,
            Err(()) => return $cpu.illegal_round_mode($mnem, $y),
        };
        // Y is the rounding-mode field and Z the literal operand, neither a
        // register.
        let y_val = $y as u64;
        let z_val = $z as u64;
        let (result, flags) = $cpu.int_to_f64_rounded(false, z_val, mode);
        $cpu.set_register($x, MMix::f64_to_u64(result));
        return $cpu.raise_exceptions(flags, $op, $x, $y, $z, y_val, z_val);
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
    ($cpu:expr, $op:expr, $x:expr, $y:expr, $z:expr) => {{
        let y_val = $cpu.get_register($y);
        let z_val = $cpu.get_register($z);
        let a = y_val as i64;
        let b = z_val as i64;
        let product = (a as i128) * (b as i128);
        $cpu.set_register($x, product as u64);
        let sign_ext = if (product as u64) as i64 >= 0 {
            0i64
        } else {
            -1i64
        };
        let flags = if (product >> 64) as i64 != sign_ext {
            RA_V
        } else {
            0
        };
        return $cpu.raise_exceptions(flags, $op, $x, $y, $z, y_val, z_val);
    }};
}

/// Macro for signed multiplication with overflow detection (register-immediate)
macro_rules! mul_ri {
    ($cpu:expr, $op:expr, $x:expr, $y:expr, $z:expr) => {{
        let y_val = $cpu.get_register($y);
        // Z is the literal operand, not a register.
        let z_val = $z as u64;
        let a = y_val as i64;
        let b = $z as i64;
        let product = (a as i128) * (b as i128);
        $cpu.set_register($x, product as u64);
        let sign_ext = if (product as u64) as i64 >= 0 {
            0i64
        } else {
            -1i64
        };
        let flags = if (product >> 64) as i64 != sign_ext {
            RA_V
        } else {
            0
        };
        return $cpu.raise_exceptions(flags, $op, $x, $y, $z, y_val, z_val);
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
    ($cpu:expr, $op:expr, $x:expr, $y:expr, $z:expr) => {{
        let y_val = $cpu.get_register($y);
        let z_val = $cpu.get_register($z);
        let dividend = y_val as i64;
        let divisor = z_val as i64;
        let flags = if divisor == 0 {
            $cpu.set_register($x, 0);
            $cpu.set_special(SpecialReg::RR, y_val);
            RA_D
        } else if dividend == i64::MIN && divisor == -1 {
            // The only quotient outside the signed range; it wraps to itself.
            $cpu.set_register($x, i64::MIN as u64);
            $cpu.set_special(SpecialReg::RR, 0);
            RA_V
        } else {
            // MMIX floors the quotient, so the remainder takes the divisor's
            // sign; Rust truncates toward zero.
            let mut quotient = dividend / divisor;
            let mut remainder = dividend % divisor;
            if remainder != 0 && (remainder < 0) != (divisor < 0) {
                quotient -= 1;
                remainder += divisor;
            }
            $cpu.set_register($x, quotient as u64);
            $cpu.set_special(SpecialReg::RR, remainder as u64);
            0
        };
        return $cpu.raise_exceptions(flags, $op, $x, $y, $z, y_val, z_val);
    }};
}

/// Macro for signed division (register-immediate)
macro_rules! div_ri {
    ($cpu:expr, $op:expr, $x:expr, $y:expr, $z:expr) => {{
        let y_val = $cpu.get_register($y);
        // Z is the literal operand, not a register.
        let z_val = $z as u64;
        let dividend = y_val as i64;
        let divisor = $z as i64;
        let flags = if divisor == 0 {
            $cpu.set_register($x, 0);
            $cpu.set_special(SpecialReg::RR, y_val);
            RA_D
        } else if dividend == i64::MIN && divisor == -1 {
            // Z is a byte, so the divisor is in `0..=255` and this arm is
            // unreachable from the immediate encoding. It mirrors div_rr,
            // where the quotient leaves the signed range and wraps to itself.
            $cpu.set_register($x, i64::MIN as u64);
            $cpu.set_special(SpecialReg::RR, 0);
            RA_V
        } else {
            // MMIX floors the quotient, so the remainder takes the divisor's
            // sign; Rust truncates toward zero.
            let mut quotient = dividend / divisor;
            let mut remainder = dividend % divisor;
            if remainder != 0 && (remainder < 0) != (divisor < 0) {
                quotient -= 1;
                remainder += divisor;
            }
            $cpu.set_register($x, quotient as u64);
            $cpu.set_special(SpecialReg::RR, remainder as u64);
            0
        };
        return $cpu.raise_exceptions(flags, $op, $x, $y, $z, y_val, z_val);
    }};
}

/// Macro for unsigned division (register-register)
macro_rules! divu_rr {
    ($cpu:expr, $x:expr, $y:expr, $z:expr) => {{
        let dividend_low = $cpu.get_register($y);
        let dividend_high = $cpu.get_special(SpecialReg::RD);
        let dividend = ((dividend_high as u128) << 64) | (dividend_low as u128);
        let divisor = $cpu.get_register($z) as u128;
        // The quotient is defined only when u($Z) > u(rD); otherwise it would
        // not fit an octabyte. Divide by zero is that case, and is not an
        // event: rD is a definition, not a divide check.
        if divisor <= dividend_high as u128 {
            $cpu.set_register($x, dividend_high);
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
        // The quotient is defined only when u($Z) > u(rD); otherwise it would
        // not fit an octabyte. Divide by zero is that case, and is not an
        // event: rD is a definition, not a divide check.
        if divisor <= dividend_high as u128 {
            $cpu.set_register($x, dividend_high);
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
    ($cpu:expr, $op:expr, $x:expr, $y:expr, $z:expr) => {{
        let y_val = $cpu.get_register($y);
        let z_val = $cpu.get_register($z);
        let a = y_val as i64;
        let b = z_val as i64;
        let flags = match a.checked_add(b) {
            Some(result) => {
                $cpu.set_register($x, result as u64);
                0
            }
            None => {
                $cpu.set_register($x, a.wrapping_add(b) as u64);
                RA_V
            }
        };
        return $cpu.raise_exceptions(flags, $op, $x, $y, $z, y_val, z_val);
    }};
}

/// Macro for signed addition with overflow detection (register-immediate)
macro_rules! add_ri {
    ($cpu:expr, $op:expr, $x:expr, $y:expr, $z:expr) => {{
        let y_val = $cpu.get_register($y);
        // Z is the literal operand, not a register.
        let z_val = $z as u64;
        let a = y_val as i64;
        let b = $z as i64;
        let flags = match a.checked_add(b) {
            Some(result) => {
                $cpu.set_register($x, result as u64);
                0
            }
            None => {
                $cpu.set_register($x, a.wrapping_add(b) as u64);
                RA_V
            }
        };
        return $cpu.raise_exceptions(flags, $op, $x, $y, $z, y_val, z_val);
    }};
}

/// Macro for signed subtraction with overflow detection (register-register)
macro_rules! sub_rr {
    ($cpu:expr, $op:expr, $x:expr, $y:expr, $z:expr) => {{
        let y_val = $cpu.get_register($y);
        let z_val = $cpu.get_register($z);
        let a = y_val as i64;
        let b = z_val as i64;
        let flags = match a.checked_sub(b) {
            Some(result) => {
                $cpu.set_register($x, result as u64);
                0
            }
            None => {
                $cpu.set_register($x, a.wrapping_sub(b) as u64);
                RA_V
            }
        };
        return $cpu.raise_exceptions(flags, $op, $x, $y, $z, y_val, z_val);
    }};
}

/// Macro for signed subtraction with overflow detection (register-immediate)
macro_rules! sub_ri {
    ($cpu:expr, $op:expr, $x:expr, $y:expr, $z:expr) => {{
        let y_val = $cpu.get_register($y);
        // Z is the literal operand, not a register.
        let z_val = $z as u64;
        let a = y_val as i64;
        let b = $z as i64;
        let flags = match a.checked_sub(b) {
            Some(result) => {
                $cpu.set_register($x, result as u64);
                0
            }
            None => {
                $cpu.set_register($x, a.wrapping_sub(b) as u64);
                RA_V
            }
        };
        return $cpu.raise_exceptions(flags, $op, $x, $y, $z, y_val, z_val);
    }};
}
