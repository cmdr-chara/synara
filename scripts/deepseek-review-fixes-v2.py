from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    file = Path(path)
    text = file.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected 1 match, got {count}: {old[:140]!r}")
    file.write_text(text.replace(old, new, 1))


support = "apps/server/src/provider/acp/DeepSeekAcpSupport.ts"
replace_once(
    support,
    '''export interface DeepSeekAcpRuntimeInput extends Omit<
  AcpSessionRuntimeOptions,
  "authMethodId" | "freshSessionRetry" | "resolveAuthMethodId" | "spawn"
> {
  readonly childProcessSpawner: ChildProcessSpawner.ChildProcessSpawner["Service"];
  readonly settings: DeepSeekAcpRuntimeSettings | null | undefined;
  readonly runtimeMode: RuntimeMode;
}''',
    '''export interface DeepSeekAcpRuntimeInput extends Omit<
  AcpSessionRuntimeOptions,
  "authMethodId" | "freshSessionRetry" | "resolveAuthMethodId" | "spawn"
> {
  readonly childProcessSpawner: ChildProcessSpawner.ChildProcessSpawner["Service"];
  readonly settings: DeepSeekAcpRuntimeSettings | null | undefined;
  readonly runtimeMode: RuntimeMode;
  readonly sessionsRoot?: string;
}''',
)
replace_once(
    support,
    'const DEFAULT_DEEPSEEK_MODEL = "deepseek-v4-pro";\nconst KNOWN_DEEPSEEK_MODELS = ["deepseek-v4-flash", "deepseek-v4-pro"] as const;\n',
    '''const DEFAULT_DEEPSEEK_MODEL = "deepseek-v4-pro";
const KNOWN_DEEPSEEK_MODELS = ["deepseek-v4-flash", "deepseek-v4-pro"] as const;

const resolveDeepSeekAuthMethodId: NonNullable<
  AcpSessionRuntimeOptions["resolveAuthMethodId"]
> = (initializeResult) => {
  const authMethodId = initializeResult.authMethods
    ?.map((method) => method.id.trim())
    .find((id) => id.length > 0);
  return authMethodId
    ? Effect.succeed(authMethodId)
    : Effect.fail(
        new AcpErrors.AcpRequestError({
          code: -32602,
          errorMessage: "DeepSeek Harness ACP did not advertise an authentication method.",
          data: { authMethods: initializeResult.authMethods ?? [] },
        }),
      );
};
''',
)
replace_once(
    support,
    "    persistenceRoot: !!js process.env.SYNARA_DEEPSEEK_SESSIONS_ROOT ?? './.synara-deepseek-sessions'",
    "    persistenceRoot: !!js process.env.SYNARA_DEEPSEEK_SESSIONS_ROOT",
)
replace_once(
    support,
    '''export function buildDeepSeekAcpSpawnInput(input: {
  readonly settings: DeepSeekAcpRuntimeSettings | null | undefined;
  readonly configPath: string;
  readonly cwd: string;
  readonly runtimeMode: RuntimeMode;
}): AcpSpawnInput {''',
    '''export function buildDeepSeekAcpSpawnInput(input: {
  readonly settings: DeepSeekAcpRuntimeSettings | null | undefined;
  readonly configPath: string;
  readonly cwd: string;
  readonly runtimeMode: RuntimeMode;
  readonly sessionsRoot?: string;
}): AcpSpawnInput {''',
)
replace_once(
    support,
    '''      ...buildProviderChildEnvironment({
        provider: "deepseek",
        inheritedSynaraKeys: ["SYNARA_DEEPSEEK_SESSIONS_ROOT"],
      }),
      DSH_PERMISSION_MODE: deepSeekPermissionMode(input.runtimeMode),''',
    '''      ...buildProviderChildEnvironment({
        provider: "deepseek",
        inheritedSynaraKeys: ["SYNARA_DEEPSEEK_SESSIONS_ROOT"],
      }),
      SYNARA_DEEPSEEK_SESSIONS_ROOT:
        process.env.SYNARA_DEEPSEEK_SESSIONS_ROOT?.trim() ||
        input.sessionsRoot ||
        path.join(os.tmpdir(), "synara-deepseek-sessions"),
      DSH_PERMISSION_MODE: deepSeekPermissionMode(input.runtimeMode),''',
)
replace_once(
    support,
    '''        ...input,
        spawn: buildDeepSeekAcpSpawnInput({''',
    '''        ...input,
        resolveAuthMethodId: resolveDeepSeekAuthMethodId,
        spawn: buildDeepSeekAcpSpawnInput({''',
)
replace_once(
    support,
    '''          cwd: input.cwd,
          runtimeMode: input.runtimeMode,
        }),''',
    '''          cwd: input.cwd,
          runtimeMode: input.runtimeMode,
          sessionsRoot: input.sessionsRoot,
        }),''',
)

