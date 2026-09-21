# Agent compatibility evidence

A successful fixture is not a verified vendor service. A successful no-credentials
protocol probe is not an authenticated coding session. Keep these evidence levels
separate when deciding whether a particular release is supported.

## Evidence levels

| Level | What it proves | What it does not prove |
| --- | --- | --- |
| Fixture integration | Synara state, transport, persistence and interaction contracts against controlled agents | Vendor compatibility |
| Reviewed release installation | Exact approved archive downloads, passes its digest and installs through Synara's registry service | Agent protocol behavior |
| No-credentials ACP probe | Vendor executable initialization, advertised capabilities, session creation or explicit auth-required response, disconnect | Model calls, subscription login, tools, authenticated restoration |
| Authenticated end-to-end | Real prompts, streaming, tools, consent, cancel and restart with authorized credentials | Other versions, platforms or every optional capability |

## Verified no-credentials results

Candidate: `a02d6b16468782e7b9d7be9158d3afc80d5e26c1`.
[CI run 35246447427](https://github.com/cmdr-chara/synara/actions/runs/35246447427),
job `105287756520`, completed successfully on September 17, 2026.
Environment: Ubuntu 24.04.5 x64, Rust 1.98.1, Node 22.23.2 for the external
JavaScript agent. The structured `vendor-acp-probe` report was retained as
artifact `10508335643`. The job log also contains the redacted report.

| Release | Installation | ACP identity | Initialize | New session | Disconnect |
| --- | --- | --- | --- | --- | --- |
| OpenCode 1.18.31 | Archive digest and receipt verified | OpenCode / 1.18.31 | Passed | Created, two configuration options returned | Passed |
| Gemini CLI 0.60.0 | Archive digest and receipt verified | gemini-cli / 0.60.0 | Passed | Explicit authentication-required response | Passed |

Both used Synara's existing registry installer and the same generic `AcpBackend`.
The recorded methods were `initialize` and `session/new`. Neither agent received
provider credentials, an authentication call or a prompt. Gemini's successful
auth-boundary detection is not a successful authenticated session.

OpenCode advertised load, resume, close and list sessions. Gemini advertised load
sessions but not resume, close or list. These are capability-discovery observations,
not evidence that all those operations have been exercised with either vendor.
The reports also retain the other normalized capability flags. The initial probe
recorded matching identities. Subsequent probe code additionally enforces an
identity/version match and embeds the CI candidate SHA in the artifact itself.
Each later candidate must still pass its own applicable checks.

The two ordinary probe tests (manifest validation and diagnostic redaction) passed.
The explicit network/executable test passed with both releases. The initial
candidate had an unrelated Rust formatting failure in this new test source, so the
vendor result alone must not be used as a green full-workspace acceptance result.

## Opt-in release probe

`crates/synara-acp/tests/fixtures/vendor_releases.json` is an explicitly reviewed
test manifest, not a downloaded official registry index or an application-wide
list of preferred agents. It pins vendor release URLs and SHA-256 digests observed
in official GitHub release metadata. Updating it is a deliberate code review.

The probe currently targets OpenCode 1.18.31 and Gemini CLI 0.60.0 on Linux x64.
The latter uses its official JavaScript bundle and the host's Node runtime, which
remains an external agent runtime rather than part of Synara's application core.

The releases are installed by the existing `RegistryStore` and `HttpsDownloader`.
Receipts and executable hashes are revalidated before obtaining `AgentSpec`.
Both releases then use the same `AcpBackend` and normal connection/session APIs.
There is no separate vendor-specific runtime or alternate raw JSON-RPC client.

Each agent receives an empty temporary project and isolated HOME/XDG directories.
Inherited SSH-agent, session-bus and display connections are cleared for the child.
No provider credentials are supplied, `authenticate` is never called, prompts are
never sent and permissions/questions are denied. The probe may contact public
vendor metadata endpoints during agent startup. This is configuration isolation
on a disposable runner, not an operating-system sandbox for untrusted binaries.

The report stores public identity/capabilities, operation states and error classes.
It omits raw vendor error text, authentication URLs, session IDs and conversation
payloads. A normal auth-required response is a successful detection of that
boundary, not a successful authenticated session. Unexpected protocol failures,
installation failures, timeouts and failed disconnects fail the probe.

## Run

Only launch downloaded agents in a disposable environment you control. On Linux
x64 with Node 20 or newer and the pinned Rust toolchain:

```sh
report_dir=$(mktemp -d)
SYNARA_VENDOR_PROBE=approved-release-check \
SYNARA_VENDOR_REPORT="$report_dir/results.json" \
  cargo test --locked -p synara-acp --test vendor_probe -- --ignored --nocapture --test-threads=1
```

The output file must not already exist. Ordinary `cargo test` validates the
manifest and redaction logic but leaves the network/executable probe ignored.
The `Reviewed vendor ACP probe` workflow explicitly runs it and retains only the
structured `vendor-acp-probe` report artifact, not downloaded programs, home
directories or keys. The workflow has no provider secrets and no repository-write
permission. Its existence is not evidence that a later candidate passed. Use its
exact candidate run and report.

## Remaining gates

This evidence closes roadmap C1 and C2 for the specified Linux OpenCode release.
Gemini provides a second independent vendor's initialization and auth-boundary
proof without another backend, but does not close C4's authenticated journey.

Authenticated prompts, tool calls, permission outcomes, cancellation, session
restoration and the native vendor-agent UI journey remain separate checks.
Registry platform/distribution coverage and generic custom-command UX also remain
open where the roadmap requires more than these two release installations.
Missing credentials do not justify approving or bypassing authentication.

## Primary references for the initial pins

- [OpenCode 1.18.31 release](https://github.com/anomalyco/opencode/releases/tag/v1.18.31)
- [OpenCode ACP command](https://opencode.ai/docs/acp/)
- [Gemini CLI 0.60.0 release](https://github.com/google-gemini/gemini-cli/releases/tag/v0.60.0)
- [Gemini CLI package/runtime metadata](https://github.com/google-gemini/gemini-cli/blob/v0.60.0/package.json)
- [Gemini release bundle packaging](https://github.com/google-gemini/gemini-cli/blob/v0.60.0/.github/actions/publish-release/action.yml)

## BCD custom-command and lifecycle extension

The BCD branch retains the vendor evidence above without rerunning those releases.
No vendor executable or provider credential has been used during this extension.
Candidate `0e9e66d46b68c27e1e50ebca1b0d63328f0c6756`, Linux x64,
[run 35258450705](https://github.com/cmdr-chara/synara/actions/runs/35258450705),
passed the full workspace and native smoke suite on its rerun. The initial focused
repeat exposed an intermittent terminal-output assertion, recorded in the BCD
handoff and not suppressed or modified by this lane.

The custom-command proof **passed locally** on source candidate
`3e91c35a440d6af7f6ec98029738b4dc53f7d6c7`, Debian 13 x86_64, Rust 1.98.1:
`cargo test --locked -p synara-acp --test custom_profile`. It launches the same
`synara-acp-fixture`, advertised version `1.0.0`, from a copied executable path
containing spaces and Unicode through a profile saved to and reloaded from SQLite.
The parent creates a clean environment and supplies only synthetic test values.
The child exercises two profile configurations, with and without one explicitly
inherited variable name. No source changes distinguish those configurations.

The ordinary test invokes its guarded helper in a separate process. The helper's
`ignored` marker does not skip the proof: the parent invokes that helper explicitly
and checks its exit status. No test mutates process-global environment unsafely.

Methods exercised: `initialize`, `session/new`, two `session/prompt` calls per
profile, and connection shutdown. Observed fixture capabilities: load, resume,
close, list, delete, additional directories, images and logout. Session responses
include modes, select configuration and boolean configuration. Capabilities not
exercised by this particular proof are not claimed as additional results.

Assertions cover literal spaces, Unicode and shell metacharacters in arguments,
correct session working directory, one connection/session across two prompts,
allowlisted-variable presence, unlisted-variable absence and no shell injection.
Fixture replies expose only environment-presence/correctness booleans, never the
synthetic value. Profiles, database/WAL files, traces and child diagnostics are
checked for absence of that value. Public fixture identity and argument strings
are not credentials. Failure classification is a local assertion, protocol or
process failure, never a claimed authenticated-vendor failure.

This is custom-profile interoperability with a deterministic external executable,
not OpenCode/Gemini authenticated support. Real model calls, vendor auth, native
vendor login and vendor session restoration remain untested here.

Recovery verification on September 17, 2026 also passed 151 non-GUI workspace
tests and focused all-feature ACP/agent/core Clippy on that source candidate.
Nine opt-in/helper entries remained ignored, including the live vendor probe.
The custom-profile parent did execute its guarded helper, as described above.
No additional vendor executable, vendor authentication or credentials were used.
Full workspace checks and the native build were blocked locally by a missing
`fontconfig.pc`, and package installation failed on DNS resolution. Therefore
this result does not close the remaining C5/C6 native UX or complete-candidate
verification gates, and does not extend the authenticated compatibility matrix.

## Native integration management

[Plugins, Skills and MCP](integrations.md) keeps provider-owned extensions outside
Synara's authority. Only explicit task/profile-scoped HTTP MCP configuration is
passed through the existing negotiated generic context contract. A successful
Synara-side probe is not agent reachability evidence. Skill documents are inserted
only into visible unsent drafts and do not imply native provider skill support.
