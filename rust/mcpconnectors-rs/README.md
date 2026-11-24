# mcpconnectors-rs

FILE: mcpConnectors.ts Purpose: The Connectors catalog plus the URL and server-name rules shared by server and web. Layer: Shared runtime utility Exports: MCP_CONNECTOR_CATEGORIES, MCP_CONNECTOR_PRESETS, findMcpConnectorPreset,   normalizeMcpConnectorUrl, defaultMcpConnectorName, mcpConnectorSlug, mcpConnectorServerName

napi-rs bridge for `packages/shared/src/mcpConnectors.ts`.

## Functions

- `findMcpConnectorPreset` (`packages/shared/src/mcpConnectors.ts:195`)
- `isMcpLoopbackHostname` (`packages/shared/src/mcpConnectors.ts:227`)
- `normalizeMcpConnectorUrl` (`packages/shared/src/mcpConnectors.ts:234`)
- `defaultMcpConnectorName` (`packages/shared/src/mcpConnectors.ts:274`)
- `mcpConnectorSlug` (`packages/shared/src/mcpConnectors.ts:284`)
- `mcpConnectorServerName` (`packages/shared/src/mcpConnectors.ts:296`)
- `mcpConnectorMentionPath` (`packages/shared/src/mcpConnectors.ts:303`)
- `isMcpConnectorMentionPath` (`packages/shared/src/mcpConnectors.ts:307`)
- `serverNameFromMcpConnectorMentionPath` (`packages/shared/src/mcpConnectors.ts:312`)
- `providerSupportsMcpConnectors` (`packages/shared/src/mcpConnectors.ts:335`)