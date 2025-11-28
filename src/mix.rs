use crate::{Instruction, Program};
use std::fmt;

#[derive(Debug)]
pub(crate) enum Comparison {
    LessThan = -1,
    EqualTo = 0,
    GreaterThan = 1,
}

pub struct Mix {
    pub(crate) a: i64,
    pub(crate) x: i64,
    pub(crate) i: Vec<i64>,
    pub(crate) j: u64,
    pub(crate) overflow: bool,
    pub(crate) cmp: Comparison,
    pub(crate) memory: Vec<i64>,
}

impl Default for Mix {
    fn default() -> Self {
        Self::new()
    }
}

impl Mix {
    pub fn new() -> Self {
        Self {
            a: 0,
            x: 0,
            i: vec![0; 10],
            j: 0,
            overflow: false,
            cmp: Comparison::EqualTo,
            memory: vec![0; 4000],
        }
    }

    pub fn execute(&mut self, program: &Program) {
        let mut pc = 0;
        while pc < program.instructions.len() {
            let instruction = &program.instructions[pc];
            println!("[PC={}] Executing: {:?}", pc, instruction);
            println!(
                "  Before: A={} X={} I1={} Overflow={}",
                self.a, self.x, self.i[1], self.overflow
            );
            match instruction {
                Instruction::ADD(addr) => {
                    let value = self.memory[*addr as usize];
                    let (result, overflow) = self.a.overflowing_add(value);
                    self.a = result;
                    self.overflow = overflow;
                }
                Instruction::SUB(addr) => {
                    let value = self.memory[*addr as usize];
                    let (result, overflow) = self.a.overflowing_sub(value);
                    self.a = result;
                    self.overflow = overflow;
                }
                Instruction::STA(addr) => {
                    self.memory[*addr as usize] = self.a;
                }
                Instruction::STX(addr) => {
                    self.memory[*addr as usize] = self.x;
                }
                Instruction::STI(n, addr) => {
                    self.memory[*addr as usize] = self.i[*n as usize];
                }
                Instruction::STJ(addr) => {
                    self.memory[*addr as usize] = self.j as i64;
                }
                Instruction::STZ(addr) => {
                    self.memory[*addr as usize] = 0;
                }
                Instruction::ENTA(value) => {
                    self.a = *value;
                }
                Instruction::ENTX(value) => {
                    self.x = *value;
                }
                Instruction::ENTI(n, value) => {
                    self.i[*n as usize] = *value;
                }
                Instruction::ENNA(value) => {
                    self.a = -*value;
                }
                Instruction::ENNX(value) => {
                    self.x = -*value;
                }
                Instruction::ENNI(n, value) => {
                    self.i[*n as usize] = -*value;
                }
                Instruction::LDA(addr) => {
                    self.a = self.memory[*addr as usize];
                }
                Instruction::LDX(addr) => {
                    self.x = self.memory[*addr as usize];
                }
                Instruction::LDI(n, addr) => {
                    self.i[*n as usize] = self.memory[*addr as usize];
                }
                Instruction::LDAN(addr) => {
                    self.a = -self.memory[*addr as usize];
                }
                Instruction::LDXN(addr) => {
                    self.x = -self.memory[*addr as usize];
                }
                Instruction::LDIN(n, addr) => {
                    self.i[*n as usize] = -self.memory[*addr as usize];
                }
                Instruction::MUL(addr) => {
                    let value = self.memory[*addr as usize];
                    let (result, overflow) = self.a.overflowing_mul(value);
                    self.a = result;
                    self.overflow = overflow;
                }
                Instruction::DIV(addr) => {
                    let value = self.memory[*addr as usize];
                    if value == 0 {
                        self.overflow = true;
                    } else {
                        let (result, overflow) = self.a.overflowing_div(value);
                        self.a = result;
                        self.overflow = overflow;
                    }
                }
                Instruction::INCA(value) => {
                    let (result, overflow) = self.a.overflowing_add(*value);
                    self.a = result;
                    self.overflow = overflow;
                }
                Instruction::INCX(value) => {
                    let (result, overflow) = self.x.overflowing_add(*value);
                    self.x = result;
                    self.overflow = overflow;
                }
                Instruction::INCI(n, value) => {
                    let (result, overflow) = self.i[*n as usize].overflowing_add(*value);
                    self.i[*n as usize] = result;
                    self.overflow = overflow;
                }
                Instruction::DECA(value) => {
                    let (result, overflow) = self.a.overflowing_sub(*value);
                    self.a = result;
                    self.overflow = overflow;
                }
                Instruction::DECX(value) => {
                    let (result, overflow) = self.x.overflowing_sub(*value);
                    self.x = result;
                    self.overflow = overflow;
                }
                Instruction::DECI(n, value) => {
                    let (result, overflow) = self.i[*n as usize].overflowing_sub(*value);
                    self.i[*n as usize] = result;
                    self.overflow = overflow;
                }
                Instruction::CMPA(addr) => {
                    let value = self.memory[*addr as usize];
                    self.cmp = if self.a < value {
                        Comparison::LessThan
                    } else if self.a > value {
                        Comparison::GreaterThan
                    } else {
                        Comparison::EqualTo
                    };
                }
                Instruction::CMPX(addr) => {
                    let value = self.memory[*addr as usize];
                    self.cmp = if self.x < value {
                        Comparison::LessThan
                    } else if self.x > value {
                        Comparison::GreaterThan
                    } else {
                        Comparison::EqualTo
                    };
                }
                Instruction::CMPI(n, addr) => {
                    let value = self.memory[*addr as usize];
                    let reg_value = self.i[*n as usize];
                    self.cmp = if reg_value < value {
                        Comparison::LessThan
                    } else if reg_value > value {
                        Comparison::GreaterThan
                    } else {
                        Comparison::EqualTo
                    };
                }
                Instruction::JMP(addr) => {
                    pc = *addr as usize;
                    continue;
                }
                Instruction::JE(addr) => {
                    if matches!(self.cmp, Comparison::EqualTo) {
                        pc = *addr as usize;
                        continue;
                    }
                }
                Instruction::JNE(addr) => {
                    if !matches!(self.cmp, Comparison::EqualTo) {
                        pc = *addr as usize;
                        continue;
                    }
                }
                Instruction::JG(addr) => {
                    if matches!(self.cmp, Comparison::GreaterThan) {
                        pc = *addr as usize;
                        continue;
                    }
                }
                Instruction::JGE(addr) => {
                    if !matches!(self.cmp, Comparison::LessThan) {
                        pc = *addr as usize;
                        continue;
                    }
                }
                Instruction::JL(addr) => {
                    if matches!(self.cmp, Comparison::LessThan) {
                        pc = *addr as usize;
                        continue;
                    }
                }
                Instruction::JLE(addr) => {
                    if !matches!(self.cmp, Comparison::GreaterThan) {
                        pc = *addr as usize;
                        continue;
                    }
                }
                Instruction::HLT => {
                    println!("Program halted");
                    break;
                }
            }
            println!(
                "  After:  A={} X={} I1={} Overflow={}",
                self.a, self.x, self.i[1], self.overflow
            );
            println!();
            pc += 1;
        }
    }
}

impl fmt::Display for Mix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Registers:")?;
        writeln!(f, "  A  = {}", self.a)?;
        writeln!(f, "  X  = {}", self.x)?;
        writeln!(
            f,
            "  I1 = {}  I2 = {}  I3 = {}  I4 = {}  I5 = {}",
            self.i[1], self.i[2], self.i[3], self.i[4], self.i[5]
        )?;
        writeln!(
            f,
            "  I6 = {}  I7 = {}  I8 = {}  I9 = {}",
            self.i[6], self.i[7], self.i[8], self.i[9]
        )?;
        writeln!(f, "  J  = {}", self.j)?;
        writeln!(f, "  Overflow = {}", self.overflow)?;
        writeln!(f, "  Comparison = {:?}", self.cmp)?;

        // Show non-zero memory locations
        writeln!(f, "\nMemory (non-zero locations):")?;
        let mut has_nonzero = false;
        for (i, &value) in self.memory.iter().enumerate() {
            if value != 0 {
                writeln!(f, "  [{}] = {}", i, value)?;
                has_nonzero = true;
            }
        }
        if !has_nonzero {
            writeln!(f, "  (all zero)")?;
        }

        Ok(())
    }
}
