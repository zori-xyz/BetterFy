# Map terrain audit report

Written by `cargo test generate_map_allowlist -- --ignored` from archives cached on a Mac. It carries no timestamp: the same inputs give the same file. The audited facts are pinned in [`src-tauri/maps/manifests.json`](../src-tauri/maps/manifests.json), and every build re-runs the map analyzer (`src-tauri/src/map_package.rs`) on the stored bytes and must reproduce them. A mirror is listed only when it answered 200 with the archive's size and, when the fetch report carries it, served bytes with the same SHA-256.

| What | Value |
| --- | --- |
| Catalog | `src/webCatalog.json`, snapshot 2026-07-26 |
| `terrains` catalog items | 20 |
| ZIP items (`maps/dota.vpk` + `Guide.txt`) | 12 |
| Audited and allowlisted | 12 |
| Not installable | 0 |
| Valve file list | `game/dota/pak01_dir.txt` (SteamDatabase), SHA-256 `6d98c763afe09ba0de5caf0fc965b6b21f26eeb2954ff1e7744996ba056db1b7`, 388116 entries |
| Raw `pakNN_dir.vpk` terrains | 8 catalog items, not handled here (ordinary VPKs, installed through the generic catalog path) |

## Allowlisted maps

| Engine ID | ZIP (bytes) | ZIP SHA-256 | `maps/dota.vpk` (bytes) | VPK SHA-256 | Entries | Payload bytes | Mirrors |
| --- | ---: | --- | ---: | --- | ---: | ---: | --- |
| `map.crownfall` | 29385875 | `fe03aa100304c5185f77421143a3fd335ec8e3c871e82ad098003c311853c238` | 53299338 | `f2447a6703d72053e1c307775f8418025912829426f996fcb66de6f6b7d9d8ed` | 1895 | 53201408 | GitHub, Hugging Face |
| `map.dota-autumn` | 18993813 | `ed1fb65b3719806e50c0b3ada8022d37e0a8d4eab8592b38ed03b371aa8356b6` | 34537287 | `4c491002520fd207cb4206d600fd13d316549a4b0a8b3768f336b0934fa3639f` | 1214 | 34472853 | GitHub, Hugging Face |
| `map.dota-spring` | 19482925 | `c9fb2596881acbd90697226be36ab0a8057213da01d01be89660209f2e10881f` | 35460094 | `d05f96d135c53acc4a04b36948e842bbc7148961fd07f171d4d030350d003856` | 1282 | 35392124 | GitHub, Hugging Face |
| `map.dota-summer` | 19211560 | `15775fd397e0c0558ab369b5a0e45dd8bf3ec4fab3cd1f5d028a2fd61ef67963` | 34778111 | `d3aa35cf139c33c44bedbb1ab5c9428be5073b6294b62ee3a30227debcd5697c` | 1214 | 34713610 | GitHub, Hugging Face |
| `map.dota-winter` | 22298561 | `9084a74717702af036aef65257b8df0802e8d485a6252084446a0cf049d7353b` | 37622865 | `8cade537206904aaa6b27606fd184d3461210f4a79f5e6dec33294181334106d` | 1336 | 37551246 | GitHub, Hugging Face |
| `map.the-kings-new-journey` | 20015844 | `890de3f3038ea3c4b1d5ba9f9125c48f8248b86f425200b0dbfb4c4841854335` | 36802575 | `a854fd446849a8e2ee8704a1fc59bac71eb4566ff36218efa38785d35de2f187` | 1354 | 36730973 | GitHub, Hugging Face |
| `map.ti10-sanctum-of-the-divine` | 24087900 | `942fce12ba76074886d8ebbc1c0438821d4f24d37c98e107a79f81de8d021c0f` | 39027093 | `b13f961c613dc98f326a42ffa284655bfa44559d850634ffad9e487a83689964` | 1375 | 38954505 | GitHub, Hugging Face |
| `map.ti5-desert-terrain` | 21506071 | `e69bc587a996daa8acbe1d563c10c785957a7648d6ace8c9086cc387739fe7a0` | 36768159 | `39462e9cee057a623be04c8d13ec97e2dbbde5ddd39cc8f420d034665853ff7b` | 1337 | 36697116 | GitHub, Hugging Face |
| `map.ti6-immortal-gardens` | 20219973 | `197ae67961dbd9345346f531d7e6269fe359a1ecd0c4393a2abf2be4af071561` | 35428003 | `8c421a47c862f7b06aaaa66e4f458843c61d874303a48d44ce868df083f57754` | 1291 | 35358904 | GitHub, Hugging Face |
| `map.ti7-reefs-edge` | 19856441 | `6b65fb883b49e5cc312088b2ef6684cbf11fb400ac155140ca98206af5eead3d` | 39998774 | `03706dc6a079fc56b1b89a970ee9c10b40f7c19197a2eb7ce00ad0e720de67af` | 2308 | 39868923 | GitHub, Hugging Face |
| `map.ti8-emerald-abyss` | 20833939 | `6a8dc25ff2010b3826dd1e58ae8b5ab2ae8df794d071c7a4b6fda4916b759f28` | 35590789 | `2a849a5fe789da4be79a1547ca0ff4521eb3f6119890574b29935b03ad6a5fc1` | 1274 | 35522932 | GitHub, Hugging Face |
| `map.ti9-overgrown-empire` | 20087995 | `f0d6b49ad22b53626ce4c3f8dfb4108a4523202eb67ca4e523f73a35d89963dc` | 37514537 | `6775f7a0e5804bd1875b37f303c86de4ba396c25cda96fa65743e358845a4592` | 1234 | 37449114 | GitHub, Hugging Face |

