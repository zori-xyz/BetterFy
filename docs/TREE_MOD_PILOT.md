# Tree Mod pilot ledger

This ledger fixes the first real-patch candidate before any resource is accepted
by BetterFy. The catalog exposes it for an internal Windows installation pilot;
that is not a compatibility or public-release claim.

- Pinned source repository: `Egezenn/dota2-minify` (Tree Mod credited upstream to `robbyz512`)
- Commit: `3a85572029f2c264e2a17cee1c9b54ce93e4fd93`
- Upstream directory: `Minify/mods/Tree Mod/files`
- Upstream license declaration: GPL-3.0
- Intended target: `game/dota_<selected-language>/pak66_dir.vpk`, with a choice
  of any Dota 2 language slot (`dutch` and `russian` observed on Windows).
- `game/dota_betterfy` (English) is offered as experimental and is not a verified
  mount point.
- Runtime actions allowed: data-only VPK construction; no upstream scripts
- Compatibility note: default terrain is required

The repository does not contain these compiled game resources. The founder reports
permission from the upstream developer for this pilot. The exact grant and required
notices are not recorded here, so public redistribution remains gated.

The pinned source path was checked against the maintained `Egezenn/dota2-minify`
repository. The previously recorded `robbyz23` owner returned 404. The resource
paths, sizes, and hashes below were not changed.

## Second pinned pilot package: Show Net Worth

The internal pilot may now build Tree Mod, Show Net Worth, or both in the exact
order selected by the user. The founder observed the Show Net Worth HUD change
in Dota 2 on Windows and checked rollback in the same session (see the
Windows evidence section below); this remains a single visually confirmed
session, not a recorded compatibility result.

- Package ID: `minify.show-networth`
- Same pinned repository and commit as Tree Mod
- Upstream directory: `Minify/mods/Show NetWorth/files`
- Runtime actions allowed: one compiled, data-only Panorama resource; no scripts

| VPK path | Bytes | SHA-256 |
| --- | ---: | --- |
| `panorama/layout/hud/dota_hud_quick_stats.vxml_c` | 2705 | `91193b3e5ced7d0d4122cfd5910aa3e16d7e2e3a38864f4146e8c9af1075a13d` |

BetterFy does not rename upstream VPK archives to resolve collisions. It verifies
the pinned resources, merges their resource maps, applies the visible selected
order (first package wins a differing same-path collision), and writes one
deterministic BetterFy-owned `pak66_dir.vpk`. Identical resources are deduplicated.
The full ordered inputs remain bound to the reviewed plan even when a later file
is shadowed.

