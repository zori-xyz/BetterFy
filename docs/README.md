# BetterFy documentation

This directory describes the product contract, the desktop architecture, and
the release process. When implementation and documentation disagree, treat the
disagreement as a bug and verify the behavior before changing either side.

## Start here

1. [Product definition](../PRODUCT.md) — audience, scope, product boundaries,
   and the current delivery order.
2. [Design system](../DESIGN.md) — visual language, typography, motion, themes,
   and asset rules.
3. [Experience constitution](EXPERIENCE_CONSTITUTION.md) — interaction rules
   that every screen must preserve.
4. [Roadmap](ROADMAP.md) — completed foundations, the active engine milestone,
   and the release gates ahead.

## Engine

- [Engine architecture](ENGINE_ARCHITECTURE.md) — trust boundaries, command
  flow, state, recovery, and the path from fixtures to production deployment.
- [Minify patching audit](MINIFY_PATCHING_AUDIT.md) — source-backed research
  into the upstream patching workflow and the constraints BetterFy must retain.

The interface may demonstrate a future operation only when it labels that
boundary clearly. A screen is not evidence that the corresponding privileged
operation is live.

## Build and release

- [Windows build guide](WINDOWS_BUILD.md) — local prerequisites, internal NSIS
  build, artifacts, and troubleshooting.
- [Release and updater guide](RELEASING.md) — versioning, signing, tag-driven
  publication, updater manifests, and rollback expectations.

Windows is the release platform. macOS is a development environment for the
interface and synthetic engine fixtures.

## Collaboration

- [Contributing](../CONTRIBUTING.md) — branch, implementation, test, and review
  expectations.
- [Repository guide](REPOSITORY_GUIDE.md) — directory ownership, branch names,
  commits, and the recommended GitHub settings.
- [Third-party notices](../THIRD_PARTY_NOTICES.md) — asset provenance and
  redistribution constraints.

## Status language

Use these labels consistently in code review and documentation:

| Label | Meaning |
| --- | --- |
| **Implemented** | The operation runs through the desktop boundary and has a tested failure path. |
| **Fixture** | The real transaction model runs only on repository-owned test data. |
| **Preview** | The interface exists, but the production backend is deliberately unavailable. |
| **Prepared** | Infrastructure exists but cannot be used publicly until a release gate is satisfied. |
| **Planned** | Design or research only; no working product claim. |

