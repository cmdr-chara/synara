# Native UI reference register

Status: evidence preparation, not a visual-parity claim.

The product reference is `Emanuele-web04/synara` at
`33333439c4b9c74d0097bc01196cccc921f67cf3` (main inspected September 18, 2026).
The implementation target is `cmdr-chara/synara`, exclusively
`astra/gpui-clean-rewrite`, retaining root
`43b1fb89bf19dadc388d18008f9ceb21b8215716`.

## Visual evidence

| Reference | Product surface | Evidence limitations |
| --- | --- | --- |
| `assets/prod/readme-split-view-dark.png` | Project/thread sidebar, conversation, composer and contextual split surface | A README image is one captured state, not proof of every current interaction. Device content is reference only, not native device support. |
| `assets/prod/readme-appearance-dark.png` | Settings hierarchy, typography, density and dark surface relationships | Exact viewport, scale, font and active preference values must be read from the capture, not guessed. |

`.github/workflows/ui-reference.yml` downloads these two images at the immutable
reference revision into a runner temporary directory. The
`native-ui-reference-inputs` artifact records original pixel dimensions, SHA-256,
source URLs and the exact native candidate, alongside its native-only Git bundle.
Reference images are not shipped, embedded in the application, committed as UI
assets or used as substitutes for real native surfaces. No web frontend source is
imported. Images are evidence of the product, not a grant to reuse their assets.

Native comparison captures must name their window dimensions, platform/display,
scale and exercised state. Record differences and subsequent corrections. Do not
claim visual equivalence from source inspection or a passing build.

## Architecture and edit boundary

The existing `synara-app` consumes `Controller`, `WorkspaceService`, normalized
`Thread` events and owned native terminal services. Domain state, ACP, SQLite,
process lifetime, filesystem containment, Git execution, registry trust and SSH
remain in their existing crates. UI work belongs in presentation modules and must
not duplicate these services. `shell.rs`, `shell/conversation.rs`, the input view,
virtual transcript and coordinate-based native smoke tests are coupled UI edit
surfaces. Preserve their safety and scroll-ownership behavior while changing layout.

The native tree has no repository-local `AGENTS.md` or Codex Toolkit installation
at the inspected baseline. The applicable Toolkit instructions were read from
`cmdr-chara/codex-toolkit`:

- `skills/repository-intelligence/SKILL.md`
- `skills/screenshot-to-interface/SKILL.md`
- `skills/verification-and-release/SKILL.md`

The reference repository's `AGENTS.md` is product guidance only. Its web build
commands and web implementation structure are not imported into the Rust rewrite.

## Initial verification constraints

The authoring container initially has no Rust compiler and cannot resolve public
repository hosts directly. Authenticated repository reads/publication use the
GitHub connector. Existing read-only CI toolchain export can supply pinned public
compiler/dependency inputs. The reference workflow also exports native development
headers/libraries outside the repository for isolated local verification. Neither
resource export proves that the UI compiles, runs or matches the reference.

No major native UI surface is accepted by this preparation checkpoint. The final
parity inventory and candidate-specific evidence must distinguish implemented,
partial, backend-blocked, platform-different and missing behavior.
