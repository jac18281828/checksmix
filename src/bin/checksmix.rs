use checksmix::{MMix, MMixAssembler, Mix, Program};
use std::env;
use std::fs;
use std::path::Path;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: {} <program_file>", args[0]);
        eprintln!("Supported file extensions:");
        eprintln!("  .mix, .mixal  - MIX computer");
        eprintln!("  .mms          - MMIX assembly source");
        eprintln!("  .mmo          - MMIX object code");
        eprintln!("\nMIX Example program format:");
        eprintln!("  ENTA 100");
        eprintln!("  STA 200");
        eprintln!("  ADD 200");
        eprintln!("\nMMIX Example program format:");
        eprintln!("  SET $2, 10");
        eprintln!("  INCL $1, $2, $3");
        eprintln!("  HALT");
        process::exit(1);
    }

    let filename = &args[1];
    let path = Path::new(filename);
    let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");

    match extension {
        "mix" | "mixal" => run_mix(filename),
        "mms" => run_mms(filename),
        "mmo" => run_mmo(filename),
        _ => {
            eprintln!(
                "Unknown file extension: .{}",
                if extension.is_empty() {
                    "(none)"
                } else {
                    extension
                }
            );
            eprintln!("Supported extensions: .mix, .mixal, .mms, .mmo");
            process::exit(1);
        }
    }
}

fn run_mix(filename: &str) {
    let input = fs::read_to_string(filename).unwrap_or_else(|err| {
        eprintln!("Error reading file '{}': {}", filename, err);
        process::exit(1);
    });

    println!("=== MIX Computer ===");
    println!("=== Loading program from: {} ===", filename);
    println!();

    let mut program = Program::new(&input);
    program.parse();

    println!(
        "Program loaded successfully with {} instructions",
        program.instruction_count()
    );
    println!();

    let mut mix = Mix::new();

    println!("=== Initial Machine State ===");
    println!("{}", mix);
    println!();

    println!("=== Executing Program ===");
    mix.execute(&program);
    println!();

    println!("=== Final Machine State ===");
    println!("{}", mix);
    println!();

    println!("Execution completed.");
}

fn run_mms(filename: &str) {
    let input = fs::read_to_string(filename).unwrap_or_else(|err| {
        eprintln!("Error reading file '{}': {}", filename, err);
        process::exit(1);
    });

    println!("=== MMIX Assembler ===");
    println!("=== Parsing assembly from: {} ===", filename);
    println!();

    let mut assembler = MMixAssembler::new(&input);
    assembler.parse();

    println!("Assembly parsed successfully");
    println!();

    let object_code = assembler.generate_object_code();
    println!("Generated {} bytes of object code", object_code.len());
    println!();

    // Execute the assembled code
    let mut mmix = MMix::new();

    // Load the object code into memory starting at address 0
    for (i, &byte) in object_code.iter().enumerate() {
        mmix.write_byte(i as u64, byte);
    }

    println!("=== Initial Machine State ===");
    println!("{}", mmix);
    println!();

    println!("=== Executing Program ===");
    let count = mmix.run();
    println!();
    println!("Executed {} instructions", count);
    println!();

    println!("=== Final Machine State ===");
    println!("{}", mmix);
    println!();

    println!("Execution completed.");
}

fn run_mmo(filename: &str) {
    let data = fs::read(filename).unwrap_or_else(|err| {
        eprintln!("Error reading file '{}': {}", filename, err);
        process::exit(1);
    });

    println!("=== MMIX Computer ===");
    println!("=== Loading program from: {} ===", filename);
    println!();

    let mut mmix = MMix::new();

    // Load the binary data into memory starting at address 0
    for (i, &byte) in data.iter().enumerate() {
        mmix.write_byte(i as u64, byte);
    }

    println!("Loaded {} bytes into memory", data.len());
    println!();

    println!("=== Initial Machine State ===");
    println!("{}", mmix);
    println!();

    println!("=== Executing Program ===");
    let count = mmix.run();
    println!();
    println!("Executed {} instructions", count);
    println!();

    println!("=== Final Machine State ===");
    println!("{}", mmix);
    println!();

    println!("Execution completed.");
}
