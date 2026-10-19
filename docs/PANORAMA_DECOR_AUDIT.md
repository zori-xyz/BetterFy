# Panorama decor audit

Status: the proofs (`src-tauri/src/panorama_decor.rs`) exist, are tested on a Mac
and in CI, have been run over the real catalog archives of the categories below,
and are now wired into the archive pipeline for `backgrounds`, `huds` and
`versus-screens` (see "In the pipeline" in ENGINE_ARCHITECTURE and
CATALOG_CATEGORIES.md). Nothing here has been installed by BetterFy on Windows or
run in Dota.

The project rule behind it is the founder's decision "only audited scripts and
layouts": BetterFy never installs a script, and installs a Panorama layout
(`vxml_c`) or style (`vcss_c`) only when a parsed-structure proof shows it cannot
run code or reach outside the game. A hero skin brings one layout shape (an
image wrapper), which an earlier proof covers. These categories bring whole
layouts, style sheets and videos.

## What was audited

The Dota2PornFx catalog categories that carry Panorama files, fetched from the
project's GitHub mirror (Hugging Face as a second) into a local cache and read
without installing anything:

| Category | Archives | Panorama files |
| --- | ---: | --- |
| `backgrounds` | 62 raw VPKs | 306 `vxml_c` layouts, 183 `vcss_c` styles, 61 `webm` videos (all under one folder of the author's, `zxc/`), 54 `txt` |
| `huds` | 4 raw VPKs | 32 `vcss_c` styles, 148 `vtex_c` textures (17.7 MiB) |
| `versus-screens` | 7 raw VPKs | 14 `vxml_c` layouts |
| `item-effects` | 1 archive with Panorama files | 5 `vxml_c` (bottle icon image wrappers) |
| `herofx` | 3 archives list `panorama/` paths | no layout or style: only `vtex_c` textures |

Most of these files are byte-identical across archives, which matters for the
review: the 306 background layouts are 7 distinct files (the same template in
every archive; what differs is the video), the 183 background styles are 3
distinct files, and the 32 HUD styles are 28.

## What the real files contain

### Layouts (`vxml_c`), 325 files

All 325 are the current compiler's container (`RED2`, `DATA` with no text,
`LaCo` tree; the five image wrappers also carry a reference list). Read with the
existing KV3 reader.

| Group | Files | What they are |
| --- | ---: | --- |
| Pure decor: `dashboard_background_manager`, `dashboard_background_last_match`, `hero_loadout_background_images` | 183 | A style include, one custom panel, a `MoviePanel` (`src` a `zxc/zxc.webm` of the same archive, `repeat`, `autoplay="onload"`), two `DOTAScenePanel`s with a scene name. No handler, no script. |
| `dashboard_page_home` | 61 | Valve's home page layout with two event attributes (`onnavigatedto`/`onnavigatedaway`, `DOTASetFriendsMenuVisible( true\|false )`). |
| `dashboard` | 62 | Valve's whole dashboard layout (75 distinct event attributes: `DOTAShowHomePage()`, `DOTAQuit();`, `AsyncEvent( 0.0, ... )` and so on), with one panel, `DOTADashboardBackgroundManager`, carrying `override-background`. |
| Versus screens | 14 | Valve's versus layout with a scene name and style chosen; it includes five of Valve's own `vts_c` scripts and a debug panel with `PlayScene()`, `ToggleFadeOutEnabled()` and, in some, `DOTAReloadCurrentPage()` handlers. |
| Image wrappers (item-effects) | 5 | `Panel > Image`, covered by the earlier wrapper proof. |

What does **not** occur anywhere: a script body (`SCRIPT_BODY`), a snippet, a
`file://` (passthrough) reference, an external URL, a `$.` call, a hotkey, an
HTML or browser panel, an `<include>` of another layout, or a text copy of the
layout in `DATA`. 41 panel types and 31 attribute names occur, all Valve's.

The important consequence: **event handlers are not decoration.** In Panorama an
`on*` attribute holds code (the Valve wiki's examples put `$.ToggleStyle(...)`
and bare function calls in `onactivate`). 137 of the 325 layouts (dashboard,
home page, versus screens) carry them, because they are copies of Valve's own
layouts with a small change. The first rule of thumb ("no event handler
attribute, no script include") would reject all 137. The earlier image-wrapper
rule accepts 5 of the 325 (320 are reported as scripted, because a style include
counts as one for that rule).

### Styles (`vcss_c`), 215 files (31 distinct)

Container: `RED2`, `DATA`, `SrMa`, with an empty image table. The text is CSS:

- 97 distinct properties (sizes, colours, spacing, transforms, transitions,
  animations, blur and so on, plus `sound`, whose three values name sound events),
  20 functions (`gradient`, `rgba`, `translateX`, `cubic-bezier`, `url` ...),
  72 `@keyframes`, 17 `@define`, 3 `@import` (one Valve style).
- 602 `url()` references, every one a quoted `s2r://panorama/images/...` texture
  that Valve's own file list has; none refers to a file of the archive and none
  is an external URL, `data:` or `file:`.
- No comment, escape, `expression`, `behavior`, `javascript` or non-ASCII byte.
- Largest selector 1,549 bytes, largest value 195, nesting depth 2.

The HUD styles are the game's own HUD rules restyled (the archives also carry
the replacement textures as ordinary `vtex_c`).

