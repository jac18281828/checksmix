//! Grammar-level tests: mnemonic word-boundary guards and whitespace adjacency.

use super::*;

// ---- Mnemonic word-boundary guard (adversarial) ------------------
// Every `mnemonic_*` and `directive_*` rule closes with the shared
// `word_end` rule: a keyword ends where a symbol could not continue, so a
// short mnemonic's literal cannot match as a bare prefix of a longer one.
// `Rule::parse` doesn't require consuming the whole input, so without the
// guard each of these three calls returns `Ok` (the rule matches only its
// own shorter literal and stops); with it each must return `Err`.

#[test]
fn test_mnemonic_boundary_guard_rejects_prefix_match() {
    use pest::Parser;

    // GET is a literal prefix of GETA/GETAB (the flagged pair).
    assert!(
        MMixalParser::parse(Rule::mnemonic_get, "GETA").is_err(),
        "mnemonic_get must not match a bare prefix of GETA"
    );
    // SET is a literal prefix of SETL/SETH/SETMH/SETML.
    assert!(
        MMixalParser::parse(Rule::mnemonic_set, "SETL").is_err(),
        "mnemonic_set must not match a bare prefix of SETL"
    );
    // SYNC is a literal prefix of SYNCD/SYNCID/SYNCDI/SYNCIDI.
    assert!(
        MMixalParser::parse(Rule::mnemonic_sync, "SYNCD").is_err(),
        "mnemonic_sync must not match a bare prefix of SYNCD"
    );
}

// ---- Mnemonic word-boundary guard: whitespace-adjacency behavior -
// The guard has two known, intentional side effects on whitespace-
// adjacent constructs. Both are real behavior changes, tested in
// both directions here rather than left to surface only as an
// unexplained corpus diff.

#[test]
fn test_boundary_guard_rejects_zero_whitespace_before_operand() {
    // `ADDa,b,c` (no space between the mnemonic and its first operand)
    // parsed as ADD with the three IS-aliased register operands before
    // this guard -- an accident of the grammar's implicit whitespace,
    // never legitimate MMIXAL syntax. `ADDa` fails `word_end` ('a' could
    // continue a symbol), so `ADDa,b,c` never matches an instruction;
    // `Main` claims the line as a bare label, but a label statement
    // holds nothing but blanks and a comment, so `ADDa,b,c` -- an
    // unrecognized word in opcode position -- is a syntax error rather
    // than commentary silently dropped.
    let source = "a IS $1\nb IS $2\nc IS $3\nMain ADDa,b,c";
    let mut asm = MMixAssembler::new(source, "<test>");
    assert!(
        asm.parse().is_err(),
        "ADDa,b,c must be rejected as an unknown operation"
    );
}

#[test]
fn test_boundary_guard_accepts_mnemonic_prefixed_label() {
    // `HaltLoop` immediately followed by more source used to fail to
    // parse as a label: unguarded `mnemonic_halt` greedily matched the
    // "Halt" prefix of "HaltLoop" as a complete zero-operand HALT
    // instruction before the grammar ever tried `label_def`, leaving
    // "Loop  ADD $1,$2,$3" unparsed. After the guard,
    // "Halt" immediately followed by 'L' (alphanumeric) fails the
    // boundary check, `instruction` no longer matches at that position,
    // and `label_def` correctly claims `HaltLoop` as a label.
    let source = "HaltLoop  ADD $1,$2,$3\n  HALT";
    let mut asm = MMixAssembler::new(source, "<test>");
    asm.parse()
        .unwrap_or_else(|e| panic!("failed to parse {source:?}: {e}"));
    assert_eq!(
        asm.labels.get("HaltLoop"),
        Some(&0),
        "HaltLoop must resolve to address 0"
    );
}

