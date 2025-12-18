use crate::{Instruction, Program};
use std::convert::TryFrom;
use std::fmt;

#[derive(Debug)]
pub(crate) enum Comparison {
    LessThan = -1,
    EqualTo = 0,
    GreaterThan = 1,
}

enum MixStep {
    Advance,
    Jump(usize),
    Halt,
}

#[derive(Debug)]
enum MixExecutionError {
    InvalidAddress(u64),
    InvalidRegister(u8),
}

impl fmt::Display for MixExecutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MixExecutionError::InvalidAddress(addr) => {
                write!(f, "Invalid memory address {}", addr)
            }
            MixExecutionError::InvalidRegister(reg) => {
                write!(f, "Invalid index register {}", reg)
            }
        }
    }
}

impl std::error::Error for MixExecutionError {}

pub struct Mix {
    pub(crate) a: i64,
    pub(crate) x: i64,
    pub(crate) i: [i64; 11],
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
            i: [0; 11],
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
            match self.execute_step(instruction) {
                Ok(MixStep::Advance) => {
                    println!(
                        "  After:  A={} X={} I1={} Overflow={}",
                        self.a, self.x, self.i[1], self.overflow
                    );
                    println!();
                    pc += 1;
                }
                Ok(MixStep::Jump(target)) => {
                    pc = target;
                    continue;
                }
                Ok(MixStep::Halt) => {
                    println!("Program halted");
                    break;
                }
                Err(err) => {
                    eprintln!("Execution error: {}", err);
                    break;
                }
            }
        }
    }

    fn execute_step(&mut self, instruction: &Instruction) -> Result<MixStep, MixExecutionError> {
        match instruction {
            Instruction::ADD(addr) => {
                let value = self.read_memory(*addr)?;
                let (result, overflow) = self.a.overflowing_add(value);
                self.a = result;
                self.overflow = overflow;
                Ok(MixStep::Advance)
            }
            Instruction::SUB(addr) => {
                let value = self.read_memory(*addr)?;
                let (result, overflow) = self.a.overflowing_sub(value);
                self.a = result;
                self.overflow = overflow;
                Ok(MixStep::Advance)
            }
            Instruction::STA(addr) => {
                self.write_memory(*addr, self.a)?;
                Ok(MixStep::Advance)
            }
            Instruction::STX(addr) => {
                self.write_memory(*addr, self.x)?;
                Ok(MixStep::Advance)
            }
            Instruction::STI(n, addr) => {
                let value = *self.index(*n)?;
                self.write_memory(*addr, value)?;
                Ok(MixStep::Advance)
            }
            Instruction::STJ(addr) => {
                self.write_memory(*addr, self.j as i64)?;
                Ok(MixStep::Advance)
            }
            Instruction::STZ(addr) => {
                self.write_memory(*addr, 0)?;
                Ok(MixStep::Advance)
            }
            Instruction::ENTA(value) => {
                self.a = *value;
                Ok(MixStep::Advance)
            }
            Instruction::ENTX(value) => {
                self.x = *value;
                Ok(MixStep::Advance)
            }
            Instruction::ENTI(n, value) => {
                *self.index_mut(*n)? = *value;
                Ok(MixStep::Advance)
            }
            Instruction::ENNA(value) => {
                self.a = -*value;
                Ok(MixStep::Advance)
            }
            Instruction::ENNX(value) => {
                self.x = -*value;
                Ok(MixStep::Advance)
            }
            Instruction::ENNI(n, value) => {
                *self.index_mut(*n)? = -*value;
                Ok(MixStep::Advance)
            }
            Instruction::LDA(addr) => {
                self.a = self.read_memory(*addr)?;
                Ok(MixStep::Advance)
            }
            Instruction::LDX(addr) => {
                self.x = self.read_memory(*addr)?;
                Ok(MixStep::Advance)
            }
            Instruction::LDI(n, addr) => {
                *self.index_mut(*n)? = self.read_memory(*addr)?;
                Ok(MixStep::Advance)
            }
            Instruction::LDAN(addr) => {
                self.a = -self.read_memory(*addr)?;
                Ok(MixStep::Advance)
            }
            Instruction::LDXN(addr) => {
                self.x = -self.read_memory(*addr)?;
                Ok(MixStep::Advance)
            }
            Instruction::LDIN(n, addr) => {
                *self.index_mut(*n)? = -self.read_memory(*addr)?;
                Ok(MixStep::Advance)
            }
            Instruction::MUL(addr) => {
                let value = self.read_memory(*addr)?;
                let (result, overflow) = self.a.overflowing_mul(value);
                self.a = result;
                self.overflow = overflow;
                Ok(MixStep::Advance)
            }
            Instruction::DIV(addr) => {
                let value = self.read_memory(*addr)?;
                if value == 0 {
                    self.overflow = true;
                } else {
                    let (result, overflow) = self.a.overflowing_div(value);
                    self.a = result;
                    self.overflow = overflow;
                }
                Ok(MixStep::Advance)
            }
            Instruction::INCA(value) => {
                let (result, overflow) = self.a.overflowing_add(*value);
                self.a = result;
                self.overflow = overflow;
                Ok(MixStep::Advance)
            }
            Instruction::INCX(value) => {
                let (result, overflow) = self.x.overflowing_add(*value);
                self.x = result;
                self.overflow = overflow;
                Ok(MixStep::Advance)
            }
            Instruction::INCI(n, value) => {
                let reg = self.index_mut(*n)?;
                let (result, overflow) = reg.overflowing_add(*value);
                *reg = result;
                self.overflow = overflow;
                Ok(MixStep::Advance)
            }
            Instruction::DECA(value) => {
                let (result, overflow) = self.a.overflowing_sub(*value);
                self.a = result;
                self.overflow = overflow;
                Ok(MixStep::Advance)
            }
            Instruction::DECX(value) => {
                let (result, overflow) = self.x.overflowing_sub(*value);
                self.x = result;
                self.overflow = overflow;
                Ok(MixStep::Advance)
            }
            Instruction::DECI(n, value) => {
                let reg = self.index_mut(*n)?;
                let (result, overflow) = reg.overflowing_sub(*value);
                *reg = result;
                self.overflow = overflow;
                Ok(MixStep::Advance)
            }
            Instruction::CMPA(addr) => {
                let value = self.read_memory(*addr)?;
                self.cmp = if self.a < value {
                    Comparison::LessThan
                } else if self.a > value {
                    Comparison::GreaterThan
                } else {
                    Comparison::EqualTo
                };
                Ok(MixStep::Advance)
            }
            Instruction::CMPX(addr) => {
                let value = self.read_memory(*addr)?;
                self.cmp = if self.x < value {
                    Comparison::LessThan
                } else if self.x > value {
                    Comparison::GreaterThan
                } else {
                    Comparison::EqualTo
                };
                Ok(MixStep::Advance)
            }
            Instruction::CMPI(n, addr) => {
                let value = self.read_memory(*addr)?;
                let reg_value = *self.index(*n)?;
                self.cmp = if reg_value < value {
                    Comparison::LessThan
                } else if reg_value > value {
                    Comparison::GreaterThan
                } else {
                    Comparison::EqualTo
                };
                Ok(MixStep::Advance)
            }
            Instruction::JMP(addr) => Ok(MixStep::Jump(self.jump_target(*addr)?)),
            Instruction::JE(addr) => {
                if matches!(self.cmp, Comparison::EqualTo) {
                    Ok(MixStep::Jump(self.jump_target(*addr)?))
                } else {
                    Ok(MixStep::Advance)
                }
            }
            Instruction::JNE(addr) => {
                if !matches!(self.cmp, Comparison::EqualTo) {
                    Ok(MixStep::Jump(self.jump_target(*addr)?))
                } else {
                    Ok(MixStep::Advance)
                }
            }
            Instruction::JG(addr) => {
                if matches!(self.cmp, Comparison::GreaterThan) {
                    Ok(MixStep::Jump(self.jump_target(*addr)?))
                } else {
                    Ok(MixStep::Advance)
                }
            }
            Instruction::JGE(addr) => {
                if !matches!(self.cmp, Comparison::LessThan) {
                    Ok(MixStep::Jump(self.jump_target(*addr)?))
                } else {
                    Ok(MixStep::Advance)
                }
            }
            Instruction::JL(addr) => {
                if matches!(self.cmp, Comparison::LessThan) {
                    Ok(MixStep::Jump(self.jump_target(*addr)?))
                } else {
                    Ok(MixStep::Advance)
                }
            }
            Instruction::JLE(addr) => {
                if !matches!(self.cmp, Comparison::GreaterThan) {
                    Ok(MixStep::Jump(self.jump_target(*addr)?))
                } else {
                    Ok(MixStep::Advance)
                }
            }
            Instruction::HLT => Ok(MixStep::Halt),
        }
    }

    fn memory_index(&self, addr: u64) -> Result<usize, MixExecutionError> {
        let idx = usize::try_from(addr).map_err(|_| MixExecutionError::InvalidAddress(addr))?;
        if idx < self.memory.len() {
            Ok(idx)
        } else {
            Err(MixExecutionError::InvalidAddress(addr))
        }
    }

    fn read_memory(&self, addr: u64) -> Result<i64, MixExecutionError> {
        let idx = self.memory_index(addr)?;
        Ok(self.memory[idx])
    }

    fn write_memory(&mut self, addr: u64, value: i64) -> Result<(), MixExecutionError> {
        let idx = self.memory_index(addr)?;
        self.memory[idx] = value;
        Ok(())
    }

    fn index(&self, reg: u8) -> Result<&i64, MixExecutionError> {
        if (1..=10).contains(&reg) {
            Ok(&self.i[reg as usize])
        } else {
            Err(MixExecutionError::InvalidRegister(reg))
        }
    }

    fn index_mut(&mut self, reg: u8) -> Result<&mut i64, MixExecutionError> {
        if (1..=10).contains(&reg) {
            Ok(&mut self.i[reg as usize])
        } else {
            Err(MixExecutionError::InvalidRegister(reg))
        }
    }

    fn jump_target(&self, addr: u64) -> Result<usize, MixExecutionError> {
        usize::try_from(addr).map_err(|_| MixExecutionError::InvalidAddress(addr))
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
            "  I6 = {}  I7 = {}  I8 = {}  I9 = {}  I10 = {}",
            self.i[6], self.i[7], self.i[8], self.i[9], self.i[10]
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
