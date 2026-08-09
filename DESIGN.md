# BetterFy design system

This document is the durable visual authority for BetterFy. Read
`docs/EXPERIENCE_CONSTITUTION.md` before designing a surface or flow.

## Direction contract

- Thesis: BetterFy is a focused, cinematic Dota customization space, not a
  launcher dashboard, terminal, or generic glass UI.
- Own world: a dark near-black field, restrained violet light, one expressive
  Dota character or mod result as the scene anchor, precise controls, and a
  single readiness signal.
- Story: select, check, build, play.
- First viewport: one dominant product state, one emotional visual anchor, and
  one unmistakable next action. Do not inherit the supplied home-screen layout.
- Form: ink-black depth, cool white type, committed violet identity, selective
  translucent layers, lime only for verified readiness, and coral only for risk.
- Reference status: the supplied July 30 screenshots define mood, not layout,
  typography, claims, or component structure.

## Product hierarchy

1. Show whether Dota 2 and the current build are ready.
2. Make the next action obvious.
3. Keep selected and unselected content unmistakable.
4. Explain safety and conflicts where the decision is made.
5. Put community content after the launch path, never ahead of it.
6. Keep one primary action per state; secondary actions must not compete with it.

## Visual language

- Simplicity is the governing rule: fewer regions, stronger hierarchy, less
  explanatory chrome.
- Surfaces are softly machined rather than bubbly: 8–20 px radii, restrained
  borders, soft depth, and no stack of nested glass cards.
- Manrope carries product copy. JetBrains Mono carries states, IDs and technical labels.
- The BetterFy wordmark uses `Days One` for “Better” and `Mrs Sheppards` for
  “Fy”. Keep “Fy” in its handwritten form, magenta-violet, slightly lifted and
  optically joined to “Better”; never typeset the whole name in the body font.
- The wordmark is the primary logo on boot, authentication, onboarding and the
  persistent top bar. The three-capsule mark is secondary and must not occupy a
  competing corner in the main shell.
- Violet is the brand atmosphere and may own large fields. It is not a status
  color. Lime means verified ready, selected, or confirmed. Coral means
  conflict, failure, or destructive action. Telegram blue is reserved for the
  Telegram action.
- Each major surface may feature one character, cosmetic, or environment image
  when it clarifies the content or gives the screen an emotional anchor. Dota
  imagery is content, never BetterFy brand identity; record its source and
  redistribution status.
- Character cutouts are compositional layers, not thumbnails. A primary hero
  occupies roughly 60–90% of scene height and companions 45–70%. Intentional
  crop, overlap, shadow, and bounded light are preferred to timid scaling.
- Large scenes use authored raster art. Controls, text, marks and state graphics
  stay semantic HTML, CSS, and SVG.
- Repository history is not an asset library. Legacy and unknown raster assets
  are prohibited unless the founder explicitly re-approves them for the current
  direction. If approved art is missing, show a quiet labeled replacement slot.
- Background imagery must be reviewed before component styling because it
  defines the perceived product world. A screen cannot be accepted when its
  background belongs to an older BetterFy direction.
- The BetterFy mark is three equal forward-leaning glass capsules with a dark
  violet core, luminous purple rim, and restrained internal reflection. Do not
  change their count, relative size, spacing, or lean between surfaces.
- Avoid terminal panels, circular KPI gauges, multicolor status dashboards,
  generic SaaS card grids, neon outlines on every element, and decorative
  background circles copied from the references.

## Product voice

- Write like the people building BetterFy, not like a SaaS landing-page
  generator. The voice is direct, informed, slightly sharp, and grounded in
  Dota, builds, files, conflicts, installation, and recovery.
- A headline must name a player action or a verifiable product fact. A body
  paragraph must explain what BetterFy does, what it does not do yet, or what
  happens after the user acts.
- Prefer concrete nouns and verbs: select, check, replace, install, restore,
  file, package, build, Dota, Windows. Delete copy that remains valid after
  replacing BetterFy with an unrelated product name.