| VPK path | Bytes | SHA-256 |
| --- | ---: | --- |
| `materials/default/default_color_tga_41192599.vtex_c` | 2184 | `31b8213992d35927c009f82a6dc25104e90179f1285317ccad4f81a78d90f247` |
| `materials/default/default_refl_tga_250508db.vtex_c` | 2280 | `f0456410fba5bf070fcb760ee2e3c8dde67748b229910e329f5627ca8f61162d` |
| `materials/tree_topiary.vmat_c` | 3541 | `4984a99ad0b98966c09fb30f3387d6e229c0011f0e17a2d2b721aef8ec6be3a2` |
| `materials/tree_topiary_block.vmat_c` | 2756 | `14fb25a924bfaaf9a422067348e3c83321530ab4433eaf8d56540731f2832e6c` |
| `materials/tree_topiary_normals_png_b25ef11b.vtex_c` | 176820 | `ed25236d789cb2c67b25055a0e1475dcb10d67e909e5bbc66720664180fa09e5` |
| `materials/tree_topiary_texture_png_6834bd45.vtex_c` | 176836 | `f53063c0d5d45a0f3d95650abbe9a0598704c708cc963358a56920ad784333c7` |
| `models/props_tree/dire_tree004.vmdl_c` | 16692 | `e38d311f65638ca693398102f0a6cf6b882c5c63d83757309949d57338180354` |
| `models/props_tree/dire_tree004b.vmdl_c` | 16709 | `4deef81ce074c7bab7065fbc093dcb6d0246750912faf1119af07c41a20bb59f` |
| `models/props_tree/dire_tree007.vmdl_c` | 16708 | `8a0a066202a1a47187958c10d473a22c52b87421c4650c3b466414e8c8b23c0f` |
| `models/props_tree/dire_tree008.vmdl_c` | 16692 | `65c3d58ca27c8e3446b3c2b8c9dd5da273eca0d835f870e7ed52e867a4bffd27` |
| `models/props_tree/tree_bamboo_01.vmdl_c` | 16584 | `74e6787b8eb7a4ce84c0b47accb34cbd62019b83965f5e66b00631305faeead1` |
| `models/props_tree/tree_bamboo_02.vmdl_c` | 16712 | `6621e86081c76356b580155ec6160a37fb9e567e7021030f78a03423c3caab73` |
| `models/props_tree/tree_cine_00_low.vmdl_c` | 16714 | `b603bb945fee943ddc9735fd1165f747ae9cebd95e18041325c7d5d9a533dd00` |
| `models/props_tree/tree_cine_02_low.vmdl_c` | 16682 | `2637581f1bf9830893540668e36722f6e7604c2ac6c8180d8cb61d9ba414d904` |
| `models/props_tree/tree_oak_01.vmdl_c` | 16581 | `17dedac072ef3f5b2ef68f5c9629046d3bd81cf41a50228f972b71fcb8a53a58` |
| `models/props_tree/tree_oak_01b.vmdl_c` | 16582 | `cdc510a3511b5011f9687ad1dd8588e9c2d74fd91b6b9fbc877b5728bf1a0351` |
| `models/props_tree/tree_oak_02.vmdl_c` | 16581 | `51afa0d18c83ef9581599efeaefadbc4b03cfee29849adee8aa7460c398ad7d1` |
| `models/props_tree/tree_pine_01.vmdl_c` | 16710 | `9c9a60aaf2ee340d7568f1dcd86b45f60b452d105f8a1f4164caac19c9e6b953` |
| `models/props_tree/tree_pine_02.vmdl_c` | 16710 | `5882bcc219dcce99549bcfaf398f6704eb58f682efe1b017855cafeed42498e7` |
| `models/props_tree/tree_pine_03b.vmdl_c` | 16711 | `5ff016bd90a915389d4d6742a88d62691b10d9d47dc974b1ed4b3f11d47cc35c` |
| `models/props_tree/tree_pine_03b_sfm.vmdl_c` | 16711 | `5ff016bd90a915389d4d6742a88d62691b10d9d47dc974b1ed4b3f11d47cc35c` |

## Enablement gates

1. Record the distribution and attribution terms for a public release.
2. Convert this ledger into a signed production package manifest without changing
   a path, size, or hash.
3. Download each resource through the pinned HTTPS content boundary and publish it
   to the immutable store only after exact verification.
4. Build the VPK in BetterFy staging and reopen it before deploy.
5. Pass the Tree Mod section of the Windows checklist and retain the safe report.

## Current internal pilot evidence

- Rust has a fixed 21-resource contract and constructs a deterministic VPK only
  after verifying every exact size and SHA-256. Unknown or missing paths fail.
- A pinned HTTPS intake can acquire these resources into BetterFy's immutable
  content-addressed cache. The VPK is built by reading verified cached objects.
- The local integration test downloaded all 21 resources from the pinned commit,
  built and reopened a VPK, staged it with a journal, then rolled staging back.
  This is a macOS synthetic test, not a Dota installation or compatibility test.
- A second network integration test downloaded all three pinned packages,
  rebuilt different selected orders from the verified cache, and confirmed 25
  resources. The ordered plan IDs differ while the non-colliding VPK bytes remain
  identical, proving priority is part of review identity rather than an accidental
  archive-name scheme.

## Third pinned pilot package: Repopulate Unit Query HUD

- Package ID: `minify.repopulate-unit-query-hud`
- Upstream directory: `Minify/mods/Repopulate Unit Query HUD/files`
- Runtime actions allowed: three compiled, data-only Panorama style resources;
  no blacklist, styling generator, or scripts

| VPK path | Bytes | SHA-256 |
| --- | ---: | --- |
| `panorama/styles/hud/dota_hud_query_unit_overrides.vcss_c` | 3957 | `bc9c831aacc37d21f5c48d6157ee6b80b9a06c3f9f51c48aed7c8446bb534e21` |
| `panorama/styles/hud/dota_hud_str_agi_int_overrides.vcss_c` | 1470 | `a6e25ccb69a40c145c75e590da8d5c19ac8a50224c2c54d09ffe4e8de8603871` |
| `panorama/styles/hud/tooltip_unit_damage_armor_overrides.vcss_c` | 1458 | `55efe3ff6bee15030c4016f6b18580d5d0877bfc8fbdcb3550beef4f23b70730` |

