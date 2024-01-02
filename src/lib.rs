use lyn::Scanner;

enum Comparison {
    LessThan = -1,
    EqualTo = 0,
    GreaterThan = 1,
}

pub struct MMix {
    a: i64,
    x: i64,
    i: Vec<i64>,
    j: u64,
    overflow: bool,
    cmp: Comparison,
    memory: Vec<i64>,
}

impl MMix {
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
            }
            pc += 1;
        }
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
}

const MAX_INSTRUCTION_LENGTH: usize = 4;

pub struct Program {
    scanner: Scanner,
    instructions: Vec<Instruction>,
    line: usize,
}

impl Program {
    pub fn new(input: &str) -> Self {
        Self {
            scanner: Scanner::new(input),
            instructions: Vec::new(),
            line: 0,
        }
    }

    pub fn parse(&mut self) {
        while let Some(instruction) = self.parse_instruction() {
            match instruction.as_str() {
                "ADD" => {
                    if let Some(value) = self.parse_address() {
                        self.instructions.push(Instruction::ADD(value));
                    } else {
                        panic!("Invalid instruction at line {}", self.line)
                    }
                }
                "SUB" => {
                    if let Some(value) = self.parse_address() {
                        self.instructions.push(Instruction::SUB(value));
                    } else {
                        panic!("Invalid instruction at line {}", self.line)
                    }
                }
                "STA" => {
                    if let Some(value) = self.parse_address() {
                        self.instructions.push(Instruction::STA(value));
                    } else {
                        panic!("Invalid instruction at line {}", self.line)
                    }
                }
                "STX" => {
                    if let Some(value) = self.parse_address() {
                        self.instructions.push(Instruction::STX(value));
                    } else {
                        panic!("Invalid instruction at line {}", self.line)
                    }
                }
                "ST1" | "ST2" | "ST3" | "ST4" | "ST5" | "ST6" | "ST7" | "ST8" | "ST9" | "ST10" => {
                    let n = instruction.chars().nth(2).unwrap().to_digit(10).unwrap() as u8;
                    if let Some(value) = self.parse_address() {
                        self.instructions.push(Instruction::STI(n, value));
                    } else {
                        panic!("Invalid instruction at line {}", self.line)
                    }
                }
                "STJ" => {
                    if let Some(value) = self.parse_address() {
                        self.instructions.push(Instruction::STJ(value));
                    } else {
                        panic!("Invalid instruction at line {}", self.line)
                    }
                }
                "STZ" => {
                    if let Some(value) = self.parse_address() {
                        self.instructions.push(Instruction::STZ(value));
                    } else {
                        panic!("Invalid instruction at line {}", self.line)
                    }
                }
                "ENTA" => {
                    if let Some(value) = self.parse_value() {
                        self.instructions.push(Instruction::ENTA(value));
                    } else {
                        panic!("Invalid instruction at line {}", self.line)
                    }
                }
                "ENTX" => {
                    if let Some(value) = self.parse_value() {
                        self.instructions.push(Instruction::ENTX(value));
                    } else {
                        panic!("Invalid instruction at line {}", self.line)
                    }
                }
                "ENT1" | "ENT2" | "ENT3" | "ENT4" | "ENT5" | "ENT6" | "ENT7" | "ENT8" | "ENT9"
                | "ENT10" => {
                    let n = instruction.chars().nth(3).unwrap().to_digit(10).unwrap() as u8;
                    if let Some(value) = self.parse_value() {
                        self.instructions.push(Instruction::ENTI(n, value));
                    } else {
                        panic!("Invalid instruction at line {}", self.line)
                    }
                }
                "ENNA" => {
                    if let Some(value) = self.parse_value() {
                        self.instructions.push(Instruction::ENNA(value));
                    } else {
                        panic!("Invalid instruction at line {}", self.line)
                    }
                }
                "ENNX" => {
                    if let Some(value) = self.parse_value() {
                        self.instructions.push(Instruction::ENNX(value));
                    } else {
                        panic!("Invalid instruction at line {}", self.line)
                    }
                }
                "ENN1" | "ENN2" | "ENN3" | "ENN4" | "ENN5" | "ENN6" | "ENN7" | "ENN8" | "ENN9"
                | "ENN10" => {
                    let n = instruction.chars().nth(3).unwrap().to_digit(10).unwrap() as u8;
                    if let Some(value) = self.parse_value() {
                        self.instructions.push(Instruction::ENNI(n, value));
                    } else {
                        panic!("Invalid instruction at line {}", self.line)
                    }
                }
                "LDA" => {
                    if let Some(value) = self.parse_address() {
                        self.instructions.push(Instruction::LDA(value));
                    } else {
                        panic!("Invalid instruction at line {}", self.line)
                    }
                }
                "LDX" => {
                    if let Some(value) = self.parse_address() {
                        self.instructions.push(Instruction::LDX(value));
                    } else {
                        panic!("Invalid instruction at line {}", self.line)
                    }
                }
                "LD1" | "LD2" | "LD3" | "LD4" | "LD5" | "LD6" | "LD7" | "LD8" | "LD9" | "LD10" => {
                    let n = instruction.chars().nth(2).unwrap().to_digit(10).unwrap() as u8;
                    if let Some(value) = self.parse_address() {
                        self.instructions.push(Instruction::LDI(n, value));
                    } else {
                        panic!("Invalid instruction at line {}", self.line)
                    }
                }
                "LDAN" => {
                    if let Some(value) = self.parse_address() {
                        self.instructions.push(Instruction::LDAN(value));
                    } else {
                        panic!("Invalid instruction at line {}", self.line)
                    }
                }
                "LDXN" => {
                    if let Some(value) = self.parse_address() {
                        self.instructions.push(Instruction::LDXN(value));
                    } else {
                        panic!("Invalid instruction at line {}", self.line)
                    }
                }
                "LD1N" | "LD2N" | "LD3N" | "LD4N" | "LD5N" | "LD6N" | "LD7N" | "LD8N" | "LD9N" => {
                    let n = instruction.chars().nth(2).unwrap().to_digit(10).unwrap() as u8;
                    if let Some(value) = self.parse_address() {
                        self.instructions.push(Instruction::LDIN(n, value));
                    } else {
                        panic!("Invalid instruction at line {}", self.line)
                    }
                }
                _ => panic!("Unknown instruction at line {}", self.line),
            }
        }
    }

