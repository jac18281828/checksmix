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
//! a `lop_loc` other than `#98010002` or whose address isn't a multiple of
//! 4, a `lop_spec` payload tetra starting `#98` that isn't the `#98000001`
//! escape, a `lop_spec` of any type but the debug-string one, a `lop_post`
//! with a nonzero Y or a *G* below 32); a preamble whose version isn't 1;
//! and a file that ends mid-record. A data tetra or `lop_quote`, plain or
//! escaped, whose load address lies past `#FFFFFFFFFFFFFFFF` is also an
//! `Err`: an item may fill the address space's last byte exactly, the
//! same shape the writer emits for one there, but nothing may load after
//! it until a `lop_loc` sets a fresh address. Reading stops once the
//! postamble's octabytes are consumed -- a symbol table after them, or
//! `lop_file`/`lop_line`/fixup records from an MMIXAL build, are never
//! read. A `.mmo` whose preamble reads `#98090001` (version 0) is rejected
//! and must be rebuilt.

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
    /// *G* stays 255 and the postamble carries only the entry.
    pub fn with_greg_inits(mut self, inits: Vec<(u8, u64)>) -> Self {
        self.greg_inits = inits;
        self
    }

    /// Generate the object file: preamble, one `lop_loc` per contiguous run
    /// of assembled bytes followed by that run's data tetras (escaped where
    /// a tetra would otherwise collide with `MM`), the `debug` string
    /// table, then the postamble. A run's `lop_loc` address is its first
    /// byte's address rounded down to a multiple of 4, the reference's own
    /// rule; the bytes between that address and the run's real start are
    /// zero, so the run's own data still lands where it was assembled.
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
                let aligned = addr & !3;
                self.emit_lop_loc(&mut mmo, aligned);
                pending_bytes.resize((addr - aligned) as usize, 0);
            }

            pending_bytes.extend_from_slice(&bytes);
            current_loc = Some(addr.wrapping_add(bytes.len() as u64));
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

    /// Emit one data tetra, preceded by `#98000001` (`lop_quote`, count 1)
    /// when its first byte would otherwise read as `MM`.
    fn emit_tetra(mmo: &mut Vec<u8>, tetra: &[u8; 4]) {
        if tetra[0] == MM {
            mmo.push(MM);
            mmo.push(MmoRecordType::LopQuote as u8);
            mmo.push(0x00);
            mmo.push(0x01);
        }
        mmo.extend_from_slice(tetra);
    }

    /// Emit `bytes` (already assembled, contiguous from the current
    /// address) as plain data tetras. A trailing partial tetra is
    /// zero-padded.
    fn emit_data_tetras(&self, mmo: &mut Vec<u8>, bytes: &[u8]) {
        for chunk in bytes.chunks(4) {
            let mut tetra = [0u8; 4];
            tetra[..chunk.len()].copy_from_slice(chunk);
            Self::emit_tetra(mmo, &tetra);
        }
    }

    /// Emit one `debug` directive's string as a `lop_spec` record: the type
    /// field `LOP_SPEC_DEBUG_STRING`, then a payload tetra holding the byte
    /// length, then the bytes zero-padded to a tetra boundary.
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
            Self::emit_tetra(mmo, tetra);
        }
    }

    /// Emit `lop_loc`: `#98010002`, then `addr`'s high and low tetras.
    /// `addr` is always a multiple of 4, `generate`'s own invariant.
    fn emit_lop_loc(&self, mmo: &mut Vec<u8>, addr: u64) {
        mmo.push(MM);
        mmo.push(MmoRecordType::LopLoc as u8);
        mmo.push(0x00);
        mmo.push(0x02);

        let high = (addr >> 32) as u32;
        mmo.extend_from_slice(&high.to_be_bytes());

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
    /// bounds-checked its slice to length 4, so the lookup cannot fail.
    fn be_u32(bytes: &[u8]) -> u32 {
        u32::from_be_bytes(
            *bytes
                .first_chunk::<4>()
                .expect("caller passes exactly 4 bytes"),
        )
    }

    /// Write one data tetra's four bytes through `write_byte`, `data[i..i+4]`
    /// landing at `addr..=addr+3`.
    fn write_tetra<F: FnMut(u64, u8)>(write_byte: &mut F, data: &[u8], i: usize, addr: u64) {
        for k in 0..4u64 {
            write_byte(addr + k, data[i + k as usize]);
        }
    }

    /// Read one payload tetra of a `lop_spec` record at `data[pos..]`,
    /// honoring the `lop_quote` escape a tetra whose first byte collides
    /// with `MM` requires: exactly `#98000001`, nothing else starting `#98`.
    /// Returns the tetra and how many bytes it and any escape consumed, or
    /// `Ok(None)` past the end of the file.
    fn read_payload_tetra(data: &[u8], pos: usize) -> Result<Option<([u8; 4], usize)>, String> {
        if pos + 4 > data.len() {
            return Ok(None);
        }
        // `pos + 4 <= data.len()` above, and every call below reads at `pos`
        // or at `pos + 4` after checking `pos + 8 <= data.len()`.
        let tetra_at = |offset: usize| *data[offset..offset + 4].first_chunk::<4>().unwrap();
        if data[pos] == MM {
            let lopcode_byte = data[pos + 1];
            let (y, z) = (data[pos + 2], data[pos + 3]);
            if lopcode_byte != MmoRecordType::LopQuote as u8 || y != 0x00 || z != 0x01 {
                return Err(Self::unsupported_lopcode(lopcode_byte, pos));
            }
            let quoted = pos + 4;
            if quoted + 4 > data.len() {
                return Ok(None);
            }
            Ok(Some((tetra_at(quoted), 8)))
        } else {
            Ok(Some((tetra_at(pos), 4)))
        }
    }

    /// An `unsupported lopcode` error naming `lopcode`'s own record, at the
    /// offset of that record's `MM` byte.
    fn unsupported_lopcode(lopcode: u8, offset: usize) -> String {
        format!(".mmo: unsupported lopcode #{lopcode:02x} at offset {offset:#x}")
    }

    /// A `data past #FFFFFFFFFFFFFFFF` error at `offset`: a plain data
    /// tetra's own offset, or an escaped one's `lop_quote` record offset.
    fn data_past_end(offset: usize) -> String {
        format!(".mmo: data past #FFFFFFFFFFFFFFFF at offset {offset:#x}")
    }

    /// A `file ends inside a record` error at `offset`, the offset of the
    /// record (or data tetra) that ran out of bytes.
    fn truncated(offset: usize) -> String {
        format!(".mmo: file ends inside a record at offset {offset:#x}")
    }

    /// Validate the preamble: `#98090101`, then a length check for the
    /// creation-time tetra that follows (never read; it is always 0).
    fn parse_preamble(data: &[u8]) -> Result<(), String> {
        if data.len() < 4 || data[0] != MM || data[1] != MmoRecordType::LopPre as u8 {
            return Err(".mmo: file does not begin with lop_pre".to_string());
        }
        let (version, pre_z) = (data[2], data[3]);
        if version != 1 {
            return Err(format!(".mmo: lop_pre version {version}, expected 1"));
        }
        if pre_z != 1 {
            return Err(Self::unsupported_lopcode(MmoRecordType::LopPre as u8, 0));
        }
        if data.len() < 8 {
            return Err(Self::truncated(0));
        }
        Ok(())
    }

    /// A plain data tetra: no `MM` prefix. Returns the bytes consumed (4).
    fn parse_data_tetra<F: FnMut(u64, u8)>(
        data: &[u8],
        i: usize,
        current_addr: u64,
        write_byte: &mut F,
    ) -> Result<usize, String> {
        if i + 4 > data.len() {
            return Err(Self::truncated(i));
        }
        Self::write_tetra(write_byte, data, i, current_addr);
        Ok(4)
    }

    /// `lop_quote`: an escaped data tetra, count exactly 1. Returns the
    /// bytes consumed from `record_offset` (the header and the tetra).
    fn parse_lop_quote<F: FnMut(u64, u8)>(
        data: &[u8],
        record_offset: usize,
        y: u8,
        z: u8,
        current_addr: u64,
        write_byte: &mut F,
    ) -> Result<usize, String> {
        let yz = ((y as u16) << 8) | z as u16;
        if yz != 1 {
            return Err(format!(
                ".mmo: lop_quote count {yz}, expected 1 at offset {record_offset:#x}"
            ));
        }
        let i = record_offset + 4;
        if i + 4 > data.len() {
            return Err(Self::truncated(record_offset));
        }
        Self::write_tetra(write_byte, data, i, current_addr);
        Ok(8)
    }

    /// `lop_loc`: `#98010002`, then an address that must be a multiple of
    /// 4. Returns the new current address and the bytes consumed (12).
    fn parse_lop_loc(
        data: &[u8],
        record_offset: usize,
        y: u8,
        z: u8,
    ) -> Result<(u64, usize), String> {
        if y != 0x00 || z != 0x02 {
            return Err(Self::unsupported_lopcode(
                MmoRecordType::LopLoc as u8,
                record_offset,
            ));
        }
        let i = record_offset + 4;
        if i + 8 > data.len() {
            return Err(Self::truncated(record_offset));
        }
        let high = Self::be_u32(&data[i..i + 4]);
        let low = Self::be_u32(&data[i + 4..i + 8]);
        let addr = ((high as u64) << 32) | low as u64;
        if !addr.is_multiple_of(4) {
            return Err(Self::unsupported_lopcode(
                MmoRecordType::LopLoc as u8,
                record_offset,
            ));
        }
        Ok((addr, 12))
    }

    /// `lop_spec`: the `debug` string table's own record type, length
    /// prefixed and zero-padded to a tetra boundary. Appends the decoded
    /// string and returns the bytes consumed from `record_offset`.
    fn parse_lop_spec(
        &self,
        data: &[u8],
        record_offset: usize,
        y: u8,
        z: u8,
    ) -> Result<usize, String> {
        let record_type = ((y as u16) << 8) | z as u16;
        if record_type != LOP_SPEC_DEBUG_STRING {
            return Err(Self::unsupported_lopcode(
                MmoRecordType::LopSpec as u8,
                record_offset,
            ));
        }
        let mut i = record_offset + 4;
        let (len_tetra, consumed) = match Self::read_payload_tetra(data, i)? {
            Some(pair) => pair,
            None => return Err(Self::truncated(record_offset)),
        };
        i += consumed;
        let byte_len = u32::from_be_bytes(len_tetra) as usize;
        // Checked against the bytes left before any arithmetic on byte_len
        // itself: on wasm32 (32-bit usize) a declared length near u32::MAX
        // overflows the padding multiply below, and one past isize::MAX
        // overflows Vec's own capacity limit. A file too short for the
        // declared length is truncated regardless, so this bails out with
        // that same diagnosis, reserving nothing.
        if byte_len > data.len().saturating_sub(i) {
            return Err(Self::truncated(record_offset));
        }
        let padded_len = byte_len.div_ceil(4) * 4;

        let mut payload = Vec::with_capacity(padded_len);
        let mut remaining = padded_len;
        while remaining > 0 {
            let (tetra, consumed) = match Self::read_payload_tetra(data, i)? {
                Some(pair) => pair,
                None => return Err(Self::truncated(record_offset)),
            };
            i += consumed;
            payload.extend_from_slice(&tetra);
            remaining -= 4;
        }
        payload.truncate(byte_len);
        self.debug_strings.borrow_mut().push(payload);
        Ok(i - record_offset)
    }

    /// `lop_post`: `#980A00`*G*, then one octabyte per register from *G*
    /// through `$255`. Returns the entry point (`$255`), *G*, and every
    /// register in that range.
    fn parse_lop_post(
        data: &[u8],
        record_offset: usize,
        y: u8,
        z: u8,
    ) -> Result<(u64, u8, HashMap<u8, u64>), String> {
        if y != 0x00 {
            return Err(Self::unsupported_lopcode(
                MmoRecordType::LopPost as u8,
                record_offset,
            ));
        }
        let g = z;
        if g < 32 {
            return Err(format!(
                ".mmo: lop_post G={g}, below 32 at offset {record_offset:#x}"
            ));
        }
        let mut i = record_offset + 4;
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
        Ok((entry_point, g, registers))
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

        Self::parse_preamble(data)?;
        // Bytes 4..8: the creation-time tetra, always 0, never read.
        let mut i = 8usize;
        // `None` once a tetra has filled the address space's last byte:
        // nothing may load after it until a `lop_loc` sets a fresh address.
        let mut current_addr = Some(0u64);

        loop {
            if i >= data.len() {
                return Err(".mmo: no lop_post".to_string());
            }

            if data[i] != MM {
                let Some(addr) = current_addr else {
                    return Err(Self::data_past_end(i));
                };
                // `addr` is always a multiple of 4 (`parse_lop_loc` rejects
                // an unaligned one, and every advance is by 4), so `addr +
                // 4` never needs a byte past `#FFFFFFFFFFFFFFFF` without
                // landing exactly on it.
                let next = addr.checked_add(4);
                let consumed = Self::parse_data_tetra(data, i, addr, write_byte)?;
                current_addr = next;
                i += consumed;
                continue;
            }

            let record_offset = i;
            if i + 4 > data.len() {
                return Err(Self::truncated(record_offset));
            }
            let lopcode_byte = data[i + 1];
            let (y, z) = (data[i + 2], data[i + 3]);

            match MmoRecordType::try_from(lopcode_byte) {
                Ok(MmoRecordType::LopQuote) => {
                    let Some(addr) = current_addr else {
                        return Err(Self::data_past_end(record_offset));
                    };
                    let next = addr.checked_add(4);
                    let consumed =
                        Self::parse_lop_quote(data, record_offset, y, z, addr, write_byte)?;
                    current_addr = next;
                    i = record_offset + consumed;
                }
                Ok(MmoRecordType::LopLoc) => {
                    let (addr, consumed) = Self::parse_lop_loc(data, record_offset, y, z)?;
                    current_addr = Some(addr);
                    i = record_offset + consumed;
                }
                Ok(MmoRecordType::LopSpec) => {
                    i = record_offset + self.parse_lop_spec(data, record_offset, y, z)?;
                }
                Ok(MmoRecordType::LopPost) => {
                    return Self::parse_lop_post(data, record_offset, y, z);
                }
                _ => return Err(Self::unsupported_lopcode(lopcode_byte, record_offset)),
            }
        }
    }

    /// Decode the object file, calling `write_byte` for every loaded byte
    /// in file order, and return the entry point (`$255`'s postamble
    /// value) on success. Each run starts on a tetra boundary, so the
    /// callback also receives the zero bytes that pad a run to it, at
    /// addresses another run may already hold: it must XOR every byte into
    /// memory, as [`MmoDecoder::load`] does, never store it. `Err` on any
    /// shape [`MmoGenerator::generate`] would not emit; see the module docs
    /// for every rejected shape.
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
        let mut buffered: Vec<(u64, u8)> = Vec::new();
        let mut collect = |addr: u64, byte: u8| buffered.push((addr, byte));
        let (entry, g, registers) = self.parse(&mut collect)?;

        for (addr, byte) in buffered {
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
    /// tetra -- byte-exact, since a build's timestamp is always 0 and two
    /// builds of the same source produce identical bytes.
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

    /// Every `lop_loc` the writer emits is a multiple of 4, even when the
    /// run it opens starts at an unaligned address: the record rounds the
    /// address down, and leading zero bytes carry the run's own data to
    /// its real address.
    #[test]
    fn every_lop_loc_the_writer_emits_is_aligned() {
        let instructions = vec![
            (0x101, MMixInstruction::BYTE(1)),
            (0x203, MMixInstruction::BYTE(2)),
        ];
        let generator = MmoGenerator::new(instructions, HashMap::new());
        let mmo_data = generator.generate();

        let loc_positions: Vec<usize> = mmo_data
            .windows(2)
            .enumerate()
            .filter(|(_, w)| w[0] == MM && w[1] == MmoRecordType::LopLoc as u8)
            .map(|(i, _)| i)
            .collect();
        assert_eq!(loc_positions.len(), 2, "one lop_loc per unaligned BYTE");

        for pos in loc_positions {
            let high = u32::from_be_bytes(mmo_data[pos + 4..pos + 8].try_into().unwrap());
            let low = u32::from_be_bytes(mmo_data[pos + 8..pos + 12].try_into().unwrap());
            let addr = ((high as u64) << 32) | low as u64;
            assert_eq!(addr % 4, 0, "lop_loc address {addr:#x} is not aligned");
        }
    }

    /// After `LOC #1001; BYTE 5; LOC #1006; BYTE 1,2,3,4`, the assembled
    /// bytes sit at `#1001` and `#1006`-`#1009` on both the direct `.mms`
    /// path and a `.mmo` round trip.
    #[test]
    fn an_unaligned_loc_run_lands_at_its_addresses_on_both_paths() {
        use crate::mmixal::MMixAssembler;

        const SOURCE: &str = "\
\tLOC\t#1001
\tBYTE\t5
\tLOC\t#1006
\tBYTE\t1,2,3,4
\tLOC\t#100
Main\tTRAP\t0,Halt,0
";
        let mut asm = MMixAssembler::new(SOURCE, "<test>");
        asm.parse().expect("program must assemble");

        let mut mms_mmix = MMix::new();
        for (addr, inst) in &asm.instructions {
            let bytes = asm.encode_instruction_bytes(inst);
            for (offset, &byte) in bytes.iter().enumerate() {
                mms_mmix.write_loaded_byte(addr + offset as u64, byte);
            }
        }

        let mmo_data = MmoGenerator::new(asm.instructions.clone(), asm.labels.clone()).generate();
        let decoder = MmoDecoder::new(mmo_data);
        let mut mmo_mmix = MMix::new();
        decoder
            .load(&mut mmo_mmix)
            .expect("well-formed object code");

        for mmix in [&mms_mmix, &mmo_mmix] {
            assert_eq!(mmix.read_byte(0x1001), 5);
            assert_eq!(mmix.read_byte(0x1006), 1);
            assert_eq!(mmix.read_byte(0x1007), 2);
            assert_eq!(mmix.read_byte(0x1008), 3);
            assert_eq!(mmix.read_byte(0x1009), 4);
        }
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
    /// `lop_loc`, a plain data tetra, an escaped one, a postamble with
    /// *G* = 254 -- loads through `load` into the memory, rG, `$254`,
    /// `$255` and entry it describes, without raising rL.
    #[test]
    fn load_applies_a_hand_built_object_files_full_contents() {
        let mut data = vec![MM, MmoRecordType::LopPre as u8, 0x01, 0x01];
        data.extend_from_slice(&[0, 0, 0, 0]); // creation time, 0
        data.extend_from_slice(&[MM, MmoRecordType::LopLoc as u8, 0x00, 0x02]);
        data.extend_from_slice(&0u32.to_be_bytes()); // address high
        data.extend_from_slice(&0x100u32.to_be_bytes()); // address low
        data.extend_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD]); // one plain data tetra
        data.extend_from_slice(&[MM, MmoRecordType::LopQuote as u8, 0x00, 0x01]); // escape
        data.extend_from_slice(&[0x98, 0x01, 0x02, 0x03]); // the escaped tetra
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
        assert_eq!(mmix.read_tetra(0x104), 0x98010203);
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

    /// A `lop_loc` address that isn't a multiple of 4 is a shape the
    /// writer never emits.
    #[test]
    fn decode_rejects_an_unaligned_lop_loc() {
        let mut data = valid_preamble();
        let loc_offset = data.len();
        data.extend_from_slice(&[MM, MmoRecordType::LopLoc as u8, 0x00, 0x02]);
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0x101u32.to_be_bytes());
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

    // ---- the address space ends at #FFFFFFFFFFFFFFFF ------------------

    /// A `lop_loc` to `#FFFFFFFFFFFFFFFC`, then one data tetra: the item
    /// fills the address space's last four bytes exactly, the same shape
    /// [`MmoGenerator::generate`] emits for an item there.
    fn boundary_item_prefix() -> Vec<u8> {
        let mut data = valid_preamble();
        data.extend_from_slice(&[MM, MmoRecordType::LopLoc as u8, 0x00, 0x02]);
        data.extend_from_slice(&0xFFFFFFFFu32.to_be_bytes());
        data.extend_from_slice(&0xFFFFFFFCu32.to_be_bytes());
        data.extend_from_slice(&[0x01, 0x02, 0x03, 0x04]);
        data
    }

    /// `lop_post`: G=255, entry `#100`.
    fn boundary_item_postamble() -> [u8; 12] {
        [0x98, 0x0A, 0x00, 0xFF, 0, 0, 0, 0, 0, 0, 0x01, 0x00]
    }

    #[test]
    fn decode_loads_an_item_that_fills_the_last_four_bytes_exactly() {
        let mut data = boundary_item_prefix();
        data.extend_from_slice(&boundary_item_postamble());
        let decoder = MmoDecoder::new(data);
        let mut memory = HashMap::new();
        let entry = decoder
            .decode(|addr, byte| {
                memory.insert(addr, byte);
            })
            .expect("well-formed object code");

        assert_eq!(entry, 0x100);
        assert_eq!(memory.get(&0xFFFFFFFFFFFFFFFC), Some(&0x01));
        assert_eq!(memory.get(&0xFFFFFFFFFFFFFFFD), Some(&0x02));
        assert_eq!(memory.get(&0xFFFFFFFFFFFFFFFE), Some(&0x03));
        assert_eq!(memory.get(&0xFFFFFFFFFFFFFFFF), Some(&0x04));
    }

    #[test]
    fn decode_rejects_a_plain_data_tetra_past_the_end() {
        let mut data = boundary_item_prefix();
        let past_end_offset = data.len();
        data.extend_from_slice(&[0x05, 0x06, 0x07, 0x08]);
        data.extend_from_slice(&boundary_item_postamble());
        let decoder = MmoDecoder::new(data);
        let expected = format!(".mmo: data past #FFFFFFFFFFFFFFFF at offset {past_end_offset:#x}");

        assert_eq!(decoder.decode(|_, _| {}).unwrap_err(), expected);

        let mut mmix = MMix::new();
        assert_eq!(decoder.load(&mut mmix).unwrap_err(), expected);
        assert_machine_untouched(&mmix);
    }

    #[test]
    fn decode_rejects_an_escaped_data_tetra_past_the_end() {
        let mut data = boundary_item_prefix();
        let record_offset = data.len();
        data.extend_from_slice(&[MM, MmoRecordType::LopQuote as u8, 0x00, 0x01]);
        data.extend_from_slice(&[0x05, 0x06, 0x07, 0x08]);
        data.extend_from_slice(&boundary_item_postamble());
        let decoder = MmoDecoder::new(data);
        let expected = format!(".mmo: data past #FFFFFFFFFFFFFFFF at offset {record_offset:#x}");

        assert_eq!(decoder.decode(|_, _| {}).unwrap_err(), expected);

        let mut mmix = MMix::new();
        assert_eq!(decoder.load(&mut mmix).unwrap_err(), expected);
        assert_machine_untouched(&mmix);
    }

    #[test]
    fn decode_a_lop_loc_clears_the_past_end_state() {
        let mut data = boundary_item_prefix();
        data.extend_from_slice(&[MM, MmoRecordType::LopLoc as u8, 0x00, 0x02]);
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0x100u32.to_be_bytes());
        data.extend_from_slice(&[0x05, 0x06, 0x07, 0x08]);
        data.extend_from_slice(&boundary_item_postamble());
        let decoder = MmoDecoder::new(data);
        let mut memory = HashMap::new();
        let entry = decoder
            .decode(|addr, byte| {
                memory.insert(addr, byte);
            })
            .expect("well-formed object code");

        assert_eq!(entry, 0x100);
        assert_eq!(memory.get(&0xFFFFFFFFFFFFFFFC), Some(&0x01));
        assert_eq!(memory.get(&0xFFFFFFFFFFFFFFFF), Some(&0x04));
        assert_eq!(memory.get(&0x100), Some(&0x05));
        assert_eq!(memory.get(&0x103), Some(&0x08));
    }

    /// The writer's own side of the boundary case: an item at
    /// `#FFFFFFFFFFFFFFFC` generates and decodes back to the same bytes
    /// there, the round trip the four tests above hand-build one half of.
    #[test]
    fn generate_and_decode_round_trip_an_item_at_the_top_of_memory() {
        let instructions = vec![(0xFFFFFFFFFFFFFFFC, MMixInstruction::SWYM(0, 0, 0))];
        let generator = MmoGenerator::new(instructions.clone(), HashMap::new());
        let mmo_data = generator.generate();

        let decoder = MmoDecoder::new(mmo_data);
        let mut memory = HashMap::new();
        decoder
            .decode(|addr, byte| {
                memory.insert(addr, byte);
            })
            .expect("well-formed object code");

        let (addr, inst) = &instructions[0];
        let bytes = encode_instruction_bytes(inst);
        for (offset, &expected_byte) in bytes.iter().enumerate() {
            assert_eq!(memory.get(&(addr + offset as u64)), Some(&expected_byte));
        }
    }

    /// A `lop_spec` of a type other than the debug-string one is a shape the
    /// writer never emits, so the reader rejects it rather than reading and
    /// discarding it.
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

    /// A `lop_spec` payload tetra beginning `#98` that isn't the exact
    /// `#98000001` escape is a shape the writer never emits.
    #[test]
    fn decode_rejects_a_lop_spec_payload_tetra_with_a_malformed_escape() {
        let mut data = valid_preamble();
        data.extend_from_slice(&[MM, MmoRecordType::LopSpec as u8, 0x44, 0x42]);
        data.extend_from_slice(&[0, 0, 0, 4]); // byte_len = 4
        let bad_offset = data.len();
        data.extend_from_slice(&[0x98, 0x12, 0x34, 0x56]);
        let decoder = MmoDecoder::new(data);
        let expected = format!(".mmo: unsupported lopcode #12 at offset {bad_offset:#x}");

        assert_eq!(decoder.decode(|_, _| {}).unwrap_err(), expected);

        let mut mmix = MMix::new();
        assert_eq!(decoder.load(&mut mmix).unwrap_err(), expected);
        assert_machine_untouched(&mmix);
    }

    /// A `lop_spec` length checked against the bytes left, before any
    /// arithmetic on it: on a 32-bit `usize` (wasm32) `0xFFFFFFFF` both
    /// overflows the padding multiply (`div_ceil(4) * 4`) and would
    /// overflow `Vec::with_capacity`'s own limit past that. On every
    /// target this 20-byte file is truncated regardless.
    #[test]
    fn decode_rejects_a_lop_spec_length_longer_than_the_file() {
        let mut data = valid_preamble();
        let spec_offset = data.len();
        data.extend_from_slice(&[MM, MmoRecordType::LopSpec as u8, 0x44, 0x42]);
        data.extend_from_slice(&0xFFFFFFFFu32.to_be_bytes()); // byte_len
        data.extend_from_slice(&0u32.to_be_bytes()); // a lone payload tetra
        let decoder = MmoDecoder::new(data);
        let expected = format!(".mmo: file ends inside a record at offset {spec_offset:#x}");

        assert_eq!(decoder.decode(|_, _| {}).unwrap_err(), expected);

        let mut mmix = MMix::new();
        assert_eq!(decoder.load(&mut mmix).unwrap_err(), expected);
        assert_machine_untouched(&mmix);
    }

    /// A `lop_pre` whose Z field (tetra count) isn't 1 is a shape the
    /// writer never emits.
    #[test]
    fn decode_rejects_a_lop_pre_with_a_tetra_count_other_than_one() {
        let mut data = vec![MM, MmoRecordType::LopPre as u8, 0x01, 0x02];
        data.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0]); // two tetras
        data.extend_from_slice(&[MM, MmoRecordType::LopPost as u8, 0x00, 0xFF]);
        data.extend_from_slice(&0x100u64.to_be_bytes());
        let decoder = MmoDecoder::new(data);
        let expected = ".mmo: unsupported lopcode #09 at offset 0x0";

        assert_eq!(decoder.decode(|_, _| {}).unwrap_err(), expected);

        let mut mmix = MMix::new();
        assert_eq!(decoder.load(&mut mmix).unwrap_err(), expected);
        assert_machine_untouched(&mmix);
    }

    /// A `lop_post` with a nonzero Y is a shape the writer never emits.
    #[test]
    fn decode_rejects_a_lop_post_with_a_nonzero_y() {
        let mut data = valid_preamble();
        let post_offset = data.len();
        data.extend_from_slice(&[MM, MmoRecordType::LopPost as u8, 0x01, 0xFF]);
        data.extend_from_slice(&0x100u64.to_be_bytes());
        let decoder = MmoDecoder::new(data);
        let expected = format!(".mmo: unsupported lopcode #0a at offset {post_offset:#x}");

        assert_eq!(decoder.decode(|_, _| {}).unwrap_err(), expected);

        let mut mmix = MMix::new();
        assert_eq!(decoder.load(&mut mmix).unwrap_err(), expected);
        assert_machine_untouched(&mmix);
    }

    /// With no `GREG` initializers, the postamble starts at *G* = 255 and
    /// carries exactly one octabyte: the entry point.
    #[test]
    fn generate_with_no_greg_postamble_is_one_octabyte() {
        let instructions = vec![(0x100, MMixInstruction::TRAP(0, 0, 0))];
        let mmo_data = MmoGenerator::new(instructions, HashMap::new()).generate();

        let post_pos = mmo_data
            .windows(2)
            .position(|w| w[0] == MM && w[1] == MmoRecordType::LopPost as u8)
            .expect("must have a lop_post record");
        assert_eq!(&mmo_data[post_pos..post_pos + 4], &[0x98, 0x0A, 0x00, 0xFF]);
        assert_eq!(
            mmo_data.len(),
            post_pos + 12,
            "exactly one octabyte follows"
        );

        let entry = u64::from_be_bytes(mmo_data[post_pos + 4..post_pos + 12].try_into().unwrap());
        assert_eq!(entry, 0x100);
    }
}
