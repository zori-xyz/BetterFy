# Skin archive audit

Status: the analyzer (`src-tauri/src/skin_archive.rs`) classifies the files of a
third-party hero skin VPK, and the embedded allowlist
(`src-tauri/wardrobe/manifests.json`, generated) pins 500 audited catalog
archives so the engine can plan and install them through the normal backup and
rollback transaction (see "Wardrobe items" in
[ENGINE_ARCHITECTURE.md](ENGINE_ARCHITECTURE.md) and the catalog-wide numbers in
[WARDROBE_AUDIT_REPORT.md](WARDROBE_AUDIT_REPORT.md)). Nothing has been run in
Dota through BetterFy: no skin has been installed by the app on Windows, and
whether any of them looks right in the game when BetterFy installs it is open.
The Windows steps are in [SKIN_WINDOWS_TEST_PLAN.md](SKIN_WINDOWS_TEST_PLAN.md).

The founder reports that the skins' authors agreed to community use. The
archives carry no licence file, so that statement is recorded in every
allowlist entry as `founder_reported_author_consent`, not as a licence.

## What changed, and why (the earlier rule is superseded)

The first version of this audit proposed, and the first analyzer implemented,
**stripping the shared game files by default**: any file in a skin that was not
the hero's own or in a namespace the author owned was dropped from the install
and listed. That rule is superseded.

