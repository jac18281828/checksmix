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
