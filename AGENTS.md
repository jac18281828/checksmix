# AI Coding Guidelines (Rust)

These guidelines apply to all AI-assisted code changes in this repository.

## Workflow
1. Read the full contents of any file you plan to change, plus directly related modules.
2. Summarize current behavior and invariants before proposing edits.
3. Propose a minimal patch plan (files + rationale) before modifying code.
4. Scope actions to the approval tier:
   1. **Free** — reads, searches, web docs, `cargo check`/`fmt`/`clippy`/`test`, local binary runs, scratch `.mms`/`.mmo` files at the repo root or outside `src/`.
   2. **Task-approved** (covered by the user's initial request) — edits under `src/`, gate fixes, iteration within the agreed plan.
   3. **Ask each time** — `Cargo.toml` deps, cross-module or public-API refactors, expansive edits, file deletions, CI or release changes.
   4. **Always ask** — `git commit`, `git push`, PRs, tags, force ops, anything visible outside the local repo.
5. Affirm all `Completion Gates` are met.

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
- Add external crates only with user approval.
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

Do not mark work complete until all gates pass.

## Release

1. Work on a branch (`claude/<topic>`); never commit directly to `main`.
2. FF-merge into `main` (`git merge --ff-only`); no force pushes to `main`.
3. Create a signed annotated tag `X.Y.Z` on the merge commit and push it — the `deploy-crate` workflow publishes on tag push.
4. Bump `Cargo.toml` to the next patch on the branch; commit as `docs: X.Y.Z`; push the branch.
5. The tag version matches the code version *at the tagged commit*; the `docs: X.Y.Z` commit prepares the *next* release.
