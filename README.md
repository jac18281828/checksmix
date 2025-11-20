# ChecksMix

A blazing fast mmix simulator.

MMIX is a computer architecture and assembly language developed by Donald Knuth. "MMIX" stands for "Mixed Integer eXperiment". It was designed as a hypothetical computer for teaching purposes and is used in Knuth's book "The Art of Computer Programming". MMIX has a 4000-word memory, 5 index registers, and a variety of instructions for arithmetic, logical, and input/output operations. It is a 32-bit architecture with a fixed instruction format. The MMIX assembly language is used to write programs for the MMIX computer.

## Implemented Instructions

### Load Instructions
- `LDA <addr>` - Load A register from memory
- `LDX <addr>` - Load X register from memory
- `LD1-LD9 <addr>` - Load index register from memory
- `LDAN <addr>` - Load negative of memory value into A
- `LDXN <addr>` - Load negative of memory value into X
- `LD1N-LD9N <addr>` - Load negative of memory value into index register

### Store Instructions
- `STA <addr>` - Store A register to memory
- `STX <addr>` - Store X register to memory
- `ST1-ST9 <addr>` - Store index register to memory
- `STJ <addr>` - Store J register to memory
- `STZ <addr>` - Store zero to memory

### Enter Instructions
- `ENTA <value>` - Enter value into A register
- `ENTX <value>` - Enter value into X register
- `ENT1-ENT9 <value>` - Enter value into index register
- `ENNA <value>` - Enter negative value into A register
- `ENNX <value>` - Enter negative value into X register
- `ENN1-ENN9 <value>` - Enter negative value into index register

### Arithmetic Instructions
- `ADD <addr>` - Add memory value to A register
- `SUB <addr>` - Subtract memory value from A register
- `MUL <addr>` - Multiply A register by memory value
- `DIV <addr>` - Divide A register by memory value

### Increment/Decrement Instructions
- `INCA <value>` - Increment A register by value
- `INCX <value>` - Increment X register by value
- `INC1-INC9 <value>` - Increment index register by value
- `DECA <value>` - Decrement A register by value
- `DECX <value>` - Decrement X register by value
- `DEC1-DEC9 <value>` - Decrement index register by value

### Comparison Instructions
- `CMPA <addr>` - Compare A register with memory value
- `CMPX <addr>` - Compare X register with memory value
- `CMP1-CMP9 <addr>` - Compare index register with memory value

### Jump Instructions
- `JMP <addr>` - Unconditional jump to address
- `JE <addr>` - Jump if equal
- `JNE <addr>` - Jump if not equal
- `JG <addr>` - Jump if greater
- `JGE <addr>` - Jump if greater or equal
- `JL <addr>` - Jump if less
- `JLE <addr>` - Jump if less or equal

### Control Instructions
- `HLT` - Halt program execution

## Features

- Full overflow detection for arithmetic operations
- Comparison indicator for conditional jumps
- 4000-word memory
- A, X registers and 9 index registers (I1-I9)
- Jump register (J)

## Usage

```bash
cargo run -- <program.mmix>
```

## Examples

See `example.mmix` and `example_full.mmix` for sample programs.

## Tribute

Donald Knuth has been one of the most formative influences in my long career. Early on—as a junior developer just beginning to feel like a mid-level engineer—I implemented his external, file-based merge sort to wrangle datasets that were far too large for memory. That experience taught me alot about how to think about programming.

Knuth’s blend of rigor, playfulness, and generosity has shaped how I write code and how I view the craft of software. His writing is equal parts instruction manual and conversation, full of wit and quiet joy.

Some time later, I worked up the courage to submit what I believed might be a genuine “bug” in The Art of Computer Programming—a chance, if I were right, to earn the Knuth “hexadecimal dollar.” His reply was short and perfect:

“e is as real as any other number.”

I laughed, I learned, and I had one of those rare moments where a hero speaks directly to you. This project carries a little of that spirit forward: curiosity, precision, and the belief that computing can be both serious and fun.
