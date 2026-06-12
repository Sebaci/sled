# Sled Agent Notes

Sled is a Rust implementation of an exploratory DSL for Advent of Code.

## Working Principles

- Treat `memory/design_notes/` as exploratory design input, not a frozen specification.
- Discuss new language semantics with the user before implementing them when they change the DSL surface meaningfully.
- Prefer early, runnable MVP slices over designing a complete language upfront.
- Keep the browser/WASM goal visible, but build the core interpreter as a reusable Rust library first.
- Tests are important. Concrete language behavior should normally arrive with focused tests.
- Before committing substantial changes, perform a code-review pass for correctness, diagnostics, and test gaps.

## Architecture Direction

- Keep the CLI thin. Put parser, analysis, and runtime behavior in the library.
- Preserve a clean boundary between:
  - syntax parsing
  - callable shape analysis (`needs_left`, `right_arity`)
  - value/runtime semantics
- Favor simple, dependency-light Rust while the language is still unstable.
- Avoid making end-user documentation promises before behavior has been exercised against real AoC problems.

## Commit Convention

Use lightweight Conventional Commits:

- `feat`: user-visible language/runtime/CLI behavior
- `fix`: bug fixes
- `test`: tests only
- `docs`: README or user-facing docs
- `refactor`: internal restructuring without behavior changes
- `chore`: repo setup, tooling, and maintenance

Prefer imperative, lowercase summaries, for example `chore: initialize Rust interpreter scaffold`.

## Current Product Direction

- Implementation language: Rust.
- Target deliverables:
  - small compiled CLI interpreter
  - reusable core suitable for WebAssembly later
  - future GitHub Pages editor for writing and testing Sled programs in the browser
- Development style: agile, implementation-guided, and open to revising earlier design notes.
