/// MMIX Assembler - Compile .mms assembly files to .mmo object code
use checksmix::MMixAssembler;
use clap::Parser;
use std::fs;
use std::path::PathBuf;
use std::process;
use tracing_subscriber::{EnvFilter, fmt};

#[derive(Parser, Debug)]
#[command(
    name = "mmixasm",
    about = "Assemble MMIX .mms files into .mmo object code",
    version,
    author
)]
struct Cli {
    /// Input MMIX assembly file (.mms)
    #[arg(value_name = "INPUT.mms")]
    input: PathBuf,

    /// Output MMO file (defaults to INPUT basename with .mmo)
    #[arg(value_name = "OUTPUT.mmo")]
    output: Option<PathBuf>,
}

fn main() {
    // Initialize tracing subscriber with RUST_LOG environment variable support
    // By default, no debug output unless RUST_LOG is set
    // Example: RUST_LOG=checksmix=debug cargo run --bin mmixasm -- file.mms
    fmt().with_env_filter(EnvFilter::from_default_env()).init();

    let cli = Cli::parse();
    let input_file = cli.input;
    let output_file = cli
        .output
        .unwrap_or_else(|| input_file.with_extension("mmo"));

    // Read the input file
    let source = fs::read_to_string(&input_file).unwrap_or_else(|err| {
        eprintln!("Error reading '{}': {}", input_file.display(), err);
        process::exit(1);
    });

    println!("Assembling: {}", input_file.display());

    // Parse the assembly
    let input_name = input_file
        .to_str()
        .unwrap_or("input.mms");
    let mut assembler = MMixAssembler::new(&source, input_name);

    if let Err(e) = assembler.parse() {
        // Format error in standard assembler format: filename:line:column: message
        // If error already has "Line X:Y:" prefix, reformat it
        if e.starts_with("Line ") {
            if let Some(rest) = e.strip_prefix("Line ") {
                if let Some((line_col, msg)) = rest.split_once(": ") {
                    eprintln!("{}:{}: {}", input_name, line_col, msg);
                } else {
                    eprintln!("{}: {}", input_name, e);
                }
            } else {
                eprintln!("{}: {}", input_name, e);
            }
        } else {
            eprintln!("{}: {}", input_name, e);
        }
        process::exit(1);
    }

    // Debug: print labels and instructions
    eprintln!("Labels:");
    for (label, addr) in &assembler.labels {
        eprintln!("  {} -> 0x{:X}", label, addr);
    }
    eprintln!("Symbols:");
    for (symbol, value) in &assembler.symbols {
        eprintln!("  {} = {}", symbol, value);
    }
    if !assembler.greg_inits.is_empty() {
        eprintln!("Global Register Initializations:");
        for (reg, value) in &assembler.greg_inits {
            eprintln!("  ${} = 0x{:X}", reg, value);
        }
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
        eprintln!("Error writing '{}': {}", output_file.display(), err);
        process::exit(1);
    });

    println!("Output written to: {}", output_file.display());
}
