//! MMIX object (`.mmo`) file generation and reading, in the shape the
//! MMIXAL reference defines.
//!
//! ## What the writer emits
//!
//! - **Preamble**: `#98090101`, then one tetra of creation time, always
//!   written as 0 (a build is reproducible from its source: two builds of
//!   the same input produce the same bytes).
//! - **Location**: `#98010002`, then the address's high and low tetras, a
//!   multiple of 4.
//! - **Data**: plain tetras (no `MM` prefix) loaded at the current address
//!   before it advances by 4; a byte the program did not assemble inside a
//!   tetra is 0.
//! - **Escape**: `#98000001` before any data tetra whose first byte would
//!   otherwise collide with `MM` -- the one place `lop_quote` appears, and
//!   always with a count of exactly 1.
//! - **Special**: a `debug` directive's string table, one `lop_spec` record
//!   per string, type [`LOP_SPEC_DEBUG_STRING`], length-prefixed and
//!   zero-padded to a tetra boundary.
//! - **Postamble**: `#980A00`*G*, then `256 - G` octabytes (high tetra
//!   first) for `$G..=$255` -- *G* is the rG [`crate::write_image`] derives
//!   from the same `GREG` pairs [`MmoGenerator::with_greg_inits`] carries,
//!   and `$255` always holds the entry point.
//!
//! ## What the reader rejects
//!
//! [`MmoDecoder::decode`] and [`MmoDecoder::load`] never misload: a file
//! that doesn't match a shape the writer above emits is an `Err`, not a
//! best-effort guess. That covers a lopcode other than `lop_quote`,
//! `lop_loc`, `lop_spec` or `lop_post` after the preamble; any of those four
//! in a shape the writer doesn't produce (a `lop_quote` count other than 1,
//! a `lop_loc` other than `#98010002`, a `lop_spec` of any type but the
//! debug-string one, a `lop_post` with a nonzero Y or a *G* below 32); a
//! preamble whose version isn't 1; and a file that ends mid-record. Reading
//! stops once the postamble's octabytes are consumed -- a symbol table
//! after them, or `lop_file`/`lop_line`/fixup records from an MMIXAL build,
//! are never read. A `.mmo` from before this reader (its preamble reads
//! `#98090001`, version 0) is rejected and must be rebuilt.

use crate::mmix::{MMix, SpecialReg};
use crate::mmixal::MMixInstruction;
use std::cell::RefCell;
use std::collections::HashMap;
use tracing::debug;

use crate::encode::encode_instruction_bytes;

/// MMO escape code - all MMO files must start with this
pub const MM: u8 = 0x98;

/// `lop_spec`'s type field for a `debug` directive's string table: the only
/// `lop_spec` type the reader accepts.
pub const LOP_SPEC_DEBUG_STRING: u16 = 0x4442;

/// The rG value MMIX derives from a program's `GREG` initializers: the
/// lowest-numbered register `GREG` allocated, floored at 32 (registers
/// below 32 are always local), or 255 when the program declares none.
/// Shared by [`crate::write_image`] and [`MmoGenerator::generate`], so the
/// register a loaded program starts global at agrees on both paths.
pub(crate) fn derive_rg(greg_inits: &[(u8, u64)]) -> u8 {
    greg_inits
        .iter()
        .map(|&(reg, _)| reg)
        .min()
        .map_or(255, |min_reg| std::cmp::max(min_reg, 32))
}

/// MMO record types (lopcodes), as the MMIXAL reference defines them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[allow(clippy::enum_variant_names)]
pub enum MmoRecordType {
    /// lop_quote (0): Literal data to load at current address
    LopQuote = 0,
    /// lop_loc (1): Set loading address
    LopLoc = 1,
    /// lop_skip (2): Advance current address
    LopSkip = 2,
    /// lop_fixo (3): Octabyte-fix lopcode
    LopFixo = 3,
    /// lop_fixr (4): Relative-fix lopcode
    LopFixr = 4,
    /// lop_fixrx (5): Extended relative-fix lopcode
    LopFixrx = 5,
    /// lop_file (6): File name lopcode
    LopFile = 6,
    /// lop_line (7): File position lopcode
    LopLine = 7,
    /// lop_spec (8): Special hook lopcode
    LopSpec = 8,
    /// lop_pre (9): Preamble lopcode
    LopPre = 9,
    /// lop_post (10): Postamble lopcode
    LopPost = 10,
    /// lop_stab (11): Symbol table lopcode
    LopStab = 11,
    /// lop_end (12): End-it-all lopcode
    LopEnd = 12,
}

impl TryFrom<u8> for MmoRecordType {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x0 => Ok(MmoRecordType::LopQuote),
            0x1 => Ok(MmoRecordType::LopLoc),
            0x2 => Ok(MmoRecordType::LopSkip),
            0x3 => Ok(MmoRecordType::LopFixo),
            0x4 => Ok(MmoRecordType::LopFixr),
            0x5 => Ok(MmoRecordType::LopFixrx),
            0x6 => Ok(MmoRecordType::LopFile),
            0x7 => Ok(MmoRecordType::LopLine),
            0x8 => Ok(MmoRecordType::LopSpec),
            0x9 => Ok(MmoRecordType::LopPre),
            0xa => Ok(MmoRecordType::LopPost),
            0xb => Ok(MmoRecordType::LopStab),
            0xc => Ok(MmoRecordType::LopEnd),
            _ => Err(format!("Unknown MMO lopcode: 0x{:02X}", value)),
        }
    }
}

/// MMO file generator
pub struct MmoGenerator {
    /// Instructions to encode, sorted by address
    instructions: Vec<(u64, MMixInstruction)>,
    /// Symbol table (labels)
    labels: HashMap<String, u64>,
    /// The `debug` directive string table, `K`-indexed, written as
    /// `lop_spec` records ahead of `lop_post`. Empty unless
    /// `with_debug_strings` was called.
    debug_strings: Vec<Vec<u8>>,
    /// `GREG` register/value pairs the postamble carries, in the shape
    /// [`crate::debugger::write_image`] applies. Empty unless
    /// `with_greg_inits` was called, in which case the postamble carries
    /// `$255`'s entry alongside them.
    greg_inits: Vec<(u8, u64)>,
}