adapter = "apps/server/src/provider/Layers/DeepSeekAdapter.ts"
replace_once(
    adapter,
    'import {\n  ApprovalRequestId,',
    'import * as nodePath from "node:path";\n\nimport {\n  ApprovalRequestId,',
)
replace_once(
    adapter,
    '''import {
  classifyAcpPromptTurnCompletion,
  mapAcpToAdapterError,
  resolveAcpPermissionPolicy,
  selectAcpPermissionOptionId,
} from "../acp/AcpAdapterSupport.ts";
''',
    '''import {
  classifyAcpPromptTurnCompletion,
  mapAcpToAdapterError,
  resolveAcpPermissionPolicy,
  selectAcpPermissionOptionId,
} from "../acp/AcpAdapterSupport.ts";
import { withAcpPlanModePrompt } from "../acp/AcpAdapterSessionSupport.ts";
''',
)
replace_once(
    adapter,
    'import { parsePermissionRequest } from "../acp/AcpRuntimeModel.ts";\n',
    '''import { parsePermissionRequest } from "../acp/AcpRuntimeModel.ts";
import {
  forkAcpTurnIdleWatchdog,
  resolveAcpTurnIdleTimeoutMs,
} from "../acp/AcpTurnIdleWatchdog.ts";
''',
)
replace_once(
    adapter,
    '''const PROVIDER = "deepseek" as const;
const MAX_INLINE_SKILL_CHARS = 48_000;
''',
    '''const PROVIDER = "deepseek" as const;
const MAX_INLINE_SKILL_CHARS = 48_000;
const DEEPSEEK_TURN_IDLE_TIMEOUT_MS = resolveAcpTurnIdleTimeoutMs({
  envVar: "SYNARA_DEEPSEEK_TURN_IDLE_TIMEOUT_MS",
  defaultMs: 600_000,
});
const DEEPSEEK_TURN_WATCHDOG_INTERVAL_MS = 15_000;
const DEEPSEEK_PLAN_MODE_PROMPT_PREFIX = [
  "Synara Plan mode is active.",
  "Do not mutate files, run commands that change state, or implement the request in this turn.",
  "Inspect and reason as needed, then return a concrete implementation plan.",
].join("\\n");
''',
)
replace_once(
    adapter,
    '''  activeTurnId: TurnId | undefined;
  activeInteractionMode: "default" | "plan" | "debug" | undefined;
  stopped: boolean;
''',
    '''  activeTurnId: TurnId | undefined;
  activeInteractionMode: "default" | "plan" | "debug" | undefined;
  lastTurnActivityAt: number | undefined;
  stopped: boolean;
''',
)
replace_once(
    adapter,
    '''    const sessions = new Map<ThreadId, DeepSeekSessionContext>();
    const runtimeEventPubSub = yield* PubSub.bounded<ProviderRuntimeEvent>(''',
    '''    const sessions = new Map<ThreadId, DeepSeekSessionContext>();
    const lifecycleScope = yield* Scope.make("sequential");
    yield* Effect.addFinalizer(() => Scope.close(lifecycleScope, Exit.void));
    const runtimeEventPubSub = yield* PubSub.bounded<ProviderRuntimeEvent>(''',
)
replace_once(
    adapter,
    '''    const startSession: DeepSeekAdapterShape["startSession"] = (input) =>
''',
    '''    const retireExitedSession = (ctx: DeepSeekSessionContext) =>
      Effect.gen(function* () {
        if (ctx.stopped || sessions.get(ctx.threadId) !== ctx) return;

        const activeTurnId = ctx.activeTurnId;
        const errorMessage = "DeepSeek Harness ACP process exited unexpectedly.";
        const promptFiber = ctx.promptFiber;
        ctx.stopped = true;
        ctx.activeTurnId = undefined;
        ctx.activeInteractionMode = undefined;
        ctx.lastTurnActivityAt = undefined;
        ctx.promptFiber = undefined;
        yield* settlePendingApprovals(ctx);
        if (promptFiber) yield* Fiber.interrupt(promptFiber);
        if (ctx.notificationFiber) yield* Fiber.interrupt(ctx.notificationFiber);
        sessions.delete(ctx.threadId);

        if (activeTurnId !== undefined) {
          yield* offerRuntimeEvent(ctx.lifecycleGeneration, {
            type: "turn.completed",
            ...(yield* makeEventStamp()),
            provider: PROVIDER,
            threadId: ctx.threadId,
            turnId: activeTurnId,
            payload: { state: "failed", errorMessage },
          });
        }
        yield* offerRuntimeEvent(ctx.lifecycleGeneration, {
          type: "session.exited",
          ...(yield* makeEventStamp()),
          provider: PROVIDER,
          threadId: ctx.threadId,
          payload: { exitKind: "error", reason: errorMessage, recoverable: true },
        });
        yield* Effect.ignore(Scope.close(ctx.scope, Exit.void));
      });

    const startSession: DeepSeekAdapterShape["startSession"] = (input) =>
''',
)
replace_once(
    adapter,
    '''          childProcessSpawner,
          cwd,
          runtimeMode: input.runtimeMode,
          clientInfo: { name: "Synara", version: "0.0.0" },''',
    '''          childProcessSpawner,
          cwd,
          runtimeMode: input.runtimeMode,
          sessionsRoot: nodePath.join(serverConfig.stateDir, "deepseek-sessions"),
          clientInfo: { name: "Synara", version: "0.0.0" },''',
)
replace_once(
    adapter,
    '''        yield* acp.handleRequestPermission((params) =>
          Effect.gen(function* () {
            const policyOutcome = resolveAcpPermissionPolicy({''',
    '''        yield* acp.handleRequestPermission((params) =>
          Effect.gen(function* () {
            if (ctx?.activeTurnId !== undefined) ctx.lastTurnActivityAt = Date.now();
            const policyOutcome = resolveAcpPermissionPolicy({''',
)
replace_once(
    adapter,
    '''        const now = yield* nowIso;
        const session: ProviderSession = {''',
    '''        if (settings.configPath?.trim() && modelSelection?.model) {
          yield* acp.setModel(modelSelection.model).pipe(
            Effect.mapError((error) =>
              mapAcpToAdapterError(PROVIDER, input.threadId, "session/set_config_option", error),
            ),
          );
        }
        const now = yield* nowIso;
        const session: ProviderSession = {''',
)
replace_once(
    adapter,
    '''          activeTurnId: undefined,
          activeInteractionMode: undefined,
          stopped: false,''',
    '''          activeTurnId: undefined,
          activeInteractionMode: undefined,
          lastTurnActivityAt: undefined,
          stopped: false,''',
)
replace_once(
    adapter,
    '''            Effect.gen(function* () {
              const turnId = ctx.activeTurnId;
              switch (event._tag) {''',
    '''            Effect.gen(function* () {
              const turnId = ctx.activeTurnId;
              if (turnId !== undefined) ctx.lastTurnActivityAt = Date.now();
              switch (event._tag) {''',
)
replace_once(
    adapter,
    '''          providerRefs: { providerThreadId: started.sessionId },
          payload: {},
        });
        return session;''',
    '''          providerRefs: { providerThreadId: started.sessionId },
          payload: {},
        });
        yield* acp.awaitExit.pipe(
          Effect.flatMap(() => retireExitedSession(ctx)),
          Effect.forkIn(lifecycleScope),
        );
        return session;''',
)
replace_once(
    adapter,
    '''    const sendTurn: DeepSeekAdapterShape["sendTurn"] = (input) =>
''',
    '''    const failDeepSeekTurnAsTimedOut = (
      ctx: DeepSeekSessionContext,
      turnId: TurnId,
      idleMs: number,
    ) =>
      Effect.gen(function* () {
        if (ctx.activeTurnId !== turnId) return;
        const promptFiber = ctx.promptFiber;
        const idleSeconds = Math.max(1, Math.round(idleMs / 1_000));
        const errorMessage = `DeepSeek Harness ACP produced no activity for ${idleSeconds}s.`;
        ctx.activeTurnId = undefined;
        ctx.activeInteractionMode = undefined;
        ctx.lastTurnActivityAt = undefined;
        ctx.promptFiber = undefined;
        ctx.session = {
          ...ctx.session,
          status: "error",
          activeTurnId: undefined,
          updatedAt: yield* nowIso,
          lastError: errorMessage,
        };
        if (promptFiber) yield* Fiber.interrupt(promptFiber);
        yield* Effect.ignore(ctx.acp.cancel);
        yield* offerRuntimeEvent(ctx.lifecycleGeneration, {
          type: "turn.completed",
          ...(yield* makeEventStamp()),
          provider: PROVIDER,
          threadId: ctx.threadId,
          turnId,
          payload: { state: "failed", errorMessage },
        });
      });

    const sendTurn: DeepSeekAdapterShape["sendTurn"] = (input) =>
''',
)
replace_once(
    adapter,
    '''            issue: "A text prompt is required.",
          });
        }

        const turnId = TurnId.makeUnsafe(crypto.randomUUID());''',
    '''            issue: "A text prompt is required.",
          });
        }
        const promptText = withAcpPlanModePrompt({
          text,
          interactionMode: input.interactionMode,
          promptPrefix: DEEPSEEK_PLAN_MODE_PROMPT_PREFIX,
        });

        const turnId = TurnId.makeUnsafe(crypto.randomUUID());''',
)
replace_once(
    adapter,
    '''        ctx.activeTurnId = turnId;
        ctx.activeInteractionMode = input.interactionMode ?? "default";
        ctx.turns.push({ id: turnId, items: [] });''',
    '''        ctx.activeTurnId = turnId;
        ctx.activeInteractionMode = input.interactionMode ?? "default";
        ctx.lastTurnActivityAt = Date.now();
        ctx.turns.push({ id: turnId, items: [] });''',
)
replace_once(
    adapter,
    '            .prompt({ prompt: [{ type: "text", text }] })',
    '            .prompt({ prompt: [{ type: "text", text: promptText }] })',
)
replace_once(
    adapter,
    '''          ctx.activeTurnId = undefined;
          ctx.activeInteractionMode = undefined;
          ctx.session = {''',
    '''          ctx.activeTurnId = undefined;
          ctx.activeInteractionMode = undefined;
          ctx.lastTurnActivityAt = undefined;
          ctx.promptFiber = undefined;
          ctx.session = {''',
)
replace_once(
    adapter,
    '''                ctx.activeTurnId = undefined;
                ctx.activeInteractionMode = undefined;
                ctx.session = {''',
    '''                ctx.activeTurnId = undefined;
                ctx.activeInteractionMode = undefined;
                ctx.lastTurnActivityAt = undefined;
                ctx.promptFiber = undefined;
                ctx.session = {''',
)
replace_once(
    adapter,
    '''        ctx.promptFiber = promptFiber;
        return { threadId: ctx.threadId, turnId };''',
    '''        ctx.promptFiber = promptFiber;
        yield* forkAcpTurnIdleWatchdog({
          idleTimeoutMs: DEEPSEEK_TURN_IDLE_TIMEOUT_MS,
          checkIntervalMs: DEEPSEEK_TURN_WATCHDOG_INTERVAL_MS,
          scope: ctx.scope,
          isTurnActive: () => ctx.activeTurnId === turnId && !ctx.stopped,
          isAwaitingHuman: () => ctx.pendingApprovals.size > 0,
          lastActivityAt: () => ctx.lastTurnActivityAt ?? Date.now(),
          touchActivity: () => {
            ctx.lastTurnActivityAt = Date.now();
          },
          onIdleTimeout: (idleMs) => failDeepSeekTurnAsTimedOut(ctx, turnId, idleMs),
        });
        return { threadId: ctx.threadId, turnId };''',
)