    pub fn parse_instruction(&mut self) -> Option<String> {
        let mut instruction = String::new();
        while !self.scanner.is_done() {
            let ch = self.scanner.pop();
            if ch.is_none() {
                break;
            }
            let c = ch.unwrap();
            match c {
                ' ' => break,
                '\n' => {
                    self.line += 1;
                    break;
                }
                '\t' => break,
                '\r' => break,
                _ => {
                    if c.is_ascii_uppercase() || (instruction.len() >= 2 && c.is_ascii_digit()) {
                        instruction.push(*c)
                    } else {
                        panic!("Invalid instruction at line {}", self.line)
                    }
                }
            }
        }
        if instruction.is_empty() {
            return None;
        }
        if instruction.len() > MAX_INSTRUCTION_LENGTH {
            panic!("Invalid instruction at line {}", self.line)
        }
        Some(instruction)
    }

    fn parse_address(&mut self) -> Option<u64> {
        let value = self.parse_digit_string();
        if let Some(value) = value {
            return Some(value.parse().unwrap());
        }
        None
    }

    fn parse_value(&mut self) -> Option<i64> {
        let ch = self.scanner.peek();
        if ch.is_none() {
            return None;
        }
        let c = ch.unwrap();
        let mut sign = 1;
        if *c == '-' {
            self.scanner.pop();
            sign = -1;
        }
        let value = self.parse_digit_string();
        if let Some(value) = value {
            let value = value.parse::<i64>().unwrap();
            return Some(sign * value);
        }
        None
    }

