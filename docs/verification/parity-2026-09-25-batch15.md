# Parity batch 15: search policy, onboarding guides and voice feedback

## S14 partial: project-search generated policy

Native project file-name and content search now matches the pinned upstream
static generated/infrastructure directory set exactly:
`.git`, `.convex`, `node_modules`, `.next`, `.turbo`, `dist`, `build`,
`out` and `.cache`.

The previous native-only file exclusions are removed. Files such as minified
JavaScript/CSS, source maps and compiled extensions, plus files under `target`,
`coverage` or virtual-environment directories, remain searchable unless an
ignore rule excludes them. The same `WorkspaceFs` implementation backs current
local and remote helper searches.

S14 is complete. Git worktrees now build the searchable index from bounded,
hardened `git ls-files --cached --others --exclude-standard -z` output and apply
chunked `git check-ignore --no-index -z --stdin`, matching the pinned upstream
owner. Git resolution excludes executables inside the workspace, commands are
time/output bounded, prompts and credential interaction are disabled, and no
shell is involved. Non-Git folders intentionally apply only the upstream static
generated-directory set. The same project-search owner is used by local search
and the SSH helper.

Regression coverage distinguishes non-Git behavior from Git standard-ignore
behavior and covers the shared helper path.

## S01 partial: provider-specific onboarding

Provider login guidance now recognizes Codex and Claude Code in addition to the
existing OpenCode and Gemini CLI guides. Copyable commands are offered only when
the configured executable matches the known provider binary; wrappers get
guidance without an invented command. S01 remains open for deeper provider-native
setup state and sign-in lifecycle.

## S02 partial: recording feedback

The composer now shows bounded recording duration and a five-level input meter
while native voice capture is active. The display stops at the existing two-minute
recording cap and exposes presentation state only; it does not change recording,
transcription, sending or provider authority. S02 remains open for broader voice
interaction controls and live provider/platform acceptance.

## Validation

Final verification is recorded against the published batch candidate. The
workspace-wide native regression still has the pre-existing `synara-server`
failure where a failed execution response omits null `route`/`remote` fields.
This batch does not modify `synara-server`.
