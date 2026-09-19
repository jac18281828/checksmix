//! rA exception routing, store-overflow flags, and TRIP/trap-vector transfer.

use super::{MMix, RA_D, RA_I, RA_O, RA_U, RA_V, RA_W, RA_X, RA_Z, SpecialReg};

impl MMix {
    /// Route every bit `flags` raises through rA's enable byte
    /// (gitraptrip.html "General"; §1 rules 1-2). A raised bit whose enable
    /// is clear sets its event bit and execution continues normally. The
    /// leftmost raised bit in `D V W I O U Z X` priority whose enable is set
    /// instead trips to its handler; any other raised bit that is enabled
    /// but not leftmost is dropped — no event, no trip (owner, 2026-09-18).
    ///
    /// `op_byte`, `x`, `y`, `z` are the raising instruction's own fields, and
    /// `y_val`/`z_val` its `$Y`/`$Z`, captured by the caller before any
    /// destination write — in `ADD $5,$5,$3` the destination is also a
    /// source. This call owns the program counter: it advances it when
    /// nothing trips, and otherwise leaves it to [`MMix::trip`]. Returns
    /// whether execution continues.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn raise_exceptions(
        &mut self,
        flags: u64,
        op_byte: u8,
        x: u8,
        y: u8,
        z: u8,
        y_val: u64,
        z_val: u64,
    ) -> bool {
        if flags == 0 {
            self.advance_pc();
            return true;
        }
        let enable = (self.get_special(SpecialReg::RA) >> 8) & 0xFF;
        let mut tripped = None;
        let mut events = 0u64;
        for bit in [RA_D, RA_V, RA_W, RA_I, RA_O, RA_U, RA_Z, RA_X] {
            if flags & bit == 0 {
                continue;
            }
            if tripped.is_none() && enable & bit != 0 {
                tripped = Some(bit);
            } else if enable & bit == 0 {
                events |= bit;
            }
        }
        if events != 0 {
            let ra = self.get_special(SpecialReg::RA);
            self.set_special(SpecialReg::RA, ra | events);
        }
        match tripped {
            Some(bit) => {
                let (vector, label) = Self::arithmetic_vector(bit);
                self.trip(vector, label, op_byte, x, y, z, y_val, z_val)
            }
            None => {
                self.advance_pc();
                true
            }
        }
    }

    /// Handler vector and diagnostic label for a tripped rA exception bit:
    /// `D V W I O U Z X` to `#10 #20 #30 #40 #50 #60 #70 #80`
    /// (gitraptrip.html, "TRIP" section).
    fn arithmetic_vector(bit: u64) -> (u64, &'static str) {
        match bit {
            RA_D => (0x10, "D"),
            RA_V => (0x20, "V"),
            RA_W => (0x30, "W"),
            RA_I => (0x40, "I"),
            RA_O => (0x50, "O"),
            RA_U => (0x60, "U"),
            RA_Z => (0x70, "Z"),
            RA_X => (0x80, "X"),
            _ => unreachable!("raise_exceptions only ever passes an rA exception bit"),
        }
    }

    /// `RA_V` when a store's value, read as signed, does not fit `lo..=hi`,
    /// the destination width's range; otherwise no flag.
    pub(super) fn store_overflow_flag(value: u64, lo: i64, hi: i64) -> u64 {
        if (lo..=hi).contains(&(value as i64)) {
            0
        } else {
            RA_V
        }
    }

    /// The aligned octabyte containing `addr`, read after a store has
    /// written it: the stored bytes in place, the rest as memory holds them
    /// (gitraptrip.html "General"). A store trip's `rZ` per §1 rule 3.
    pub(super) fn merged_store_octa(&self, addr: u64) -> u64 {
        self.read_octa(addr)
    }

    /// Whether every byte of the tetra at `addr` came from `write_image` (or
    /// a test's `write_loaded_byte`) rather than reading as zero by default.
    /// Backs the unloaded-vector halt in [`MMix::trip`]: unassembled memory
    /// decodes as `TRAP 0,0,0`, a silent zero-exit halt that would hide the
    /// fault a trip to that vector was supposed to report.
    fn vector_loaded(&self, addr: u64) -> bool {
        (0..4).all(|offset| self.loaded.contains(&(addr + offset)))
    }

    /// Transfer control to a `TRIP` or arithmetic-exception handler
    /// (gitraptrip.html "TRIP"; §1 rule 3): `rB` takes `$255`, `$255` takes
    /// `rJ`, `rW` takes the address following the raising instruction, `rX`
    /// takes `#80000000` with that instruction's own opcode/X/Y/Z, and
    /// `rY`/`rZ` take `y_val`/`z_val`. PC then jumps to `vector`.
    ///
    /// A vector nothing ever loaded halts with a diagnostic naming `label`
    /// and `vector`, PC unmoved and a nonzero exit code, after every
    /// register above is set — so a debugger sees why. This does not route
    /// through [`MMix::reject`]: that helper promises no prior state change,
    /// which does not hold here by design.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn trip(
        &mut self,
        vector: u64,
        label: &str,
        op_byte: u8,
        x: u8,
        y: u8,
        z: u8,
        y_val: u64,
        z_val: u64,
    ) -> bool {
        let next_pc = self.pc.wrapping_add(4);
        self.set_special(SpecialReg::RB, self.get_register(255));
        self.set_register(255, self.get_special(SpecialReg::RJ));
        self.set_special(SpecialReg::RW, next_pc);
        let rx = 0x8000_0000_0000_0000u64
            | ((op_byte as u64) << 24)
            | ((x as u64) << 16)
            | ((y as u64) << 8)
            | z as u64;
        self.set_special(SpecialReg::RX, rx);
        self.set_special(SpecialReg::RY, y_val);
        self.set_special(SpecialReg::RZ, z_val);
        if !self.vector_loaded(vector) {
            self.exit_code = 1;
            self.host.diagnostic(&format!(
                "{label} trip to unloaded vector {vector:#04x} at PC={:#018x}",
                self.pc
            ));
            return false;
        }
        self.pc = vector;
        true
    }
}
