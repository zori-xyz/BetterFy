# Tree Mod pilot ledger

This ledger fixes the first real-patch candidate before any resource is accepted
by BetterFy. It is evidence, not an enabled catalog package.

- Upstream: `robbyz23/dota2-minify`
- Commit: `3a85572029f2c264e2a17cee1c9b54ce93e4fd93`
- Upstream directory: `Minify/mods/Tree Mod/files`
- Upstream license declaration: GPL-3.0
- Intended target: `game/dota_dutch/pak66_dir.vpk`
- Runtime actions allowed: data-only VPK construction; no upstream scripts
- Compatibility note: default terrain is required

The repository does not contain these compiled game resources. Distribution and
source-notice review must be accepted before a production registry points to them.

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

1. Accept the distribution and attribution decision for the compiled resources.
2. Convert this ledger into a signed production package manifest without changing
   a path, size, or hash.
3. Download each resource through the pinned HTTPS content boundary and publish it
   to the immutable store only after exact verification.
4. Build the VPK in BetterFy staging and reopen it before deploy.
5. Pass the Tree Mod section of the Windows checklist and retain the safe report.
