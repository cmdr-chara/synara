# Electron-to-GPUI parity

## Reference and scope

The target is `Emanuele-web04/synara` at `eaa61eded31b6755d4f30ba8eabc5d905cf817cb`, the Electron `main` revision inspected for this request and the source of v0.9.1. The native starting point is `03fe420bd6379113e59ea64c574dd04254d36a3b` on `astra/gpui-clean-rewrite`. Implementation is isolated on `astra/electron-1to1`.

The supplied reference corpus contains 157 Electron captures and 51 native CI captures. Those are input counts, not implemented-feature or parity-pass counts. `reference-cases.tsv` inventories every Electron capture individually. The native captures are regression evidence for their own scenarios, not substitutes for matching Electron states.

**Overall 1:1 parity remains OPEN.** A successful native build does not prove feature completeness or visual equivalence.

## Implemented foundation

The native source now uses neutral Codex-style light/dark surfaces, blue accent defaults, a 13px default UI base size, 28px base navigation rows and the reference 0.85/1.0/1.15 density factors. Settings use 12px card radii, density-aware 10px vertical row padding, smaller text gaps, medium-weight labels and an explicit top inset rather than a negative offset. Shared actions and buttons read the configured UI size.

Existing explicitly saved font sizes and families are preserved. Custom colorways, custom accents, high-contrast preferences, provider choices, permission policies, stored conversations and native tool ownership remain unchanged. Existing installations retain their saved appearance until the user changes it or restores defaults. Platform font-family equivalence and exact dark overlay blending still need matched visual review.

Six new unit tests cover reference metrics, neutral theme roles, default typography, legacy explicit preferences and minimal version-1 settings. The isolated pre-publication verification run `35793477640` passed native library/application regressions, application/fixture compilation, native navigation and Chat Behavior journeys, and roadmap checks. The branch workflow additionally verifies the actual committed source without applying a code-generation patch.

## Acceptance gates

| Required outcome | State | Deciding evidence |
| --- | --- | --- |
| Shell, home, navigation, project and thread menus | OPEN | Same-state captures and keyboard/pointer journeys |
| Composer, agent/model/permission controls, active transcript and follow-ups | OPEN | Capability-aware interactions, persistence and captures |
| Project creation/import, environments and worktrees | OPEN | Filesystem behavior, reopen/recovery and captures |
| Kanban, pull requests and automations | OPEN | Native mutations, empty/error/populated states and captures |
| Terminal, browser, files, editor, changes and Git | OPEN | Actual native tools, lifecycle/recovery and captures |
| All Electron settings sections and controls | OPEN | Defaults, persistence, actual consumers, reset and captures |
| Themes, typography, density and responsive layout | OPEN | Matched content at the original capture size and intermediate widths |
| Onboarding and welcome flow | OPEN | Every step, skip/back/finish, restart and keyboard checks |
| Accessibility and supported platforms | OPEN | Native focus/activation and platform-specific evidence |

Computer control, Agent Gateway, external client access, provider-specific capabilities, and any other missing runtime must not be represented by an enabled cosmetic substitute. Native-only capabilities must remain discoverable during navigation changes. Use the existing feature-gap inventory and ROADMAP.md alongside this visual case list.

## Reproduction and evidence

`Native Electron parity verification` builds and tests the committed source with Rust 1.98.1 on Ubuntu 24.04. It retains logs, result JSON and actual native PNG captures as the `electron-parity-foundation-evidence` artifact. No production profile, credentials or real agent account is used by fixture journeys.

The assistant-side screenshot/container runtime stopped accepting commands with `ClientError` during implementation. GitHub compilation and native CI interaction remained available, but the new artifact could not be unpacked or visually inspected in that runtime. Therefore no new screenshot is marked visually accepted. This is a verification limitation, not a claim that GitHub publication is unavailable.

For every case, record the exact Electron and native revisions, theme, viewport, display scale, font configuration, project/thread fixture and scroll position. Keep behavioral and visual outcomes separate. Pair the supplied reference with a newly captured native state before changing OPEN to PASS. A missing pair, unsupported operation or uninspected screenshot cannot pass.

## Completion rules

- Do not use screenshot images as application surfaces.
- Do not substitute static inventories for working controls.
- Do not claim fixture-agent results prove authenticated providers or computer-use permissions.
- Do not alter release readiness based solely on this foundation batch.
- Keep upstream licenses and asset notices in place. Screenshot artifacts must not bundle font files.