impl MmoGenerator {
    /// Create a new MMO generator
    pub fn new(instructions: Vec<(u64, MMixInstruction)>, labels: HashMap<String, u64>) -> Self {
        Self {
            instructions,
            labels,
            debug_strings: Vec::new(),
            greg_inits: Vec::new(),
        }
    }

    /// Attach the `debug` directive string table this object file's
    /// `lop_spec` records should carry, in `K` order.
    pub fn with_debug_strings(mut self, strings: Vec<Vec<u8>>) -> Self {
        self.debug_strings = strings;
        self
    }

    /// Attach the `GREG` register/value pairs the postamble should carry.
    /// *G*, the postamble's starting register, becomes the rG
    /// [`crate::write_image`] would derive from the same pairs; `$G..$254`
    /// carry their values and `$255` the entry point. With none supplied,
    /// *G* stays 255 and the postamble carries only the entry, as before.
    pub fn with_greg_inits(mut self, inits: Vec<(u8, u64)>) -> Self {
        self.greg_inits = inits;
        self
    }

    /// Generate the object file: preamble, one `lop_loc` per contiguous run
    /// of assembled bytes followed by that run's data tetras (escaped where
    /// a tetra would otherwise collide with `MM`), the `debug` string
    /// table, then the postamble.
    pub fn generate(&self) -> Vec<u8> {
        debug!("Generating MMIX object code (.mmo format)");
        let mut mmo = Vec::new();

        self.emit_preamble(&mut mmo);

        // Group instructions by contiguous address ranges
        let mut sorted_instructions: Vec<_> = self.instructions.iter().collect();
        sorted_instructions.sort_by_key(|(addr, _)| *addr);

        let mut current_loc: Option<u64> = None;
        let mut pending_bytes = Vec::new();

        for (addr, instruction) in sorted_instructions {
            let addr = *addr;
            let bytes = encode_instruction_bytes(instruction);

            let need_new_loc = current_loc != Some(addr);
            if need_new_loc {
                if !pending_bytes.is_empty() {
                    self.emit_data_tetras(&mut mmo, &pending_bytes);
                    pending_bytes.clear();
                }
                self.emit_lop_loc(&mut mmo, addr);
            }

            pending_bytes.extend_from_slice(&bytes);
            current_loc = Some(addr + bytes.len() as u64);
        }

        if !pending_bytes.is_empty() {
            self.emit_data_tetras(&mut mmo, &pending_bytes);
        }

        for text in &self.debug_strings {
            self.emit_debug_string(&mut mmo, text);
        }

        // Find Main label or use first instruction address
        let entry_point = self
            .labels
            .get("Main")
            .or_else(|| {
                self.instructions
                    .iter()
                    .find(|(addr, _)| *addr < 0x2000000000000000)
                    .map(|(addr, _)| addr)
            })
            .copied()
            .unwrap_or(0x100);

        self.emit_lop_post(&mut mmo, entry_point);

        debug!("Generated {} bytes of .mmo object code", mmo.len());
        mmo
    }

    /// Emit the preamble: `#98090101` (lop_pre, version 1, one tetra
    /// follows), then that tetra -- the creation time, always 0.
    fn emit_preamble(&self, mmo: &mut Vec<u8>) {
        mmo.push(MM);
        mmo.push(MmoRecordType::LopPre as u8);
        mmo.push(0x01); // Y = version 1
        mmo.push(0x01); // Z = one tetra follows
        mmo.extend_from_slice(&0u32.to_be_bytes());
    }

    /// Emit `bytes` (already assembled, contiguous from the current
    /// address) as plain data tetras: no `MM` prefix, except a tetra whose
    /// first byte is `MM`, which `#98000001` (`lop_quote`, count 1) escapes.
    /// A trailing partial tetra is zero-padded.
    fn emit_data_tetras(&self, mmo: &mut Vec<u8>, bytes: &[u8]) {
        for chunk in bytes.chunks(4) {
            let mut tetra = [0u8; 4];
            tetra[..chunk.len()].copy_from_slice(chunk);
            if tetra[0] == MM {
                mmo.push(MM);
                mmo.push(MmoRecordType::LopQuote as u8);
                mmo.push(0x00);
                mmo.push(0x01);
            }
            mmo.extend_from_slice(&tetra);
        }
    }

    /// Emit one `debug` directive's string as a `lop_spec` record: the type
    /// field `LOP_SPEC_DEBUG_STRING`, then a payload tetra holding the byte
    /// length, then the bytes zero-padded to a tetra boundary. A payload
    /// tetra whose first byte collides with `MM` is preceded by `lop_quote`,
    /// as the object format requires of every data tetra.
    fn emit_debug_string(&self, mmo: &mut Vec<u8>, text: &[u8]) {
        mmo.push(MM);
        mmo.push(MmoRecordType::LopSpec as u8);
        mmo.push((LOP_SPEC_DEBUG_STRING >> 8) as u8);
        mmo.push((LOP_SPEC_DEBUG_STRING & 0xFF) as u8);

        let mut payload = (text.len() as u32).to_be_bytes().to_vec();
        payload.extend_from_slice(text);
        while !payload.len().is_multiple_of(4) {
            payload.push(0);
        }

        for tetra in payload.as_chunks::<4>().0 {
            if tetra[0] == MM {
                mmo.push(MM);
                mmo.push(MmoRecordType::LopQuote as u8);
                mmo.push(0x00);
                mmo.push(0x01);
            }
            mmo.extend_from_slice(tetra);
        }
    }

    /// Emit lop_loc: set current loading address
    /// Format: MM lop_loc YZ X (4 bytes), followed by 2 tetras (8 bytes) for 64-bit address
    /// Total: 12 bytes
    fn emit_lop_loc(&self, mmo: &mut Vec<u8>, addr: u64) {
        // Record header: MM lop_loc with YZ=2 (two tetras of address data follow)
        mmo.push(MM); // MM escape code
        mmo.push(MmoRecordType::LopLoc as u8); // lop_loc
        mmo.push(0x00); // Y
        mmo.push(0x02); // Z = 2 (two tetras follow)

        // Tetra 1: high 32 bits
        let high = (addr >> 32) as u32;
        mmo.extend_from_slice(&high.to_be_bytes());

        // Tetra 2: low 32 bits
        let low = (addr & 0xFFFFFFFF) as u32;
        mmo.extend_from_slice(&low.to_be_bytes());
    }