    fn parse_digit_string(&mut self) -> Option<String> {
        let mut value = String::new();
        while !self.scanner.is_done() {
            let ch = self.scanner.pop();
            if ch.is_none() {
                break;
            }
            let c = ch.unwrap();
            match c {
                ' ' => break,
                '\n' => {
                    self.line += 1;
                    break;
                }
                '\t' => break,
                '\r' => break,
                _ => {
                    if c.is_ascii_digit() {
                        value.push(*c)
                    } else if value.is_empty() {
                        break;
                    } else {
                        panic!("Invalid value at line {}", self.line)
                    }
                }
            }
        }
        if value.is_empty() {
            return None;
        }
        Some(value)
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
        for i in 1..10 {
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
        for i in 1..10 {
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
        for i in 1..10 {
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
        for i in 1..10 {
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
        for i in 1..10 {
            let mut program = Program::new(format!("ENN{} 100\n", i).as_str());
            assert_eq!(program.parse_instruction(), Some(format!("ENN{}", i)));
        }
    }
    #[test]
    fn test_parse_value() {
        let mut program = Program::new("100\n");
        assert_eq!(program.parse_value(), Some(100));
    }

    #[test]
    fn test_parse_value_neg() {
        let mut program = Program::new("-100\n");
        assert_eq!(program.parse_value(), Some(-100));
    }

    #[test]
    fn test_parse_value_invalid() {
        let mut program = Program::new("abc\n");
        assert_eq!(program.parse_value(), None);
    }

    #[test]
    fn test_parse_value_empty() {
        let mut program = Program::new("\n");
        assert_eq!(program.parse_value(), None);
    }

    #[test]
    fn test_parse_address() {
        let mut program = Program::new("128\n");
        assert_eq!(program.parse_address(), Some(128));
    }

    #[test]
    fn test_parse_program_load() {
        let mut program = Program::new("LDA 100\nLDX 200\nLD1 400\nLD5 500\n");
        program.parse();
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
        program.parse();
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
        program.parse();
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
        program.parse();
        assert_eq!(program.instructions, vec![Instruction::STZ(100)]);
    }

    #[test]
    fn test_parse_program_enter() {
        let mut program =
            Program::new("ENTA 100\nENTX 200\nENT1 300\nENNA 300\nENN1 400\nENN5 500\n");
        program.parse();
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
        program.parse();
        assert_eq!(program.instructions, vec![Instruction::ADD(100)]);
    }

    #[test]
    fn test_parse_program_sub() {
        let mut program = Program::new("SUB 100\n");
        program.parse();
        assert_eq!(program.instructions, vec![Instruction::SUB(100)]);
    }

    #[test]
    fn test_program_ent_sto_a() {
        let mut program = Program::new("ENTA 112\nSTA 200\n");
        program.parse();
        let mut mmix = MMix::new();
        mmix.execute(&program);
        assert_eq!(mmix.a, 112);
        assert_eq!(mmix.memory[200], 112);
    }

    #[test]
    fn test_program_ent_sto_x() {
        let mut program = Program::new("ENTX 112\nSTX 200\n");
        program.parse();
        let mut mmix = MMix::new();
        mmix.execute(&program);
        assert_eq!(mmix.x, 112);
        assert_eq!(mmix.memory[200], 112);
    }

    #[test]
    fn test_program_ent_sto_i() {
        for i in 1..10 {
            let mut program = Program::new(format!("ENT{} 112\nST{} 200\n", i, i).as_str());
            program.parse();
            let mut mmix = MMix::new();
            mmix.execute(&program);
            assert_eq!(mmix.i[i as usize], 112);
            assert_eq!(mmix.memory[200], 112);
        }
    }

    #[test]
    fn test_program_ent_sto_neg_a() {
        let mut program = Program::new("ENNA 112\nSTA 200\n");
        program.parse();
        let mut mmix = MMix::new();
        mmix.execute(&program);
        assert_eq!(mmix.a, -112);
        assert_eq!(mmix.memory[200], -112);
    }

    #[test]
    fn test_program_ent_sto_neg_x() {
        let mut program = Program::new("ENNX 112\nSTX 200\n");
        program.parse();
        let mut mmix = MMix::new();
        mmix.execute(&program);
        assert_eq!(mmix.x, -112);
        assert_eq!(mmix.memory[200], -112);
    }

    #[test]
    fn test_program_ent_sto_neg_i() {
        for i in 1..10 {
            let mut program = Program::new(format!("ENN{} 112\nST{} 200\n", i, i).as_str());
            program.parse();
            let mut mmix = MMix::new();
            mmix.execute(&program);
            assert_eq!(mmix.i[i as usize], -112);
            assert_eq!(mmix.memory[200], -112);
        }
    }

    #[test]
    fn test_program_load_a() {
        let mut program = Program::new("LDA 100\n");
        program.parse();
        let mut mmix = MMix::new();
        mmix.memory[100] = 175;
        mmix.execute(&program);
        assert_eq!(mmix.a, 175);
    }

    #[test]
    fn test_program_load_x() {
        let mut program = Program::new("LDX 100\n");
        program.parse();
        let mut mmix = MMix::new();
        mmix.memory[100] = 175;
        mmix.execute(&program);
        assert_eq!(mmix.x, 175);
    }

    #[test]
    fn test_program_load_i() {
        for i in 1..10 {
            let mut program = Program::new(format!("LD{} 100\n", i).as_str());
            program.parse();
            let mut mmix = MMix::new();
            mmix.memory[100] = 175;
            mmix.execute(&program);
            assert_eq!(mmix.i[i as usize], 175);
        }
    }

    #[test]
    fn test_program_load_neg_a() {
        let mut program = Program::new("LDAN 100\n");
        program.parse();
        let mut mmix = MMix::new();
        mmix.memory[100] = -175;
        mmix.execute(&program);
        assert_eq!(mmix.a, 175);
    }

    #[test]
    fn test_program_load_neg_x() {
        let mut program = Program::new("LDXN 100\n");
        program.parse();
        let mut mmix = MMix::new();
        mmix.memory[100] = -175;
        mmix.execute(&program);
        assert_eq!(mmix.x, 175);
    }

    #[test]
    fn test_program_load_neg_i() {
        for i in 1..10 {
            let mut program = Program::new(format!("LD{}N 100\n", i).as_str());
            program.parse();
            let mut mmix = MMix::new();
            mmix.memory[100] = -175;
            mmix.execute(&program);
            assert_eq!(mmix.i[i as usize], 175);
        }
    }

    #[test]
    fn test_program_add() {
        let mut program = Program::new("ADD 100\n");
        program.parse();
        let mut mmix = MMix::new();
        mmix.a = 100;
        mmix.memory[100] = 75;
        mmix.execute(&program);
        assert_eq!(mmix.a, 175);
    }

    #[test]
    fn test_program_sub() {
        let mut program = Program::new("SUB 100\n");
        program.parse();
        let mut mmix = MMix::new();
        mmix.a = 100;
        mmix.memory[100] = 75;
        mmix.execute(&program);
        assert_eq!(mmix.a, 25);
    }

    #[test]
    fn test_program_add_overflow() {
        let mut program = Program::new("ADD 100\n");
        program.parse();
        let mut mmix = MMix::new();
        mmix.a = 100;
        mmix.memory[100] = i64::MAX;
        mmix.execute(&program);
        assert_eq!(mmix.overflow, true);
    }

    #[test]
    fn test_program_sub_overflow() {
        let mut program = Program::new("SUB 100\n");
        program.parse();
        let mut mmix = MMix::new();
        mmix.a = 100;
        mmix.memory[100] = i64::MIN;
        mmix.execute(&program);
        assert_eq!(mmix.a, i64::MIN + 100);
        assert_eq!(mmix.overflow, true);
    }
}
