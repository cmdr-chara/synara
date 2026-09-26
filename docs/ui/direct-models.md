# Direct model conversations

Synara's direct inference owner is `synara-model`. It does not depend on ACP,
start an agent process, own Git, or execute tools. `Controller::submit_direct`
uses the existing task reservation, cancellation, workspace store and transcript.
ACP remains the separate, provider-independent coding-agent integration layer.

## Native workflow

Open **Settings > Direct models**. Add a custom profile, use **Add Google API**,
or explicitly refresh the community catalog and review an entry. Review the API
base, protocol, model identities and capability provenance before saving. The
Google setup template is inert and requires replacing its placeholder model.
Catalog entries are candidates, not installed providers or authenticated accounts.

Saved profiles have explicit model discovery and an endpoint-bound credential
editor. Model discovery opens an editable review, not an automatic configuration
replacement. Select **Use in conversation**, review the target and output/effort
options, and confirm. Only explicit **Send** starts a request. **Stop** cancels the
owned request and retains partial output. Restart restores data, not execution.
Model rows can be starred into a persistent, searchable **Favorites** section.
Choosing a favorite opens the same route review and confirmation flow; it does
not send a prompt or silently carry route options. A favorite for a removed
provider or model remains visible for explicit removal.
Switching an existing task between ACP and direct inference explicitly retires the
old live agent session before changing the route. A failed retirement keeps the
route unchanged. For a separate reviewed continuation, use [Continue with](provider-handoff.md).

## Shared protocol families

| Family identifier | Authentication | Implemented runtime | Deliberately unsupported |
| --- | --- | --- | --- |
| `open_ai_chat` | Bearer API key | Chat Completions SSE, text/inline images, offered tool-call proposals, explicit effort, usage, reviewed JSON-schema output | Responses-only endpoints, provider-specific OAuth/cloud signing, native tool execution |
| `anthropic_messages` | `x-api-key` plus protocol-version header | Messages SSE, text/inline images, offered tool-call proposals, input/cache/output accounting | Reviewed JSON-schema output and effort controls in this adapter, tool execution |
| `google_generate_content` | `x-goog-api-key`, never a query-string key | Generative Language streaming, text/inline images, thought-text separation, usage, reviewed JSON-schema output, bounded paginated model discovery | Function calls and signature-dependent replay, effort controls, Vertex/cloud OAuth, non-text generated output |

Inline image encoding is a runtime contract, **not complete native multimodal
conversations**. The current direct-chat controller accepts visible text history
only and rejects composer attachments rather than dropping them silently. Tool
calls are proposals in the transport contract. Direct chat offers no tools and
cannot execute a proposed command, filesystem operation, or external write.

The registry normalizes explicit SDK identifiers into these families and accepts
custom endpoints. It represents tools, images and structured output as supported,
unsupported or unknown. Names never imply capabilities, quotas or authentication.
Identity-only discovery does not invent advanced capabilities. Google discovery
also retains reported input/output limits, but leaves tools/images/schema support
unknown until separately reviewed. Boolean reasoning metadata does not invent a
list of effort values. Community metadata is labeled as metadata, not live proof.

This is **partial progress toward roughly 75+ interoperable providers**, not a
claim of 75-provider support. An adaptable catalog entry and a transport-family
test do not prove account interoperability. Additional authentication families,
provider compatibility evidence, native multimodal context and approved tool
execution remain high-priority feature work.

## Locally validated structured output

A selected model must explicitly report structured-output support. In the native
model-options review, `output` accepts, for example:

```json
{
  "type": "json_schema",
  "name": "result",
  "schema": {
    "type": "object",
    "properties": {"ok": {"type": "boolean", "const": true}},
    "required": ["ok"],
    "additionalProperties": false
  }
}
```

The request and schema are checked before network I/O. On a normal stop, the
complete decoded JSON must also match locally **before** a successful finished
event can reach the transcript. Invalid JSON, a schema mismatch, a truncated
stream or an incomplete Google stop leaves the partial output and fails the turn.
There is no automatic retry or misleading successful completion.

