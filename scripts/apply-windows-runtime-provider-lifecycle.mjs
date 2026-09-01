import { replace } from "./windows-runtime-edit-helpers.mjs";

replace(
  "apps/server/src/provider/Layers/ProviderService.ts",
  'import { carryProviderAttachmentPaths } from "../providerAttachmentPaths.ts";',
  'import { carryProviderAttachmentPaths } from "../providerAttachmentPaths.ts";\nimport {\n  classifyProviderStartupFailure,\n  ProviderStartupLifecycle,\n} from "../providerStartupLifecycle.ts";',
);

replace(
  "apps/server/src/provider/Layers/ProviderService.ts",
  `            const adapter = yield* registry.getByProvider(input.provider);\n            let replacementStarted = false;\n            const startAndPersistReplacement = Effect.gen(function* () {`,
  `            const adapter = yield* registry.getByProvider(input.provider);\n            let replacementStarted = false;\n            const startupLifecycle = new ProviderStartupLifecycle();\n            const startAndPersistReplacement = Effect.gen(function* () {`,
);

replace(
  "apps/server/src/provider/Layers/ProviderService.ts",
  `              const started = yield* adapter\n                .startSession(resolvedAdapterStartInput)\n                .pipe(Effect.timeoutOption(PROVIDER_START_SESSION_TIMEOUT));`,
  `              startupLifecycle.transition("starting");\n              startupLifecycle.transition("handshaking");\n              const started = yield* adapter.startSession(resolvedAdapterStartInput).pipe(\n                Effect.tapError((cause) =>\n                  Effect.gen(function* () {\n                    startupLifecycle.fail(classifyProviderStartupFailure(cause));\n                    yield* Effect.logError("provider.session.start_failed", {\n                      threadId,\n                      provider: input.provider,\n                      startup: startupLifecycle.snapshot(),\n                      cause: cause instanceof Error ? cause.message : String(cause),\n                    });\n                  }),\n                ),\n                Effect.onInterrupt(() =>\n                  Effect.gen(function* () {\n                    startupLifecycle.stop("Cancelled");\n                    yield* Effect.logInfo("provider.session.start_cancelled", {\n                      threadId,\n                      provider: input.provider,\n                      startup: startupLifecycle.snapshot(),\n                    });\n                  }),\n                ),\n                Effect.timeoutOption(PROVIDER_START_SESSION_TIMEOUT),\n              );`,
);

replace(
  "apps/server/src/provider/Layers/ProviderService.ts",
  `              if (Option.isNone(started)) {\n                yield* Effect.logError("provider session start exceeded its deadline", {\n                  threadId,\n                  provider: input.provider,\n                  timeoutMs: Duration.toMillis(PROVIDER_START_SESSION_TIMEOUT),\n                });`,
  `              if (Option.isNone(started)) {\n                startupLifecycle.fail("HandshakeTimeout");\n                yield* Effect.logError("provider session start exceeded its deadline", {\n                  threadId,\n                  provider: input.provider,\n                  timeoutMs: Duration.toMillis(PROVIDER_START_SESSION_TIMEOUT),\n                  startup: startupLifecycle.snapshot(),\n                });`,
);

replace(
  "apps/server/src/provider/Layers/ProviderService.ts",
  `              const session = started.value;\n              replacementStarted = true;`,
  `              const session = started.value;\n              startupLifecycle.transition("ready");\n              replacementStarted = true;`,
);

replace(
  "apps/server/src/provider/Layers/ProviderService.ts",
  `              lease.commit();\n              if (\n                replacementFence !== undefined &&`,
  `              lease.commit();\n              startupLifecycle.transition("running");\n              yield* Effect.logDebug("provider.session.started", {\n                threadId,\n                provider: input.provider,\n                startup: startupLifecycle.snapshot(),\n              });\n              if (\n                replacementFence !== undefined &&`,
);

console.log("provider lifecycle migration applied");
