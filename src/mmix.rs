use std::fmt;

/// Special register identifiers for MMIX.
/// These 32 special registers control various aspects of MMIX operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SpecialReg {
    // Standard special registers (rA through rZ)
    RA = 0, // Arithmetic status register
    RB = 1,
    RC = 2,
    RD = 3,
    RE = 4,
    RF = 5,
    RG = 6,
    RH = 7,
    RI = 8,
    RJ = 9,
    RK = 10,
    RL = 11,
    RM = 12,
    RN = 13,
    RO = 14,
    RP = 15,
    RQ = 16,
    RR = 17, // Remainder register
    RS = 18,
    RT = 19,
    RU = 20,
    RV = 21,
    RW = 22,
    RX = 23,
    RY = 24,
    RZ = 25,
    // Additional special registers
    RBB = 26,
    RTT = 27,
    RWW = 28,
    RXX = 29,
    RYY = 30,
    RZZ = 31,
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
    pub fn read_byte(&self, addr: u64) -> u8 {
        *self.memory.get(&addr).unwrap_or(&0)
    }

    /// Write a byte to memory at the given address.
    pub fn write_byte(&mut self, addr: u64, value: u8) {
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

    /// Execute a single instruction at the current program counter.
    /// Returns true if execution should continue, false if halted.
    pub fn execute_instruction(&mut self) -> bool {
        let (op, x, y, z) = self.fetch_instruction();

        match op {
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
                // LDHT $X, $Y, $Z - Load high tetra
                // High 32 bits: M4[$Y + $Z], Low 32 bits: 0
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let tetra = self.read_tetra(addr);
                let value = (tetra as u64) << 32;
                self.set_register(x, value);
                self.advance_pc();
                true
            }
            0x91 => {
                // LDHT $X, $Y, Z - Load high tetra (immediate)
                let addr = self.get_register(y).wrapping_add(z as u64);
                let tetra = self.read_tetra(addr);
                let value = (tetra as u64) << 32;
                self.set_register(x, value);
                self.advance_pc();
                true
            }
            0x22 => {
                // ADDU $X, $Y, $Z - Add unsigned (also LDA)
                // u($X) <- u($Y) + u($Z)
                let y_val = self.get_register(y);
                let z_val = self.get_register(z);
                let result = y_val.wrapping_add(z_val);
                self.set_register(x, result);
                self.advance_pc();
                true
            }
            0x23 => {
                // ADDU $X, $Y, Z - Add unsigned immediate (also LDA)
                let y_val = self.get_register(y);
                let result = y_val.wrapping_add(z as u64);
                self.set_register(x, result);
                self.advance_pc();
                true
            }
            // SET instructions
            0xE0 => {
                // INCL $X, $Y, $Z - Increment register $X by $Y + $Z
                // s($X) <- s($X) + (u($Y) + u($Z))
                let y_val = self.get_register(y);
                let z_val = self.get_register(z);
                let sum = y_val.wrapping_add(z_val);
                let current = self.get_register(x);
                let result = current.wrapping_add(sum);
                self.set_register(x, result);
                self.advance_pc();
                true
            }
            0xE1 => {
                // SETH $X, YZ - Set high wyde (OR with existing value)
                // u($X) <- u($X) OR (u(YZ) × 2^48)
                let yz = ((y as u64) << 8) | (z as u64);
                let value = yz << 48;
                let current = self.get_register(x);
                self.set_register(x, current | value);
                self.advance_pc();
                true
            }
            0xE2 => {
                // SETMH $X, YZ - Set medium high wyde (OR with existing value)
                // u($X) <- u($X) OR (u(YZ) × 2^32)
                let yz = ((y as u64) << 8) | (z as u64);
                let value = yz << 32;
                let current = self.get_register(x);
                self.set_register(x, current | value);
                self.advance_pc();
                true
            }
            0xE3 => {
                // SETML $X, YZ - Set medium low wyde (OR with existing value)
                // u($X) <- u($X) OR (u(YZ) × 2^16)
                let yz = ((y as u64) << 8) | (z as u64);
                let value = yz << 16;
                let current = self.get_register(x);
                self.set_register(x, current | value);
                self.advance_pc();
                true
            }
            0xE4 => {
                // SETL $X, YZ - Set low wyde (OR with existing value)
                // u($X) <- u($X) OR u(YZ)
                let yz = ((y as u64) << 8) | (z as u64);
                let current = self.get_register(x);
                self.set_register(x, current | yz);
                self.advance_pc();
                true
            }
            // Store instructions
            0xA0 => {
                // STB $X, $Y, $Z - Store byte (with overflow check)
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                let value = self.get_register(x);
                // Check if value fits in signed byte range [-128, 127]
                let signed_value = value as i64;
                if signed_value < -128 || signed_value > 127 {
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
                if signed_value < -128 || signed_value > 127 {
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
                if signed_value < -32768 || signed_value > 32767 {
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
                if signed_value < -32768 || signed_value > 32767 {
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
                if signed_value < -2147483648 || signed_value > 2147483647 {
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
                if signed_value < -2147483648 || signed_value > 2147483647 {
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
                // STCO X, $Y, $Z - Store constant octabyte
                // X is unsigned byte value (not a register)
                let addr = self.get_register(y).wrapping_add(self.get_register(z));
                self.write_octa(addr, x as u64);
                self.advance_pc();
                true
            }
            0xB1 => {
                // STCO X, $Y, Z - Store constant octabyte immediate
                let addr = self.get_register(y).wrapping_add(z as u64);
                self.write_octa(addr, x as u64);
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
                // STHT $X, $Y, Z - Store high tetra immediate
                let addr = self.get_register(y).wrapping_add(z as u64);
                let value = self.get_register(x);
                let high_tetra = (value >> 32) as u32;
                self.write_tetra(addr, high_tetra);
                self.advance_pc();
                true
            }
            // Arithmetic instructions - Add and Subtract (§9)
            0x20 => {
                // ADD $X, $Y, $Z - Add with overflow check
                let a = self.get_register(y) as i64;
                let b = self.get_register(z) as i64;
                match a.checked_add(b) {
                    Some(result) => {
                        self.set_register(x, result as u64);
                    }
                    None => {
                        // Overflow occurred - for now just wrap
                        // TODO: Set overflow flag in rA
                        self.set_register(x, a.wrapping_add(b) as u64);
                    }
                }
                self.advance_pc();
                true
            }
            0x21 => {
                // ADD $X, $Y, Z - Add immediate with overflow check
                let a = self.get_register(y) as i64;
                let b = z as i64;
                match a.checked_add(b) {
                    Some(result) => {
                        self.set_register(x, result as u64);
                    }
                    None => {
                        // Overflow occurred
                        self.set_register(x, a.wrapping_add(b) as u64);
                    }
                }
                self.advance_pc();
                true
            }
            // 0x22 and 0x23 are ADDU, already implemented above as LDA
            0x24 => {
                // 2ADDU $X, $Y, $Z - Times 2 and add unsigned
                let doubled = self.get_register(y).wrapping_mul(2);
                let sum = doubled.wrapping_add(self.get_register(z));
                self.set_register(x, sum);
                self.advance_pc();
                true
            }
            0x25 => {
                // 2ADDU $X, $Y, Z - Times 2 and add unsigned immediate
                let doubled = self.get_register(y).wrapping_mul(2);
                let sum = doubled.wrapping_add(z as u64);
                self.set_register(x, sum);
                self.advance_pc();
                true
            }
            0x26 => {
                // 4ADDU $X, $Y, $Z - Times 4 and add unsigned
                let quadrupled = self.get_register(y).wrapping_mul(4);
                let sum = quadrupled.wrapping_add(self.get_register(z));
                self.set_register(x, sum);
                self.advance_pc();
                true
            }
            0x27 => {
                // 4ADDU $X, $Y, Z - Times 4 and add unsigned immediate
                let quadrupled = self.get_register(y).wrapping_mul(4);
                let sum = quadrupled.wrapping_add(z as u64);
                self.set_register(x, sum);
                self.advance_pc();
                true
            }
            0x28 => {
                // 8ADDU $X, $Y, $Z - Times 8 and add unsigned
                let octupled = self.get_register(y).wrapping_mul(8);
                let sum = octupled.wrapping_add(self.get_register(z));
                self.set_register(x, sum);
                self.advance_pc();
                true
            }
            0x29 => {
                // 8ADDU $X, $Y, Z - Times 8 and add unsigned immediate
                let octupled = self.get_register(y).wrapping_mul(8);
                let sum = octupled.wrapping_add(z as u64);
                self.set_register(x, sum);
                self.advance_pc();
                true
            }
            0x2A => {
                // 16ADDU $X, $Y, $Z - Times 16 and add unsigned
                let multiplied = self.get_register(y).wrapping_mul(16);
                let sum = multiplied.wrapping_add(self.get_register(z));
                self.set_register(x, sum);
                self.advance_pc();
                true
            }
            0x2B => {
                // 16ADDU $X, $Y, Z - Times 16 and add unsigned immediate
                let multiplied = self.get_register(y).wrapping_mul(16);
                let sum = multiplied.wrapping_add(z as u64);
                self.set_register(x, sum);
                self.advance_pc();
                true
            }
            0x30 => {
                // SUB $X, $Y, $Z - Subtract with overflow check
                let a = self.get_register(y) as i64;
                let b = self.get_register(z) as i64;
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
            0x31 => {
                // SUB $X, $Y, Z - Subtract immediate with overflow check
                let a = self.get_register(y) as i64;
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
            0x32 => {
                // SUBU $X, $Y, $Z - Subtract unsigned
                let diff = self.get_register(y).wrapping_sub(self.get_register(z));
                self.set_register(x, diff);
                self.advance_pc();
                true
            }
            0x33 => {
                // SUBU $X, $Y, Z - Subtract unsigned immediate
                let diff = self.get_register(y).wrapping_sub(z as u64);
                self.set_register(x, diff);
                self.advance_pc();
                true
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
            _ => {
                // Unknown opcode - halt
                eprintln!("Unknown opcode: {:#02x} at PC={:#018x}", op, self.pc);
                false
            }
        }
    }

    /// Execute instructions starting from the current PC until a halt condition.
    /// Returns the number of instructions executed.
    pub fn run(&mut self) -> usize {
        let mut count = 0;
        while self.execute_instruction() {
            count += 1;
            // Safety limit to prevent infinite loops during development
            if count >= 10000 {
                eprintln!("Warning: Execution limit reached (10000 instructions)");
                break;
            }
        }
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
        // INCL $1, $2, $3 - opcode 0xE0, X=1, Y=2, Z=3
        mmix.write_tetra(0, 0xE0010203);
        mmix.set_register(1, 50);
        mmix.set_register(2, 30);
        mmix.set_register(3, 20);

        let result = mmix.execute_instruction();
        assert!(result); // Should continue
        assert_eq!(mmix.get_register(1), 100); // 50 + (30 + 20)
        assert_eq!(mmix.get_pc(), 4); // PC advanced
    }

    #[test]
    fn test_incl_with_zero() {
        let mut mmix = MMix::new();
        // INCL $2, $0, $0 - both $0 are zero
        mmix.write_tetra(0, 0xE0020000);
        mmix.set_register(2, 42);

        mmix.execute_instruction();
        assert_eq!(mmix.get_register(2), 42); // No change
    }

    #[test]
    fn test_incl_overflow() {
        let mut mmix = MMix::new();
        // INCL $3, $4, $5
        mmix.write_tetra(0, 0xE0030405);
        mmix.set_register(3, u64::MAX - 5);
        mmix.set_register(4, 3);
        mmix.set_register(5, 10);

        mmix.execute_instruction();
        assert_eq!(mmix.get_register(3), 7); // Wraps around
    }

    #[test]
    fn test_incl_large_values() {
        let mut mmix = MMix::new();
        // INCL $1, $2, $3
        mmix.write_tetra(0, 0xE0010203);
        mmix.set_register(1, 100);
        mmix.set_register(2, 1000000);
        mmix.set_register(3, 2000000);

        mmix.execute_instruction();
        assert_eq!(mmix.get_register(1), 3000100); // 100 + (1000000 + 2000000)
    }

    #[test]
    fn test_incl_register_255() {
        let mut mmix = MMix::new();
        // INCL $255, $1, $2 - should not modify $255
        mmix.write_tetra(0, 0xE0FF0102);
        mmix.set_register(1, 50);
        mmix.set_register(2, 50);

        mmix.execute_instruction();
        assert_eq!(mmix.get_register(255), 0); // Still zero
    }

    #[test]
    fn test_incl_using_255() {
        let mut mmix = MMix::new();
        // INCL $1, $255, $2 - $255 is always 0
        mmix.write_tetra(0, 0xE001FF02);
        mmix.set_register(1, 100);
        mmix.set_register(2, 50);

        mmix.execute_instruction();
        assert_eq!(mmix.get_register(1), 150); // 100 + (0 + 50)
    }

    #[test]
    fn test_run_multiple_incl() {
        let mut mmix = MMix::new();
        // Program: 3 INCL instructions
        mmix.write_tetra(0, 0xE0010000); // INCL $1, $0, $0 (no change)
        mmix.write_tetra(4, 0xE0010203); // INCL $1, $2, $3
        mmix.write_tetra(8, 0xE0010203); // INCL $1, $2, $3
        mmix.write_tetra(12, 0x00000000); // Invalid opcode (halt)

        mmix.set_register(2, 10);
        mmix.set_register(3, 5);

        let count = mmix.run();
        assert_eq!(count, 3);
        assert_eq!(mmix.get_register(1), 30); // 0 + (10+5) + (10+5)
        assert_eq!(mmix.get_pc(), 12);
    }

    #[test]
    fn test_unknown_opcode_halts() {
        let mut mmix = MMix::new();
        // Invalid opcode
        mmix.write_tetra(0, 0x12345678);

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
    fn test_ldht_high_tetra() {
        let mut mmix = MMix::new();
        // LDHT $1, $2, $3 - Load high tetra
        mmix.write_tetra(0, 0x90010203);
        mmix.set_register(2, 100);
        mmix.set_register(3, 50);
        mmix.write_tetra(150, 0x12345678);

        mmix.execute_instruction();
        assert_eq!(mmix.get_register(1), 0x1234567800000000);
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_ldht_immediate() {
        let mut mmix = MMix::new();
        // LDHT $1, $2, 12 - Load high tetra with immediate
        mmix.write_tetra(0, 0x9101020C);
        mmix.set_register(2, 200);
        mmix.write_tetra(212, 0xABCDEF00);

        mmix.execute_instruction();
        assert_eq!(mmix.get_register(1), 0xABCDEF0000000000);
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
        mmix.write_tetra(0, 0xE1010001);
        mmix.execute_instruction();
        assert_eq!(mmix.get_register(1), 0x0001000000000000);

        // SETMH $2, 0x0002
        mmix.set_pc(4);
        mmix.write_tetra(4, 0xE2020002);
        mmix.execute_instruction();
        assert_eq!(mmix.get_register(2), 0x0000000200000000);

        // SETML $3, 0x0003
        mmix.set_pc(8);
        mmix.write_tetra(8, 0xE3030003);
        mmix.execute_instruction();
        assert_eq!(mmix.get_register(3), 0x0000000000030000);

        // SETL $4, 0x0004
        mmix.set_pc(12);
        mmix.write_tetra(12, 0xE4040004);
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
        mmix.write_tetra(0, 0xE0010203);
        mmix.set_register(2, 10);
        mmix.set_register(3, 20);
        mmix.execute_instruction();
        assert_eq!(mmix.get_register(1), 30);
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
        mmix.write_tetra(0, 0xB02A0203); // X=42 (0x2A)
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
        mmix.write_tetra(0, 0xB1FF025A); // X=255 (0xFF)
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
        mmix.write_tetra(0, 0x24010203);
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
        mmix.write_tetra(0, 0x25010207);
        mmix.set_register(2, 12);

        mmix.execute_instruction();
        assert_eq!(mmix.get_register(1), 31); // 2*12 + 7 = 31
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_4addu_register() {
        let mut mmix = MMix::new();
        // 4ADDU $1, $2, $3
        mmix.write_tetra(0, 0x26010203);
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
        mmix.write_tetra(0, 0x27010208);
        mmix.set_register(2, 10);

        mmix.execute_instruction();
        assert_eq!(mmix.get_register(1), 48); // 4*10 + 8 = 48
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_8addu_register() {
        let mut mmix = MMix::new();
        // 8ADDU $1, $2, $3
        mmix.write_tetra(0, 0x28010203);
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
        mmix.write_tetra(0, 0x2901020F);
        mmix.set_register(2, 10);

        mmix.execute_instruction();
        assert_eq!(mmix.get_register(1), 95); // 8*10 + 15 = 95
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_16addu_register() {
        let mut mmix = MMix::new();
        // 16ADDU $1, $2, $3
        mmix.write_tetra(0, 0x2A010203);
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
        mmix.write_tetra(0, 0x2B010214);
        mmix.set_register(2, 10);

        mmix.execute_instruction();
        assert_eq!(mmix.get_register(1), 180); // 16*10 + 20 = 180
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_sub_positive_result() {
        let mut mmix = MMix::new();
        // SUB $1, $2, $3
        mmix.write_tetra(0, 0x30010203);
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
        mmix.write_tetra(0, 0x30010203);
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
        mmix.write_tetra(0, 0x31010219);
        mmix.set_register(2, 100);

        mmix.execute_instruction();
        assert_eq!(mmix.get_register(1), 75);
        assert_eq!(mmix.get_pc(), 4);
    }

    #[test]
    fn test_subu_wrapping() {
        let mut mmix = MMix::new();
        // SUBU $1, $2, $3
        mmix.write_tetra(0, 0x32010203);
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
        mmix.write_tetra(0, 0x3301021E);
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
        mmix.write_tetra(0, 0x28010203);
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
}
