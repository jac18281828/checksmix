# Contributing to `checksmix`

Contributions are welcome: a bug report, a fix, a program that assembles or
runs wrong, or a whole feature.

## Pull requests

1. Fork the repo and branch from `main`.
2. Add tests for anything that changes behavior.
3. Update the docs when you change what they describe.
4. Make the checks below pass.
5. Open the pull request.

`main` only ever fast-forwards. Rebase rather than merge.

## Before you push

```sh
cargo check
cargo fmt --check
cargo clippy --all-targets --all-features --no-deps -- -D warnings
cargo test
cargo run --release --bin checksmix -- run examples/all_instructions_test.mms
cargo check --lib --no-default-features --target wasm32-unknown-unknown
cargo test --no-default-features
```

The `cargo run` line prints `All tests passed!` and exits 0. Green on all of
them, and green in CI.

`tests/soundness.rs` runs whole programs and checks each two ways: a known
answer, worked out without running `checksmix`, and a golden state dump
under `tests/resources/soundness/golden/`. A red golden with a green known
answer means the machine state moved.
`CHECKSMIX_REWRITE_GOLDEN=1 cargo test --test soundness` retakes the goldens
— run it only when that change is intended.

## Tests

Add tests for behavior changes, and prove each one fails when its target
breaks: break the code on purpose, watch the test go red, put it back. A
vacuous test covers nothing. Unit tests must be hermetic: no network, no
external files or assets. Integration tests under `tests/` may read files.

Every mnemonic the assembler accepts belongs in
`examples/all_instructions_test.mms`, in both operand forms where both exist.

## Fuzzing

`fuzz/` holds two `cargo-fuzz` targets: `assemble` (the parser and code
generator) and `mmo_decode` (the `.mmo` reader). One-time setup, if you don't
already have them:

```sh
rustup toolchain install nightly
cargo install cargo-fuzz
```

Then, from the repo root, seed each target's corpus from the repo's own
example and soundness programs -- split into pieces of at most 4 KB, since a
seed larger than `-max_len` is wasted -- and run it with the flags CI uses:
value profiling, so a run rewards inputs whose comparisons come closer to
flipping, and `assemble`'s dictionary of every mnemonic and directive plus
each field boundary. CI also passes `-rss_limit_mb=2048`, `-timeout=10` and
`-max_total_time=60`, which bound one run's memory, one input's time and the
whole run. `mmo_decode`'s seeds are the same programs assembled to `.mmo`, so
build `mmixasm` first. The corpora land in `fuzz/corpus/`, which
`fuzz/.gitignore` covers:

```sh
cargo build --release --bin mmixasm
./fuzz/seed_corpus.sh assemble fuzz/corpus/assemble
./fuzz/seed_corpus.sh mmo_decode ./target/release/mmixasm fuzz/corpus/mmo_decode
cargo +nightly fuzz run assemble -- -rss_limit_mb=2048 -timeout=10 \
  -max_total_time=60 -use_value_profile=1 -max_len=4096 \
  -dict=fuzz/mmixal.dict fuzz/corpus/assemble
cargo +nightly fuzz run mmo_decode -- -rss_limit_mb=2048 -timeout=10 \
  -max_total_time=60 -use_value_profile=1 -max_len=4096 \
  fuzz/corpus/mmo_decode
```

`assemble` skips an input with a line of more than 256 `(` and prefix
operator (`+ - ~ $ &`) characters. The assembler sets no nesting limit, so an
input nested past what the machine's stack holds only probes that stack, and a
crash there is no finding; 256 levels is the depth checksmix tests in every
environment it supports.

`.github/workflows/fuzz.yml` runs both targets, with the same flags, for a
minute on every push to `main` or an `agent/**` branch, and on every pull
request to `main`.

## Instruction semantics

The [Instruction Reference](https://mmix.cs.hm.edu/doc/instructions/) is the
canonical definition of what each instruction does. Establish the behavior
from it before changing an instruction, and cite it in the pull request.

## Commits

[Conventional Commits](https://www.conventionalcommits.org), signed and in
lower case: `feat(mmixal): …`, `fix(debugger): …`, `docs(readme): …`.

## Style

The tree is `rustfmt` clean and `clippy` clean with warnings as errors.
Otherwise match the file you are in: semantic names with no type or
namespace affixes, small single-purpose functions, `Result` and `Option`
rather than `unwrap` outside tests, and source files under about 2,500 lines.

Write new `.mms` source in canonical MMIXAL: the base mnemonic, letting the
assembler pick the immediate opcode (`ADD`, never `ADDI`).

Ask in an issue before adding a dependency. The library must still build for
`wasm32-unknown-unknown` without default features.

## Bug reports

Open an issue with a summary, the steps to reproduce, what you expected and
what you got.

For a program that assembles or runs wrong, **the smallest `.mms` that shows
it is the reproduction**. Paste it into the issue with the command you ran and
say what you expected the output, a register or the exit code to be. A
[playmmix](https://playmmix.2ad.com) share link works too.

## Working with an AI agent

`AGENTS.md` is the brief for AI agents working in this repo: the conventions
at length and the completion gates. Point your agent at it.

## License

`checksmix` is distributed under the BSD 3-Clause License (`LICENCE`). By
contributing you agree that your contributions are licensed under the same
terms.

---

Adapted from the open-source contribution guidelines for
[Facebook's Draft](https://github.com/facebook/draft-js).
