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
permission. Its existence is not evidence that the probe passed. Use its exact
candidate run and report.

## Current evidence and remaining gates

Fixture agents have exercised local desktop behavior and controlled real SSH.
The first reviewed vendor probe is pending its own CI evidence. Do not mark an
agent release compatible until the report identifies its actual result.

Remaining roadmap gates include C1-C6 and E2/E7. Authenticated prompts, tool calls,
permission outcomes, cancellation, session restoration and the native vendor-agent
UI journey remain separate checks. Missing credentials do not justify approving
or bypassing an authentication boundary.

## Primary references for the initial pins

- [OpenCode 1.18.31 release](https://github.com/anomalyco/opencode/releases/tag/v1.18.31)
- [OpenCode ACP command](https://opencode.ai/docs/acp/)
- [Gemini CLI 0.60.0 release](https://github.com/google-gemini/gemini-cli/releases/tag/v0.60.0)
- [Gemini CLI package/runtime metadata](https://github.com/google-gemini/gemini-cli/blob/v0.60.0/package.json)
- [Gemini release bundle packaging](https://github.com/google-gemini/gemini-cli/blob/v0.60.0/.github/actions/publish-release/action.yml)
