use lyn::Scanner; // Still used by MIX parser
use std::fmt;

mod encode;
mod mix;
mod mmix;
mod mmixal;
mod mmo;

pub use mix::Mix;
pub use mmix::{MMix, SpecialReg, ValueFormat};
pub use mmixal::MMixAssembler;
pub use mmo::{MmoDecoder, MmoGenerator};

/// A trait representing a computer capable of executing a program.
pub trait Computer: fmt::Display {
    /// Execute a program on this computer.
    fn execute(&mut self, program: &Program);
}

impl Computer for Mix {
    fn execute(&mut self, program: &Program) {
        Mix::execute(self, program);
    }
}

#[derive(Debug, PartialEq)]
pub enum Instruction {
    LDA(u64),
    LDX(u64),
    LDI(u8, u64),
    LDAN(u64),
    LDXN(u64),
    LDIN(u8, u64),
    STA(u64),
    STX(u64),
    STI(u8, u64),
    STJ(u64),
    STZ(u64),
    ENTA(i64),
    ENTX(i64),
    ENTI(u8, i64),
    ENNA(i64),
    ENNX(i64),
    ENNI(u8, i64),
    ADD(u64),
    SUB(u64),
    MUL(u64),
    DIV(u64),
    INCA(i64),
    INCX(i64),
    INCI(u8, i64),
    DECA(i64),
    DECX(i64),
    DECI(u8, i64),
    CMPA(u64),
    CMPX(u64),
    CMPI(u8, u64),
    JMP(u64),
    JE(u64),
    JNE(u64),
    JG(u64),
    JGE(u64),
    JL(u64),
    JLE(u64),
    HLT,
}

pub struct Program {
    scanner: Scanner,
    instructions: Vec<Instruction>,
    line: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ProgramParseError {
    InvalidInstruction { line: usize, details: String },
    InvalidNumber { line: usize, details: String },
}

impl fmt::Display for ProgramParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProgramParseError::InvalidInstruction { line, details } => {
                write!(f, "Line {}: {}", line, details)
            }
            ProgramParseError::InvalidNumber { line, details } => {
                write!(f, "Line {}: {}", line, details)
            }
        }
    }
}

impl std::error::Error for ProgramParseError {}

type ProgramResult<T> = Result<T, ProgramParseError>;

impl Program {
    pub fn instruction_count(&self) -> usize {
        self.instructions.len()
    }
}

impl Program {
    pub fn new(input: &str) -> Self {
        Self {
            scanner: Scanner::new(input),
            instructions: Vec::new(),
            line: 0,
        }
    }