The supported subset includes boolean schemas, `type`/type unions, `enum`, `const`,
`properties`, `required`, `additionalProperties`, `items`, `allOf`, `anyOf`, `oneOf`,
`not`, and minimum/maximum item, property and Unicode-character counts. Recognized
annotations are `title`, `description`, `$comment`, `default` and the explicit
2020-12 `$schema` identifier. Object/array keywords have normal type applicability.
This is a bounded subset, not a general-purpose JSON Schema implementation.

References, remote resolution, regex/format, numeric-range and unevaluated-vocabulary
keywords are rejected up front. No unsupported constraint is silently ignored.
Limits are 64 KiB of schema, 4096 schema nodes, depth 32 and 65,536 validation steps.
Providers can reject otherwise locally supported schema combinations, in which
case the HTTP error is reported without retry.

## Live provider/account telemetry

Each configured direct provider has an explicit **Refresh live account** action.
It performs one credentialed provider metadata request against the exact reviewed
endpoint. A successful response proves that the configured credential was
accepted for that endpoint at refresh time. Profiles that explicitly require no
key report endpoint reachability instead of claiming authentication.

Synara reads only bounded provider response headers with documented/common
rate-limit semantics. It shows request/token limit, remaining and reset values
when they are actually returned, plus a bounded Retry-After value. Missing
headers remain “not reported.” Billing, credits, subscription tier and account
identity are never inferred from model names, local token history or credential
store success; the UI explicitly reports billing/credits as unavailable from
this metadata endpoint.

The refresh is live and deliberately not persisted as durable account state, so
restart cannot present stale quota data as current. Generic ACP coding-agent
sessions still expose only the telemetry negotiated/reported by ACP; Synara does
not invent a provider account API for agents whose protocol has none.

## Security and lifecycle

Profiles and bindings persist references, never key material. `NativeSecretStore`
uses supported OS stores, starts unverified, serializes blocking operations and
redacts native error payloads. Locked, unavailable or absent keys fail closed.
There is no plaintext file, environment-variable or mock-store fallback. Changing
the canonical endpoint or protocol changes the key reference. A key is never
silently carried to a new destination. Account validity is not inferred from a
successful OS-store access probe.

Endpoints require HTTPS except for explicit numeric-loopback HTTP opt-in.
Credentials, query strings, fragments and encoded paths are prohibited in the
configured base. Redirects, automatic retries and ambient proxies are disabled.
Google model IDs cannot inject a path, query or operation. Pagination follows
encoded data tokens on the same reviewed origin, never provider-supplied URLs.
Discovery is bounded to eight Google pages, 4096 models and 120 seconds overall.
Other `/models` discovery explicitly reports its bounded identity-only first page.

Requests are limited to 1 MiB, decoded response content to 8 MiB, SSE frames to
1 MiB and wire input to 32 MiB. Event count, idle time and HTTP request time are
bounded. Stop does not guarantee that a remote provider did not already process
or bill the request. No retry is inferred from an ambiguous result.

Provider configuration/key mutation and agent model/mode/option changes respect
the existing active-task reservation. Request producer and transcript consumer
are owned by the same prompt, with no detached callback writing into a later turn.
Saved routes and schemas never execute on restore.

## Evidence and remaining acceptance

See the [sprint receipt](../verification/max-feature-sprint.md) for exact candidate,
checks and failures. Native evidence is Linux/X11 using owned HTTP and ACP fixtures.
It is not authenticated OpenAI/Anthropic/Google account acceptance, real OS-keychain
interaction, cross-platform acceptance, or comprehensive accessibility/IME proof.

Protocol references: [Google generation](https://ai.google.dev/api/generate-content),
[model discovery](https://ai.google.dev/api/models),
[API-key transport](https://ai.google.dev/gemini-api/docs/api-key),
[structured output](https://ai.google.dev/gemini-api/docs/structured-output), and
[JSON Schema object applicability](https://json-schema.org/understanding-json-schema/reference/object).
