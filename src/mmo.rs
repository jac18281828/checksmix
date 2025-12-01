//! MMIX Object (MMO) file format support
//!
//! This module handles generation and parsing of MMIX object files (.mmo format).
//! The MMO format uses special loader opcodes (lop_*) to encode instructions and data
//! at arbitrary 64-bit addresses with proper segment support.

use crate::mmixal::MMixInstruction;
use std::collections::HashMap;
use tracing::debug;

use crate::encode::encode_instruction_bytes;

/// MMO file generator
pub struct MmoGenerator {
    /// Instructions to encode, sorted by address
    instructions: Vec<(u64, MMixInstruction)>,
    /// Symbol table (labels)
    labels: HashMap<String, u64>,
}

impl MmoGenerator {
    /// Create a new MMO generator
    pub fn new(instructions: Vec<(u64, MMixInstruction)>, labels: HashMap<String, u64>) -> Self {
        Self {
            instructions,
            labels,
        }
    }

    /// Generate MMIX object code in .mmo format
    /// The format uses tetrabytes (4-byte words) with special loader opcodes:
    /// - lop_quote (0x98): Literal data bytes to load at current address
    /// - lop_loc (0x9A): Set loading address (custom 64-bit format)
    /// - lop_pre (0x9D): Preamble
    /// - lop_post (0x9F): Postamble
    pub fn generate(&self) -> Vec<u8> {
        debug!("Generating MMIX object code (.mmo format)");
        let mut mmo = Vec::new();

        // Write preamble: lop_pre
        // Format: single tetra 0x9D YZ (version in YZ, typically 1)
        mmo.extend_from_slice(&[0x9D, 0x00, 0x00, 0x01]); // lop_pre, version 1

        // Group instructions by contiguous address ranges
        let mut sorted_instructions: Vec<_> = self.instructions.iter().collect();
        sorted_instructions.sort_by_key(|(addr, _)| *addr);

        let mut current_loc: Option<u64> = None;
        let mut pending_bytes = Vec::new();

        for (addr, instruction) in sorted_instructions {
            let addr = *addr;
            // Encode a single instruction to bytes
            let bytes = encode_instruction_bytes(instruction);

            // Check if we need to emit a new lop_loc directive
            let need_new_loc = match current_loc {
                None => true,
                Some(loc) => {
                    // Need new LOC if not contiguous or pending buffer would be too large
                    addr != loc || pending_bytes.len() + bytes.len() > 252
                }
            };

            if need_new_loc {
                // Flush any pending bytes first
                if !pending_bytes.is_empty() {
                    self.emit_lop_quote(&mut mmo, &pending_bytes);
                    pending_bytes.clear();
                }

                // Emit lop_loc to set new address
                self.emit_lop_loc(&mut mmo, addr);
            }

            // Add bytes to pending buffer
            pending_bytes.extend_from_slice(&bytes);
            current_loc = Some(addr + bytes.len() as u64);
        }

        // Flush any remaining bytes
        if !pending_bytes.is_empty() {
            self.emit_lop_quote(&mut mmo, &pending_bytes);
        }

        // Write postamble: lop_post with entry point
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

    /// Emit lop_quote: literal bytes to load at current address
    fn emit_lop_quote(&self, mmo: &mut Vec<u8>, bytes: &[u8]) {
        // lop_quote format: 0x98, YZ=number of tetras, then data
        // Data must be padded to tetra boundary

        // Split into chunks of max 255 tetras (1020 bytes)
        for chunk in bytes.chunks(1020) {
            let chunk_tetras = chunk.len().div_ceil(4);
            let yz = chunk_tetras as u16;

            mmo.push(0x98); // lop_quote
            mmo.push((yz >> 8) as u8);
            mmo.push((yz & 0xFF) as u8);
            mmo.push(0x01); // Z field (loader hint)

            // Write data, padding to tetra boundary
            mmo.extend_from_slice(chunk);
            let padding = (4 - (chunk.len() % 4)) % 4;
            for _ in 0..padding {
                mmo.push(0);
            }
        }
    }

    /// Emit lop_loc: set current loading address
    /// Uses 3 tetras (12 bytes) for full 64-bit address support
    /// Tetra 1: 0x9A 00 00 00 (lop_loc opcode)
    /// Tetra 2: high 32 bits of address
    /// Tetra 3: low 32 bits of address
    fn emit_lop_loc(&self, mmo: &mut Vec<u8>, addr: u64) {
        // Tetra 1: lop_loc opcode with zero YZ
        mmo.extend_from_slice(&[0x9A, 0x00, 0x00, 0x00]);

        // Tetra 2: high 32 bits
        let high = (addr >> 32) as u32;
        mmo.push((high >> 24) as u8);
        mmo.push(((high >> 16) & 0xFF) as u8);
        mmo.push(((high >> 8) & 0xFF) as u8);
        mmo.push((high & 0xFF) as u8);

        // Tetra 3: low 32 bits
        let low = (addr & 0xFFFFFFFF) as u32;
        mmo.push((low >> 24) as u8);
        mmo.push(((low >> 16) & 0xFF) as u8);
        mmo.push(((low >> 8) & 0xFF) as u8);
        mmo.push((low & 0xFF) as u8);
    }

    /// Emit lop_post: postamble with entry point
    fn emit_lop_post(&self, mmo: &mut Vec<u8>, _entry_point: u64) {
        // lop_post format: Single tetra 0x9F YZ G
        // For simplicity, we'll use a minimal postamble
        mmo.push(0x9F); // lop_post
        mmo.push(0x00); // YZ = 0 (no symbol table)
        mmo.push(0x00);
        mmo.push(0x00); // G = 0 (no GREG allocation)
    }
}
