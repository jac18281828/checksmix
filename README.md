# ChecksMix - Blazing Fast MMIX Emulator

MMIX assembler and emulator with fast feedback for learning, experimenting, and debugging Knuth’s 64-bit machine. MIX source still parses and emulates, but the checksmix now focuses on MMIX with `.mms` and `.mmo` workflows.

## What’s inside
- `checksmix`: execute `.mms` assembly directly or run prebuilt `.mmo` object files.
- `mmixasm`: assemble `.mms` to `.mmo` for reuse or distribution.
- Emulator: 256 general-purpose registers, 32 special registers, sparse 64-bit address space, and basic TRAP support (Halt and Fputs for console output).

## Quick start
1) Install Rust (stable toolchain).  
2) From the repo root, run an example immediately:

```bash
cargo run --bin checksmix -- examples/hello_world.mms
```

Or build once for repeat runs:

```bash
cargo build --release
./target/release/checksmix examples/hello_world.mms
```

Set `RUST_LOG=checksmix=debug` to see instruction decoding and TRAP handling while you experiment.

## Common workflows

### Run MMIX assembly directly
`checksmix` parses and executes `.mms` without producing an object file:

```bash
cargo run --bin checksmix -- examples/linked_list.mms
```

### Assemble to MMO, then emulate
Generate a reusable object file with `mmixasm`, then run it with `checksmix`:

```bash
# Assemble
cargo run --bin mmixasm -- examples/hello_world.mms target/hello_world.mmo

# Execute the MMO
cargo run --bin checksmix -- target/hello_world.mmo
```

### Use your own program
1) Write MMIX assembly (see the snippet below).  
2) Run it directly with `checksmix` **or** assemble with `mmixasm` and run the resulting `.mmo`.  
3) Inspect register and memory dumps printed before and after execution.

## Example programs

- `examples/hello_world.mms`: prints a string via `TRAP 0,Fputs,StdOut`.
- `examples/linked_list.mms`: walks a statically allocated list and sums node values.
- `examples/all_instructions_test.mms`: broad instruction coverage for regression checks.

Hello World (trimmed):

```asm
        LOC     Data_Segment
        GREG    @
Text    BYTE    "Hello world!",'\n',0

        LOC     #100
Main    LDA     $0,Text
        TRAP    0,Fputs,StdOut
        TRAP    0,Halt,0
```

## Legacy MIX support
`.mix` and `.mixal` files still run through `checksmix`, but MMIX is the primary target. Prefer `.mms`/`.mmo` for new work.

## Tribute

Donald Knuth has been one of the formative influences in my career. Early on—as a junior developer just beginning to feel like a mid-level engineer—I implemented his external, file-based merge sort to collate insurance datasets that were far too large for memory. That experience taught me alot about how to think about programming and system design.

Knuth’s blend of rigor, playfulness, and generosity has shaped how I write code and how I view the craft of software.  Some time later, I submitted a “bug” in The Art of Computer Programming- to earn the coveted Knuth “hexadecimal dollar.” His reply was short and perfect:

“e is as real as any other number.”

Evidently!

This project carries a little of that spirit forward: curiosity, precision, and the belief that programming can be serious fun.
