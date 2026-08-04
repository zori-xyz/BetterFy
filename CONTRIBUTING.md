# Contributing to BetterFy

BetterFy is a desktop product with a privileged patching engine. A polished
screen and a safe transaction are both required; neither can substitute for the
other.

## Before you start

Read:

- [PRODUCT.md](PRODUCT.md) for product scope and current truth;
- [DESIGN.md](DESIGN.md) for layout, motion, typography, and asset rules;
- [the experience constitution](docs/EXPERIENCE_CONSTITUTION.md) for interaction
  guarantees;
- [the engine architecture](docs/ENGINE_ARCHITECTURE.md) before changing Rust
  commands or transaction state.

Use an existing issue or open one before beginning a material change. For
branch names and commits, follow the [repository guide](docs/REPOSITORY_GUIDE.md).

## Product boundaries

- UI state is not proof that a privileged operation completed.
- React may request and present an operation; Rust owns validation and writes.
- Preview behavior must be labelled as preview in both RU and EN.
- Do not claim VAC safety, ban immunity, universal compatibility, or successful
  recovery without a verified production path.
- BetterFy starts Steam after activation. It does not automatically start Dota 2.

## Interface changes

Keep visible copy in Russian and English. Verify both themes and the supported
desktop widths. New motion must respect reduced-motion preferences and must not
hide progress or failure states.

Run:

```bash
npm run check
npm run visual:check
```

Include before and after screenshots for layout, theme, catalog, modal, or
animation changes. Do not add temporary third-party artwork to a release path.

## Engine changes

Privileged work must be deterministic, bounded, journaled, and recoverable.
Validate every path and external input in Rust. Refuse stale plans and ambiguous
state. A failed operation must leave enough durable evidence to explain what
happened without trusting React memory.

Run:

```bash
cargo fmt --all --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --locked --manifest-path src-tauri/Cargo.toml -- -D warnings
cargo test --locked --manifest-path src-tauri/Cargo.toml
```

Tests should cover cancellation, partial failure, stale inputs, verification
failure, rollback, and unrelated external edits where applicable.

## Pull requests

A reviewable pull request includes:

- the user problem and the chosen behavior;
- a clear list of affected product boundaries;
- what is implemented, fixture-backed, prepared, or preview-only;
- local checks and the operating system used;
- screenshots for visible changes;
- risk, failure behavior, and recovery evidence for privileged changes;
- new asset provenance or dependency rationale.

Keep unrelated refactors out of the change. Never include secrets, local user
data, generated installers, or assistant attribution in commits.

