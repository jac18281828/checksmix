# AI Coding Guidelines (Rust)

These guidelines apply to all AI-assisted code changes in this repository.

## Workflow
1. Summarize current behavior and invariants before proposing edits.
2. **Ask each time** — `Cargo.toml` deps, cross-module or public-API refactors, file deletions,
   CI or release changes.
3. **Always ask** — merging to `main`, opening a PR, tags, force ops.

## Rust Style & Design
- Correctness first; then idiomatic, reviewable Rust.
- Prefer clarity over cleverness: small functions, early returns, shallow nesting.
- Keep diffs small and reviewable; avoid cosmetic churn.
- Do not include expository or 'my way' style comments.
- Do not include comments that focus on the change itself and lack suitable generality ('low overhead version', 'fully optimal version', etc.).
- Comments should document the code, not the change being made.

## Naming
- Naming must be semantic, not pattern-based.
- Avoid suffixes like `State`, `Context`, `Manager` unless there is a real contrast (e.g., `Config` vs `Runtime`, `Snapshot` vs `Live`).
- Do not use prefixes or suffixes as namespaces. If everything starts with or ends with `_name_`, nothing should.
- Rust is strongly typed; do not express type information through naming.

## Abstraction
- Abstract only when it removes duplication or encodes invariants.
- Prefer concrete domain types over generic wrappers.
- Avoid `unwrap`/`expect` outside of tests; truly-infallible uses with a justifying comment are acceptable.
- Use effective error handling patterns including `Result` and `Option`.

## Dependencies and Imports
- Prefer the standard library.
- Declare imports at the top of each module; keep them explicit and organized so dependencies are clear.

## Tests
- Test project behavior and contracts, not language or dependency internals.
- Avoid vacuous tests: removing or breaking target code must cause a test to fail.
- Unit tests must be hermetic: no network, no external files or assets.
- Integration tests may access external files.
- Add or update tests for every behavior change.

## Completion Gates

Before marking work complete, run and report:

1. `cargo check`
2. `cargo fmt --check`
3. `cargo clippy --all-targets --all-features --no-deps -- -D warnings`
4. All tests pass (unit, doc, and integration)
5. `checksmix run examples/all_instructions_test.mms` prints `All tests passed!` and exits 0
6. `cargo check --lib --no-default-features --target wasm32-unknown-unknown`
7. `cargo test --no-default-features`

Do not mark work complete until all gates pass.

## Release

All commits land on the branch; `main` only ever sees a fast-forward.

Two commits on a feature branch (`claude/<topic>`), landed together:

1. Add an `X.Y.Z` entry to `CHANGELOG.md` and commit — this is R, the release
   commit.
2. Tag R `X.Y.Z`, signed and annotated.
3. Bump `Cargo.toml` to `X.Y.(Z+1)`, set the `.TH` version in every `man/*.1`
   to match, run `cargo check` so `Cargo.lock` refreshes, and commit all of
   it as `docs: X.Y.(Z+1)` — this is S, staging the next release.
4. FF-merge into `main`; push `main`, then the tag — `deploy-crate` publishes
   on tag push.
5. Delete the feature branch (local and remote).

The tag version equals the code version at the tagged commit. `Cargo.toml`
and the man pages must agree at every commit —
`docs_consistency::man_page_versions_match_cargo_toml` enforces it.
