# Parity continuation verification - batch 39

Date: 2026-09-26

## Accepted

### A04 Fresh-install onboarding with real provider accounts

Hosted Copilot onboarding acceptance run `36248953688`, job
`108423260218`, completed successfully on exact candidate
`a51a87fa684f26de8616ab04c4a7cbd78d79c351`.

The acceptance journey used the pinned GitHub Copilot CLI through ACP and proved:

- fresh Synara state with no pre-existing task or prompt;
- setup-task preparation remains inert until explicit Connect;
- a real provider account can initialize through the normal ACP path;
- an existing local project is added through the first-run Project step;
- Finish setup persists without implicitly sending a prompt;
- one explicit user turn receives a real provider response;
- restart restores the task, project and history without provider autostart.

Provider credentials remained in the provider-owned environment. The evidence
artifact was scrubbed of provider state, token material, transcripts and raw
application logs.

### D1 Onboarding

The same run supplies the previously missing real-provider/project-state proof
for D1. Product setup/login UX was already complete, so D1 is now PASS.

## Inventory effect

- A04 accepted: acceptance/integration remaining 3 -> 2.
- Shipped feature slices 85 -> 86.
- Execution total 13 -> 12.
- Broad parity gates OPEN 20 -> 19.