#[test]
fn test_keyword_boundary_admits_underscore_in_label() {
    // A symbol continues on '_', so a keyword must not end before one:
    // `Halt_Loop` is a label, not HALT trailing garbage. These cover the
    // three shapes that misparsed -- an operand-less mnemonic, a mnemonic
    // whose operand would absorb the tail, and a directive.
    let source = "Halt_Loop SETL $1,1\n\
                      Swym_x SETL $2,2\n\
                      Resume_x SETL $3,3\n\
                      Loc_Start SETL $4,4\n\
                      Greg_Base SETL $5,5\n";
    let mut asm = MMixAssembler::new(source, "<test>");
    asm.parse()
        .unwrap_or_else(|e| panic!("failed to parse {source:?}: {e}"));
    for (name, address) in [
        ("Halt_Loop", 0u64),
        ("Swym_x", 4),
        ("Resume_x", 8),
        ("Loc_Start", 12),
        ("Greg_Base", 16),
    ] {
        assert_eq!(
            asm.labels.get(name),
            Some(&address),
            "{name} must be claimed as a label at {address}"
        );
    }
}

#[test]
fn test_swym_carries_its_operands() {
    let source = "SWYM 1,2,3\nSWYM";
    let mut asm = MMixAssembler::new(source, "<test>");
    asm.parse()
        .unwrap_or_else(|e| panic!("failed to parse {source:?}: {e}"));
    assert_eq!(asm.instructions.len(), 2, "both SWYM forms must assemble");
    assert_eq!(asm.instructions[0].1, MMixInstruction::SWYM(1, 2, 3));
    assert_eq!(asm.instructions[1].1, MMixInstruction::SWYM(0, 0, 0));
}

#[test]
fn test_swym_rejects_a_partial_operand_list() {
    // SWYM 1,2 is the two-operand form, SWYM(1,0,2). A trailing
    // comma with nothing after it is a partial list: no operand count
    // SWYM takes matches "1,2,", so it falls back to the one-operand
    // form on "1", leaving ",2," -- a leading comma reads as a dropped
    // operand, not commentary, so this is a syntax error rather than
    // silently becoming SWYM 1.
    let mut asm = MMixAssembler::new("SWYM 1,2,", "<test>");
    assert!(
        asm.parse().is_err(),
        "a trailing comma leaves a partial list"
    );
}

#[test]
fn test_parse_simple_label() {
    let mut asm = MMixAssembler::new("LOOP: HALT", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.labels.get("LOOP"), Some(&0));
    assert_eq!(asm.instructions.len(), 1);
}

#[test]
fn test_parse_octa_directive() {
    let mut asm = MMixAssembler::new("OCTA #123456789ABCDEF0", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions.len(), 1);
    assert_eq!(
        asm.instructions[0].1,
        MMixInstruction::OCTA(0x123456789ABCDEF0)
    );
}

#[test]
fn test_parse_node_structure() {
    let mut asm = MMixAssembler::new("NODE: OCTA 42\n      OCTA 0", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.labels.get("NODE"), Some(&0));
    assert_eq!(asm.instructions.len(), 2);
}

#[test]
fn test_parse_seti() {
    let mut asm = MMixAssembler::new("SETI $2, 10", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::SET(2, 10));
}

#[test]
fn test_parse_set_register() {
    let mut asm = MMixAssembler::new("SET $1, $7", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::SETRR(1, 7));
}

#[test]
fn test_parse_negative_literal_seti() {
    let mut asm = MMixAssembler::new("SETI $1, -1", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::SET(1, u64::MAX));
}

#[test]
fn test_parse_negative_literal_8bit_is_an_error() {
    assert_eq!(
        assemble_err("ADDI $1, $2, -1"),
        "<test>:1:14: immediate operand -1 out of range 0..255 for ADDI"
    );
}

#[test]
fn test_byte_string_pass1_pass2_agree() {
    // Pass 1 sizes the string and pass 2 expands it; the two must agree.
    // The forward OCTA reads pass 1's counter, because pass 2 resolves it
    // before reaching the label and overwriting the entry; the emitted
    // bytes read pass 2's. The label sits on a BYTE so that no rounding
    // can absorb a disagreement between them. A backslash is an ordinary
    // byte, so "a\nb" is four bytes.
    let mut asm = MMixAssembler::new("OCTA LABEL\nBYTE \"a\\nb\",0\nLABEL BYTE 7", "<test>");
    asm.parse().unwrap();
    let bytes: Vec<_> = asm.instructions[1..6]
        .iter()
        .map(|(addr, inst)| (*addr, inst.clone()))
        .collect();
    assert_eq!(
        bytes,
        vec![
            (8, MMixInstruction::BYTE(b'a')),
            (9, MMixInstruction::BYTE(b'\\')),
            (10, MMixInstruction::BYTE(b'n')),
            (11, MMixInstruction::BYTE(b'b')),
            (12, MMixInstruction::BYTE(0)),
        ]
    );
    assert_eq!(asm.instructions[0].1, MMixInstruction::OCTA(13));
}