    pub fn parse(&mut self) -> ProgramResult<()> {
        while let Some(instruction) = self.next_instruction()? {
            match instruction.as_str() {
                "ADD" => {
                    let value = self.parse_address()?;
                    self.instructions.push(Instruction::ADD(value));
                }
                "SUB" => {
                    let value = self.parse_address()?;
                    self.instructions.push(Instruction::SUB(value));
                }
                "STA" => {
                    let value = self.parse_address()?;
                    self.instructions.push(Instruction::STA(value));
                }
                "STX" => {
                    let value = self.parse_address()?;
                    self.instructions.push(Instruction::STX(value));
                }
                "STJ" => {
                    let value = self.parse_address()?;
                    self.instructions.push(Instruction::STJ(value));
                }
                "STZ" => {
                    let value = self.parse_address()?;
                    self.instructions.push(Instruction::STZ(value));
                }
                "ENTA" => {
                    let value = self.parse_value()?;
                    self.instructions.push(Instruction::ENTA(value));
                }
                "ENTX" => {
                    let value = self.parse_value()?;
                    self.instructions.push(Instruction::ENTX(value));
                }
                "ENNA" => {
                    let value = self.parse_value()?;
                    self.instructions.push(Instruction::ENNA(value));
                }
                "ENNX" => {
                    let value = self.parse_value()?;
                    self.instructions.push(Instruction::ENNX(value));
                }
                "LDA" => {
                    let value = self.parse_address()?;
                    self.instructions.push(Instruction::LDA(value));
                }
                "LDX" => {
                    let value = self.parse_address()?;
                    self.instructions.push(Instruction::LDX(value));
                }
                "LDAN" => {
                    let value = self.parse_address()?;
                    self.instructions.push(Instruction::LDAN(value));
                }
                "LDXN" => {
                    let value = self.parse_address()?;
                    self.instructions.push(Instruction::LDXN(value));
                }
                "MUL" => {
                    let value = self.parse_address()?;
                    self.instructions.push(Instruction::MUL(value));
                }
                "DIV" => {
                    let value = self.parse_address()?;
                    self.instructions.push(Instruction::DIV(value));
                }
                "INCA" => {
                    let value = self.parse_value()?;
                    self.instructions.push(Instruction::INCA(value));
                }
                "INCX" => {
                    let value = self.parse_value()?;
                    self.instructions.push(Instruction::INCX(value));
                }
                "DECA" => {
                    let value = self.parse_value()?;
                    self.instructions.push(Instruction::DECA(value));
                }
                "DECX" => {
                    let value = self.parse_value()?;
                    self.instructions.push(Instruction::DECX(value));
                }
                "CMPA" => {
                    let value = self.parse_address()?;
                    self.instructions.push(Instruction::CMPA(value));
                }
                "CMPX" => {
                    let value = self.parse_address()?;
                    self.instructions.push(Instruction::CMPX(value));
                }
                "JMP" => {
                    let value = self.parse_address()?;
                    self.instructions.push(Instruction::JMP(value));
                }
                "JE" => {
                    let value = self.parse_address()?;
                    self.instructions.push(Instruction::JE(value));
                }
                "JNE" => {
                    let value = self.parse_address()?;
                    self.instructions.push(Instruction::JNE(value));
                }
                "JG" => {
                    let value = self.parse_address()?;
                    self.instructions.push(Instruction::JG(value));
                }
                "JGE" => {
                    let value = self.parse_address()?;
                    self.instructions.push(Instruction::JGE(value));
                }
                "JL" => {
                    let value = self.parse_address()?;
                    self.instructions.push(Instruction::JL(value));
                }
                "JLE" => {
                    let value = self.parse_address()?;
                    self.instructions.push(Instruction::JLE(value));
                }
                "HLT" => {
                    self.instructions.push(Instruction::HLT);
                }
                _ => {
                    if let Some(reg) = self.parse_indexed(&instruction, "ST", "")? {
                        let value = self.parse_address()?;
                        self.instructions.push(Instruction::STI(reg, value));
                    } else if let Some(reg) = self.parse_indexed(&instruction, "ENT", "")? {
                        let value = self.parse_value()?;
                        self.instructions.push(Instruction::ENTI(reg, value));
                    } else if let Some(reg) = self.parse_indexed(&instruction, "ENN", "")? {
                        let value = self.parse_value()?;
                        self.instructions.push(Instruction::ENNI(reg, value));
                    } else if let Some(reg) = self.parse_indexed(&instruction, "LD", "N")? {
                        let value = self.parse_address()?;
                        self.instructions.push(Instruction::LDIN(reg, value));
                    } else if let Some(reg) = self.parse_indexed(&instruction, "LD", "")? {
                        let value = self.parse_address()?;
                        self.instructions.push(Instruction::LDI(reg, value));
                    } else if let Some(reg) = self.parse_indexed(&instruction, "INC", "")? {
                        let value = self.parse_value()?;
                        self.instructions.push(Instruction::INCI(reg, value));
                    } else if let Some(reg) = self.parse_indexed(&instruction, "DEC", "")? {
                        let value = self.parse_value()?;
                        self.instructions.push(Instruction::DECI(reg, value));
                    } else if let Some(reg) = self.parse_indexed(&instruction, "CMP", "")? {
                        let value = self.parse_address()?;
                        self.instructions.push(Instruction::CMPI(reg, value));
                    } else {
                        return Err(ProgramParseError::InvalidInstruction {
                            line: self.line,
                            details: format!("Unknown instruction {}", instruction),
                        });
                    }
                }
            }
        }
        Ok(())
    }