## What each map package contains

| Engine ID | VPK | Types (entries) | Under `maps/` | Contents identity | Paths also in Valve's list |
| --- | --- | --- | --- | --- | ---: |
| `map.crownfall` | v2 | gnv 2, trm 2, vents_c 10, vgrass 2, vhcg 2, vmap_c 1, vmdl_c 1858, vrman_c 6, vtex_c 6, vvis_c 2, vwnod_c 2, vwrld_c 2 | (top) 7, dota 1026, dota_crownfall 862 | `8fa7c4a1c7f60c3b046d9e86997c70917985f1b6435bd2f2a72f377c15ca8ed8` | 0 |
| `map.dota-autumn` | v2 | gnv 1, trm 1, vents_c 5, vgrass 1, vhcg 1, vmap_c 1, vmdl_c 1195, vrman_c 3, vtex_c 3, vvis_c 1, vwnod_c 1, vwrld_c 1 | (top) 4, dota 1210 | `d5ea007bcc8efbe8a97f4618442ea7ab1210f6740562a44fdc834d5e0fb9d16f` | 0 |
| `map.dota-spring` | v2 | gnv 1, trm 1, vents_c 5, vgrass 1, vhcg 1, vmap_c 1, vmdl_c 1263, vrman_c 3, vtex_c 3, vvis_c 1, vwnod_c 1, vwrld_c 1 | (top) 4, dota 1278 | `d8ed25316caff7d05f5cd93e262ac694cdf8ff3e15e5beba7d18f38a57a5f5d0` | 0 |
| `map.dota-summer` | v2 | gnv 1, trm 1, vents_c 5, vgrass 1, vhcg 1, vmap_c 1, vmdl_c 1195, vrman_c 3, vtex_c 3, vvis_c 1, vwnod_c 1, vwrld_c 1 | (top) 4, dota 1210 | `f085c80b88d9d6cd89ab0d850f8e9518aaced033885c49c24233f8925db0e33b` | 0 |
| `map.dota-winter` | v2 | gnv 1, trm 1, vents_c 5, vgrass 1, vhcg 1, vmap_c 1, vmdl_c 1317, vrman_c 3, vtex_c 3, vvis_c 1, vwnod_c 1, vwrld_c 1 | (top) 4, dota 1332 | `ec4b67912c544cd2c5d63c369e872029cb8c5731f864ce8ff46b7d517df24777` | 0 |
| `map.the-kings-new-journey` | v2 | gnv 1, trm 1, vents_c 5, vgrass 1, vhcg 1, vmap_c 1, vmdl_c 1335, vrman_c 3, vtex_c 3, vvis_c 1, vwnod_c 1, vwrld_c 1 | (top) 4, dota 1350 | `c46d4a88afff4b17f077ab304f59e90e978fa5aae3227990fe2ac108faed7c69` | 0 |
| `map.ti10-sanctum-of-the-divine` | v2 | gnv 1, trm 1, vents_c 6, vgrass 1, vhcg 1, vmap_c 1, vmdl_c 1355, vrman_c 3, vtex_c 3, vvis_c 1, vwnod_c 1, vwrld_c 1 | (top) 4, dota 1371 | `ab861e3a4962d7c50be8896732a02f27586cfd3a8716558ab2339381f694b37f` | 0 |
| `map.ti5-desert-terrain` | v2 | gnv 1, trm 1, vents_c 5, vgrass 1, vhcg 1, vmap_c 1, vmdl_c 1318, vrman_c 3, vtex_c 3, vvis_c 1, vwnod_c 1, vwrld_c 1 | (top) 4, dota 1333 | `eacceb17df18fe61e048d2759a289d779bcc35ddc752bfb8548944897840a07f` | 0 |
| `map.ti6-immortal-gardens` | v2 | gnv 1, trm 1, vents_c 5, vgrass 1, vhcg 1, vmap_c 1, vmdl_c 1272, vrman_c 3, vtex_c 3, vvis_c 1, vwnod_c 1, vwrld_c 1 | (top) 4, dota 1287 | `77ef80464fe48d6b9bdf4b492b7bb6397a47c4eb81653b37867d39e64718c869` | 0 |
| `map.ti7-reefs-edge` | v2 | gnv 1, trm 1, vents_c 5, vgrass 1, vhcg 1, vmap_c 1, vmdl_c 2289, vrman_c 3, vtex_c 3, vvis_c 1, vwnod_c 1, vwrld_c 1 | (top) 4, dota 2304 | `a07cf8fe6baf3716862ac63882a26b4ad6b8d1158dbae42c1926bf8297385f27` | 0 |
| `map.ti8-emerald-abyss` | v2 | gnv 1, trm 1, vents_c 5, vgrass 1, vhcg 1, vmap_c 1, vmdl_c 1255, vrman_c 3, vtex_c 3, vvis_c 1, vwnod_c 1, vwrld_c 1 | (top) 4, dota 1270 | `c29c2964801987a70c056510bbf654eb03ee6c28c58448dbbbfb7dd6be8b4801` | 0 |
| `map.ti9-overgrown-empire` | v2 | gnv 1, trm 1, vents_c 5, vgrass 1, vhcg 1, vmap_c 1, vmdl_c 1215, vrman_c 3, vtex_c 3, vvis_c 1, vwnod_c 1, vwrld_c 1 | (top) 4, dota 1230 | `84f7eafe11797c309fff18950b79aab6c18dae58c5bbc90a4fda8585f112474d` | 0 |

## Guide.txt

Every ZIP carries a `Guide.txt`. 1 distinct file(s) among the 12 archives, pinned by size and SHA-256 in each manifest:

- `716b14d5e198b98d748e5478318fa3fde1bd40becebc977007bd87b3cf10403f`, 3292 bytes, in 12 archive(s). The English part says: create `dota_<name>` in `steamapps/common/dota 2 beta/game/`, move the `maps` folder into it, and add `-language <name>` to the launch options; for Minify, put `maps` in `dota_minify` with `-language minify`; for another language, put it in that language's `dota_<language>` folder. It also has Russian and Ukrainian sections with the same instructions. It names no map-specific note.

## Not installable

None: every ZIP item of the catalog passed.