    /// Emit `lop_post`: `#980A00`*G*, then one octabyte per register from
    /// *G* through `$255` -- a `GREG`-initialized register carries its
    /// value, `$255` carries `entry_point`, and every other register in the
    /// range carries 0. *G* is [`derive_rg`] applied to `self.greg_inits`.
    fn emit_lop_post(&self, mmo: &mut Vec<u8>, entry_point: u64) {
        let g = derive_rg(&self.greg_inits);
        let mut values: HashMap<u8, u64> = self.greg_inits.iter().copied().collect();
        values.insert(255, entry_point);

        mmo.push(MM);
        mmo.push(MmoRecordType::LopPost as u8);
        mmo.push(0x00); // Y must be 0
        mmo.push(g); // Z = G: the first register this record carries

        for reg in g..=255u8 {
            let value = values.get(&reg).copied().unwrap_or(0);
            mmo.extend_from_slice(&((value >> 32) as u32).to_be_bytes());
            mmo.extend_from_slice(&(value as u32).to_be_bytes());
        }
    }
}

/// MMO file decoder
pub struct MmoDecoder {
    data: Vec<u8>,
    /// The `debug` directive string table `decode` collected from any
    /// `lop_spec` records of type `LOP_SPEC_DEBUG_STRING`, `K`-indexed.
    /// Interior-mutable because `decode` takes `&self`.
    debug_strings: RefCell<Vec<Vec<u8>>>,
}

