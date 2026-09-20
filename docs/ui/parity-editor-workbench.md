# Native editor and command workspace

Base: `fe0fc11430892ca4c2ddcccd153b00e2d6046e6d` on
`cmdr-chara/synara:astra/gpui-clean-rewrite`.

## Implemented in this batch

- Up to 24 file tabs with separate native input entities, undo/redo histories,
  selection and scroll state. An existing tab is reused rather than reread.
- Save-and-close, discard-and-close and keep-open handling for unsaved tabs.
  Project/mode changes and closing Environment still respect unsaved buffers.
  Application close reviews all dirty tabs, not merely the visible file.
- Generation-checked file opens and visible loading feedback. A late read cannot
  replace the most recently selected file. Local and SSH reads/writes retain the
  existing contained, version-checked services.
- Case-sensitive literal search, previous/next with wrap, match counts, replace
  one and replace all. All replacements remain in the unsaved buffer. Replace all
  is one undo operation and enforces the existing input-size limit.
- One-based line:column navigation and direct navigation from content-search
  results. Unicode code points, not byte offsets, determine columns.
- Markdown source/preview from the current buffer. Original code copy and link
  safety remain in the shared native renderer. No file is executed by previewing.
- Cursor line/column, selected character count, byte count and line-ending labels.
- Adaptive Explorer layout, collapsible tree, folder breadcrumbs, folders-first
  sorting, dotfile visibility and pagination based on the filtered file count.
- Native command palette with keyboard/pointer activation, command/thread/project
  filters, open-file switching and dispatch to existing application actions.

## Shortcuts

| Context | Shortcut | Action |
| --- | --- | --- |
| Application | Ctrl/Cmd+Shift+P | Command palette |
| Editor | Ctrl/Cmd+F | Find literal text |
| Editor | Ctrl/Cmd+H | Replace literal text |
| Editor | Ctrl/Cmd+G | Go to line:column |
| Editor | F3 / Shift+F3 | Next / previous match |
| Explorer | Ctrl/Cmd+Tab / Ctrl/Cmd+Shift+Tab | Next / previous file tab |
| Explorer | Ctrl/Cmd+W | Close current file tab |

IME composition and existing modal owners retain priority. Keyboard shortcuts do
not grant agent permissions, auto-start terminals or execute arbitrary text.

## Development status

The user requested feature-first development on September 20, 2026. No new test
infrastructure or test suite is included. Existing CI is not disabled or weakened.
Source review and whitespace checks are separate from the deferred compile,
interaction, regression, visual and platform acceptance. This receipt does not
claim those checks passed. No release/default branch/PR is changed.

Tabs are retained in the active workspace process, not restored after restart.
Switching projects still requires saving or explicitly discarding modified files.
Regex/case-insensitive matching, multiple terminal sessions, embedded Browser,
Side chats, syntax highlighting and the remainder of product parity remain open.