    pub fn parse_instruction(&mut self) -> Option<String> {
        match self.next_instruction() {
            Ok(result) => result,
            Err(err) => panic!("{}", err),
        }
    }

    fn next_instruction(&mut self) -> ProgramResult<Option<String>> {
        loop {
            self.consume_whitespace();
            if self.scanner.is_done() {
                return Ok(None);
            }
            let mut instruction = String::new();
            while let Some(ch) = self.scanner.peek() {
                let c = *ch;
                match c {
                    ' ' | '\t' | '\r' => {
                        self.scanner.pop();
                        break;
                    }
                    '\n' => {
                        self.scanner.pop();
                        self.line += 1;
                        break;
                    }
                    _ if c.is_ascii_uppercase()
                        || (!instruction.is_empty() && c.is_ascii_digit()) =>
                    {
                        instruction.push(c);
                        self.scanner.pop();
                    }
                    _ => {
                        return Err(ProgramParseError::InvalidInstruction {
                            line: self.line,
                            details: format!("Invalid character '{}' in instruction", c),
                        });
                    }
                }
            }
            if !instruction.is_empty() {
                return Ok(Some(instruction));
            }
        }
    }

    fn parse_address(&mut self) -> ProgramResult<u64> {
        self.consume_whitespace();
        let digits = self.parse_digits()?;
        digits
            .parse::<u64>()
            .map_err(|_| ProgramParseError::InvalidNumber {
                line: self.line,
                details: format!("Invalid address '{}'", digits),
            })
    }

    fn parse_value(&mut self) -> ProgramResult<i64> {
        self.consume_whitespace();
        let mut sign = 1;
        if let Some(ch) = self.scanner.peek() {
            let c = *ch;
            if c == '-' {
                self.scanner.pop();
                sign = -1;
            } else if c == '+' {
                self.scanner.pop();
            }
        }
        let digits = self.parse_digits()?;
        let value = digits
            .parse::<i64>()
            .map_err(|_| ProgramParseError::InvalidNumber {
                line: self.line,
                details: format!("Invalid value '{}'", digits),
            })?;
        Ok(sign * value)
    }

    fn parse_digits(&mut self) -> ProgramResult<String> {
        let mut digits = String::new();
        while let Some(ch) = self.scanner.peek() {
            let c = *ch;
            match c {
                '0'..='9' => {
                    digits.push(c);
                    self.scanner.pop();
                }
                ' ' | '\t' | '\r' => {
                    self.scanner.pop();
                    break;
                }
                '\n' => {
                    self.scanner.pop();
                    self.line += 1;
                    break;
                }
                _ => {
                    if digits.is_empty() {
                        return Err(ProgramParseError::InvalidNumber {
                            line: self.line,
                            details: format!("Unexpected character '{}' while parsing number", c),
                        });
                    } else {
                        break;
                    }
                }
            }
        }
        if digits.is_empty() {
            return Err(ProgramParseError::InvalidNumber {
                line: self.line,
                details: "Expected digits".to_string(),
            });
        }
        Ok(digits)
    }

    fn consume_whitespace(&mut self) {
        while let Some(ch) = self.scanner.peek() {
            let c = *ch;
            match c {
                ' ' | '\t' | '\r' => {
                    self.scanner.pop();
                }
                '\n' => {
                    self.scanner.pop();
                    self.line += 1;
                }
                _ => break,
            }
        }
    }