- Do not use synthetic contrast slogans or vague atmosphere as product copy:
  no "noise versus clarity", "calm workspace", "no fog", "seamless journey",
  "built together", "next level", or similar AI/SaaS filler in either Russian
  or English.
- Keep labels short. Put implementation detail in supporting text, and never
  make unsupported safety, release, authentication, or patching claims for the
  sake of a stronger sentence.

## Motion grammar

- Motion expresses causality and route continuity, never decoration.
- Direct feedback: 90–160 ms. Component transitions: 160–240 ms. Route or scene
  transitions: 240–420 ms. Use `cubic-bezier(.23, 1, .32, 1)` for settling and a
  restrained ease-in-out for reversible movement.
- Entrances move 4–16 px. Related objects travel from their source where
  possible: a selected catalog item resolves into the build tray rather than
  disappearing and reappearing.
- A route transition keeps the persistent shell stable, moves the departing
  content first, and reveals the new task around its primary action.
- Ambient motion is limited to one scene layer, scanning/verification, the
  BetterFy mark, or a readiness signal. Never animate every card.
- The boot wordmark uses one coordinated entrance: “Better” fades/reveals into
  place, then handwritten “Fy” is uncovered left-to-right as a restrained
  simulated line-drawing gesture. It plays once and resolves before navigation.
- An approved Dota hero used as the scene anchor breathes at 0.8–1.5% scale over
  5–8 seconds while one bounded light pass travels across it. This is the
  canonical hero treatment on every surface; nearby UI remains still and
  reduced motion disables both loops.
- Character scenes also carry one slow backlight layer between the background
  and the cutout. It may drift and pulse over 6–9 seconds but must remain
  bounded by the scene and must not reduce text contrast.
- Repeated motion uses transform and opacity. Expensive blur is static or
  bounded to a small layer.
- Every motion path respects `prefers-reduced-motion` and the in-app motion
  switch. Reduced motion preserves state comprehension with crossfades and
  immediate spatial updates.
- A press scales to `.97`; hover behavior is only enabled for fine pointers. Primary actions lift by 2 px, run a restrained light sweep and move their directional arrow by 3 px.

## Canonical tokens

These values are the approved baseline. Change them only through an explicit
design-system decision.

| Role | Token | Value |
| --- | --- | --- |
| App background | `--bg` | `#07070B` |
| Primary surface | `--surface` | `#0D0D13` |
| Raised surface | `--surface-high` | `#14131D` |
| Primary text | `--text` | `#F7F5FB` |
| Secondary text | `--muted` | `#8F8C9B` |
| Quiet text | `--quiet` | `#5D5A67` |
| Brand violet | `--violet` | `#A84DFF` |
| Violet highlight | `--violet-hot` | `#D157FF` |
| Violet depth | `--violet-deep` | `#5F1CB3` |
| Telegram action | `--telegram` | `#229ED9` |
| Verified ready | `--ready` | `#A8F04B` |
| Attention | `--warning` | `#FFB84D` |
| Failure/destructive | `--danger` | `#FF6E70` |
| Hairline | `--line` | `rgba(255,255,255,.09)` |
| Strong hairline | `--line-high` | `rgba(255,255,255,.16)` |

- Brand action gradient: `#9C37E8 → #CE57F4`, 115 degrees.
- Telegram gradient: `#168DC4 → #2AABDF`, 135 degrees.
- Primary violet glow: `rgba(143,36,216,.19)`, 22/55 px.
- Surface radii: 10 px controls, 14–17 px actions, 20 px grouped panels,
  27 px image-led hero surfaces.
- Direct feedback: 90–160 ms. Components: 160–240 ms. Panels/routes:
  240–420 ms. Long ambient loops: 5–26 seconds.
- Headline tracking: `-.055em` to `-.085em`, but English multiline headlines
  use at least `.94` line-height to prevent collisions.
- Large task headings use one restrained white-to-violet gradient on the final
  semantic word. Do not gradient entire paragraphs, controls, or small card
  titles.
