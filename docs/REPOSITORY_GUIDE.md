# Repository guide

## Working model

`main` is the integration branch. It should always build and remain suitable
for an internal Windows installer. Product and engine work belongs in a
short-lived branch and reaches `main` through a reviewed pull request.

Use a branch name that describes the change:

```text
feature/content-manifest
feature/package-intake
fix/steam-vdf-rollback
fix/light-theme-contrast
docs/engine-recovery
chore/windows-toolchain
release/v0.4.0
```

Avoid personal names, ticket-only branch names, and branches that collect
unrelated work.

## Change flow

1. Open or choose an issue that describes the user problem.
2. Branch from the latest green `main`.
3. Keep privileged engine work separate from visual cleanup when practical.
4. Run the checks that match the changed boundary.
5. Open a pull request and state what is real, fixture-backed, or preview-only.
6. Attach screenshots for visible changes and recovery evidence for engine work.
7. Wait for the native Windows workflow before merging.
8. Delete the merged branch. Release branches are the exception.

## Commit messages

Use a short imperative subject with a stable area prefix:

```text
feat: verify staged package manifests
fix: preserve external Steam launch options
docs: explain interrupted deployment recovery
test: cover stale build plan rejection
chore: update Windows packaging checks
```

The body should explain decisions that are not obvious from the diff. Do not
add generated signatures, assistant credits, or promotional filler.

## Directory ownership

| Path | Responsibility |
| --- | --- |
| `src/` | React views, state, localization, visual components, and the typed Tauri bridge |
| `src-tauri/src/` | privileged filesystem, process, Steam, Dota, preset, and transaction logic |
| `src-tauri/fixtures/` | safe inputs for deterministic engine and recovery tests |
| `scripts/` | local and CI verification, Windows packaging helpers, and audits |
| `services/auth-worker/` | Telegram webhook, identity, sessions, avatars, entitlements, D1 migrations, and Worker tests |
| `website/` | public static site and its unprivileged browser account surface |
| `docs/` | product, engine, build, release, and contribution contracts |
| `.github/workflows/` | native Windows validation and signed release publication |

Changes to Rust commands, auth/session contracts, D1 migrations, payment state,
release workflows, updater settings, or recovery contracts require maintainer
review.

## Recommended GitHub settings

Protect `main` with:

- pull requests required before merge;
- at least one approval for engine and release changes;
- conversation resolution required;
- the Windows build job required;
- force pushes and branch deletion disabled;
- linear history preferred.

Enable Dependabot and secret scanning if available for the repository plan.
Signing keys, updater private keys, Telegram secrets, and release tokens belong
in GitHub Actions secrets, never in files or issue comments.

## Repository hygiene

Never commit:

- signing material, tokens, cookies, or personal Steam data;
- Dota installations, extracted game archives, or user backups;
- `target`, `node_modules`, local app-data journals, or generated installers;
- third-party art without recorded provenance and redistribution permission;
- logs containing usernames, machine paths, Telegram identities, or access data.
