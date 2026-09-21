# Zen, transparent Glass and draft branches

Recovery checkpoint: 2026-09-21, based on published `bf67608af4f6174b6b79cee062805a7e5552e05c`.
Upstream reviewed: `e7cd15281e6d16cf8fc55a91496dcff035475e54`, unchanged.

## Material and interaction

Glass is desktop transparency plus a native compositor blur request, not an opaque
conversation rectangle over a decorative gradient. One continuous window tint is
shared by the transcript and existing Environment. Panel opacity represents target
total coverage. With a 70% base and an 82% panel target, the panel adds only 40%
additional tint, rather than multiplying two almost-opaque backgrounds.

The editor, terminal and Git review roots no longer paint opaque backing slabs.
Default terminal cells leave that material visible while explicit terminal colors,
selection and cursor remain independently legible. Permission and question panels
retain solid semantic backgrounds. Text is not faded with the whole window.

Use desktop glass removes the local wallpaper selection and requests native blur.
The separate local PNG/JPEG path validates and decodes on the background worker,
rejects animated PNG, bounds size/allocation and caches a downsampled blurred image.
The decoder semaphore stays with the blocking worker even if its caller cancels.
Desktop blur strength and availability remain OS/compositor dependent. This is not
a refractive Liquid Glass shader or a claim of per-element native backdrop blur.

Zen reveals the existing Environment tabs rather than replacing them. A narrow
window can show tools with an explicit Back to conversation action. Hidden tools
retain their processes and editor buffers but must not steal input focus. IME and
pending terminal paste review defer layout switching. Reduced motion is respected.

## Conversation branching

Branch from a completed assistant message creates a new unsent draft with quoted
user/assistant context and source provenance in the same workspace and task scope.
It does not clone a provider session, permission decisions, tool state or files.
Context is bounded to 256 messages and 1 MiB, and overflow is rejected rather than
silently truncated. Creation and draft storage use the existing atomic service.
Late creation responses do not clear a newly typed thread title.

## Reference boundary

Zeron is a UX reference for Zen only. Research concerns progressive disclosure,
composer ergonomics and keeping task attention reachable in a quiet layout. No
Zeron or MonoCode source, assets, tokens or distinctive screen composition is used.
Synara retains its own logo, icons, normal navigation and Environment ownership.
The supplied capture archives have 71 unique Zeron and 50 unique MonoCode image
contents. Curated/raw duplicates are not separate screens.

## Acceptance

This commit recovers prepared source rather than claiming native acceptance.
The published base was reconstructed locally and its complete source tree matched
`11a040268cfb6cd0148d8189a333fdf557fcd59f` before integration. The sandbox still lacks
Cargo/rustc and cannot resolve the pinned toolchain host. No current GPUI build,
compositor behavior, native focus journey or native screenshot is claimed.
No GitHub test workflow was dispatched and workflow configuration is unchanged.
Publication uses `[skip ci]` at the user's request.

I7/I10, D9 and the broader platform gates in ROADMAP.md remain open. Optional Hubs
are the next integration batch. Browser, multimedia attachments, Side chats,
Pull Requests, Automations and provider handoff are not implemented by this commit.