- Canonical vertical rhythm is 18–20 px from eyebrow to heading, 16–20 px from
  heading to description, and 24–28 px from description to the primary action.
  Do not let explanatory copy touch an action surface.

## Composition rules

- Auth is a calm two-part composition: identity/atmosphere and one compact
  access task. Telegram code is primary; do not show email unless the product
  decision enables it.
- Dota connection uses the BetterFy text wordmark without the three-capsule
  mark. Its verification graphic is directional and linear; circular scanner
  diagrams and orbit controls are prohibited.
- Home is designed from the user’s next decision, not from the supplied
  dashboard sketch. It must not become a grid of metrics, guide text, and logs.
- Catalog is image-led but task-oriented: discovery, preview, variant choice,
  compatibility, and adding to the active build.
- Build review compresses the selection into a readable sequence and surfaces
  only decisions that block progress.
- Progress is a continuous story with current operation, overall progress,
  safe cancellation state, and recovery information; do not imitate a terminal.
- Success visually releases tension and makes Play the sole primary action.

## User guidance

- Lead through progressive disclosure, contextual actions, preserved selection,
  and visible state—not through instruction panels.
- Never tell users to visit another tab and come back. Offer the relevant action
  or deep-link at the point of need.
- Preserve the active build across catalog navigation and surface its summary
  without obstructing browsing.
- Resolve conflicts at selection time with a clear consequence and two honest
  choices.
- Do not use color alone for selection, compatibility, progress, or failure.

## Community presence

- Community is a recurring authored thread, not a floating advertisement:
  Early Access at sign-in, contextual setup help, a Home editorial, community
  sources in Library, and support in Profile.
- Community promotion always follows the screen's primary product action and
  never competes with game connection, build review, recovery, or Play.
- Link only to verified BetterFy destinations. The current canonical contact is
  `https://t.me/BeterFyBot`.
- Never fabricate member counts, activity, ratings, testimonials, drops, or
  release cadence. Empty and upcoming community surfaces must say so directly.
- Community content must retain author, source, permission, and verification
  status before it can be presented as installable content.

## Asset inventory

| Asset | Role | Medium |
| --- | --- | --- |
| `src/BetterFyMark.tsx` | Brand and readiness signal | Semantic SVG |
| `src/BetterFyWordmark.tsx` | Canonical Better/Fy typography | Semantic HTML + vendored fonts |
| `src/assets/dota-spirits-setup.png` | Dota connection companions | Founder-approved prototype screenshot; temporary and prohibited from release |
| `src/assets/witch-doctor-auth.png` | Authentication scene anchor | Founder-approved prototype screenshot; temporary and prohibited from release |
| `src/assets/bane-home.png` | Home scene anchor | Founder-approved prototype screenshot; temporary and prohibited from release |
| `src/assets/wukong-build.png` | Build review scene anchor | Founder-approved prototype screenshot; temporary and prohibited from release |
| `src/assets/enigma-progress.webp` | Build progress environment | Founder-approved prototype screenshot; temporary and prohibited from release |
| `src/assets/courier-success.png` | Success → Play scene anchor | Founder-approved 1200×1200 prototype screenshot; temporary and prohibited from release |
| `src/assets/pudge-recovery.png` | Recovery / Restore scene anchor | Founder-approved prototype screenshot; temporary and prohibited from release |
| `src/assets/minify/*.jpg` | Functional-mod previews on the Minify catalog page | Imported from the official Egezenn/dota2-minify snapshot by founder request; prototype-only pending Valve/redistribution review |
| `src-tauri/icons/icon.svg` | Temporary application-icon master | Black BetterFy wordmark placeholder; replace with the final brand icon before release |
| Lucide icons | Controls and status cues | Existing icon library |

Legacy files under `src/assets/workshop-*.png` are not approved for current
surfaces and must not be rendered without explicit founder re-approval.
The retired generated Faceless Void concept was removed and must not be
reintroduced.

### Founder-approved incoming asset map — July 30, batch 2

These files are approved for the named prototype direction. The founder created
them from screenshots and explicitly marked them as temporary: none may ship in
a release. Replace every Dota-derived raster before release.

