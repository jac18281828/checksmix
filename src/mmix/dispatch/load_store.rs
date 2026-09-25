//! The load/store family: LDB through SYNCIDI, excluding GO/GOI, which
//! transfer control and dispatch to `control.rs`.

use super::super::{MMix, SpecialReg};
use crate::mmixal::Opcode;

impl MMix {
    /// Dispatches the load/store family; `dispatch` routes only these
    /// opcodes here.
    pub(super) fn dispatch_load_store(
        &mut self,
        opcode: Opcode,
        op_byte: u8,
        x: u8,
        y: u8,
        z: u8,
    ) -> bool {
        match opcode {
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
                self.set_register(x, Self::widen_short_float(tetra));
                self.advance_pc();
                true
            }
            Opcode::LDSFI => {
                // LDSFI $X, $Y, Z - Load short float immediate (32-bit float to 64-bit)
                let addr = self.get_register(y).wrapping_add(z as u64);
                let tetra = self.read_tetra(addr);
                self.set_register(x, Self::widen_short_float(tetra));
                self.advance_pc();
                true
            }
            // Special Load/Store instructions (0x92-0x9D)
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
                // $X gets zero.
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
                let (bits, flags) = self.narrow_for_store(value);
                self.write_tetra(addr, bits);
                let merged = self.merged_store_octa(addr);
                self.raise_exceptions(flags, op_byte, x, y, z, addr, merged)
            }
            Opcode::STSFI => {
                // A store trip: rY takes the address, rZ the merged octabyte
                // after the store, per §1 rule 3.
                let addr = self.get_register(y).wrapping_add(z as u64);
                let value = Self::u64_to_f64(self.get_register(x));
                let (bits, flags) = self.narrow_for_store(value);
                self.write_tetra(addr, bits);
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
            _ => unreachable!("dispatch routes only load/store opcodes to dispatch_load_store"),
        }
    }
}
