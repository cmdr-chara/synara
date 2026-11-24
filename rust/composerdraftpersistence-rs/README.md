# composerdraftpersistence-rs

Schema-v6 composer-draft persistence: migrations, merge normalization, and
hydration. Model-selection helpers are vendored from the sibling
`composerdraftmodels-rs` port (`mod models`), domain helpers from
`composerdraftdomain-rs` (`mod domain`), attachment helpers from
`composerdraftattachments-rs` (`mod attachments`), pending-user-input
normalization from `pendinguserinputrecovery-rs` (`mod pui`), and computer
control resolution from `computercontrolmode-rs` (`mod ccm`).

napi-rs bridge for `apps/web/src/composerDraftPersistence.ts`.

## Functions

- `migratePersistedComposerDraftStoreState` (`apps/web/src/composerDraftPersistence.ts:1198`)
- `normalizeCurrentPersistedComposerDraftStoreState` (`apps/web/src/composerDraftPersistence.ts:1548`)
- `toHydratedThreadDraft` (`apps/web/src/composerDraftPersistence.ts:1688`)