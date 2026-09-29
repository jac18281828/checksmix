#![no_main]

use checksmix::MMixAssembler;
use checksmix::MmoDecoder;
use libfuzzer_sys::fuzz_target;

/// Nesting depth past which an input only probes the machine's stack.
const SKIP_DEPTH: usize = 256;

/// True when some line holds more than `SKIP_DEPTH` characters that can open
/// a nesting level: `(` and the prefix operators `+ - ~ $ &`. `)` is not
/// subtracted, since one inside a string or character constant closes
/// nothing. The count over-estimates, which only narrows what the fuzzer
/// explores.
fn nested_too_deep(text: &str) -> bool {
    text.split('\n').any(|line| {
        line.chars()
            .filter(|c| matches!(c, '(' | '+' | '-' | '~' | '$' | '&'))
            .count()
            > SKIP_DEPTH
    })
}

fn assemble(text: &str) {
    let mut assembler = MMixAssembler::new(text, "fuzz.mms");
    if assembler.parse().is_err() {
        return;
    }

    // Generator and decoder must agree: a decode error on code this
    // release's own generator just emitted is a finding.
    let object = assembler.generate_object_code();
    let decoder = MmoDecoder::new(object);
    if let Err(err) = decoder.decode(|_, _| {}) {
        panic!("generated object code failed to decode: {err}");
    }
}

// A file the CLI would reject before ever assembling it: non-UTF-8 input
// never reaches `MMixAssembler::new`.
fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    if nested_too_deep(text) {
        return;
    }
    assemble(text);
});
