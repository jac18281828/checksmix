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
    /// Input MMIX assembly file(s) (.mms). Multiple inputs are loaded into
    /// one shared symbol space, as if their contents were concatenated.
    #[arg(value_name = "INPUT.mms", required = true, num_args = 1..)]
    inputs: Vec<PathBuf>,

    /// Output MMO file (defaults to first input's basename with .mmo)
    #[arg(short = 'o', long = "output", value_name = "OUTPUT.mmo")]
    output: Option<PathBuf>,
}

fn main() {
    // Initialize tracing subscriber with RUST_LOG environment variable support
    // By default, no debug output unless RUST_LOG is set
    // Example: RUST_LOG=checksmix=debug cargo run --bin mmixasm -- file.mms
    fmt().with_env_filter(EnvFilter::from_default_env()).init();

    let cli = Cli::parse();
    let inputs = cli.inputs;
    let output_file = cli
        .output
        .unwrap_or_else(|| inputs[0].with_extension("mmo"));

    let mut sources: Vec<(String, String)> = Vec::with_capacity(inputs.len());
    for path in &inputs {
        let src = fs::read_to_string(path).unwrap_or_else(|err| {
            eprintln!("Error reading '{}': {}", path.display(), err);
            process::exit(1);
        });
        let name = path
            .to_str()
            .map(|s| s.to_string())
            .unwrap_or_else(|| path.display().to_string());
        sources.push((name, src));
    }

    if sources.len() == 1 {
        println!("Assembling: {}", sources[0].0);
    } else {
        println!("Assembling {} inputs:", sources.len());
        for (n, _) in &sources {
            println!("  {}", n);
        }
    }

    let (first_name, first_src) = &sources[0];
    let mut assembler = MMixAssembler::new(first_src, first_name);
    for (n, s) in sources.iter().skip(1) {
        assembler.add_source(s, n);
    }

    if let Err(e) = assembler.parse() {
        eprintln!("{}", e);
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