### Videos (`webm`), 61 files

17,931 to 20,855,680 bytes (the largest archive, 21 MB, is mostly its video).
Every one is a Matroska/WebM file: EBML header with doctype `webm`, one
segment whose size is exactly the rest of the file, top-level elements only from
SeekHead, Void, Info, Tracks, Cluster, Cues and Tags (no attachments, chapters or
other elements), VP9 video, and Opus (4 files) or Vorbis (1) audio in five of
them. They are referenced by the `MoviePanel` of the archive's own layout.

## The proofs

All three refuse by default and say why (`Refusal::code()` is stable and is what
a report counts).

### `prove_static_layout`

Reads the parsed tree, never the file name. Accepts only:

- the current container (`RED2`, `DATA` without text, `LaCo`), a tree of the
  shape `root > styles? scripts? panel`, at most 48 deep and 4,000 elements;
- panel types from a closed list of 41 Valve types (no HTML, carousel or other
  browser-like panel), attributes from a closed list of 31, each with a fixed
  value shape (an identifier, a boolean, a number or duration, a lower-case
  scene name, short display text); no `src` except on `Image` (a texture) and
  `MoviePanel` (a video);
- references that name a file of the same archive (recorded as a dependency) or
  one of Valve's own; a style include can be either, a **script** include only
  a Valve script that the archive does not replace;
- an event attribute only as an **exact pinned binding**: element type, `id`,
  attribute and code identical to one of 79 read in the real archives
  (`panorama_decor/pins.rs`), and the code must also pass a shape check (calls
  only; no `$`, member access, quotes other than simple single-quoted names,
  operators or URLs).

A layout with neither handlers nor scripts is `Tier::PureDecor`; one with pinned
handlers or Valve scripts is `Tier::ReviewedValveBindings`, which one switch
(`allow_reviewed_valve_bindings`) turns off.

### `prove_style_sheet`

Container as above with an empty image table. Text: ASCII only, no comment, rules
and `@keyframes` (depth at most 3), `@define`, and `@import url("s2r://....vcss")`
only; declarations only of the 97 properties; values from a fixed character set,
only the 20 listed functions, `url()` only as a quoted `s2r://` texture that is
in the archive or Valve's list (a missing target is refused), and a whole quoted
value only as a plain name. Anything else (`@font-face`, `@media`, `behavior`,
unknown functions, `!important`, escapes, `http:`, `data:`, `file:`, unterminated
strings or blocks) is refused with the offending name.

### `check_webm`

A structural check of the container. It establishes: EBML header, doctype `webm`,
a segment exactly as long as the rest of the file, only the top-level elements
listed above, element sizes that chain without gap or overrun, one `Info`, one
`Tracks`, at least one cluster, tracks naming VP8/VP9/AV1 video and at most
Vorbis or Opus audio, at most 64 MiB. **It does not decode a frame.** It cannot
show that the stream is well formed inside a cluster or that the game's video
decoder handles it safely, and nothing here claims so. A video is installable
only if an accepted layout of the same archive plays it, at a path of the shape
`folder/name.webm` that is not a Valve file.

### `prove_archive` and the category scopes

Runs the three over an archive, then resolves in-archive references: a file whose
dependency was refused is refused, a loop (two styles importing each other) is
refused, and a video no accepted layout plays is refused. Each category also
takes only the files its purpose needs: backgrounds replace `dashboard.vxml_c`,
`dashboard_page_home.vxml_c`, `hero_loadout_background_images.vxml_c`,
`dashboard_background_*.vxml_c` and the matching `dashboard*` styles; HUD skins
replace `panorama/styles/hud/**`, `dota_hud_*` and `ui_stat_branch` styles only;
versus screens replace `panorama/layout/hud/versus/**` layouts. A background that
tried to replace the accept-match popup is refused as `decor_not_for_category`.

## Results over the cached archives

Run locally on a Mac (`panorama_decor::tests::audit_cached_decor_archives`, an
`#[ignore]`d test that needs the downloaded archives and Valve's file list of
2026-10-17, 388,116 entries). This is a run of the code, not a Windows or
in-game result.

| Category | Archives | Every layout, style and video accepted |
| --- | ---: | ---: |
| `backgrounds` | 62 | 62 (183 pure-decor layouts, 123 with pinned bindings, 183 styles, 61 videos) |
| `huds` | 4 | 4 (32 styles) |
| `versus-screens` | 7 | 7 (14 layouts, all with pinned bindings and Valve scripts) |

