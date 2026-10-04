# Piece: onboarding (First-run onboarding)

## Scope

The first-run flow on a fresh profile: welcome modal and every step (Get started, Skip setup, step dots, back/next), provider step, project step, finish, and what the app shows after finishing or skipping.

Out of scope: palette tokens (`chrome`), rail/sidebar (`rail-sidebar`), composer (`home-composer`),
settings pages. Consume shared building blocks; do not restyle them here.

## Upstream source of truth (under `~/parity/upstream/apps/web/src/`)

onboarding/*, components/Onboarding*, components/BetaWelcomeDialog.tsx, components/AnnouncementSheet.tsx, components/AppSnapWelcomeDialog.tsx, components/WhatsNewDialog.tsx

## Port files (likely, under `crates/synara-app/src/`)

shell/onboarding*.rs, shell/onboarding/*.rs

## Required states (start from a fresh seeded launch unless stated)

1. `welcome` — fresh profile WITHOUT the seed onboarding-dismiss (use the scenario to launch with an empty profile), first screen
2. `step-2` — after pressing the primary button once
3. `step-3` — after pressing it again
4. `after-skip` — fresh profile, press Skip setup

If upstream cannot reach a state with the seeded data (for example it needs a GitHub login), capture
what upstream really shows there (its empty / signed-out state) and make the port show the same.

Judge region: whole window.

## Port-only things to delete (in scope)

Anything in this area that upstream does not have: extra panels, tabs, buttons, menu rows, settings,
commands. Delete the code, settings keys and tests behind them, and list them in builder.md.
