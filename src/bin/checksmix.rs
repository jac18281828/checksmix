use checksmix::{MMix, MMixAssembler, Mix, MmoDecoder, Program, ValueFormat};
use clap::{Parser, Subcommand};
use std::fs;
use std::path::{Path, PathBuf};
use std::process;
use tracing_subscriber::{EnvFilter, fmt};

#[derive(Parser, Debug)]
#[command(
    name = "checksmix",
    about = "Run MIX/MMIX programs and assemblers",
    version,
    author,
    args_conflicts_with_subcommands = true,
    subcommand_negates_reqs = true
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// Display register values as unsigned decimals (run mode)
    #[arg(long)]
    unsigned: bool,

    /// Program file(s) to execute. A single file dispatches by extension
    /// (.mix/.mixal/.mms/.mmo); multiple files must all be .mms and are
    /// assembled into one shared symbol space before execution.
    #[arg(required = true, num_args = 1..)]
    program_files: Vec<String>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Run a program (equivalent to omitting the subcommand)
    Run {
        /// Display register values as unsigned decimals
        #[arg(long)]
        unsigned: bool,
        /// Program file(s) to execute
        #[arg(required = true, num_args = 1..)]
        program_files: Vec<String>,
    },
    /// Parse and encode .mms source(s); silent on success, errors on failure
    Check {
        /// MMIX assembly source file(s)
        #[arg(required = true, num_args = 1.., value_name = "FILE.mms")]
        files: Vec<PathBuf>,
    },
    /// Assemble one or more .mms sources into a .mmo object file
    Build {
        /// Output .mmo file (default: first input's basename with .mmo extension)
        #[arg(short = 'o', long, value_name = "OUT.mmo")]
        output: Option<PathBuf>,
        /// MMIX assembly source file(s)
        #[arg(required = true, num_args = 1.., value_name = "FILE.mms")]
        files: Vec<PathBuf>,
    },
}

fn main() {
    fmt().with_env_filter(EnvFilter::from_default_env()).init();

    let cli = Cli::parse();
    match cli.command {
        Some(Command::Run {
            unsigned,
            program_files,
        }) => {
            let vfmt = if unsigned {
                ValueFormat::Unsigned
            } else {
                ValueFormat::Signed
            };
            dispatch_run(&program_files, vfmt);
        }
        Some(Command::Check { files }) => cmd_check(&files),
        Some(Command::Build { output, files }) => cmd_build(&files, output.as_deref()),
        None => {
            let vfmt = if cli.unsigned {
                ValueFormat::Unsigned
            } else {
                ValueFormat::Signed
            };
            dispatch_run(&cli.program_files, vfmt);
        }
    }
}