impl MmoDecoder {
    /// Create a new MMO decoder
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            data,
            debug_strings: RefCell::new(Vec::new()),
        }
    }

    /// The `debug` directive string table the last `decode` call collected,
    /// in `K` order.
    pub fn debug_strings(&self) -> Vec<Vec<u8>> {
        self.debug_strings.borrow().clone()
    }

    /// Big-endian decode of exactly 4 bytes. Every call site has already
    /// bounds-checked its slice to length 4, so the conversion cannot fail.
    fn be_u32(bytes: &[u8]) -> u32 {
        u32::from_be_bytes(bytes.try_into().expect("caller passes exactly 4 bytes"))
    }

    /// Read one payload tetra of a `lop_spec` record at `self.data[pos..]`,
    /// honoring the `lop_quote` escape a tetra whose first byte collides
    /// with `MM` requires. Returns the tetra and how many bytes it and any
    /// escape consumed, or `None` past the end of the file.
    fn read_payload_tetra(&self, pos: usize) -> Option<([u8; 4], usize)> {
        if pos + 4 > self.data.len() {
            return None;
        }
        if self.data[pos] == MM {
            let quoted = pos + 4;
            if quoted + 4 > self.data.len() {
                return None;
            }
            let tetra = [
                self.data[quoted],
                self.data[quoted + 1],
                self.data[quoted + 2],
                self.data[quoted + 3],
            ];
            Some((tetra, 8))
        } else {
            let tetra = [
                self.data[pos],
                self.data[pos + 1],
                self.data[pos + 2],
                self.data[pos + 3],
            ];
            Some((tetra, 4))
        }
    }

    /// An `unsupported lopcode` error naming `lopcode`'s own record, at the
    /// offset of that record's `MM` byte.
    fn unsupported_lopcode(lopcode: u8, offset: usize) -> String {
        format!(".mmo: unsupported lopcode #{lopcode:02x} at offset {offset:#x}")
    }

    /// A `file ends inside a record` error at `offset`, the offset of the
    /// record (or data tetra) that ran out of bytes.
    fn truncated(offset: usize) -> String {
        format!(".mmo: file ends inside a record at offset {offset:#x}")
    }

    /// Parse the file, calling `write_byte` for every loaded byte in file
    /// order -- a doubly assembled address is called twice, once per
    /// occurrence, XOR-combining it is the caller's job. Returns the entry
    /// point and the postamble's register range (*G*, and the values from
    /// *G* through `$255`) on success. Rejects anything that isn't a shape
    /// [`MmoGenerator::generate`] emits; see the module docs.
    fn parse<F>(&self, write_byte: &mut F) -> Result<(u64, u8, HashMap<u8, u64>), String>
    where
        F: FnMut(u64, u8),
    {
        debug!("Decoding MMIX object code (.mmo format)");
        self.debug_strings.borrow_mut().clear();
        let data = &self.data;

        if data.len() < 4 || data[0] != MM || data[1] != MmoRecordType::LopPre as u8 {
            return Err(".mmo: file does not begin with lop_pre".to_string());
        }
        let version = data[2];
        let pre_z = data[3];
        if version != 1 {
            return Err(format!(".mmo: lop_pre version {version}, expected 1"));
        }
        if pre_z != 1 {
            return Err(Self::unsupported_lopcode(MmoRecordType::LopPre as u8, 0));
        }
        if data.len() < 8 {
            return Err(Self::truncated(0));
        }
        // Bytes 4..8: the creation-time tetra, always 0, never read.
        let mut i = 8usize;
        let mut current_addr = 0u64;

        loop {
            if i >= data.len() {
                return Err(".mmo: no lop_post".to_string());
            }

            if data[i] != MM {
                if i + 4 > data.len() {
                    return Err(Self::truncated(i));
                }
                for k in 0..4u64 {
                    write_byte(current_addr + k, data[i + k as usize]);
                }
                current_addr += 4;
                i += 4;
                continue;
            }

            let record_offset = i;
            if i + 4 > data.len() {
                return Err(Self::truncated(record_offset));
            }
            let lopcode_byte = data[i + 1];
            let y = data[i + 2];
            let z = data[i + 3];

            match MmoRecordType::try_from(lopcode_byte) {
                Ok(MmoRecordType::LopQuote) => {
                    let yz = ((y as u16) << 8) | z as u16;
                    if yz != 1 {
                        return Err(format!(
                            ".mmo: lop_quote count {yz}, expected 1 at offset {record_offset:#x}"
                        ));
                    }
                    i += 4;
                    if i + 4 > data.len() {
                        return Err(Self::truncated(record_offset));
                    }
                    for k in 0..4u64 {
                        write_byte(current_addr + k, data[i + k as usize]);
                    }
                    current_addr += 4;
                    i += 4;
                }
                Ok(MmoRecordType::LopLoc) => {
                    if y != 0x00 || z != 0x02 {
                        return Err(Self::unsupported_lopcode(lopcode_byte, record_offset));
                    }
                    i += 4;
                    if i + 8 > data.len() {
                        return Err(Self::truncated(record_offset));
                    }
                    let high = Self::be_u32(&data[i..i + 4]);
                    let low = Self::be_u32(&data[i + 4..i + 8]);
                    current_addr = ((high as u64) << 32) | low as u64;
                    i += 8;
                }
                Ok(MmoRecordType::LopSpec) => {
                    let record_type = ((y as u16) << 8) | z as u16;
                    if record_type != LOP_SPEC_DEBUG_STRING {
                        return Err(Self::unsupported_lopcode(lopcode_byte, record_offset));
                    }
                    i += 4;
                    let Some((len_tetra, consumed)) = self.read_payload_tetra(i) else {
                        return Err(Self::truncated(record_offset));
                    };
                    i += consumed;
                    let byte_len = u32::from_be_bytes(len_tetra) as usize;
                    let padded_len = byte_len.div_ceil(4) * 4;

                    let mut payload = Vec::with_capacity(padded_len);
                    let mut remaining = padded_len;
                    while remaining > 0 {
                        let Some((tetra, consumed)) = self.read_payload_tetra(i) else {
                            return Err(Self::truncated(record_offset));
                        };
                        i += consumed;
                        payload.extend_from_slice(&tetra);
                        remaining -= 4;
                    }
                    payload.truncate(byte_len);
                    self.debug_strings.borrow_mut().push(payload);
                }
                Ok(MmoRecordType::LopPost) => {
                    if y != 0x00 {
                        return Err(Self::unsupported_lopcode(lopcode_byte, record_offset));
                    }
                    let g = z;
                    if g < 32 {
                        return Err(format!(
                            ".mmo: lop_post G={g}, below 32 at offset {record_offset:#x}"
                        ));
                    }
                    i += 4;
                    let mut registers = HashMap::new();
                    let mut reg = g;
                    loop {
                        if i + 8 > data.len() {
                            return Err(Self::truncated(record_offset));
                        }
                        let high = Self::be_u32(&data[i..i + 4]);
                        let low = Self::be_u32(&data[i + 4..i + 8]);
                        registers.insert(reg, ((high as u64) << 32) | low as u64);
                        i += 8;
                        if reg == 255 {
                            break;
                        }
                        reg += 1;
                    }
                    let entry_point = registers.get(&255).copied().unwrap_or(0);
                    debug!("Decoded .mmo file, entry point: 0x{:X}", entry_point);
                    return Ok((entry_point, g, registers));
                }
                _ => return Err(Self::unsupported_lopcode(lopcode_byte, record_offset)),
            }
        }
    }

    /// Decode the object file, calling `write_byte` for every loaded byte
    /// in file order, and return the entry point (`$255`'s postamble
    /// value) on success. `Err` on any shape [`MmoGenerator::generate`]
    /// would not emit; see the module docs for every rejected shape.
    pub fn decode<F>(&self, mut write_byte: F) -> Result<u64, String>
    where
        F: FnMut(u64, u8),
    {
        self.parse(&mut write_byte).map(|(entry, _, _)| entry)
    }

    /// Load the object file into `mmix`: every byte through
    /// [`MMix::write_loaded_byte`] (so an address assembled twice combines
    /// by XOR, matching [`crate::debugger::write_image`]), the `debug`
    /// string table, rG set to the postamble's *G*, and `$G..=$255` set to
    /// the postamble's values without raising rL (every register in that
    /// range is `>= rG`). Returns the entry point. On `Err`, `mmix` is
    /// untouched -- parsing runs to completion before anything is applied.
    pub fn load(&self, mmix: &mut MMix) -> Result<u64, String> {
        let mut combined: HashMap<u64, u8> = HashMap::new();
        let mut collect = |addr: u64, byte: u8| {
            *combined.entry(addr).or_insert(0) ^= byte;
        };
        let (entry, g, registers) = self.parse(&mut collect)?;

        for (addr, byte) in combined {
            mmix.write_loaded_byte(addr, byte);
        }
        mmix.set_debug_strings(self.debug_strings());
        mmix.set_special(SpecialReg::RG, g as u64);
        for reg in g..=255u8 {
            mmix.set_register(reg, registers.get(&reg).copied().unwrap_or(0));
        }
        Ok(entry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mmixal::MMixInstruction;

    #[test]
    fn test_record_type_enum() {
        // Test conversion from u8 to MmoRecordType
        assert_eq!(MmoRecordType::try_from(0).unwrap(), MmoRecordType::LopQuote);
        assert_eq!(MmoRecordType::try_from(1).unwrap(), MmoRecordType::LopLoc);
        assert_eq!(MmoRecordType::try_from(2).unwrap(), MmoRecordType::LopSkip);
        assert_eq!(MmoRecordType::try_from(9).unwrap(), MmoRecordType::LopPre);
        assert_eq!(MmoRecordType::try_from(10).unwrap(), MmoRecordType::LopPost);
        assert_eq!(MmoRecordType::try_from(12).unwrap(), MmoRecordType::LopEnd);

        // Test invalid record type
        assert!(MmoRecordType::try_from(13).is_err());
        assert!(MmoRecordType::try_from(255).is_err());
    }

    #[test]
    fn test_mm_escape_code() {
        // Verify MM constant is correct
        assert_eq!(MM, 0x98);
    }

    /// The preamble the writer emits: `#98090101`, then a zero timestamp
    /// tetra -- byte-exact, since C9's proofs compare `.mmo` files built at
    /// different times.
    #[test]
    fn preamble_is_version_one_with_a_zero_timestamp() {
        let mmo_data = MmoGenerator::new(Vec::new(), HashMap::new()).generate();
        assert_eq!(&mmo_data[0..8], &[0x98, 0x09, 0x01, 0x01, 0, 0, 0, 0]);
    }

    #[test]
    fn test_mmo_encode_decode_simple() {
        // Create a simple program with one instruction
        // Note: SET expands to 4 instructions (SETH, SETMH, SETML, SETL) = 16 bytes
        let instructions = vec![(0x100, MMixInstruction::SETL(1, 42))];
        let labels = HashMap::new();

        let generator = MmoGenerator::new(instructions, labels);
        let mmo_data = generator.generate();

        // Verify the MMO data starts with MM escape code and preamble
        assert_eq!(mmo_data[0], MM);
        assert_eq!(mmo_data[1], MmoRecordType::LopPre as u8);

        // Decode and verify
        let decoder = MmoDecoder::new(mmo_data);
        let mut memory = HashMap::new();
        let entry_point = decoder
            .decode(|addr, byte| {
                memory.insert(addr, byte);
            })
            .expect("well-formed object code");

        // Verify entry point
        assert_eq!(entry_point, 0x100);

        // Verify instruction was loaded at 0x100
        // SETL $1, 42 = E3 01 00 2A
        assert_eq!(memory.get(&0x100), Some(&0xE3));
        assert_eq!(memory.get(&0x101), Some(&0x01));
        assert_eq!(memory.get(&0x102), Some(&0x00));
        assert_eq!(memory.get(&0x103), Some(&0x2A));
    }

    #[test]
    fn test_mmo_encode_decode_multiple_instructions() {
        // Create a program with multiple contiguous instructions
        let instructions = vec![
            (0x100, MMixInstruction::SETL(1, 10)),
            (0x104, MMixInstruction::SETL(2, 20)),
            (0x108, MMixInstruction::ADD(3, 1, 2)),
        ];
        let labels = HashMap::new();

        let generator = MmoGenerator::new(instructions, labels);
        let mmo_data = generator.generate();

        // Decode
        let decoder = MmoDecoder::new(mmo_data);
        let mut memory = HashMap::new();
        decoder
            .decode(|addr, byte| {
                memory.insert(addr, byte);
            })
            .expect("well-formed object code");

        // Verify all three instructions are loaded
        assert_eq!(memory.len(), 12); // 3 instructions * 4 bytes each
        assert!(memory.contains_key(&0x100));
        assert!(memory.contains_key(&0x104));
        assert!(memory.contains_key(&0x108));
    }

    #[test]
    fn test_mmo_encode_decode_non_contiguous() {
        // Create a program with non-contiguous instructions (should emit multiple LOC records)
        let instructions = vec![
            (0x100, MMixInstruction::SET(1, 1)),
            (0x200, MMixInstruction::SET(2, 2)),
            (0x300, MMixInstruction::SET(3, 3)),
        ];
        let labels = HashMap::new();

        let generator = MmoGenerator::new(instructions, labels);
        let mmo_data = generator.generate();

        // Count LOC records - each should be preceded by MM
        let loc_count = mmo_data
            .windows(2)
            .filter(|w| w[0] == MM && w[1] == MmoRecordType::LopLoc as u8)
            .count();
        assert_eq!(loc_count, 3); // Should have 3 LOC records for 3 non-contiguous addresses

        // Decode and verify
        let decoder = MmoDecoder::new(mmo_data);
        let mut memory = HashMap::new();
        decoder
            .decode(|addr, byte| {
                memory.insert(addr, byte);
            })
            .expect("well-formed object code");

        // Verify all instructions are at correct addresses
        assert!(memory.contains_key(&0x100));
        assert!(memory.contains_key(&0x200));
        assert!(memory.contains_key(&0x300));
    }

    #[test]
    fn test_mmo_encode_decode_with_main_label() {
        // Create a program with Main label
        let instructions = vec![(0x1000, MMixInstruction::SET(1, 99))];
        let mut labels = HashMap::new();
        labels.insert("Main".to_string(), 0x1000);

        let generator = MmoGenerator::new(instructions, labels);
        let mmo_data = generator.generate();

        let decoder = MmoDecoder::new(mmo_data);
        let mut memory = HashMap::new();
        let entry_point = decoder
            .decode(|addr, byte| {
                memory.insert(addr, byte);
            })
            .expect("well-formed object code");

        // Entry point should be Main label address
        assert_eq!(entry_point, 0x1000);
    }

    #[test]
    fn test_mmo_encode_decode_64bit_addresses() {
        // Test with high addresses (data segment)
        let instructions = vec![
            (0x100, MMixInstruction::SET(1, 1)),
            (0x2000000000000000, MMixInstruction::BYTE(65)), // 'A' in data segment
        ];
        let labels = HashMap::new();

        let generator = MmoGenerator::new(instructions, labels);
        let mmo_data = generator.generate();

        let decoder = MmoDecoder::new(mmo_data);
        let mut memory = HashMap::new();
        decoder
            .decode(|addr, byte| {
                memory.insert(addr, byte);
            })
            .expect("well-formed object code");

        // Verify both low and high addresses are loaded
        assert!(memory.contains_key(&0x100));
        assert!(memory.contains_key(&0x2000000000000000));
        assert_eq!(memory.get(&0x2000000000000000), Some(&65));
    }

    #[test]
    fn test_mmo_lop_loc_format() {
        // Verify LOC record format is correct (12 bytes total: MM + lopcode + YZ + 8 bytes address)
        let instructions = vec![(0x123456789ABCDEF0, MMixInstruction::SET(1, 1))];
        let labels = HashMap::new();

        let generator = MmoGenerator::new(instructions, labels);
        let mmo_data = generator.generate();

        // Find the LOC record (MM followed by lop_loc)
        let loc_pos = mmo_data
            .windows(2)
            .position(|w| w[0] == MM && w[1] == MmoRecordType::LopLoc as u8)
            .expect("Should have LOC record");

        // Verify LOC record structure (12 bytes):
        // Byte 0: MM (0x98)
        // Byte 1: lop_loc (1)
        // Bytes 2-3: YZ (should be 2 for two tetras)
        // Bytes 4-7: high 32 bits
        // Bytes 8-11: low 32 bits
        assert_eq!(mmo_data[loc_pos], MM);
        assert_eq!(mmo_data[loc_pos + 1], MmoRecordType::LopLoc as u8);
        assert_eq!(mmo_data[loc_pos + 2], 0x00); // Y
        assert_eq!(mmo_data[loc_pos + 3], 0x02); // Z = 2 (two tetras follow)

        // Verify the address is stored correctly (big-endian)
        let high = u32::from_be_bytes([
            mmo_data[loc_pos + 4],
            mmo_data[loc_pos + 5],
            mmo_data[loc_pos + 6],
            mmo_data[loc_pos + 7],
        ]);
        let low = u32::from_be_bytes([
            mmo_data[loc_pos + 8],
            mmo_data[loc_pos + 9],
            mmo_data[loc_pos + 10],
            mmo_data[loc_pos + 11],
        ]);
        let reconstructed_addr = ((high as u64) << 32) | (low as u64);
        assert_eq!(reconstructed_addr, 0x123456789ABCDEF0);
    }

    /// Data shorter than a tetra is a plain (unescaped) tetra, zero-padded
    /// -- `lop_quote` never appears here since none of these bytes collide
    /// with `MM`.
    #[test]
    fn a_partial_trailing_tetra_is_zero_padded_plain_data() {
        let instructions = vec![
            (0x100, MMixInstruction::BYTE(1)),
            (0x101, MMixInstruction::BYTE(2)),
            (0x102, MMixInstruction::BYTE(3)),
            // 3 bytes should be padded to 4
        ];
        let labels = HashMap::new();

        let generator = MmoGenerator::new(instructions, labels);
        let mmo_data = generator.generate();

        assert!(
            !mmo_data
                .windows(2)
                .any(|w| w[0] == MM && w[1] == MmoRecordType::LopQuote as u8),
            "no byte here collides with MM, so no escape should appear"
        );

        let decoder = MmoDecoder::new(mmo_data);
        let mut memory = HashMap::new();
        decoder
            .decode(|addr, byte| {
                memory.insert(addr, byte);
            })
            .expect("well-formed object code");
        assert_eq!(memory.get(&0x100), Some(&1));
        assert_eq!(memory.get(&0x101), Some(&2));
        assert_eq!(memory.get(&0x102), Some(&3));
        assert_eq!(memory.get(&0x103), Some(&0), "the trailing pad byte is 0");
    }

    /// A data tetra whose first byte would read as `MM` is preceded by the
    /// `#98000001` escape, and the decoder still recovers the original byte.
    #[test]
    fn a_data_tetra_beginning_with_mm_is_escaped() {
        let instructions = vec![
            (0x100, MMixInstruction::BYTE(MM)),
            (0x101, MMixInstruction::BYTE(0x01)),
            (0x102, MMixInstruction::BYTE(0x02)),
            (0x103, MMixInstruction::BYTE(0x03)),
        ];
        let generator = MmoGenerator::new(instructions, HashMap::new());
        let mmo_data = generator.generate();

        assert!(
            mmo_data
                .windows(4)
                .any(|w| w == [MM, MmoRecordType::LopQuote as u8, 0x00, 0x01])
        );

        let decoder = MmoDecoder::new(mmo_data);
        let mut memory = HashMap::new();
        decoder
            .decode(|addr, byte| {
                memory.insert(addr, byte);
            })
            .expect("well-formed object code");
        assert_eq!(memory.get(&0x100), Some(&MM));
        assert_eq!(memory.get(&0x101), Some(&0x01));
        assert_eq!(memory.get(&0x102), Some(&0x02));
        assert_eq!(memory.get(&0x103), Some(&0x03));
    }

    /// A single `BYTE` at an address not aligned to a tetra boundary still
    /// lands at that exact address after a round trip.
    #[test]
    fn an_unaligned_byte_lands_at_its_address_after_a_round_trip() {
        let instructions = vec![(0x103, MMixInstruction::BYTE(0xAB))];
        let generator = MmoGenerator::new(instructions, HashMap::new());
        let mmo_data = generator.generate();

        let decoder = MmoDecoder::new(mmo_data);
        let mut memory = HashMap::new();
        decoder
            .decode(|addr, byte| {
                memory.insert(addr, byte);
            })
            .expect("well-formed object code");
        assert_eq!(memory.get(&0x103), Some(&0xAB));
    }

    #[test]
    fn test_mmo_roundtrip() {
        // Test that encode -> decode produces the same memory layout
        let instructions = vec![
            (0x100, MMixInstruction::SETL(1, 42)),
            (0x104, MMixInstruction::ADD(2, 1, 1)),
            (0x108, MMixInstruction::TRAP(0, 0, 0)),
        ];
        let labels = HashMap::new();

        // Encode
        let generator = MmoGenerator::new(instructions.clone(), labels);
        let mmo_data = generator.generate();

        // Decode
        let decoder = MmoDecoder::new(mmo_data);
        let mut memory = HashMap::new();
        decoder
            .decode(|addr, byte| {
                memory.insert(addr, byte);
            })
            .expect("well-formed object code");

        // Verify each instruction
        for (addr, inst) in &instructions {
            let bytes = encode_instruction_bytes(inst);
            for (offset, &expected_byte) in bytes.iter().enumerate() {
                assert_eq!(
                    memory.get(&(addr + offset as u64)),
                    Some(&expected_byte),
                    "Mismatch at address 0x{:X} offset {}",
                    addr,
                    offset
                );
            }
        }
    }

    #[test]
    fn test_debug_strings_round_trip_through_lop_spec_records() {
        let instructions = vec![(0x100, MMixInstruction::TRAP(0, 0, 0))];
        let strings = vec![b"one".to_vec(), b"two, longer".to_vec()];

        let generator =
            MmoGenerator::new(instructions, HashMap::new()).with_debug_strings(strings.clone());
        let mmo_data = generator.generate();

        let decoder = MmoDecoder::new(mmo_data);
        let mut memory = HashMap::new();
        decoder
            .decode(|addr, byte| {
                memory.insert(addr, byte);
            })
            .expect("well-formed object code");

        assert_eq!(decoder.debug_strings(), strings);
        // The instruction stream survives alongside the string table.
        assert!(memory.contains_key(&0x100));
    }

    #[test]
    fn test_debug_string_payload_starting_with_mm_is_quoted() {
        // A string whose bytes happen to start with the MM escape code: the
        // encoder must precede that payload tetra with lop_quote, and the
        // decoder must still recover the original bytes.
        let text = vec![MM, 0x02, 0x03, 0x04];

        let generator =
            MmoGenerator::new(Vec::new(), HashMap::new()).with_debug_strings(vec![text.clone()]);
        let mmo_data = generator.generate();

        // The escaped tetra shows up as MM, lop_quote, 00, 01, then MM again
        // (the payload byte itself), which a naive "every MM starts a
        // record" reader would misparse -- proof the generator emits it and
        // the decoder's own reader (below) consumes it correctly.
        assert!(
            mmo_data
                .windows(4)
                .any(|w| w == [MM, MmoRecordType::LopQuote as u8, 0x00, 0x01])
        );

        let decoder = MmoDecoder::new(mmo_data);
        decoder.decode(|_, _| {}).expect("well-formed object code");
        assert_eq!(decoder.debug_strings(), vec![text]);
    }

    /// `with_greg_inits` with one `GREG` derives *G* = 254 (floored at 32,
    /// same as `write_image`) and a postamble carrying `$254` and `$255`.
    #[test]
    fn with_greg_inits_one_greg_shapes_the_postamble() {
        let mmo_data = MmoGenerator::new(Vec::new(), HashMap::new())
            .with_greg_inits(vec![(254, 1000)])
            .generate();

        let post_pos = mmo_data
            .windows(2)
            .position(|w| w[0] == MM && w[1] == MmoRecordType::LopPost as u8)
            .expect("must have a lop_post record");
        assert_eq!(&mmo_data[post_pos..post_pos + 4], &[0x98, 0x0A, 0x00, 0xFE]);

        let reg254 = u64::from_be_bytes(mmo_data[post_pos + 4..post_pos + 12].try_into().unwrap());
        let reg255 = u64::from_be_bytes(mmo_data[post_pos + 12..post_pos + 20].try_into().unwrap());
        assert_eq!(reg254, 1000);
        assert_eq!(reg255, 0x100, "no instructions: entry falls back to 0x100");
        assert_eq!(
            mmo_data.len(),
            post_pos + 20,
            "exactly two octabytes follow"
        );
    }

    /// A hand-built object file in the reference's format -- preamble, one
    /// `lop_loc` and a data tetra, a postamble with *G* = 254 -- loads
    /// through `load` into the memory, rG, `$254`, `$255` and entry it
    /// describes, without raising rL.
    #[test]
    fn load_applies_a_hand_built_object_files_full_contents() {
        let mut data = vec![MM, MmoRecordType::LopPre as u8, 0x01, 0x01];
        data.extend_from_slice(&[0, 0, 0, 0]); // creation time, 0
        data.extend_from_slice(&[MM, MmoRecordType::LopLoc as u8, 0x00, 0x02]);
        data.extend_from_slice(&0u32.to_be_bytes()); // address high
        data.extend_from_slice(&0x100u32.to_be_bytes()); // address low
        data.extend_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD]); // one plain data tetra
        data.extend_from_slice(&[MM, MmoRecordType::LopPost as u8, 0x00, 254]);
        data.extend_from_slice(&1234u64.to_be_bytes()); // $254
        data.extend_from_slice(&0x100u64.to_be_bytes()); // $255 = entry

        let decoder = MmoDecoder::new(data);
        let mut mmix = MMix::new();
        let entry = decoder.load(&mut mmix).expect("well-formed object code");

        assert_eq!(entry, 0x100);
        assert_eq!(mmix.get_special(SpecialReg::RG), 254);
        assert_eq!(mmix.get_register(254), 1234);
        assert_eq!(mmix.get_register(255), 0x100);
        assert_eq!(mmix.get_special(SpecialReg::RL), 0);
        assert_eq!(mmix.read_tetra(0x100), 0xAABBCCDD);
    }

    /// A program assembling two `BYTE`s to the same address (`#0f` then
    /// `#3c`) built to `.mmo` and loaded with `load` reads their XOR,
    /// `#33`, the same as the direct `.mms` path.
    #[test]
    fn load_combines_two_bytes_assembled_to_one_address_by_xor() {
        use crate::mmixal::MMixAssembler;

        const SOURCE: &str = "\
\tLOC\t#1000
\tBYTE\t#0f
\tLOC\t#1000
\tBYTE\t#3c
\tLOC\t#100
Main\tTRAP\t0,Halt,0
";
        let mut asm = MMixAssembler::new(SOURCE, "<test>");
        asm.parse().expect("program must assemble");

        let mmo_data = MmoGenerator::new(asm.instructions.clone(), asm.labels.clone()).generate();
        let decoder = MmoDecoder::new(mmo_data);
        let mut mmix = MMix::new();
        decoder.load(&mut mmix).expect("well-formed object code");

        assert_eq!(mmix.read_byte(0x1000), 0x33);
    }

    /// A fresh machine's key observable state: what every rejected-load
    /// test below confirms `load` leaves untouched.
    fn assert_machine_untouched(mmix: &MMix) {
        assert_eq!(mmix.get_special(SpecialReg::RG), 32);
        assert_eq!(mmix.get_register(255), 0);
        assert_eq!(mmix.occupied().count(), 0);
    }

    /// The preamble `MmoGenerator` emits, for a hand-built error case that
    /// needs a valid preamble ahead of the record under test.
    fn valid_preamble() -> Vec<u8> {
        vec![MM, MmoRecordType::LopPre as u8, 0x01, 0x01, 0, 0, 0, 0]
    }

    #[test]
    fn decode_rejects_a_file_that_does_not_begin_with_lop_pre() {
        let data = vec![0x00, 0x01, 0x02, 0x03];
        let decoder = MmoDecoder::new(data);

        assert_eq!(
            decoder.decode(|_, _| {}).unwrap_err(),
            ".mmo: file does not begin with lop_pre"
        );

        let mut mmix = MMix::new();
        assert_eq!(
            decoder.load(&mut mmix).unwrap_err(),
            ".mmo: file does not begin with lop_pre"
        );
        assert_machine_untouched(&mmix);
    }

    /// A `.mmo` from 0.3.12 or earlier begins `#98090001`: version 0.
    #[test]
    fn decode_rejects_a_pre_0_3_13_preamble_version() {
        let mut data = vec![MM, MmoRecordType::LopPre as u8, 0x00, 0x01];
        data.extend_from_slice(&[MM, MmoRecordType::LopPost as u8, 0x00, 0xFF]);
        data.extend_from_slice(&0x100u64.to_be_bytes());
        let decoder = MmoDecoder::new(data);

        assert_eq!(
            decoder.decode(|_, _| {}).unwrap_err(),
            ".mmo: lop_pre version 0, expected 1"
        );

        let mut mmix = MMix::new();
        assert_eq!(
            decoder.load(&mut mmix).unwrap_err(),
            ".mmo: lop_pre version 0, expected 1"
        );
        assert_machine_untouched(&mmix);
    }

    #[test]
    fn decode_rejects_a_lop_quote_with_a_count_other_than_one() {
        let mut data = valid_preamble();
        let quote_offset = data.len();
        data.extend_from_slice(&[MM, MmoRecordType::LopQuote as u8, 0x00, 0x02]);
        data.extend_from_slice(&[0u8; 8]);
        let decoder = MmoDecoder::new(data);
        let expected = format!(".mmo: lop_quote count 2, expected 1 at offset {quote_offset:#x}");

        assert_eq!(decoder.decode(|_, _| {}).unwrap_err(), expected);

        let mut mmix = MMix::new();
        assert_eq!(decoder.load(&mut mmix).unwrap_err(), expected);
        assert_machine_untouched(&mmix);
    }

    /// The reference's other `lop_loc` shape -- a high byte in Y, one
    /// address tetra -- is a shape the writer never emits.
    #[test]
    fn decode_rejects_a_foreign_shaped_lop_loc() {
        let mut data = valid_preamble();
        let loc_offset = data.len();
        data.extend_from_slice(&[MM, MmoRecordType::LopLoc as u8, 0x20, 0x01]);
        data.extend_from_slice(&0x100u32.to_be_bytes());
        let decoder = MmoDecoder::new(data);
        let expected = format!(".mmo: unsupported lopcode #01 at offset {loc_offset:#x}");

        assert_eq!(decoder.decode(|_, _| {}).unwrap_err(), expected);

        let mut mmix = MMix::new();
        assert_eq!(decoder.load(&mut mmix).unwrap_err(), expected);
        assert_machine_untouched(&mmix);
    }

    #[test]
    fn decode_rejects_a_postamble_g_below_32() {
        let mut data = valid_preamble();
        let post_offset = data.len();
        data.extend_from_slice(&[MM, MmoRecordType::LopPost as u8, 0x00, 10]);
        let decoder = MmoDecoder::new(data);
        let expected = format!(".mmo: lop_post G=10, below 32 at offset {post_offset:#x}");

        assert_eq!(decoder.decode(|_, _| {}).unwrap_err(), expected);

        let mut mmix = MMix::new();
        assert_eq!(decoder.load(&mut mmix).unwrap_err(), expected);
        assert_machine_untouched(&mmix);
    }

    #[test]
    fn decode_rejects_a_file_that_ends_inside_a_record() {
        // A preamble with no creation-time tetra following it.
        let data = vec![MM, MmoRecordType::LopPre as u8, 0x01, 0x01];
        let decoder = MmoDecoder::new(data);
        let expected = ".mmo: file ends inside a record at offset 0x0";

        assert_eq!(decoder.decode(|_, _| {}).unwrap_err(), expected);

        let mut mmix = MMix::new();
        assert_eq!(decoder.load(&mut mmix).unwrap_err(), expected);
        assert_machine_untouched(&mmix);
    }

    #[test]
    fn decode_rejects_a_file_with_no_postamble() {
        let mut data = valid_preamble();
        data.extend_from_slice(&[MM, MmoRecordType::LopLoc as u8, 0x00, 0x02]);
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0x100u32.to_be_bytes());
        data.extend_from_slice(&[1, 2, 3, 4]); // a data tetra, then nothing else
        let decoder = MmoDecoder::new(data);
        let expected = ".mmo: no lop_post";

        assert_eq!(decoder.decode(|_, _| {}).unwrap_err(), expected);

        let mut mmix = MMix::new();
        assert_eq!(decoder.load(&mut mmix).unwrap_err(), expected);
        assert_machine_untouched(&mmix);
    }

    /// A `lop_spec` of a type other than the debug-string one is a shape the
    /// writer never emits, so the reader now rejects it instead of skipping
    /// it -- unlike before this unit, when an unrelated `lop_spec` record
    /// was read and discarded without disturbing the string table.
    #[test]
    fn decode_rejects_a_lop_spec_of_another_type() {
        let mut data = valid_preamble();
        let spec_offset = data.len();
        data.extend_from_slice(&[MM, MmoRecordType::LopSpec as u8, 0x12, 0x34]);
        data.extend_from_slice(&[0, 0, 0, 4]);
        data.extend_from_slice(&[9, 9, 9, 9]);
        let decoder = MmoDecoder::new(data);
        let expected = format!(".mmo: unsupported lopcode #08 at offset {spec_offset:#x}");

        assert_eq!(decoder.decode(|_, _| {}).unwrap_err(), expected);

        let mut mmix = MMix::new();
        assert_eq!(decoder.load(&mut mmix).unwrap_err(), expected);
        assert_machine_untouched(&mmix);
    }
}