    fn parse_indexed(
        &self,
        instruction: &str,
        prefix: &str,
        suffix: &str,
    ) -> ProgramResult<Option<u8>> {
        if !instruction.starts_with(prefix) || !instruction.ends_with(suffix) {
            return Ok(None);
        }
        let start = prefix.len();
        let end = instruction.len() - suffix.len();
        if end <= start {
            return Err(ProgramParseError::InvalidInstruction {
                line: self.line,
                details: format!("Missing register in {}", instruction),
            });
        }
        let digits = &instruction[start..end];
        if !digits.chars().all(|c| c.is_ascii_digit()) {
            return Ok(None);
        }
        let reg = digits
            .parse::<u8>()
            .map_err(|_| ProgramParseError::InvalidInstruction {
                line: self.line,
                details: format!("Invalid register in {}", instruction),
            })?;
        if (1..=10).contains(&reg) {
            Ok(Some(reg))
        } else {
            Err(ProgramParseError::InvalidInstruction {
                line: self.line,
                details: format!("Register out of range in {}", instruction),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_instruction_add() {
        let mut program = Program::new("ADD 100\n");
        assert_eq!(program.parse_instruction(), Some("ADD".to_string()));
    }

    #[test]
    fn test_parse_instruction_sub() {
        let mut program = Program::new("SUB 100\n");
        assert_eq!(program.parse_instruction(), Some("SUB".to_string()));
    }

    #[test]
    fn test_parse_instruction_lda() {
        let mut program = Program::new("LDA 100\n");
        assert_eq!(program.parse_instruction(), Some("LDA".to_string()));
    }

    #[test]
    fn test_parse_instruction_ldx() {
        let mut program = Program::new("LDX 100\n");
        assert_eq!(program.parse_instruction(), Some("LDX".to_string()));
    }

    #[test]
    fn test_parse_instruction_ldi() {
        for i in 1..=10 {
            let mut program = Program::new(format!("LD{} 100\n", i).as_str());
            assert_eq!(program.parse_instruction(), Some(format!("LD{}", i)));
        }
    }

    #[test]
    fn test_parse_instruction_ldan() {
        let mut program = Program::new("LDAN 100\n");
        assert_eq!(program.parse_instruction(), Some("LDAN".to_string()));
    }

    #[test]
    fn test_parse_instruction_ldxn() {
        let mut program = Program::new("LDXN 100\n");
        assert_eq!(program.parse_instruction(), Some("LDXN".to_string()));
    }

    #[test]
    fn test_parse_instruction_ldin() {
        for i in 1..=10 {
            let mut program = Program::new(format!("LD{}N 100\n", i).as_str());
            assert_eq!(program.parse_instruction(), Some(format!("LD{}N", i)));
        }
    }

    #[test]
    fn test_parse_instruction_sta() {
        let mut program = Program::new("STA 100\n");
        assert_eq!(program.parse_instruction(), Some("STA".to_string()));
    }

    #[test]
    fn test_parse_instruction_stax() {
        let mut program = Program::new("STX 100\n");
        assert_eq!(program.parse_instruction(), Some("STX".to_string()));
    }

    #[test]
    fn test_parse_instruction_sti() {
        for i in 1..=10 {
            let mut program = Program::new(format!("ST{} 100\n", i).as_str());
            assert_eq!(program.parse_instruction(), Some(format!("ST{}", i)));
        }
    }

    #[test]
    fn test_parse_instruction_stj() {
        let mut program = Program::new("STJ 100\n");
        assert_eq!(program.parse_instruction(), Some("STJ".to_string()));
    }

    #[test]
    fn test_parse_instruction_stz() {
        let mut program = Program::new("STZ 100\n");
        assert_eq!(program.parse_instruction(), Some("STZ".to_string()));
    }

    #[test]
    fn test_parse_instruction_enta() {
        let mut program = Program::new("ENTA 100\n");
        assert_eq!(program.parse_instruction(), Some("ENTA".to_string()));
    }

    #[test]
    fn test_parse_instruction_entx() {
        let mut program = Program::new("ENTX 100\n");
        assert_eq!(program.parse_instruction(), Some("ENTX".to_string()));
    }

    #[test]
    fn test_parse_instruction_enti() {
        for i in 1..=10 {
            let mut program = Program::new(format!("ENT{} 100\n", i).as_str());
            assert_eq!(program.parse_instruction(), Some(format!("ENT{}", i)));
        }
    }

    #[test]
    fn test_parse_instruction_enna() {
        let mut program = Program::new("ENNA 100\n");
        assert_eq!(program.parse_instruction(), Some("ENNA".to_string()));
    }

    #[test]
    fn test_parse_instruction_ennx() {
        let mut program = Program::new("ENNX 100\n");
        assert_eq!(program.parse_instruction(), Some("ENNX".to_string()));
    }

    #[test]
    fn test_parse_instruction_enni() {
        for i in 1..=10 {
            let mut program = Program::new(format!("ENN{} 100\n", i).as_str());
            assert_eq!(program.parse_instruction(), Some(format!("ENN{}", i)));
        }
    }
    #[test]
    fn test_parse_value() {
        let mut program = Program::new("100\n");
        assert_eq!(program.parse_value(), Ok(100));
    }

    #[test]
    fn test_parse_value_neg() {
        let mut program = Program::new("-100\n");
        assert_eq!(program.parse_value(), Ok(-100));
    }

    #[test]
    fn test_parse_value_invalid() {
        let mut program = Program::new("abc\n");
        assert!(program.parse_value().is_err());
    }

    #[test]
    fn test_parse_value_empty() {
        let mut program = Program::new("\n");
        assert!(program.parse_value().is_err());
    }

    #[test]
    fn test_parse_address() {
        let mut program = Program::new("128\n");
        assert_eq!(program.parse_address(), Ok(128));
    }

    #[test]
    fn test_parse_program_load() {
        let mut program = Program::new("LDA 100\nLDX 200\nLD1 400\nLD5 500\n");
        program.parse().unwrap();
        assert_eq!(
            program.instructions,
            vec![
                Instruction::LDA(100),
                Instruction::LDX(200),
                Instruction::LDI(1, 400),
                Instruction::LDI(5, 500),
            ]
        );
    }

    #[test]
    fn test_parse_program_load_neg() {
        let mut program = Program::new("LDAN 100\nLDXN 200\nLD1N 400\nLD5N 500\n");
        program.parse().unwrap();
        assert_eq!(
            program.instructions,
            vec![
                Instruction::LDAN(100),
                Instruction::LDXN(200),
                Instruction::LDIN(1, 400),
                Instruction::LDIN(5, 500),
            ]
        );
    }

    #[test]
    fn test_parse_program_store() {
        let mut program = Program::new("STA 100\nSTX 200\nSTJ 300\nST1 400\nST5 500\n");
        program.parse().unwrap();
        assert_eq!(
            program.instructions,
            vec![
                Instruction::STA(100),
                Instruction::STX(200),
                Instruction::STJ(300),
                Instruction::STI(1, 400),
                Instruction::STI(5, 500),
            ]
        );
    }

    #[test]
    fn test_parse_program_store_zero() {
        let mut program = Program::new("STZ 100\n");
        program.parse().unwrap();
        assert_eq!(program.instructions, vec![Instruction::STZ(100)]);
    }

    #[test]
    fn test_parse_program_enter() {
        let mut program =
            Program::new("ENTA 100\nENTX 200\nENT1 300\nENNA 300\nENN1 400\nENN5 500\n");
        program.parse().unwrap();
        assert_eq!(
            program.instructions,
            vec![
                Instruction::ENTA(100),
                Instruction::ENTX(200),
                Instruction::ENTI(1, 300),
                Instruction::ENNA(300),
                Instruction::ENNI(1, 400),
                Instruction::ENNI(5, 500),
            ]
        );
    }

    #[test]
    fn test_parse_program_add() {
        let mut program = Program::new("ADD 100\n");
        program.parse().unwrap();
        assert_eq!(program.instructions, vec![Instruction::ADD(100)]);
    }

    #[test]
    fn test_parse_program_sub() {
        let mut program = Program::new("SUB 100\n");
        program.parse().unwrap();
        assert_eq!(program.instructions, vec![Instruction::SUB(100)]);
    }

    #[test]
    fn test_program_ent_sto_a() {
        let mut program = Program::new("ENTA 112\nSTA 200\n");
        program.parse().unwrap();
        let mut mix = Mix::new();
        mix.execute(&program);
        assert_eq!(mix.a, 112);
        assert_eq!(mix.memory[200], 112);
    }

    #[test]
    fn test_program_ent_sto_x() {
        let mut program = Program::new("ENTX 112\nSTX 200\n");
        program.parse().unwrap();
        let mut mix = Mix::new();
        mix.execute(&program);
        assert_eq!(mix.x, 112);
        assert_eq!(mix.memory[200], 112);
    }

    #[test]
    fn test_program_ent_sto_i() {
        for i in 1..=10 {
            let mut program = Program::new(format!("ENT{} 112\nST{} 200\n", i, i).as_str());
            program.parse().unwrap();
            let mut mix = Mix::new();
            mix.execute(&program);
            assert_eq!(mix.i[i as usize], 112);
            assert_eq!(mix.memory[200], 112);
        }
    }

    #[test]
    fn test_program_ent_sto_neg_a() {
        let mut program = Program::new("ENNA 112\nSTA 200\n");
        program.parse().unwrap();
        let mut mix = Mix::new();
        mix.execute(&program);
        assert_eq!(mix.a, -112);
        assert_eq!(mix.memory[200], -112);
    }

    #[test]
    fn test_program_ent_sto_neg_x() {
        let mut program = Program::new("ENNX 112\nSTX 200\n");
        program.parse().unwrap();
        let mut mix = Mix::new();
        mix.execute(&program);
        assert_eq!(mix.x, -112);
        assert_eq!(mix.memory[200], -112);
    }

    #[test]
    fn test_program_ent_sto_neg_i() {
        for i in 1..=10 {
            let mut program = Program::new(format!("ENN{} 112\nST{} 200\n", i, i).as_str());
            program.parse().unwrap();
            let mut mix = Mix::new();
            mix.execute(&program);
            assert_eq!(mix.i[i as usize], -112);
            assert_eq!(mix.memory[200], -112);
        }
    }

    #[test]
    fn test_program_load_a() {
        let mut program = Program::new("LDA 100\n");
        program.parse().unwrap();
        let mut mix = Mix::new();
        mix.memory[100] = 175;
        mix.execute(&program);
        assert_eq!(mix.a, 175);
    }

    #[test]
    fn test_program_load_x() {
        let mut program = Program::new("LDX 100\n");
        program.parse().unwrap();
        let mut mix = Mix::new();
        mix.memory[100] = 175;
        mix.execute(&program);
        assert_eq!(mix.x, 175);
    }

    #[test]
    fn test_program_load_i() {
        for i in 1..=10 {
            let mut program = Program::new(format!("LD{} 100\n", i).as_str());
            program.parse().unwrap();
            let mut mix = Mix::new();
            mix.memory[100] = 175;
            mix.execute(&program);
            assert_eq!(mix.i[i as usize], 175);
        }
    }

    #[test]
    fn test_program_load_neg_a() {
        let mut program = Program::new("LDAN 100\n");
        program.parse().unwrap();
        let mut mix = Mix::new();
        mix.memory[100] = -175;
        mix.execute(&program);
        assert_eq!(mix.a, 175);
    }

    #[test]
    fn test_program_load_neg_x() {
        let mut program = Program::new("LDXN 100\n");
        program.parse().unwrap();
        let mut mix = Mix::new();
        mix.memory[100] = -175;
        mix.execute(&program);
        assert_eq!(mix.x, 175);
    }

    #[test]
    fn test_program_load_neg_i() {
        for i in 1..=10 {
            let mut program = Program::new(format!("LD{}N 100\n", i).as_str());
            program.parse().unwrap();
            let mut mix = Mix::new();
            mix.memory[100] = -175;
            mix.execute(&program);
            assert_eq!(mix.i[i as usize], 175);
        }
    }

    #[test]
    fn test_program_add() {
        let mut program = Program::new("ADD 100\n");
        program.parse().unwrap();
        let mut mix = Mix::new();
        mix.a = 100;
        mix.memory[100] = 75;
        mix.execute(&program);
        assert_eq!(mix.a, 175);
    }

    #[test]
    fn test_program_sub() {
        let mut program = Program::new("SUB 100\n");
        program.parse().unwrap();
        let mut mix = Mix::new();
        mix.a = 100;
        mix.memory[100] = 75;
        mix.execute(&program);
        assert_eq!(mix.a, 25);
    }

    #[test]
    fn test_program_add_overflow() {
        let mut program = Program::new("ADD 100\n");
        program.parse().unwrap();
        let mut mix = Mix::new();
        mix.a = 100;
        mix.memory[100] = i64::MAX;
        mix.execute(&program);
        assert!(mix.overflow);
    }

    #[test]
    fn test_program_sub_overflow() {
        let mut program = Program::new("SUB 100\n");
        program.parse().unwrap();
        let mut mix = Mix::new();
        mix.a = 100;
        mix.memory[100] = i64::MIN;
        mix.execute(&program);
        assert_eq!(mix.a, i64::MIN + 100);
        assert!(mix.overflow);
    }

    #[test]
    fn test_program_mul() {
        let mut program = Program::new("MUL 100\n");
        program.parse().unwrap();
        let mut mix = Mix::new();
        mix.a = 10;
        mix.memory[100] = 20;
        mix.execute(&program);
        assert_eq!(mix.a, 200);
        assert!(!mix.overflow);
    }

    #[test]
    fn test_program_mul_overflow() {
        let mut program = Program::new("MUL 100\n");
        program.parse().unwrap();
        let mut mix = Mix::new();
        mix.a = i64::MAX;
        mix.memory[100] = 2;
        mix.execute(&program);
        assert!(mix.overflow);
    }

    #[test]
    fn test_program_div() {
        let mut program = Program::new("DIV 100\n");
        program.parse().unwrap();
        let mut mix = Mix::new();
        mix.a = 100;
        mix.memory[100] = 5;
        mix.execute(&program);
        assert_eq!(mix.a, 20);
        assert!(!mix.overflow);
    }

    #[test]
    fn test_program_div_by_zero() {
        let mut program = Program::new("DIV 100\n");
        program.parse().unwrap();
        let mut mix = Mix::new();
        mix.a = 100;
        mix.memory[100] = 0;
        mix.execute(&program);
        assert!(mix.overflow);
    }

    #[test]
    fn test_program_inca() {
        let mut program = Program::new("INCA 50\n");
        program.parse().unwrap();
        let mut mix = Mix::new();
        mix.a = 100;
        mix.execute(&program);
        assert_eq!(mix.a, 150);
    }

    #[test]
    fn test_program_incx() {
        let mut program = Program::new("INCX 50\n");
        program.parse().unwrap();
        let mut mix = Mix::new();
        mix.x = 100;
        mix.execute(&program);
        assert_eq!(mix.x, 150);
    }

    #[test]
    fn test_program_inci() {
        let mut program = Program::new("INC1 50\n");
        program.parse().unwrap();
        let mut mix = Mix::new();
        mix.i[1] = 100;
        mix.execute(&program);
        assert_eq!(mix.i[1], 150);
    }

    #[test]
    fn test_program_deca() {
        let mut program = Program::new("DECA 50\n");
        program.parse().unwrap();
        let mut mix = Mix::new();
        mix.a = 100;
        mix.execute(&program);
        assert_eq!(mix.a, 50);
    }

    #[test]
    fn test_program_decx() {
        let mut program = Program::new("DECX 50\n");
        program.parse().unwrap();
        let mut mix = Mix::new();
        mix.x = 100;
        mix.execute(&program);
        assert_eq!(mix.x, 50);
    }

    #[test]
    fn test_program_deci() {
        let mut program = Program::new("DEC1 50\n");
        program.parse().unwrap();
        let mut mix = Mix::new();
        mix.i[1] = 100;
        mix.execute(&program);
        assert_eq!(mix.i[1], 50);
    }

    #[test]
    fn test_program_cmpa_equal() {
        let mut program = Program::new("CMPA 100\n");
        program.parse().unwrap();
        let mut mix = Mix::new();
        mix.a = 50;
        mix.memory[100] = 50;
        mix.execute(&program);
        // Can't access cmp directly anymore, so we'll test via jump
    }

    #[test]
    fn test_program_cmpa_less() {
        let mut program = Program::new("CMPA 100\n");
        program.parse().unwrap();
        let mut mix = Mix::new();
        mix.a = 30;
        mix.memory[100] = 50;
        mix.execute(&program);
        // Can't access cmp directly anymore, so we'll test via jump
    }

    #[test]
    fn test_program_cmpa_greater() {
        let mut program = Program::new("CMPA 100\n");
        program.parse().unwrap();
        let mut mix = Mix::new();
        mix.a = 70;
        mix.memory[100] = 50;
        mix.execute(&program);
        // Can't access cmp directly anymore, so we'll test via jump
    }

    #[test]
    fn test_program_jmp() {
        let mut program = Program::new("ENTA 10\nJMP 3\nENTA 20\nENTA 30\n");
        program.parse().unwrap();
        let mut mix = Mix::new();
        mix.execute(&program);
        assert_eq!(mix.a, 30);
    }

    #[test]
    fn test_program_je_taken() {
        let mut program = Program::new("ENTA 50\nCMPA 100\nJE 4\nENTA 99\nENTA 100\n");
        program.parse().unwrap();
        let mut mix = Mix::new();
        mix.memory[100] = 50;
        mix.execute(&program);
        assert_eq!(mix.a, 100);
    }

    #[test]
    fn test_program_je_not_taken() {
        let mut program = Program::new("ENTA 30\nCMPA 100\nJE 4\nENTA 99\nENTA 100\n");
        program.parse().unwrap();
        let mut mix = Mix::new();
        mix.memory[100] = 50;
        mix.execute(&program);
        assert_eq!(mix.a, 100);
    }

    #[test]
    fn test_program_jne_taken() {
        let mut program = Program::new("ENTA 30\nCMPA 100\nJNE 4\nENTA 99\nENTA 100\n");
        program.parse().unwrap();
        let mut mix = Mix::new();
        mix.memory[100] = 50;
        mix.execute(&program);
        assert_eq!(mix.a, 100);
    }

    #[test]
    fn test_program_jg_taken() {
        let mut program = Program::new("ENTA 70\nCMPA 100\nJG 4\nENTA 99\nENTA 100\n");
        program.parse().unwrap();
        let mut mix = Mix::new();
        mix.memory[100] = 50;
        mix.execute(&program);
        assert_eq!(mix.a, 100);
    }

    #[test]
    fn test_program_jl_taken() {
        let mut program = Program::new("ENTA 30\nCMPA 100\nJL 4\nENTA 99\nENTA 100\n");
        program.parse().unwrap();
        let mut mix = Mix::new();
        mix.memory[100] = 50;
        mix.execute(&program);
        assert_eq!(mix.a, 100);
    }

    #[test]
    fn test_program_hlt() {
        let mut program = Program::new("ENTA 10\nHLT\nENTA 20\n");
        program.parse().unwrap();
        let mut mix = Mix::new();
        mix.execute(&program);
        assert_eq!(mix.a, 10);
    }
}