#[test]
fn test_wyde_list_mixes_numbers_and_string_pass1_pass2_agree() {
    // A leading BYTE leaves the counter unaligned so List's WYDE must
    // round up. The forward OCTA reads pass 1's size for the list;
    // pass 2 must compute the same size or Next's address disagrees.
    let mut asm = MMixAssembler::new(
        "OCTA Next\nBYTE 1\nList WYDE 10,\"ab\",20\nNext BYTE 99",
        "<test>",
    );
    asm.parse().unwrap();
    assert_eq!(asm.labels.get("List"), Some(&10));
    let list: Vec<_> = asm.instructions[2..6]
        .iter()
        .map(|(addr, inst)| (*addr, inst.clone()))
        .collect();
    assert_eq!(
        list,
        vec![
            (10, MMixInstruction::WYDE(10)),
            (12, MMixInstruction::WYDE(b'a' as u16)),
            (14, MMixInstruction::WYDE(b'b' as u16)),
            (16, MMixInstruction::WYDE(20)),
        ]
    );
    assert_eq!(asm.labels.get("Next"), Some(&18));
    assert_eq!(asm.instructions[0].1, MMixInstruction::OCTA(18));
}

#[test]
fn test_tetra_list_mixes_numbers_and_string_pass1_pass2_agree() {
    let mut asm = MMixAssembler::new(
        "OCTA Next\nBYTE 1\nList TETRA 10,\"ab\",20\nNext BYTE 99",
        "<test>",
    );
    asm.parse().unwrap();
    assert_eq!(asm.labels.get("List"), Some(&12));
    let list: Vec<_> = asm.instructions[2..6]
        .iter()
        .map(|(addr, inst)| (*addr, inst.clone()))
        .collect();
    assert_eq!(
        list,
        vec![
            (12, MMixInstruction::TETRA(10)),
            (16, MMixInstruction::TETRA(b'a' as u32)),
            (20, MMixInstruction::TETRA(b'b' as u32)),
            (24, MMixInstruction::TETRA(20)),
        ]
    );
    assert_eq!(asm.labels.get("Next"), Some(&28));
    assert_eq!(asm.instructions[0].1, MMixInstruction::OCTA(28));
}

#[test]
fn test_octa_list_mixes_numbers_and_string_pass1_pass2_agree() {
    let mut asm = MMixAssembler::new(
        "OCTA Next\nBYTE 1\nList OCTA 10,\"ab\",20\nNext BYTE 99",
        "<test>",
    );
    asm.parse().unwrap();
    assert_eq!(asm.labels.get("List"), Some(&16));
    let list: Vec<_> = asm.instructions[2..6]
        .iter()
        .map(|(addr, inst)| (*addr, inst.clone()))
        .collect();
    assert_eq!(
        list,
        vec![
            (16, MMixInstruction::OCTA(10)),
            (24, MMixInstruction::OCTA(b'a' as u64)),
            (32, MMixInstruction::OCTA(b'b' as u64)),
            (40, MMixInstruction::OCTA(20)),
        ]
    );
    assert_eq!(asm.labels.get("Next"), Some(&48));
    assert_eq!(asm.instructions[0].1, MMixInstruction::OCTA(48));
}

#[test]
fn test_byte_string_no_auto_terminator() {
    // MMIXAL appends no terminator to a BYTE string: "Hi" is two bytes and
    // nothing more.
    let mut asm = MMixAssembler::new("BYTE \"Hi\"", "<test>");
    asm.parse().unwrap();
    let bytes: Vec<_> = asm
        .instructions
        .iter()
        .map(|(addr, inst)| (*addr, inst.clone()))
        .collect();
    assert_eq!(
        bytes,
        vec![
            (0, MMixInstruction::BYTE(b'H')),
            (1, MMixInstruction::BYTE(b'i')),
        ]
    );
}

