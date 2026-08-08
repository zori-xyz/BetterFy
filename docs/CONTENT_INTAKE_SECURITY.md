# Trusted content intake

This document defines the boundary between catalog metadata and bytes that the
BetterFy engine is allowed to use. It precedes archive extraction, VPK
construction, and any write to Dota 2.

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
- artifact format, filename, engine-owned HTTPS download URL, media type, byte
  size, and SHA-256;
- dependencies and incompatibilities;
- recipe version;
- Dota compatibility state and last verification state;
- signature state, including an explicit `not_provided` value.

An absent license, signature, or compatibility result is represented as
`unknown`/`not_provided`; it is never promoted to trusted by omission.

## Publication transaction

1. Acquire the BetterFy content-store lock.
2. Validate the complete manifest with strict size and count limits.
3. Resolve artifact bytes from an engine-owned source. The current remote slice
   accepts repository fixture IDs only; URLs and frontend paths are not accepted.
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

## Remote fixture acquisition

The Tauri bridge starts, inspects, and cancels a download by opaque operation ID.
It never accepts or returns a URL or filesystem path. Rust resolves the URL from
the validated fixture manifest, requires HTTPS on port 443, disables proxies and
automatic redirects, rejects non-public DNS answers, pins the resolved addresses
into the HTTP client, and verifies that the connected peer is one of those
addresses. Redirects are bounded, manual, and same-origin only.

The response streams into a unique BetterFy-owned `.part` file with a 64 MiB hard
limit. Cancellation, byte count, SHA-256, UTF-8/text checks, a second read, and
immutable-store publication are distinct phases. A failed or cancelled operation
removes its sidecar and never publishes a final object. DNS and connection work
can take up to the bounded connect timeout before cancellation is observed; no
game or Steam state is touched while waiting.

ZIP preflight is implemented as a metadata-only inspector. It rejects traversal,
absolute and device-style paths, non-ASCII or case-colliding names, Windows
reserved names, symlinks, encrypted entries, executable extensions, unsupported
compression, excess entry or expanded size, and extreme compression ratios. It
does not extract. Any future extractor must repeat containment and byte limits
while streaming because central-directory metadata is not proof of safe output.

## Explicitly excluded from this slice

- production catalog URLs, cross-origin redirects, resume, or authenticated CDN delivery;
- ZIP/VPK extraction or normalization;
- signatures and remote key rotation;
- user-selected local files;
- scripts, DLLs, executable tools, or shell commands;
- Dota deployment, backup, Steam shutdown, or launch-option changes.

Those surfaces require their own limits, failure injection, and release evidence
before they can consume the verified content store. Remote fixture acquisition is
an engine contract and is not yet connected to the catalog interface.
