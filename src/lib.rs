#![doc = include_str!("../README.md")]

mod debugger;
mod encode;
mod mmix;
mod mmixal;
mod mmo;

pub use debugger::{
    Command, Debugger, PrintFormat, entry_point, parse_command, start_program, write_image,
};
pub use mmix::{Host, MMix, SpecialReg, StdHost, Stop, TrapCode, ValueFormat};
pub use mmixal::{MMixAssembler, SourceLoc};
pub use mmo::MmoDecoder;

#[cfg(doctest)]
mod closed_leaks {
    /// ```compile_fail
    /// let asm = checksmix::MMixAssembler::new("", "a.mms");
    /// let _ = &asm.instructions;
    /// ```
    ///
    /// ```
    /// let asm = checksmix::MMixAssembler::new("", "a.mms");
    /// let _ = &asm;
    /// ```
    pub struct AssemblerInstructions;

    /// ```compile_fail
    /// let asm = checksmix::MMixAssembler::new("", "a.mms");
    /// let _ = &asm.symbols;
    /// ```
    ///
    /// ```
    /// let asm = checksmix::MMixAssembler::new("", "a.mms");
    /// let _ = &asm;
    /// ```
    pub struct AssemblerSymbols;

    /// ```compile_fail
    /// let asm = checksmix::MMixAssembler::new("", "a.mms");
    /// let _ = &asm.labels;
    /// ```
    ///
    /// ```
    /// let asm = checksmix::MMixAssembler::new("", "a.mms");
    /// let _ = asm.labels();
    /// ```
    pub struct AssemblerLabels;

    /// ```compile_fail
    /// let asm = checksmix::MMixAssembler::new("", "a.mms");
    /// let _ = &asm.greg_inits;
    /// ```
    ///
    /// ```
    /// let asm = checksmix::MMixAssembler::new("", "a.mms");
    /// let _ = asm.greg_inits();
    /// ```
    pub struct AssemblerGregInits;

    /// ```compile_fail
    /// let _ = checksmix::MMixAssembler::encode_instruction_bytes;
    /// ```
    pub struct AssemblerEncoder;

    /// ```compile_fail
    /// use checksmix::MmoGenerator;
    /// ```
    ///
    /// ```
    /// let asm = checksmix::MMixAssembler::new("", "a.mms");
    /// let _ = checksmix::MmoDecoder::new(asm.generate_object_code());
    /// ```
    pub struct ObjectFileGenerator;

    /// ```compile_fail
    /// let file = String::from("a.mms");
    /// let line = 1;
    /// let _ = checksmix::SourceLoc { file, line };
    /// ```
    ///
    /// ```
    /// let mut asm = checksmix::MMixAssembler::new("LOC #100\nHALT\n", "a.mms");
    /// asm.parse().unwrap();
    /// let loc = asm.source_loc(0x100).unwrap();
    /// assert_eq!((loc.file.as_str(), loc.line), ("a.mms", 2));
    /// assert_eq!(loc, loc);
    /// ```
    pub struct SourceLocLiteral;

    /// ```compile_fail
    /// let _ = checksmix::StdHost;
    /// ```
    ///
    /// ```
    /// let _ = checksmix::StdHost::new();
    /// let _ = checksmix::StdHost::default();
    /// ```
    pub struct StdHostUnitName;
}
