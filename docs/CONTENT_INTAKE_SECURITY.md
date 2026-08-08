# Trusted content intake

This document defines the boundary between catalog metadata and bytes that the
BetterFy engine is allowed to use. It precedes archive extraction, downloads,
VPK construction, and any write to Dota 2.

## Assets and trust boundaries

| Surface | Trust | Rule |
| --- | --- | --- |
| React catalog selection | Untrusted request | May provide stable package IDs only. It never provides a URL, path, hash, recipe, or destination. |
| Catalog description | Display data | Never interpreted as installation instructions. |
| Versioned package manifest | Untrusted until Rust validation | Unknown fields, unsupported versions, invalid identities, missing provenance, and impossible limits are rejected. |
| Artifact bytes | Untrusted until verification | Exact byte count and SHA-256 must match the validated manifest before publication. |
| BetterFy content store | Application-owned | Content-addressed, immutable, symlink-free, and published atomically under app data. |
| Build planner | Trusted consumer | Cross-checks the verified content identity against a separately validated declarative recipe and binds both into the plan ID. |
| Dota 2 and Steam | Out of scope | Trusted content intake never reads or writes either product. |

## Version 1 manifest

The manifest keeps product description, provenance, artifact identity,
compatibility, and recipe metadata distinct. Required fields include:

- stable package ID and semantic package version;
- Russian and English name and description;
- category, resource type, author, and original HTTPS source;
- license/permission state and trust rationale;
- artifact format, filename, media type, byte size, and SHA-256;
- dependencies and incompatibilities;
- recipe version;
- Dota compatibility state and last verification state;
- signature state, including an explicit `not_provided` value.

An absent license, signature, or compatibility result is represented as
`unknown`/`not_provided`; it is never promoted to trusted by omission.

## Publication transaction

1. Acquire the BetterFy content-store lock.
2. Validate the complete manifest with strict size and count limits.
3. Resolve artifact bytes from an engine-owned source. The current slice uses
   repository fixtures only; URLs and frontend paths are not accepted.
4. Verify byte size and SHA-256.
5. Write and sync a temporary object inside the content store.
6. Re-read and verify the temporary object.
7. Atomically claim `objects/sha256/<hash>` with a same-directory hard link,
   without overwriting an existing object.
8. Serialize the validated manifest and atomically publish its record.
9. On retry, verify existing bytes and return the same identity without writing
   a duplicate.

A crash may leave a uniquely named BetterFy-owned temporary file. It is ignored
by reads and diagnostics; bounded stale-sidecar cleanup remains a separate future
maintenance operation. Existing immutable content is never replaced: a hash
mismatch is corruption and blocks the operation.

Before staging, the planner requires the package version, recipe version,
artifact filename, byte size, and SHA-256 to agree with the build recipe. The
SHA-256 content identity is part of the deterministic plan ID. Execution requires
explicit confirmation of that exact plan ID, so a re-planned or substituted input
cannot silently replace what was reviewed.

## Explicitly excluded from this slice

- remote downloads and redirects;
- ZIP/VPK inspection or extraction;
- signatures and remote key rotation;
- user-selected local files;
- scripts, DLLs, executable tools, or shell commands;
- Dota deployment, backup, Steam shutdown, or launch-option changes.

Those surfaces require their own limits, failure injection, and release evidence
before they can consume the verified content store.
