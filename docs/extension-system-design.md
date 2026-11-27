# Extension system design (pre-implementation)

Status: design only, gated behind the Rust port completion. Nothing here is a
commitment to ship dates; it is the shape implementation should follow so the
ecosystem does not ossify around accidental behavior.

## What exists today

Skills are folders on disk: Synara scans `~/.synara/skills` plus every
provider-native skills folder and catalogs what it finds. A skill is prompt
content and supporting files a thread can pull in. There is no distribution,
no versioning, no trust model and no lifecycle — a skill exists because
something, usually the user by hand, put files in the right place.

## The gap

Everything valuable about a working setup is currently unshareable. A user
who tunes a workflow cannot hand it to a teammate without copying directories
by hand; a team cannot pin what their automations depend on; a broken update
cannot be rolled back because nothing has versions. Meanwhile the surface
area that wants packaging is wider than skills: prompt templates, themes,
connector presets, and eventually executable extensions.

## Design: packages, not files

The unit of distribution is a **package** — a directory with a manifest at
its root. A package carries a name, a version, a human description, and one
or more **contributions**, each typed: `skill`, `prompt-template`, `theme`,
`connector-preset`. (Executable extensions — code that runs inside Synara —
are explicitly out of scope for the first implementation; see trust model.)

Packages install from two source kinds:

- **git** — a URL plus optional ref; the package is the repository or a
  subdirectory of it. Pinning a commit is the normal way to pin a version.
- **local path** — for development and for teams that vendor their tools.

Installed packages live under the Synara home in content-addressed storage
(`packages/<name>/<version-or-commit>/`), so two versions of the same package
can coexist and rollback is a pointer change, never a file repair. The active
set is a small durable record: name, source, pinned version, enabled flag.

## Resolution rules

- A contribution type plus its name forms the lookup key. Conflicts between
  two installed packages are errors at install time, never silent precedence
  at read time — silent precedence is how configurations become unexplainable.
- Disabling a package hides all of its contributions without deleting
  anything. Uninstalling removes only versions nothing references.
- Provider-native skill folders keep working exactly as today: they are the
  compatibility layer, not the target. Packages are how content arrives;
  provider folders are how providers consume it.

## Trust model

Packages are content, not code. Installing a package must never execute
anything: no install scripts, no postinstall hooks, no arbitrary binary
artifacts activated by presence alone. This is the same boundary Synara
already applies to repository contents — untrusted data does not authorize
actions. Executable extensions, if they ever arrive, need a separate design
with explicit user approval per capability, and they are not smuggled in
through the content-package path.

Updates are explicit: the user (or an automation acting visibly on their
behalf) asks for a package to move to a newer ref, and the change is recorded
so a regression has an obvious revert. Nothing self-updates.

## What this is deliberately not

- Not a plugin API surface for running third-party code inside the server.
- Not a package manager with dependency resolution: packages are
  self-contained; if two need shared content, that content is its own
  package.
- Not a store. A registry index may come later as a curated list of git
  sources, but installation never requires a central service.