The founder observed this third package's HUD change in Dota 2 on Windows in
the same session as Tree Mod and Show Net Worth (see the Windows evidence
section below); a recorded compatibility result is still open.

- The internal pilot exposes only the pinned Tree Mod, Show Net Worth, and
  Repopulate Unit Query HUD paths. Generic staged-VPK
  deployment remains debug-only. The fixed pilot command rechecks the exact
  pinned VPK after staging before any game-directory write.
- Destination preview does not create a language folder in Dota. Synthetic
  tests cover all four allowlisted destination folders, foreign-file isolation,
  rollback, and interrupted Russian-folder recovery; these are not in-game
  compatibility evidence.
- macOS does not permit game deployment. The founder installed the ordered
  three-package bundle (Tree Mod, Show Net Worth, and Repopulate Unit Query
  HUD) on a Windows installer run and visually confirmed all three changes in
  Dota, then ran the internal stress artifact's controlled recovery pass —
  interrupt before publish, interrupt after publish, Steam before publish, and
  Steam after publish — with every check reporting `PASS`, including the
  visible BetterFy rollback returning the game to its normal state.
  This pass was observed visually in the installer and in Dota; the Windows
  test report JSON was not saved from this run, so exact previous-byte
  comparison against a recorded report, the three newly selectable language
  folders, and a second machine remain unverified. English has no separate
  game language folder.

## 2026-09-30 Windows pass on the manifest-driven engine

- Artifact: `BetterFy-Windows-Stress-x64-nsis` from Windows CI run
  `36742858198`, commit `cec9cf6` (package manifests, blacklist placeholders
  and zero-length resources, `minify.remove-river`, retained evidence reports,
  installed profile).
- Checklist given to the founder: Remove River in game (river water and
  splashes, Dire lava, fountains and waterfall, wading sounds) and its rollback;
  install and rollback of the three original packages; the Unit Query HUD
  change; `reports\` receiving an evidence file after install, the installed
  profile appearing after install and disappearing after rollback, and the
  "Open reports folder" button.
- Founder report, verbatim in substance: everything on that checklist was
  checked and works.
- Language folder: the founder confirmed afterwards that the pass used
  `dutch`; the Remove River manifest now records `dutch` as verified.
- Not recorded with this report: the saved evidence JSON itself and which Unit
  Query HUD change was observed. The evidence files are written automatically
  to `%APPDATA%\app.betterfy.desktop\reports\`.

## 2026-09-30 Windows pass on the signed catalog

- Artifact: `BetterFy-Windows-Stress-x64-nsis` from Windows CI run
  `36752379826`, commit `a28d1b5` (signed remote catalog, review hardening,
  catalog sequence 2 marking Remove River as verified with `dutch`).
- Checklist given to the founder: the app starts;
  `%APPDATA%\app.betterfy.desktop\engine-v1\catalog\` contains `index.json`,
  `index.json.sig` and `contracts.json`; Remove River no longer shows the
  "not verified in game" note.
- Founder report: all of it was checked and works.
- Not recorded with this report: the catalog `sequence` the device accepted and
  the contents of `contracts.json`.

## 2026-09-30 Windows pass through the Russian slot

- Build: BetterFy 0.1.4 on Windows, as recorded in the evidence file; the exact
  CI artifact was not recorded with this report.
- Evidence file `evidence-20260930T210552Z-2db4064f.json`, operation 3:
  language `russian`, phase `committed`, `backupVerified: true`, no error code,
  bundle `sha256:c3826d3c…d564574` with all four packages (`minify.tree-mod`,
  `minify.show-networth`, `minify.repopulate-unit-query-hud`,
  `minify.remove-river`). The matching Steam launch options entry is also
  `committed` with a verified backup. Operation 2 (Dutch, three packages)
  shows `rolled_back` with `rollbackVerified: true`.
- Founder report: all four mods work in game, and BetterFy ID registration
  works. All four manifests now record `russian` as verified in addition to
  `dutch`.
- Not recorded with this report: an evidence file after rolling back
  operation 3, and which Unit Query HUD change was observed. `koreana` and
  `schinese` remain unverified.
