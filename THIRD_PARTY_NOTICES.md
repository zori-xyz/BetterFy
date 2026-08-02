# Third-party catalog data

BetterFy includes a metadata snapshot derived from
[h6rd/Dota2PornFxWeb](https://github.com/h6rd/Dota2PornFxWeb).

- Source snapshot date: 2026-07-26
- Upstream license: GNU General Public License v3.0
- Included data: catalog names, categories, preview references, style variants,
  dates, tags, and attribution links
- Excluded data: downloadable VPK/ZIP archives and bundled preview media

BetterFy keeps the upstream source visible in the interface and preserves
available author attribution. Preview images are requested from the upstream
repository and fall back to a local visual placeholder when offline.

## Dota2 Minify catalog

BetterFy also includes a source snapshot derived from
[Egezenn/dota2-minify](https://github.com/Egezenn/dota2-minify).

- Source snapshot: commit `db8c43d7df707851eca8eac3478ca5d355b6bcf1`,
  dated 2026-07-29
- Upstream repository license: GNU General Public License v3.0
- Included data: the 30 visible mod directories, Russian and English notes,
  author attribution, structural effect counts, and 19 upstream preview images
- Excluded data: mod payloads, patch scripts, blacklists, compiled Dota files,
  and executable tooling

The upstream repository states that some preview and mod assets originate from
Dota 2 and remain property of Valve Corporation. Included previews are approved
for the BetterFy prototype only and require a redistribution review or
replacement before release.
