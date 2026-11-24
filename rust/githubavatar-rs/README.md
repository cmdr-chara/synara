# githubavatar-rs

Canonical GitHub avatar URL derived from an account login. The GitHub CLI's JSON output never includes avatar URLs, so server-side actor normalizers derive a login-addressed URL here when — and only when — GitHub identified the actor as a user.

napi-rs bridge for `packages/shared/src/githubAvatar.ts`.

## Functions

- `githubAvatarUrlForLogin` (`packages/shared/src/githubAvatar.ts:9`)