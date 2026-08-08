# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Users

BetterFy is for Dota 2 players who want a customized game but do not want to search through scattered Telegram channels, unfamiliar websites, and outdated packs. The primary user chooses a visual setup, expects the app to resolve the technical work, and wants to get into the game with minimal friction.

## Product Purpose

BetterFy is a desktop mod and skin manager for Dota 2. A user selects mods and skins, points BetterFy to the game, and launches. BetterFy is responsible for preparing a compatible profile, protecting original files, applying the selection, and starting the game.

## Positioning

BetterFy turns fragmented community content into one curated, understandable, and automated workflow. Its value is not only access to mods. It is confidence that a chosen build is coherent, reversible, and ready to play.

## Operating Context

- Windows is the primary release platform.
- Development and interface testing also happen on macOS.
- The app is built with Tauri, React, and TypeScript.
- Early Access authentication is presented through a BetterFy Telegram bot, with email access for invited testers.
- BetterFy has a curated catalog and allows users to import personal skins.
- The skin library includes a BetterFy-curated view and a web catalog based on the team's Dota2PornFxWeb project.

## Capabilities and Constraints

- Preserve the Home, Mods, Skin Library, Settings, and Profile workflows.
- Preserve Russian and English localization.
- Mod selection must communicate added, removed, conflicting, building, ready, and error states unambiguously.
- The current engine and authentication are prototypes; the interface must not imply that demo data is verified production behavior.
- Original game files must be described as protected only where the prototype already presents that intended capability.
- Avoid dependencies on Dota-owned artwork for core product identity. Dota-inspired assets should be transformed into a distinct BetterFy production language.
- The repository is intended to be open source.

## Brand Commitments

- Product name: BetterFy.
- The primary BetterFy logo is the text wordmark: `Days One` for “Better” and
  handwritten `Mrs Sheppards` for “Fy”. The three-capsule mark remains a
  secondary application/avatar symbol and must not replace the wordmark in
  boot, authentication, onboarding, or persistent shell branding.
- The brand is dark, premium, energetic, and precise without becoming a monochrome purple interface.
- Violet is the identity color, not the only interface color.
- The product voice is direct, friendly, author-led, and available in Russian and English.

## Evidence on Hand

- Working React/Tauri prototype with real catalog-shaped data and complete prototype flows.
- Existing BetterFy mark component in `src/BetterFyMark.tsx`.
- Existing catalog and localization in `src/catalog.ts` and `src/i18n.tsx`.
- Read-only Steam/Dota discovery and deterministic fixture BuildPlan commands
  in `src-tauri/src/main.rs`.
- A versioned fixture-package contract and immutable SHA-256 content store that
  verifies bytes before the fixture builder may consume them. Remote downloads,
  archives, and production Dota deployment remain disabled.
- A real local config manager backed by validated, atomic app-data storage,
  plus three built-in BetterFy Workshop presets resolved against the same
  Minify catalog IDs as the selection UI.
- Telegram code verification is isolated behind `src/auth.ts`; without a
  configured backend it remains explicitly simulated.
- Team web catalog: `https://h6rd.github.io/Dota2PornFxWeb/`.
- No production authentication, subscription, compatibility benchmark, or community activity data is available yet. Future surfaces must not fabricate these claims.

## Product Principles

1. The next useful action is obvious within seconds.
2. Selection and system state are never communicated by color alone.
3. Technical complexity stays under the hood while recovery and safety remain visible.
4. BetterFy feels authored and collectible, but familiar controls stay familiar.
5. Every visual flourish must support identity, hierarchy, or feedback.

## Accessibility & Inclusion

Keyboard focus, readable contrast, reduced-motion behavior, and non-color state indicators are baseline requirements. The interface must remain usable at the configured Tauri minimum window size.