#[test]
fn test_octa_label_rounds_up_after_byte() {
    // MMIXAL rounds the counter to the item's width before assembling it,
    // so the octabyte -- and the label on it -- lands at 8, not 1.
    let mut asm = MMixAssembler::new("BYTE 1\nDATA: OCTA 0", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.labels.get("DATA"), Some(&8));
}

#[test]
fn test_wyde_and_tetra_labels_round_to_their_widths() {
    let mut asm = MMixAssembler::new("BYTE 1\nDATA: WYDE 0", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.labels.get("DATA"), Some(&2));

    let mut asm = MMixAssembler::new("BYTE 1\nDATA: TETRA 0", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.labels.get("DATA"), Some(&4));
}

#[test]
fn test_instruction_label_rounds_up_after_byte() {
    // Instructions align to 4 like any tetra-wide item.
    let mut asm = MMixAssembler::new("BYTE 1\nCODE: HALT", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.labels.get("CODE"), Some(&4));
}

#[test]
fn test_forward_reference_resolves_to_aligned_address() {
    // Pass 2 overwrites asm.labels with its own addresses, so a pass-1 and
    // pass-2 disagreement survives only in the operand pass 2 encoded from
    // the stale pass-1 entry. DATA rounds to 8, four tetras past the JMP's
    // two.
    let mut asm = MMixAssembler::new("JMP DATA\nBYTE 1\nDATA: OCTA 0", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.instructions[0].1, MMixInstruction::JMP(2));
}

#[test]
fn test_two_debug_directives_assemble_to_one_tetra_each() {
    // Each directive costs exactly one tetra: Main and the second
    // directive's TRAP sit four bytes apart, and Halt follows another
    // four bytes on -- no label, no data, no generated block.
    let source = "        LOC     #100\nMain    debug \"one\"\nSecond  debug \"two\"\n        TRAP    0,Halt,0\n";
    let mut asm = MMixAssembler::new(source, "<test>");
    asm.parse().unwrap();

    let main_addr = *asm.labels.get("Main").expect("Main label");
    let second_addr = *asm.labels.get("Second").expect("Second label");
    assert_eq!(second_addr, main_addr + 4);
    assert_eq!(asm.instructions.len(), 3);
    assert_eq!(asm.debug_strings(), &[b"one".to_vec(), b"two".to_vec()]);
}

/// `K` is one byte: a 257th `debug` directive in one program is an
/// assembly error naming its file and line, not a wrapped or truncated
/// index.
#[test]
fn test_a_257th_debug_directive_is_an_assembly_error() {
    let mut source = String::new();
    for _ in 0..256 {
        source.push_str("debug \"x\"\n");
    }
    source.push_str("debug \"overflow\"\n"); // line 257
    let mut asm = MMixAssembler::new(&source, "many.mms");
    let err = asm
        .parse()
        .expect_err("a 257th directive must not assemble");
    assert!(
        err.starts_with("many.mms:257:"),
        "must name the overflowing directive's file and line, got {err:?}"
    );
}

/// Two translation units contribute to one shared, program-order `K`
/// space: the second unit's directive picks up where the first left off.
#[test]
fn test_debug_strings_span_translation_units_in_program_order() {
    let mut asm = MMixAssembler::new("debug \"first\"\n", "a.mms");
    asm.add_source("debug \"second\"\n", "b.mms");
    asm.parse().unwrap();
    assert_eq!(
        asm.debug_strings(),
        &[b"first".to_vec(), b"second".to_vec()]
    );
}

#[test]
fn test_multibyte_byte_string_is_not_aligned() {
    // Alignment comes from the item's kind, not its size: a four-byte BYTE
    // list is still placed wherever the counter stands. Deriving alignment
    // from data_directive_size would put TEXT at 4.
    let mut asm = MMixAssembler::new("BYTE 1\nTEXT: BYTE \"abcd\"", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.labels.get("TEXT"), Some(&1));
}

#[test]
fn test_loc_sets_the_counter_exactly() {
    // LOC assigns the counter; it does not align. The next aligned item
    // rounds from wherever LOC left it.
    let mut asm = MMixAssembler::new("LOC #101\nHERE: BYTE 0", "<test>");
    asm.parse().unwrap();
    assert_eq!(asm.labels.get("HERE"), Some(&0x101));
}
