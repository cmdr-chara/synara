# windowmaterial-rs

FILE: windowMaterial.ts Purpose: Decides how the desktop window backs the translucent shell (macOS vibrancy, a          compositor-blurred transparent window on Hyprland, or none), and applies the          renderer's material on macOS: native vibrancy for the opaque shell, or an          adjustable desktop blur (native/window-material addon) when translucent. Layer: Desktop main process Exports: MAC_WINDOW_VIBRANCY, resolveDesktopWindowTranslucency, parseDesktopWindowMaterial,          createWindowMaterialApplier

napi-rs bridge for `apps/desktop/src/windowMaterial.ts`.

## Functions

- `parseHyprlandBlurEnabled` (`apps/desktop/src/windowMaterial.ts:45`)
- `parseDesktopWindowMaterial` (`apps/desktop/src/windowMaterial.ts:66`)