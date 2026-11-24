# notificationretention-rs

FILE: notificationRetention.ts Purpose: Keep shown desktop notifications strongly referenced until they can          no longer be activated. Electron clears a Notification's native          delegate when its JS wrapper is garbage-collected, so a toast whose          object is only held by a local variable silently drops its `click`          (and `close`) events a few seconds after it is shown.

napi-rs bridge for `apps/desktop/src/notificationRetention.ts`.

## Functions

- `createDesktopNotificationRetainer` (`apps/desktop/src/notificationRetention.ts:21`)