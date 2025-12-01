use checksmix::{MMix, MMixAssembler, Mix, Program};
use std::env;
use std::fs;
use std::path::Path;
use std::process;
use tracing_subscriber::{EnvFilter, fmt};

fn main() {
    // Initialize tracing subscriber with RUST_LOG environment variable support
    // By default, no debug output unless RUST_LOG is set
    // Example: RUST_LOG=checksmix=debug cargo run --bin checksmix -- file.mms
    fmt().with_env_filter(EnvFilter::from_default_env()).init();

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

    let mut assembler = MMixAssembler::new(&input, filename);

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
    let mut entry_point = 0x100u64; // Default entry point

    // Parse .mmo format
    let mut i = 0;
    let mut current_addr = 0u64;

    while i < data.len() {
        if i + 4 > data.len() {
            break;
        }

        let opcode = data[i];

        match opcode {
            0x98 => {
                // lop_quote: YZ tetras of literal data follow
                let yz = ((data[i + 1] as usize) << 8) | (data[i + 2] as usize);
                i += 4;

                // Load yz tetras (4*yz bytes) at current_addr
                let byte_count = yz * 4;
                for offset in 0..byte_count {
                    if i + offset < data.len() {
                        mmix.write_byte(current_addr + offset as u64, data[i + offset]);
                    }
                }
                current_addr += byte_count as u64;
                i += byte_count;
            }
            0x9A => {
                // lop_loc: Set loading address
                // Custom format: 3 tetras (12 bytes)
                // Tetra 1: 0x9A 00 00 00
                // Tetra 2: high 32 bits of address
                // Tetra 3: low 32 bits of address
                i += 4; // Skip the lop_loc tetra

                if i + 8 <= data.len() {
                    let high = u32::from_be_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]);
                    let low =
                        u32::from_be_bytes([data[i + 4], data[i + 5], data[i + 6], data[i + 7]]);
                    current_addr = ((high as u64) << 32) | (low as u64);
                    i += 8;
                }
            }
            0x9D => {
                // lop_pre: Preamble (skip)
                i += 4;
            }
            0x9F => {
                // lop_post: Postamble with entry point
                // Entry point is 8 bytes following this tetra
                i += 4;
                if i + 8 <= data.len() {
                    entry_point = u64::from_be_bytes([
                        data[i],
                        data[i + 1],
                        data[i + 2],
                        data[i + 3],
                        data[i + 4],
                        data[i + 5],
                        data[i + 6],
                        data[i + 7],
                    ]);
                    i += 8;
                }
            }
            _ => {
                // Unknown or literal data - treat as data at current address
                mmix.write_byte(current_addr, data[i]);
                current_addr += 1;
                i += 1;
            }
        }
    }

    // Set PC to entry point from postamble
    mmix.set_pc(entry_point);

    println!("Loaded object file (entry point: 0x{:X})", entry_point);
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