The reason is evidence from real Windows use, reported by the founder: when he
drops these skin VPKs into the language folder as they are (the way the
upstream project's own install guide describes), everything works, with the
colours and the full new look. Installing the whole archive is therefore the
known-good behaviour, and stripping files can break the look (textures and
materials missing or replaced by the game's own). The default is now
**install every accepted file**. Stripping the shared game files is an explicit,
per-skin opt-in ("Keep the game's shared files untouched" in the build review),
off by default, and the interface says it may change how the skin looks.

What stays exactly as before: scripts, executables, unsafe paths and name
collisions are rejected, and the pinned hashes, the analyzer re-run on every
build, the backup, the journal and the rollback are unchanged. Valve's file list
is still used, but to *describe* the archive (which files override a shared game
file) instead of to drop them.

What changed afterwards, for the same reason (the archive should be kept as the
author made it, with nothing left out that the game would load): zero-length
compiled files now install, the size bounds follow the VPK's own, the proof for
Panorama image wrappers covers the shapes that really occur, and files that are
not game files are counted apart from files that are really dropped. Each is
described under "Rules as implemented" and "Panorama image wrappers".

## Sample: Scarlet Keeper of the Light

| Field | Value |
| --- | --- |
| Catalog entry | `heroes-scarlet-keeper-of-the-light` ("Scarlet Keeper of the Light") |
| Catalog author | Darkness (per the catalog's author link) |
| Catalog source | Dota2PornFxWeb snapshot 2026-07-26; mirrored on the project's GitHub repository and Hugging Face dataset |
| Archive | `Scarlet Keeper.zip`, 24,071,988 bytes, SHA-256 `f9f0c2f63a9989ea6fd2779cfea1676a2e07bd94eabab9e0c2127a68e7e268a7` |
| Contents | one file, `pak59_dir.vpk`, 58,096,981 bytes, VPK version 2, every entry embedded in the directory file (archive index 32767) |

It was picked as a hard case on purpose: custom particle effects, custom
materials, replaced hero models, replaced icons, and many files. It holds 1,010
entries: 447 `vpcf_c` particle systems, 290 `vtex_c` textures, 225 `vsnap_c`
snapshots, 18 `vmat_c` materials, 14 uncompiled `vpcf` leftovers, 11 `vxml_c`
Panorama icon wrappers and 5 `vmdl_c` models; no scripts and no executables.

## What the analyzer finds on the sample

Run locally on a Mac against `pak59_dir.vpk` (a run of the code, not a Windows
or in-game result), with the hero `keeper_of_the_light` and Valve's file list of
2026-10-17 (388,116 entries):

| Class | Files | Bytes |
| --- | ---: | ---: |
| Hero-scoped (includes the 11 image wrappers, all proven) | 214 | 1,343,162 |
| Author-owned (files the game does not ship) | 755 | 52,365,123 |
| Overrides a shared Valve file | 27 | 4,147,352 |
| Ignored, not game files (the 14 uncompiled `.vpcf`) | 14 | 191,574 |
| **Install, default mode** | **996** | **57,855,637** |
| Install with the shared files left out (opt-in) | 969 | 53,708,285 |

- The first audit counted 45 "shared" files by folder name and stripped them.
  Valve's own list says only 27 of them exist in the game (15 default
  textures with a hash suffix, cubemaps, `net_color`, status effects). The other
  18 (`particles/basic_*/*.vpcf_c`, `materials/particle/basic_*.vtex_c`,
  `particles/models/basic_trail`) exist nowhere in Valve's tree: they are new
  files the skin's own effects very likely refer to, and stripping them is what
  could leave references dangling. They are author-owned and always install.
- Identities: archive `6dfba0d998fb954c36a1bb7a844e92b7bf6805d5f691ff9560525d86bbe70e57`,
  install (default) `c9c9ac4bc822df19c6daf8b01ffecbf44c09dde8f051423a0ab3273ee5da0dc1`,
  install (opt-in) `371151d7332ba44baf2dce9ad86a32bbaa23aa19a5730be418607d630adbe521`,
  inner VPK SHA-256 `12137a743ff91c8dc1341e32718580109f86c70df735e24b022929e38cc9a190`.
- The merged BetterFy VPK built from the 996 files is 57,904,917 bytes, SHA-256
  `69de05a590aa33a01479b166034198ef360ca554c7a0efbf530967a57fcd586b`, plan ID
  `sha256:ba18ccfe2988d4266a7f9baaf145de4822e03597841948a8df8fe4c4eaf56fa3`
  (reproduced by a second build). The opt-in build is 53,756,163 bytes.

## Rules as implemented

- **Types.** Allowed: `vpcf_c`, `vsnap_c`, `vtex_c`, `vmat_c`, `vmdl_c`,
  `vmesh_c`, `vanim_c`, `vsndevts_c`, `vsnd_c`, `vmorf_c`, `vagrp_c`, `vphys_c`,
  `vseq_c`. Rejected: scripts (`vjs_c`, `js`, `lua`, ...), executables, and every
  other type. Rejected types split in two (see "Ignored files").
- **Empty files.** A zero-length compiled resource installs. VPK authors ship
  them on purpose as blank overrides (a silenced model, sound or snapshot, as in
  `models/items/axe/demon_blood_armor.vmdl_c` and nine `vsnap_c` files of one
  Hoodwink skin), a zero-length VPK entry is valid, and BetterFy's writer
  already supported it. They carry no bytes to execute. Zero-length scripts and
  executables are still hostile, and a zero-length layout proves nothing and is
  rejected. 11 files in the catalog.
- **Size bounds.** One file may be as large as the install, and the install may
  fill the merged VPK less 8 MiB for its directory (248 MiB, `vpk::MAX_VPK_BYTES`
  minus the reserve). The 16 MiB bound on one file was arbitrary: nothing in the
  VPK format needs it, and it dropped ten 4K textures and morph targets of 18 to
  22 MiB from six skins. The 128 MiB bound on the install refused two skins whose
  VPKs are 170 and 210 MiB. Staging a build for install enforced its own 64 MiB
  bound (`build_engine::MAX_STAGED_BYTES`), lower than both, so a skin over
  64 MiB could be planned but not installed; the staged VPK is now bounded by
  the same 256 MiB as the merge and the deployment. Memory use for the largest
  installs (about 207 MiB for `heroes-nude-marci`) has not been measured on
  Windows.
- **Paths.** Upper-case letters are folded to lower case (the game looks files
  up case-insensitively, and BetterFy's VPK writer is lower-case only); each
  fold is counted in the report ("names lower-cased"). Two entries equal once
  folded are both rejected. Traversal, spaces, empty segments and any other odd
  byte are an unsafe path and block the whole install. That is strict on
  purpose and costs 21 catalog items (see the report); relaxing it for spaces is
  a possible follow-up, not done here.
- **`vxml_c`.** Accepted only when the layout reader proves, from the parsed
  structure and not from the file name, that the file is Valve's image wrapper:
  `root > Panel[class] > Image[id class src]`, no styles, scripts, includes or
  event attributes, plain-text attribute values, and one `src` that is the
  sibling image of the wrapper. A reference list that is present names that one
  image and nothing else. Two containers are read (see "Panorama image
  wrappers"). A layout with a script body, a scripts or include element or an
  event attribute is rejected as `panorama_scripted`; anything else the reader
  cannot prove is `panorama_unproven`.
- **Ignored files.** A rejected file is *ignored*, not dropped, when the game
  does not load it from a VPK, so leaving it out changes nothing in game: the
  uncompiled sources (`vpcf`, `vmdl`, `vmat`, `vtex`, `fbx`, `tga`, ... ; Valve's
  own list of 388,116 files holds none of them), raw PNG/JPG outside the folders
  where Valve ships its own, and tool or reading leftovers (`thumbs.db`,
  `imageCompiler.xml`, readmes). The review says "Not game files, ignored: N",
  counts them apart from "Dropped", and lists them with the reason. What is
  *not* ignored, because the game may load it and BetterFy does not install it:
  `webm` hero videos, `usm`, `psf`, `dds`, `vcss_c` styles, `scripts/npc/*.txt`
  and `resource/*.txt`, and PNGs under `resource/`, `materials/vgui/` and
  `panorama/` (Valve ships PNGs there). These stay "Dropped" with the reason
  "file type not installed". Whether the game would read them from a mod VPK is
  not verified.
- **Scope (v2).** Every accepted path is hero-scoped, shared or author-owned.
  *Hero-scoped*: the folders and files of the heroes the audit names (a hero can
  have several internal names: `shadow_fiend` models and `nevermore` particles,
  `queen_of_pain` and `queenofpain`, so the audit carries a set). The rules
  cover `models/heroes|items/<h>`, `particles/units/heroes/hero_<h>`,
  `particles/econ/items/<h>`, `materials/models/heroes|items/<h>`, the hero's
  voice and weapon sounds, spell icon folders, hero and selection icons, the
  `game_sounds_*` sound events and `status_effect_<h>_*`. *Shared*: a path that
  exists in Valve's own file list at audit time and is not hero-scoped; the
  audit pins that list (`sharedPaths`). *Author-owned*: everything else, that
  is, files the game does not ship, whether in the author's own folders or new
  files in Valve-looking folders. The class of a path is decided by the pinned
  list, never by a name heuristic.
- **Policy.** `install` (default): every accepted file goes into the build and
  the shared files are counted and listed as information. `strip` (opt-in):
  the shared files are left out and listed. `refuse` (analyzer only, not an
  item mode): refuse an archive that carries any shared file.
- **Identity.** The report carries SHA-256 identities over sorted path and
  payload hash: one for every entry as the archive spells it, and one for the
  files that will be installed. The allowlist entry pins the archive identity,
  the default install result and the opt-in install result, and the analyzer
  must reproduce the requested mode's numbers on every build.

## Panorama image wrappers

The earlier proof accepted only wrappers stored as `<name>.vxml_c` that point
at `<name>_png.vtex`, with a reference list. The cached archives hold 527
`vxml_c` files; 307 passed that proof and 220 did not. Dumping their parsed
structure showed which were really plain image wrappers:

| Group | Files | What it is | Now |
| --- | ---: | --- | --- |
| Named `<name>_png.vxml_c`, tree in `LaCo`, image `<name>_png.vtex` | 131 | The tool compiled a PNG that was already called `x_png`; same tree shape as the passing wrappers (`Panel[class=AddonLoadingRoot] > Image[id=gamemode class=SeqImg src=s2r://...]`). | accepted |
| Older compiler: no `LaCo` block, the layout as XML text in `DATA`, reference list and `REDI` | 75 (74 named `<name>_png.vxml_c`, 1 named `<name>.vxml_c`) | The same three elements as text: `<root><Panel class="AddonLoadingRoot"><Image id="gamemode" class="SeqImg" src="s2r://..."/></Panel></root>`. | accepted, text parsed with the strict snippet reader |
| Tree in `LaCo`, no reference list (`RERL`) | 12 | Same tree; the compiler left the list out. | accepted |
| `SCRIPT_BODY` node next to a `Panel`, stored as `models/heroes/<Steam ID>.vxml_c` | 2 | An obfuscated packed script (`$.Msg("init")`) in `heroes-pudge-clown-toy` and `heroes-void-spirit-keqing`. | rejected, `panorama_scripted` |

Facts behind this:

- Valve ships no `vxml_c` under `panorama/images` (0 of 1,885 `vxml_c`; they are
  under `panorama/layout` and `panorama/hud_skins`), so there is no Valve original
  to compare with. The real icons are `*_png.vtex_c` textures, which Valve does
  ship (`panorama/images/heroes/npc_dota_hero_axe_png.vtex_c`) and which install.
  The wrappers are by-products of the authors' Workshop Tools addons: their
  `RED2` block lists the PNG and an `.xml` as inputs from `dota_addons/<name>`.
  Nothing known in the base game loads them; this is not verified in Dota.
- The older container stores the layout as text after the image table in `DATA`
  (the same place a `vcss_c` keeps its CSS). A layout with a `LaCo` tree must
  have no text there, so there is never a second copy the game might prefer.
- The sibling rule is evidence-based, not a safety rule: the image can only be
  `<name>_png.vtex`, or, for a wrapper named `<name>_png`, also the doubled
  `<name>_png_png.vtex` seen once (`nevermore_frenzy_png.vxml_c` of
  `heroes-ghost-shadow-fiend`, which the earlier rule accepted and the new
  spellings had to keep accepting). Any other image, a second element, a style
  block, any attribute but `class` on the panel and `id`/`class`/`src` on the
  image, and any non-plain value is rejected.
- Scripts are reported as scripted wherever the file is stored, not only under
  `panorama/images`. BetterFy never installs those two files. The two archives
  still carry them, and the authors of those skins should be asked about it; the
  founder may prefer to delist both. Neither is blocked, because the rest of the
  archive is what the audit describes.

Result over the 500 allowlisted archives: 0 wrappers dropped as unproven (209
before), 2 layouts dropped as scripted.

## Catalog-wide audit

`heroes` (460 items) and `hero-items` (52) of the catalog were audited by the
local generator (`wardrobe_audit::generate_wardrobe_allowlist`) against
Valve's file list, `game/dota/pak01_dir.txt` of SteamDatabase/GameTracking-Dota2
(SHA-256 `6d98c763afe09ba0de5caf0fc965b6b21f26eeb2954ff1e7744996ba056db1b7`,
388,116 entries). The result, item by item, is in
[WARDROBE_AUDIT_REPORT.md](WARDROBE_AUDIT_REPORT.md). In short: 500 of 512 items
are allowlisted; the rest are listed with their reason (an archive over the
256 MiB bound, an unsafe path or script, an unreadable container, never
fetched). Across the 500, 49,143 files are hero-scoped, 34,014 are files the
game does not ship, and 2,887 override a shared Valve file; 273 items override
none. Left out of the installs: 351 files really dropped (2 scripted layouts, and
349 of a type BetterFy does not install: 231 PNG, 67 `webm`, 17 `usm`, 15 `psf`,
11 `vcss_c`, 5 `txt`, 1 `dds`, 2 `.vmat_` typos) and 572 ignored as not game
files (538 uncompiled sources, 34 readmes and tool leftovers). Before this change
the same run dropped 209 unproven layouts, 11 empty and 10 oversized files, 769
uncompiled sources and 152 files of other types. The biggest install is
`heroes-nude-marci`, 207.3 MiB (a 217 MB VPK; 292 files). Valve's list was taken
from a public mirror on the day of the audit and reflects the game files at that
time, not necessarily the player's patch; the build can also compare a skin with
the player's own `pak01_dir.vpk` and report files the game ships now that the
audit did not know (information only, see ENGINE_ARCHITECTURE).

## Not verified

- Whether any skin works in the current Dota patch when BetterFy installs it.
  The founder's hand-copy evidence covers the as-is behaviour, not BetterFy's
  merged BetterFy VPK; the Windows plan tests it.
- Whether the Valve resources a skin replaces still exist at those paths after
  the 2026-07-23 patch, beyond what the player-side comparison reports.
- The meaning of every byte in the `vxml_c` files beyond their parsed
  structure and strings, and whether the game loads these wrapper layouts at all.
- Whether the game reads `webm`, `usm`, `vcss_c` or the other dropped types from
  a mod VPK. They stay dropped; allowing a type is a separate decision.
- Peak memory and time for the largest installs (a 207 MiB selection) on a real
  Windows machine; the largest build was only made on a Mac.
- Whether the bytes of any compiled file are well formed: the type allowlist
  limits what the game is asked to load, it does not parse every resource.
- Whether the hero-name rules cover every Valve path a skin for another hero
  needs. The hero names are inferred from Valve-present paths; 49 emitted items
  carry more than one name and 44 still override a file in another hero's
  folder (89 files in all), which is only information in the default mode.
- Licensing beyond the founder's report.
