# BetterFy experience constitution

This document governs how BetterFy guides a person through the application. It
is binding for product structure, UX, visual design, motion, copy, and frontend
prototypes. `PRODUCT.md` owns product truth; `DESIGN.md` owns the visual system.

## Product experience

BetterFy turns a technically risky, fragmented Dota 2 customization process into
one calm sequence:

```text
Enter → connect Dota → discover → preview → choose → resolve → review
→ build → verify → play → recover or rebuild
```

The interface must make the current state, the next useful action, and the
consequence of that action understandable within seconds.

## Interaction principles

1. Show one primary task per state.
2. Keep technical complexity available but collapsed until it affects a choice.
3. Place guidance where the decision happens; avoid manuals embedded in Home.
4. Preserve context between routes: active build, selected variants, filters,
   scroll intent, and unresolved decisions.
5. Explain blocking problems before the final build step.
6. Make reversible actions feel light and destructive actions deliberate.
7. Never represent prototype behavior as verified engine behavior.
8. Never claim VAC safety, ban immunity, compatibility, backup, or recovery
   without evidence from the corresponding implementation.

## Navigation model

Use a stable application shell with five product destinations:

- Home: current state and the next useful action.
- Discover: mods, skins, collections, and search.
- Build: selected content, dependencies, conflicts, and readiness.
- Library: local imports, saved builds, downloads, and installed content.
- Settings/Profile: application, account, language, accessibility, diagnostics,
  and advanced controls.

Do not split closely related tasks into extra top-level destinations. Contextual
subnavigation belongs inside a destination. The shell stays visually quiet and
does not compete with the current content.

Community is woven through relevant destinations instead of becoming another
top-level route. It may support onboarding, editorial discovery, source
attribution, feedback, and help, but it always follows the current primary task.
Only verified channels and evidence-backed activity may be shown.

## Route behavior

- First launch goes directly to the missing prerequisite: access, Dota
  connection, or first selection.
- Returning users land on Home with their active build and current Dota status.
- A catalog selection updates the persistent build summary immediately.
- Opening Build never discards the catalog position or filters.
- A conflict interrupts only the affected choice, not the whole application.
- Build progress replaces editing actions with progress, safe cancellation, and
  recovery state.
- Success leads to Play. Failure leads to one recommended recovery action plus
  optional details.

## Catalog journey

The catalog is not a file browser and selection is not a shopping cart.
Users compose a Dota appearance and behavior profile.

1. Discover through search, category, hero, content type, collection, or source.
2. Preview the actual visual or audible result before technical metadata.
3. Select a variant before adding it to the active build.
4. Show source, author, trust, last verification, compatibility, size, and
   affected content close to the action.
5. Detect replacement, dependency, and conflict consequences immediately.
6. Animate accepted selection toward the build summary to preserve causality.
7. Keep selected, installed, outdated, local, incompatible, and unverified
   states explicit in text and iconography.
8. Let users compare or replace content in the same hero/resource slot without
   forcing manual removal first.
9. Keep search and categories reachable while results scroll. Over the first
   180px the shell rail yields; after it clears, filters reflow into the released
   width. Hover or keyboard focus may restore the rail as a temporary overlay.

## Motion model

Every animation must serve one of four purposes:

- Orientation: show where the user arrived and what remained stable.
- Causality: connect an action with the object or state it changed.
- Continuity: preserve identity across selection, detail, build, and result.
- Feedback: acknowledge input, progress, success, warning, or failure.

Use at most one dominant motion event per transition. Avoid simultaneous card
entrances, endless floating, large parallax, cursor-following decoration, and
blur-heavy animation. Motion must remain coherent at the configured minimum
window size and have a reduced-motion equivalent.

Route and panel changes include both a restrained outgoing state and an
incoming state; abrupt unmounts and one-frame flashes are defects. Theme changes
recolor the live interface rather than stacking captured frames. The reduced
motion mode removes ambient loops and collapses transitions to an immediate,
stable state.

Loading motion must be finite and composited: no endless marquee, moving blur,
or looping progress sweep. A successful asynchronous state remains visibly
settled for at least 900ms before navigation so confirmation can be perceived.

## Theme model

Dark is the default BetterFy atmosphere. Light mode is a deliberate ivory/plum
composition with violet continuity, tinted shadows, readable dark typography,
and preserved cinematic character scenes. It is not a color inversion and must
not become a generic white dashboard. Both themes share spacing, hierarchy,
interaction states, focus treatment, and semantic status colors. Theme choice
persists safely and applies to loading, access, setup, workspace, dialogs, and
recovery.

## Screen review

Before accepting a surface, answer:

- What is the user trying to do now?
- What is the only primary action?
- What information can be removed or delayed?
- Does the screen preserve the previous choice and imply the next state?
- Are loading, empty, overflow, offline, error, conflict, success, and keyboard
  states designed?
- Does Russian text fit with realistic expansion?
- Does motion explain a relationship or merely decorate it?
- Could the same screen belong to a generic launcher? If yes, strengthen the
  BetterFy/Dota content relationship without adding noise.
- Are all character and game assets sourced and legally reviewable?
- Does any visible background, illustration, or component skin come from an
  older product direction? If yes, reject the screen unless that exact asset was
  explicitly re-approved.

## Reference interpretation: July 30 mood sketches

Keep:

- deep near-black and violet atmosphere;
- bold, readable focal composition;
- selective translucent depth;
- recognizable Dota character imagery;
- softness, glow, and fluid transitions;
- a simple primary action.

Reject:

- the supplied Home layout;
- terminal/log-as-interface;
- circular KPI gauges;
- multicolor status clutter;
- nested glass panels;
- instructional paragraphs replacing navigation;
- email/password assumptions;
- decorative circles as a reusable motif;
- any `VAC-safe` or no-ban promise.

Future references update this interpretation only after an explicit review:
what to keep, what to reject, and how to translate it into BetterFy rather than
copying the source.
