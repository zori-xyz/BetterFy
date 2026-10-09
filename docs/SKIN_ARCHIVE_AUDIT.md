# Skin archive audit: first sample

Status: the first version of this audit was analysis only. The rules below are
now implemented as an analyzer (`src-tauri/src/skin_archive.rs`) and this one
archive is pinned in an embedded allowlist, so the engine can plan and install
it through the normal backup and rollback transaction (see "Wardrobe items" in
[ENGINE_ARCHITECTURE.md](ENGINE_ARCHITECTURE.md)). Nothing has been run in
Dota: no skin has been installed on Windows, and whether this one looks right
in the game, with or without its shared files, is open. The Windows steps are
in [SKIN_WINDOWS_TEST_PLAN.md](SKIN_WINDOWS_TEST_PLAN.md). This note records what
a real third-party hero skin contains, so the intake rules for wardrobe content
rest on one concrete archive instead of a guess.

The founder reports that the skin's authors agreed to community use. The
archive carries no licence file, so that statement is recorded in the allowlist
entry as reported, not as a licence.

## Sample

| Field | Value |
| --- | --- |
| Catalog entry | `heroes-scarlet-keeper-of-the-light` ("Scarlet Keeper of the Light") |
| Catalog author | Darkness (per the catalog's author link) |
| Catalog source | Dota2PornFxWeb snapshot 2026-07-26; archive hosted on the project's Hugging Face dataset |
| Archive | `Scarlet Keeper.zip`, 24,071,988 bytes, SHA-256 `f9f0c2f63a9989ea6fd2779cfea1676a2e07bd94eabab9e0c2127a68e7e268a7` |
| Contents | one file, `pak59_dir.vpk`, 58,096,981 bytes, VPK version 2, every entry embedded in the directory file (archive index 32767) |

It was picked as a hard case on purpose: custom particle effects, custom
materials, replaced hero models, replaced icons, and many files.

## What is inside (1,010 entries)

| Extension | Count | Meaning |
| --- | ---: | --- |
| `vpcf_c` | 447 | compiled particle systems |
| `vtex_c` | 290 | compiled textures |
| `vsnap_c` | 225 | particle snapshots (baked point clouds) |
| `vmat_c` | 18 | compiled materials |
| `vpcf` | 14 | **uncompiled** particle source text |
| `vxml_c` | 11 | compiled Panorama layouts (spell and hero icon wrappers) |
| `vmdl_c` | 5 | compiled models |

No `vjs_c` scripts and no executables. The 11 `vxml_c` files are about 1.9 KB
each and have the shape of Valve's image wrappers: one `.png` and one
`_png.vtex` dependency, nothing else readable in their strings.

### By location

| Location | Files | Kind |
| --- | ---: | --- |
| `particles/` | 583 | hero-scoped effects plus the shared files listed below |
| `kisilev_ind/` | 185 | the author's own namespace, no Valve path |
| `darkness/` | 178 | the author's own namespace, no Valve path |
| `materials/` | 38 | hero and item materials plus shared ones, see below |
| `panorama/images/` | 22 | spell and hero icons |
| `models/heroes/keeper_of_the_light/` | 4 | hood, mount, skirt, staff (replacements for Valve models) |

## Findings that shape the intake rules

1. **The archive overrides shared game files.** Besides the hero's own paths
   (`models/heroes/keeper_of_the_light`, `particles/units/heroes/hero_keeper_of_the_light`,
   `particles/econ/items/keeper_of_the_light`, `materials/models/heroes|items/keeper_of_the_light`),
   it replaces files every hero and the interface use:
   `particles/basic_ambient|basic_explosion|basic_projectile|basic_rope|basic_trail`,
   `particles/models/basic_trail`, `materials/default/*` (14 default textures),
   `materials/particle/basic_*` (5 textures), `materials/models/cubemaps/*`
   (9 cubemaps) and `materials/models/particle/net_color.vmat_c`. Installing it
   as is would change how other heroes' effects and materials look. A skin must
   be installed with a **scope filter**: only paths that belong to the hero (or
   to a namespace the archive owns) are allowed by default; shared paths are
   either stripped with a visible note or refuse the install. Which of the two
   is right depends on whether the skin still looks correct without them, and
   that has to be confirmed in the game.
2. **Uncompiled `.vpcf` text files are present** (14 of them under
   `particles/darkness/`). Dota loads compiled `_c` resources, so these are
   most likely leftovers. They are not an allowed type and would be dropped.
3. **`vxml_c` appears in a skin.** The project's script policy allows only
   audited Panorama layouts. These 11 are icon wrappers, so the intake can
   either keep them out of the first slice or admit a layout only when the
   parsed structure matches the image-wrapper shape exactly (one image
   dependency, no events, no scripts). The existing layout reader already
   parses this container; a shape check is a smaller promise than a hash list,
   but it is a new promise and needs its own tests.
4. **Size.** 58 MB uncompressed in one archive. The deployment builder embeds
   all entries into one VPK, so a build of several skins grows quickly; the
   build plan should show the size before installing.
5. **Provenance.** The catalog credits an author but the archive carries no
   licence or credit file, and the catalog's own notice says it cannot
   license other authors' work or Valve files inside mods. Public use needs the
   author's permission, not only the catalog's licence.

## Proposed rules for wardrobe archives

- Allowed types: `vpcf_c`, `vsnap_c`, `vtex_c`, `vmat_c`, `vmdl_c`, `vmesh_c`,
  `vanim_c`, `vsndevts_c`, `vsnd_c`. Everything else is rejected, including
  `vjs_c`, uncompiled sources and executables.
- `vxml_c` only through a structural image-wrapper check, or not in the first
  slice.
- Every path is classified before installing: hero-scoped, author-owned, or
  shared. Shared paths never pass silently.
- The reviewed plan lists counts per class, total size, and every rejected or
  stripped path, and the user confirms exactly that plan.
- The first end-to-end proof is this one archive, installed alone, played in
  Dota on Windows, and restored; then two skins together, to cover conflicts.

## Rules as implemented

The proposed rules above became `skin_archive.rs`. Where the code is more
specific than the proposal, this is what it does:

- **Types.** Allowed: `vpcf_c`, `vsnap_c`, `vtex_c`, `vmat_c`, `vmdl_c`,
  `vmesh_c`, `vanim_c`, `vsndevts_c`, `vsnd_c`, `vmorf_c`, `vagrp_c`, `vphys_c`.
  Rejected: scripts (`vjs_c`), uncompiled sources (`.vpcf`, `.vmdl`, ...),
  executables, empty files, files over 16 MiB, and every other type. Case
  collisions between two entries reject both. An unsafe path (traversal,
  uppercase, spaces, empty segments), a script or an executable also blocks the
  whole install instead of being dropped.
- **`vxml_c`.** Accepted only when the layout reader proves the file is Valve's
  image wrapper: `root > Panel[class] > Image[id class src]`, no styles,
  scripts, includes or event attributes, plain-text attribute values, one `src`
  that is the sibling `<name>_png.vtex` of the wrapper, and a resource
  reference list that names that one image and nothing else. If the reader
  cannot prove it, the file is rejected.
- **Scope.** Hero-scoped paths are the exact folders of one hero
  (`models/heroes/<hero>`, `particles/units/heroes/hero_<hero>`,
  `particles/econ/items/<hero>`, `materials/models/heroes|items/<hero>`, hero
  files named `<hero>_*` directly in `particles/units/heroes`, and the hero and
  spell icons under `panorama/images/heroes` and `panorama/images/spellicons`).
  Author-owned paths are a top-level folder that is not one of Valve's, or a
  folder or file under `particles/`, `materials/` or `models/` named after a
  namespace a person declared in the allowlist entry (`darkness` here, which
  covers `particles/darkness_snaps/...`). Declared namespaces are never derived
  from the archive, because an archive that names its own top-level folder
  `units` must not be able to claim `particles/units/...`. Everything else is
  shared. The list of Valve's top-level folders is deliberately generous, and
  one name on it only means a folder of that name is never author-owned.
- **Shared policy.** `strip` (this entry) drops shared files and lists each one;
  `refuse` refuses the install.
- **Identity.** The report carries two SHA-256 identities over sorted path and
  payload hash: one for every entry in the archive and one for the files that
  will be installed. The allowlist entry pins both, and the analyzer must
  reproduce them on every build.

## What the analyzer found on the sample

Run locally on a Mac against `pak59_dir.vpk` (VPK v2, all 1,010 entries
embedded), with the hero `keeper_of_the_light`, the declared namespace
`darkness` and the `strip` policy. This is a run of the code, not a Windows or
in-game result.

| Class | Files | Bytes |
| --- | ---: | ---: |
| Hero-scoped (includes the 11 image wrappers, all proven) | 214 | 1,343,162 |
| Author-owned (`darkness/`, `kisilev_ind/`, `particles/darkness*`) | 737 | 51,937,190 |
| Shared, stripped | 45 | 4,575,285 |
| Rejected | 14 | 191,574 |
| Archive total | 1,010 | 58,047,211 |
| To install | 951 | 53,280,352 |

- The 14 rejected files are all uncompiled `.vpcf` leftovers under
  `particles/darkness/`, as predicted. No script, executable, unsafe path or
  duplicate was found.
- The 45 shared files are the ones the first audit named: 15 under
  `materials/default`, 9 cubemaps under `materials/models/cubemaps`, 5 under
  `materials/particle`, `materials/models/particle/net_color.vmat_c`, 12 under
  `particles/basic_*`, `particles/models/basic_trail/basic_trail.vpcf_c`, and
  the two `particles/status_fx/status_effect_keeper_spirit_form*` files. The
  last two are named after the hero but live in a shared folder, so they are
  shared by rule.
- By bytes, about 97% of the install is the author's own namespaces; the
  hero-scoped replacements are 214 files and 1.3 MB. The merged BetterFy VPK built from the 951 files
  is 53,327,413 bytes.
- Archive identity `6dfba0d998fb954c36a1bb7a844e92b7bf6805d5f691ff9560525d86bbe70e57`,
  install identity `eaa2727992025de0f2e820859c673dfae9ad6a2c553a76c1dbb9761ebff98c82`,
  inner VPK SHA-256 `12137a743ff91c8dc1341e32718580109f86c70df735e24b022929e38cc9a190`.

## Not verified

- Whether the skin works in the current Dota patch, with and without the shared
  files. This decides whether `strip` is the right policy for it; the Windows
  plan tests both the stripped install and, if it looks wrong, notes what is
  missing.
- Whether the Valve resources it replaces still exist at those paths after the
  2026-07-23 patch.
- The meaning of every byte in the `vxml_c` files beyond their parsed
  structure and strings.
- Whether the bytes of any compiled file are well formed: the type allowlist
  limits what the game is asked to load, it does not parse every resource.
- Whether the hero-name rules cover every Valve path a skin for another hero
  needs (only this archive's shapes have been exercised), and whether Valve
  has top-level folders missing from the analyzer's list.
- Licensing beyond the founder's report.
