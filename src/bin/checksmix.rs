use checksmix::{MMix, Program};
use std::env;
use std::fs;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: {} <program_file>", args[0]);
        eprintln!("\nExample program file format:");
        eprintln!("  ENTA 100");
        eprintln!("  STA 200");
        eprintln!("  ADD 200");
        process::exit(1);
    }

    let filename = &args[1];
    let input = fs::read_to_string(filename).unwrap_or_else(|err| {
        eprintln!("Error reading file '{}': {}", filename, err);
        process::exit(1);
    });

    println!("=== Loading program from: {} ===", filename);
    println!();

    let mut program = Program::new(&input);
    program.parse();

    println!(
        "Program loaded successfully with {} instructions",
        program.instruction_count()
    );
    println!();

    let mut mmix = MMix::new();

    println!("=== Initial Machine State ===");
    println!("{}", mmix);
    println!();

    println!("=== Executing Program ===");
    mmix.execute(&program);
    println!();

    println!("=== Final Machine State ===");
    println!("{}", mmix);
    println!();

    println!("Execution completed.");
}
