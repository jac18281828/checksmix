//! `ValueFormat`, `MMixDisplay`, and the `Display` impls for `MMix` and its state dump.

use super::{MMix, SpecialReg};
use std::fmt;

impl fmt::Display for MMix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_with_mode(f, ValueFormat::Signed)
    }
}

#[derive(Copy, Clone)]
#[non_exhaustive]
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