Read this carefully: **the pinned bindings were taken from these same archives,
so the 100% is by construction for them.** What it shows is that the grammar,
the property list, the reference rules and the container checks accept every
layout, style and video of the real archives, and that the only things which
needed reviewing by hand were the 79 bindings. It says nothing about how an
archive added to the catalog later would fare: it would pass only if it reuses
those bindings and Valve's scripts, and otherwise be refused with a reason.

With the bindings switch off, the proofs accept 183 layouts (all the
pure-decor ones), 215 styles and 61 videos; 4 of 4 HUD archives pass whole,
and the 123 background layouts with handlers and the 14 versus layouts are
refused (`decor_event_handler` 123, `decor_script_include` 14). Against the
earlier image-wrapper rule: 5 of 325 layouts, 0 of 215 styles and 0 of 61
videos (`vcss_c` and `webm` were dropped as "file type not installed").

Other files in these archives: 54 `panorama/version.txt` (24 bytes, a `content_<hex>`
string, in the backgrounds). It is not an allowed type and would be dropped.
What the game does with that file is not known.

Mutation runs: 110,200 mutated copies of the real layouts, styles and small
videos, 3,000 mutated synthetic layouts, 5,000 mutated synthetic videos and
4,000 mutated styles were read without a panic, and every truncation of a valid
file is refused. The mutation test found a real defect in the existing KV3
reader (a corrupt count could size an allocation or overflow an offset; a
panic, reachable from a hostile layout, also in the image-wrapper proof; hero
skin archives are hash-pinned, which limited the exposure). It is fixed in the
same branch, with a test.

## Decisions and risks

1. **Handlers.** The decision that matters. The strict reading of the safety rule
   cannot install a main-menu background (the archives replace Valve's whole
   dashboard) or a versus screen. The proof therefore accepts an event attribute
   only as one of 79 exact bindings, each read by hand by the author of this
   change (not yet by the founder): every call is a Valve interface event or a
   function of a Valve script; none opens a URL, reaches the network, runs a
   console command or uses `$`. The residual risk is that the review was wrong,
   or that a Valve event misbehaves when bound to the same element on a
   modified layout. The list should be read by the founder before this is
   switched on; the switch is one constant at the wiring site.
2. **Replacing Valve's files with smaller ones.** The background archives replace
   `dashboard.vxml_c` (7,638 or 7,654 bytes; Valve's is 7,779), `dashboard_page_home.vxml_c`
   (1,613; Valve's 1,911), `dashboard_background_last_match.vcss_c` (2,436; Valve's 11,462)
   and `hero_loadout_background_images.vcss_c` (2,461; Valve's 93,715), and add
   `hero_loadout_background_images.vxml_c`, which Valve does not have. These
   archives were made for an earlier Dota patch or are trimmed; whether the
   dashboard and the loadout screen still look and work as they should after
   installing is not known and is the first thing the Windows test checks.
   Not a security finding.
3. **Scene names.** `map="..."` attributes name scenes (for example
   `backgrounds/dashboard_parallax_test`, `scenes/versus/versus_muerta_radiant`).
   They are plain text; the proofs do not check that the scene exists.
4. **Video decoding** is unproven (see `check_webm`).
5. **`sound` in styles** is accepted because it only names a sound event Valve
   defines; a name that does not exist plays nothing. Three audited values.
6. **Text shown to the player** is limited to a short character set but not
   checked for honesty.
7. **What was refused on purpose:** comments in styles (none occurs; a comment is
   where a differently parsing reader hides text), `file://` and `data:` anywhere,
   single-quoted strings other than animation names, `calc()` and every
   function not seen, the older layout container (XML text in `DATA`), a reference
   list on a layout, a video that replaces a Valve file or sits in a nested folder,
   and an unreferenced video.

## What is left (wiring)

The per-category type policy of the audited-archive pipeline has to accept the
proven files: `vxml_c`/`vcss_c` through `panorama_decor::prove_archive` with the
category's scope, `webm` only in `backgrounds`, the size bound as above; the
local generator then emits the allowlist entries for these categories, the
catalog shows them as installable with the Windows pilot mark and an honest "not
run in Dota" note, and the review text says what the item changes, what was
proven and what was refused, and how to undo (restore). That part is not done
because the generalisation it plugs into had not merged when this was written.
The Windows test plan for the three categories is in
[SKIN_WINDOWS_TEST_PLAN.md](SKIN_WINDOWS_TEST_PLAN.md), section 6.

## Not verified

- That any of these installs load or look right in Dota; that Dota reads a
  layout, style or video from a mod VPK at these paths.
- That the 79 bindings behave in the current Dota patch exactly as they do in
  Valve's own layouts.
- Anything on Windows.
