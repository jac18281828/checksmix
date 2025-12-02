/// MMIX Assembler - Compile .mms assembly files to .mmo object code
use checksmix::MMixAssembler;
use std::env;
use std::fs;
use std::path::Path;
use std::process;
use tracing_subscriber::{EnvFilter, fmt};

fn main() {
    // Initialize tracing subscriber with RUST_LOG environment variable support
    // By default, no debug output unless RUST_LOG is set
    // Example: RUST_LOG=checksmix=debug cargo run --bin mmixasm -- file.mms
    fmt().with_env_filter(EnvFilter::from_default_env()).init();

    let args: Vec<String> = env::args().collect();

    if args.len() < 2 || args.len() > 3 {
        eprintln!("Usage: {} <input.mms> [output.mmo]", args[0]);
        eprintln!("\nAssembles MMIX assembly language (.mms) to object code (.mmo)");
        eprintln!("\nIf output file is not specified, uses input basename with .mmo extension");
        process::exit(1);
    }

    let input_file = &args[1];
    let output_file = if args.len() == 3 {
        args[2].clone()
    } else {
        // Replace extension with .mmo
        let path = Path::new(input_file);
        path.with_extension("mmo")
            .to_str()
            .unwrap_or("output.mmo")
            .to_string()
    };

    // Read the input file
    let source = fs::read_to_string(input_file).unwrap_or_else(|err| {
        eprintln!("Error reading '{}': {}", input_file, err);
        process::exit(1);
    });

    println!("Assembling: {}", input_file);

    // Parse the assembly
    let mut assembler = MMixAssembler::new(&source, input_file);

    if let Err(e) = assembler.parse() {
        // Format error in standard assembler format: filename:line:column: message
        // If error already has "Line X:Y:" prefix, reformat it
        if e.starts_with("Line ") {
            if let Some(rest) = e.strip_prefix("Line ") {
                if let Some((line_col, msg)) = rest.split_once(": ") {
                    eprintln!("{}:{}: {}", input_file, line_col, msg);
                } else {
                    eprintln!("{}: {}", input_file, e);
                }
            } else {
                eprintln!("{}: {}", input_file, e);
            }
        } else {
            eprintln!("{}: {}", input_file, e);
        }
        process::exit(1);
    }

    // Debug: print labels and instructions
    eprintln!("Labels:");
    for (label, addr) in &assembler.labels {
        eprintln!("  {} -> 0x{:X}", label, addr);
    }
    eprintln!("Instructions ({}):", assembler.instructions.len());
    for (addr, inst) in &assembler.instructions {
        eprintln!("  0x{:X}: {:?}", addr, inst);
    }

    // Check if there are any instructions to assemble
    if assembler.instructions.is_empty() {
        eprintln!("Error: No instructions to assemble");
        process::exit(1);
    }

    // Generate object code
    let object_code = assembler.generate_object_code();

    println!("Generated {} bytes of object code", object_code.len());

    // Write the output file
    fs::write(&output_file, &object_code).unwrap_or_else(|err| {
        eprintln!("Error writing '{}': {}", output_file, err);
        process::exit(1);
    });

    println!("Output written to: {}", output_file);
}