providers = "apps/web/src/components/settings/ProvidersSettingsPanel.tsx"
replace_once(
    providers,
    '''  | "antigravityBinaryPath"
  | "grokBinaryPath"
  | "droidBinaryPath"''',
    '''  | "antigravityBinaryPath"
  | "grokBinaryPath"
  | "deepSeekBinaryPath"
  | "deepSeekConfigPath"
  | "droidBinaryPath"''',
)
deepseek_settings = '''  {
    provider: "deepseek",
    docs: [{ label: "Harness", href: "https://github.com/deepseek-ai/deepseek-harness" }],
    fields: [
      {
        kind: "text",
        settingsKey: "deepSeekBinaryPath",
        label: "DeepSeek Harness binary path",
        placeholder: "dsh-acp-demo",
        description: (
          <>
            Leave blank to use <code>dsh-acp-demo</code> from your PATH.
          </>
        ),
      },
      {
        kind: "text",
        settingsKey: "deepSeekConfigPath",
        label: "DeepSeek Harness config path",
        placeholder: "Cordis config path",
        description:
          "Optional custom Harness composition. Leave blank to let Synara generate the Cordis config.",
      },
    ],
  },
'''
replace_once(
    providers,
    '  {\n    provider: "antigravity",',
    deepseek_settings + '  {\n    provider: "antigravity",',
)

library = "apps/web/src/components/PluginLibrary.tsx"
replace_once(
    library,
    '''  const grokCapabilitiesQuery = useQuery(providerComposerCapabilitiesQueryOptions("grok"));
  const droidCapabilitiesQuery = useQuery(providerComposerCapabilitiesQueryOptions("droid"));''',
    '''  const grokCapabilitiesQuery = useQuery(providerComposerCapabilitiesQueryOptions("grok"));
  const deepSeekCapabilitiesQuery = useQuery(
    providerComposerCapabilitiesQueryOptions("deepseek"),
  );
  const droidCapabilitiesQuery = useQuery(providerComposerCapabilitiesQueryOptions("droid"));''',
)
replace_once(
    library,
    '    deepseek: { plugins: false, skills: false },',
    '''    deepseek: {
      plugins: supportsPluginDiscovery(deepSeekCapabilitiesQuery.data),
      skills: supportsSkillDiscovery(deepSeekCapabilitiesQuery.data),
    },''',
)