fn dispatch_run(program_files: &[String], value_format: ValueFormat) {
    if program_files.len() == 1 {
        let program_file = &program_files[0];
        let path = Path::new(program_file);
        let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");

        match extension {
            "mix" | "mixal" => run_mix(program_file),
            "mms" => run_mms(program_files, value_format),
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
        for f in program_files {
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
        run_mms(program_files, value_format);
    }
}

fn assemble_sources(paths: &[PathBuf]) -> Result<MMixAssembler, String> {
    let mut sources: Vec<(String, String)> = Vec::with_capacity(paths.len());
    for path in paths {
        let src = fs::read_to_string(path)
            .map_err(|err| format!("error reading '{}': {}", path.display(), err))?;
        sources.push((path.to_string_lossy().into_owned(), src));
    }
    let (first_name, first_src) = &sources[0];
    let mut asm = MMixAssembler::new(first_src, first_name);
    for (name, src) in sources.iter().skip(1) {
        asm.add_source(src, name);
    }
    asm.parse()?;
    Ok(asm)
}

fn cmd_check(files: &[PathBuf]) {
    let asm = assemble_sources(files).unwrap_or_else(|e| {
        eprintln!("{}", e);
        process::exit(1);
    });
    let _ = asm.generate_object_code();
}

fn cmd_build(files: &[PathBuf], output: Option<&Path>) {
    let asm = assemble_sources(files).unwrap_or_else(|e| {
        eprintln!("{}", e);
        process::exit(1);
    });
    if asm.instructions.is_empty() {
        eprintln!("error: no instructions to assemble");
        process::exit(1);
    }
    let object_code = asm.generate_object_code();
    let out_path = output
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| files[0].with_extension("mmo"));
    fs::write(&out_path, &object_code).unwrap_or_else(|err| {
        eprintln!("error writing '{}': {}", out_path.display(), err);
        process::exit(1);
    });
    println!("{}", out_path.display());
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
    println!("=== MMIX Assembler ===");
    if filenames.len() == 1 {
        println!("=== Parsing assembly from: {} ===", filenames[0]);
    } else {
        println!("=== Parsing {} assembly inputs ===", filenames.len());
        for f in filenames {
            println!("  {}", f);
        }
    }
    println!();

    let paths: Vec<PathBuf> = filenames.iter().map(PathBuf::from).collect();
    let assembler = assemble_sources(&paths).unwrap_or_else(|e| {
        eprintln!("Error: {}", e);
        process::exit(1);
    });

    println!("Assembly parsed successfully");
    println!();

    let mut mmix = MMix::new();

    for (addr, inst) in &assembler.instructions {
        let bytes = assembler.encode_instruction_bytes(inst);
        for (offset, &byte) in bytes.iter().enumerate() {
            mmix.write_byte(addr + offset as u64, byte);
        }
    }

    if let Some(&main_addr) = assembler.labels.get("Main") {
        mmix.set_pc(main_addr);
    } else {
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

    let exit_code = mmix.get_exit_code();
    process::exit(exit_code as i32);
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn no_subcommand_routes_to_run() {
        let cli = Cli::try_parse_from(["checksmix", "file.mms"]).unwrap();
        assert!(cli.command.is_none());
        assert_eq!(cli.program_files, vec!["file.mms"]);
        assert!(!cli.unsigned);
    }

    #[test]
    fn no_subcommand_unsigned_flag() {
        let cli = Cli::try_parse_from(["checksmix", "--unsigned", "file.mms"]).unwrap();
        assert!(cli.command.is_none());
        assert!(cli.unsigned);
        assert_eq!(cli.program_files, vec!["file.mms"]);
    }

    #[test]
    fn run_subcommand_parses() {
        let cli = Cli::try_parse_from(["checksmix", "run", "file.mms"]).unwrap();
        match cli.command {
            Some(Command::Run {
                unsigned,
                program_files,
            }) => {
                assert!(!unsigned);
                assert_eq!(program_files, vec!["file.mms"]);
            }
            _ => panic!("expected Run"),
        }
    }

    #[test]
    fn run_subcommand_unsigned() {
        let cli = Cli::try_parse_from(["checksmix", "run", "--unsigned", "file.mms"]).unwrap();
        match cli.command {
            Some(Command::Run { unsigned, .. }) => assert!(unsigned),
            _ => panic!("expected Run"),
        }
    }

    #[test]
    fn run_subcommand_multiple_files() {
        let cli = Cli::try_parse_from(["checksmix", "run", "a.mms", "b.mms"]).unwrap();
        match cli.command {
            Some(Command::Run { program_files, .. }) => {
                assert_eq!(program_files, vec!["a.mms", "b.mms"]);
            }
            _ => panic!("expected Run"),
        }
    }

    #[test]
    fn check_subcommand_parses() {
        let cli = Cli::try_parse_from(["checksmix", "check", "a.mms", "b.mms"]).unwrap();
        match cli.command {
            Some(Command::Check { files }) => {
                assert_eq!(files, vec![PathBuf::from("a.mms"), PathBuf::from("b.mms")]);
            }
            _ => panic!("expected Check"),
        }
    }

    #[test]
    fn build_subcommand_short_output() {
        let cli = Cli::try_parse_from(["checksmix", "build", "-o", "out.mmo", "a.mms"]).unwrap();
        match cli.command {
            Some(Command::Build { output, files }) => {
                assert_eq!(output, Some(PathBuf::from("out.mmo")));
                assert_eq!(files, vec![PathBuf::from("a.mms")]);
            }
            _ => panic!("expected Build"),
        }
    }

    #[test]
    fn build_subcommand_long_output() {
        let cli =
            Cli::try_parse_from(["checksmix", "build", "--output", "out.mmo", "a.mms"]).unwrap();
        match cli.command {
            Some(Command::Build { output, .. }) => {
                assert_eq!(output, Some(PathBuf::from("out.mmo")));
            }
            _ => panic!("expected Build"),
        }
    }

    #[test]
    fn build_subcommand_default_output() {
        let cli = Cli::try_parse_from(["checksmix", "build", "a.mms"]).unwrap();
        match cli.command {
            Some(Command::Build { output, .. }) => {
                assert_eq!(output, None);
            }
            _ => panic!("expected Build"),
        }
    }

    #[test]
    fn unsigned_rejected_on_check() {
        // --unsigned is not defined on the check subcommand
        assert!(Cli::try_parse_from(["checksmix", "check", "--unsigned", "a.mms"]).is_err());
    }

    #[test]
    fn output_flag_rejected_on_run() {
        // -o is only defined on build
        assert!(Cli::try_parse_from(["checksmix", "run", "-o", "out.mmo", "a.mms"]).is_err());
    }
}
