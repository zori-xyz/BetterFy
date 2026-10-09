# Skin archive audit: first sample

Status: analysis only. Nothing here is installable yet, and no skin has been
run in Dota. This note records what a real third-party hero skin contains, so
the intake rules for wardrobe content rest on one concrete archive instead of
a guess.

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

## Not verified

- Whether the skin works in the current Dota patch, with and without the shared
  files.
- Whether the Valve resources it replaces still exist at those paths after the
  2026-07-23 patch.
- The meaning of every byte in the `vxml_c` files beyond their strings.
- Licensing.
