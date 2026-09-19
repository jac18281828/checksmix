//! Sparse byte memory, the write journal, and multi-byte read/write helpers.

use super::MMix;
use tracing::{instrument, trace};

impl MMix {
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
            self.memory.remove(&addr); // Don't store zeros (sparse memory)
        } else {
            self.memory.insert(addr, value);
        }
        if self.journal_enabled {
            self.journal.insert(addr);
        }
    }

    /// Every address with a nonzero byte, in ascending address order — the
    /// sparse memory's contents for a caller that wants to display or diff
    /// them, since the underlying map's iteration order is unspecified.
    pub fn occupied(&self) -> impl Iterator<Item = (u64, u8)> {
        let mut addrs: Vec<u64> = self.memory.keys().copied().collect();
        addrs.sort_unstable();
        addrs.into_iter().map(|addr| (addr, self.memory[&addr]))
    }

    /// [`MMix::write_byte`], plus recording `addr` into `loaded` regardless
    /// of value. `write_image`'s per-byte loop calls this in place of
    /// `write_byte` so that a zero byte the loaded program actually placed
    /// stays visible to [`MMix::loaded_extent`] even after the sparse-memory
    /// write drops it from `self.memory`. Not for general writes: a running
    /// program's own stores (register-stack spills, `STO`, ...) must go
    /// through plain `write_byte`, or `loaded_extent` would degrade into the
    /// write journal's noise.
    pub(crate) fn write_loaded_byte(&mut self, addr: u64, value: u8) {
        self.write_byte(addr, value);
        self.loaded.insert(addr);
    }

    /// Every address `write_image` loaded, in ascending order, paired with
    /// its **current** byte value (a later overwrite during execution shows
    /// the new value here, same as [`MMix::occupied`] would if that value is
    /// nonzero).
    ///
    /// Unlike [`MMix::occupied`], this includes a byte that the loaded
    /// program set to zero — sparse memory drops a zero-write from
    /// `self.memory`, so `occupied` can't tell "loaded and zero" from
    /// "never written." This can, so it is what a caller wants for the
    /// real loaded extent instead of a set with silent holes.
    pub fn loaded_extent(&self) -> impl Iterator<Item = (u64, u8)> + '_ {
        self.loaded.iter().map(|&addr| (addr, self.read_byte(addr)))
    }

    /// Enable or disable the write journal. While enabled, every
    /// [`MMix::write_byte`] call records its address (including a
    /// zero-write, which removes the address from [`MMix::occupied`] — that
    /// removal is itself a state change worth journaling). Survives
    /// [`MMix::reset`]; the accumulated addresses do not.
    pub fn set_journal(&mut self, on: bool) {
        self.journal_enabled = on;
    }

    /// Drain the journal, returning every address written since the last
    /// call, deduplicated and in ascending order — matching
    /// [`MMix::occupied`]'s ordering.
    pub fn take_journal(&mut self) -> Vec<u64> {
        let mut addrs: Vec<u64> = self.journal.drain().collect();
        addrs.sort_unstable();
        addrs
    }

    /// Read a wyde (2 bytes) from memory at the address rounded down to a
    /// multiple of 2: `M_2[A] = M_2[2*floor(A/2)]`.
    /// Example: a wyde at #103 reads bytes #102..#103.
    pub fn read_wyde(&self, addr: u64) -> u16 {
        let addr = addr & !1;
        let b0 = self.read_byte(addr) as u16;
        let b1 = self.read_byte(addr.wrapping_add(1)) as u16;
        (b0 << 8) | b1
    }

    /// Write a wyde (2 bytes) to memory at the address rounded down to a
    /// multiple of 2: `M_2[A] = M_2[2*floor(A/2)]`.
    pub fn write_wyde(&mut self, addr: u64, value: u16) {
        let addr = addr & !1;
        self.write_byte(addr, (value >> 8) as u8);
        self.write_byte(addr.wrapping_add(1), value as u8);
    }

    /// Read a tetra (4 bytes) from memory at the address rounded down to a
    /// multiple of 4: `M_4[A] = M_4[4*floor(A/4)]`.
    pub fn read_tetra(&self, addr: u64) -> u32 {
        let addr = addr & !3;
        let b0 = self.read_byte(addr) as u32;
        let b1 = self.read_byte(addr.wrapping_add(1)) as u32;
        let b2 = self.read_byte(addr.wrapping_add(2)) as u32;
        let b3 = self.read_byte(addr.wrapping_add(3)) as u32;
        (b0 << 24) | (b1 << 16) | (b2 << 8) | b3
    }

    /// Write a tetra (4 bytes) to memory at the address rounded down to a
    /// multiple of 4: `M_4[A] = M_4[4*floor(A/4)]`.
    pub fn write_tetra(&mut self, addr: u64, value: u32) {
        let addr = addr & !3;
        self.write_byte(addr, (value >> 24) as u8);
        self.write_byte(addr.wrapping_add(1), (value >> 16) as u8);
        self.write_byte(addr.wrapping_add(2), (value >> 8) as u8);
        self.write_byte(addr.wrapping_add(3), value as u8);
    }

    /// Read an octa (8 bytes) from memory at the address rounded down to a
    /// multiple of 8: `M_8[A] = M_8[8*floor(A/8)]`.
    /// Example: any address in #100..#107 reads bytes #100..#107.
    pub fn read_octa(&self, addr: u64) -> u64 {
        let addr = addr & !7;
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

    /// Write an octa (8 bytes) to memory at the address rounded down to a
    /// multiple of 8: `M_8[A] = M_8[8*floor(A/8)]`.
    pub fn write_octa(&mut self, addr: u64, value: u64) {
        let addr = addr & !7;
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
}
