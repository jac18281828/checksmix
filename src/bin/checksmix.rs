use checksmix::{MMix, MMixAssembler, Mix, MmoDecoder, Program, ValueFormat};
use clap::Parser;
use std::fs;
use std::path::Path;
use std::process;
use tracing_subscriber::{EnvFilter, fmt};

#[derive(Parser, Debug)]
#[command(
    name = "checksmix",
    about = "Run MIX/MMIX programs and assemblers",
    version,
    author
)]
struct Cli {
    /// Display register values as unsigned decimals (hex output unchanged)
    #[arg(long)]
    unsigned: bool,

    /// Program file(s) to execute. A single file dispatches by extension
    /// (.mix/.mixal/.mms/.mmo); multiple files must all be .mms and are
    /// assembled into one shared symbol space before execution.
    #[arg(required = true, num_args = 1..)]
    program_files: Vec<String>,
}

fn main() {
    // Initialize tracing subscriber with RUST_LOG environment variable support
    // By default, no debug output unless RUST_LOG is set
    // Example: RUST_LOG=checksmix=debug cargo run --bin checksmix -- file.mms
    fmt().with_env_filter(EnvFilter::from_default_env()).init();

    let opts = Cli::parse();
    let value_format = if opts.unsigned {
        ValueFormat::Unsigned
    } else {
        ValueFormat::Signed
    };

    if opts.program_files.len() == 1 {
        let program_file = &opts.program_files[0];
        let path = Path::new(program_file);
        let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");

        match extension {
            "mix" | "mixal" => run_mix(program_file),
            "mms" => run_mms(std::slice::from_ref(program_file), value_format),
            "mmo" => run_mmo(program_file, value_format),
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
    } else {
        for f in &opts.program_files {
            let ext = Path::new(f)
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("");
            if ext != "mms" {
                eprintln!(
                    "When passing multiple inputs, all files must be .mms (got '{}')",
                    f
                );
                process::exit(1);
            }
        }
        run_mms(&opts.program_files, value_format);
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
    if let Err(err) = program.parse() {
        eprintln!("Error: {}", err);
        process::exit(1);
    }

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

fn run_mms(filenames: &[String], value_format: ValueFormat) {
    let mut sources = Vec::with_capacity(filenames.len());
    for f in filenames {
        let input = fs::read_to_string(f).unwrap_or_else(|err| {
            eprintln!("Error reading file '{}': {}", f, err);
            process::exit(1);
        });
        sources.push((f.clone(), input));
    }

    println!("=== MMIX Assembler ===");
    if sources.len() == 1 {
        println!("=== Parsing assembly from: {} ===", sources[0].0);
    } else {
        println!("=== Parsing {} assembly inputs ===", sources.len());
        for (f, _) in &sources {
            println!("  {}", f);
        }
    }
    println!();

    let (first_file, first_src) = &sources[0];
    let mut assembler = MMixAssembler::new(first_src, first_file);
    for (f, src) in sources.iter().skip(1) {
        assembler.add_source(src, f);
    }

    if let Err(e) = assembler.parse() {
        eprintln!("Error: {}", e);
        process::exit(1);
    }

    println!("Assembly parsed successfully");
    println!();

    // Execute the assembled code
    let mut mmix = MMix::new();

    // Load instructions directly at their addresses
    for (addr, inst) in &assembler.instructions {
        let bytes = assembler.encode_instruction_bytes(inst);
        for (offset, &byte) in bytes.iter().enumerate() {
            mmix.write_byte(addr + offset as u64, byte);
        }
    }

    // Set PC to the Main label if it exists, otherwise to #100 or first code instruction
    if let Some(&main_addr) = assembler.labels.get("Main") {
        mmix.set_pc(main_addr);
    } else {
        // Default to #100 (common MMIX convention) or first instruction < Data_Segment
        let code_addr = assembler
            .instructions
            .iter()
            .find(|(addr, _)| *addr < 0x2000000000000000)
            .map(|(addr, _)| *addr)
            .unwrap_or(0x100);
        mmix.set_pc(code_addr);
    }
    println!("=== Initial Machine State ===");
    println!("{}", mmix.display_with(value_format));
    println!();

    println!("=== Executing Program ===");
    let count = mmix.run();
    println!();
    println!("Executed {} instructions", count);
    println!();

    println!("=== Final Machine State ===");
    println!("{}", mmix.display_with(value_format));
    println!();

    println!("Execution completed.");

    // Exit with the code set by HALT trap
    let exit_code = mmix.get_exit_code();
    process::exit(exit_code as i32);
}

fn run_mmo(filename: &str, value_format: ValueFormat) {
    let data = fs::read(filename).unwrap_or_else(|err| {
        eprintln!("Error reading file '{}': {}", filename, err);
        process::exit(1);
    });

    println!("=== MMIX Computer ===");
    println!("=== Loading program from: {} ===", filename);
    println!();

    let mut mmix = MMix::new();

    // Decode the MMO file and load into memory
    let decoder = MmoDecoder::new(data);
    let entry_point = decoder.decode(|addr, byte| {
        mmix.write_byte(addr, byte);
    });

    // Temporary debug: inspect instruction bytes at 0x370 to debug big_fib issues
    let debug_addr = 0x370;
    let word = mmix.read_tetra(debug_addr);
    println!(
        "Debug: instr@0x{debug_addr:03X} = 0x{word:08X} (bytes {:02X} {:02X} {:02X} {:02X})",
        (word >> 24) as u8,
        (word >> 16) as u8,
        (word >> 8) as u8,
        word as u8
    );

    // Set PC to entry point from postamble
    mmix.set_pc(entry_point);

    println!("Loaded object file (entry point: 0x{:X})", entry_point);
    println!();

    println!("=== Initial Machine State ===");
    println!("{}", mmix.display_with(value_format));
    println!();

    println!("=== Executing Program ===");
    let count = mmix.run();
    println!();
    println!("Executed {} instructions", count);
    println!();

    println!("=== Final Machine State ===");
    println!("{}", mmix.display_with(value_format));
    println!();

    println!("Execution completed.");

    // Exit with the code set by HALT trap
    let exit_code = mmix.get_exit_code();
    process::exit(exit_code as i32);
}
