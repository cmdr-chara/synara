# Parity continuation verification - batch 40

Date: 2026-09-26

## Delivered

### M07 Protected browser session/cookie import

Private provider sign-in flows can now import a user-selected Netscape/Mozilla
`cookies.txt` jar. The source is read through a no-follow bounded reader,
normalized into a validated Netscape jar, and loaded only into the exact
request-owned `Authentication { flow }` WebKit profile.

The import deliberately does not inspect browser databases or copy cookies from
Chrome, Firefox, Manual tabs or agent tabs. It accepts one explicitly selected
file, rejects malformed, expired, oversized and suspicious cookie records, and
never logs cookie values or source paths.

Import replaces the current authentication WebKit profile with a fresh temporary
profile seeded from the normalized jar, then reloads the existing sign-in tabs
through the normal Session navigation owner. Popup authority and request lifetime
remain unchanged.

The temporary profile directory and its cookie store are destroyed when the final
authentication tab closes, the request is cancelled, expires, is restarted or is
finished. Manual and AgentTask profiles are never import targets.

## Inventory effect

- M07 complete: major remaining 8 -> 7.
- Shipped feature slices 88 -> 89.
- Execution total 10 -> 9.
- D2 remains OPEN for M09, M11 and M12.