| Incoming file | Reserved role | Composition contract |
| --- | --- | --- |
| BetterFy three-capsule render supplied in the previous message | Account avatar and final brand-render reference | Use the existing scalable `BetterFyMark` in interface chrome; raster render is a material reference, never a repeated background ornament |
| `Dota 2 Sun Wukong Wallpaper.png` | Build scene anchor, imported as `src/assets/wukong-build.png` | Large layered cutout with breathing, bounded backlight and light pass; do not combine it with a competing full-scene character background |
| `Dota 2 Courier PNG.png` | Success → Play scene anchor, imported as `src/assets/courier-success.png` | The courier carries the visual release after verification; Play remains the only primary action |
| `Dota 2 PNG.png` | Recovery and Restore scene anchor, imported as `src/assets/pudge-recovery.png` | Pudge may overlap the recovery composition and hook may lead toward the restore action; danger color remains semantic and restrained |
| `1img.webp` | Build progress environment, imported as `src/assets/enigma-progress.webp` | Strong Enigma/black-hole scene; reserved for this single immersive moment and never used as the default app background |
| `4img.avif` | Candidate bounded violet scene texture | May frame a quiet empty or transition state; never repeat across routes or place behind dense copy |
| `3img.jpg` | Candidate Library/saved-build editorial art | Use only as a bounded image-led surface after contrast and source review |
| `2img.jpg` | Accent-only atmospheric crop | Resolution and aspect ratio are insufficient for a full desktop background |
| `5img.jpg` | Accent-only recovery/environment crop | Resolution is insufficient for a full desktop background; reject if scaling artifacts are visible |
| `Dota 2 Item PNG (1).png` | Optional item-effect/editorial accent | Not a navigation icon and not a generic decoration |
| `Dota 2 Onibi HUD 2016.png` | Catalog preview candidate | Preserve as content preview, not a global scene anchor |
| `Dota 2 International 2016.png` | Catalog preview candidate | Preserve as content preview, not a global scene anchor |
| `Dota 2 Juggernaut PNG.png` | Catalog preview candidate | Preserve as content preview; do not duplicate the same subject on nearby surfaces |
| `Dota 2 Sly Cooper Icon.png` | Catalog preview candidate | Preserve as content preview, not an application icon |

Do not place multiple approved assets merely because they are available.
Choose one visual anchor per major state, establish its silhouette and focal
light first, then position copy and the primary action. Validate the complete
viewport at desktop and minimum-window sizes before accepting scale or crop.

The current imported Minify/Dota2PornFxWeb snapshot is
`src/webCatalog.json`: 1,135 real catalog records across 39 categories. Its
record names, groups, authors, source links, archive names and preview paths
are the catalog truth. Preview binaries must be sourced from that repository
and retain provenance; invented replacement cards or lookalike previews are
prohibited.

## Interaction rules

- Selected mods have a lime edge, tinted row, check icon and explicit selected label.
- The catalog has two explicit product layers: functional Minify mods and the
  visual Wardrobe. Never mix a skin with an optimization mod in one result set.
- Minify uses seven stable task filters: all, optimization, map/world, HUD/menu,
  audio, customization and utilities. Category counts come from the imported
  source snapshot and must never be hand-authored.
- Wardrobe uses seven broad navigation groups and a horizontally scrollable
  second level of exact upstream categories. Only records whose upstream type is
  `mod` are shown as wardrobe items; guides, sites and packs remain separate
  content types.
- A catalog card must expose category, name, a one-purpose description,
  provenance/author, artifact or evidence summary and a text-labelled selection
  action without relying on hover.
- “Selected only” is available in both catalog layers, and empty filtered states
  always provide one control that resets every active filter.
- Skin selection is reflected in a full-width fitting-room rail that stays in document flow.
- A conflict blocks the change until the user keeps the current mod or confirms replacement.
- Settings navigation changes the visible settings group instead of acting as decoration.
- Russian and English use the same hierarchy and update immediately.
