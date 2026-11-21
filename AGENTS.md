# AGENTS.md

Desktop Figma client: a Tauri 2 window that loads `https://www.figma.com/`. Replaces the outdated, buggy Electron wrapper. There is no frontend and no Node project; the only custom Rust is the popup handler in `src/main.rs`. Keep it that way unless asked.

## Layout

- `tauri.conf.json`: one window, `url` = figma.com, `create: false` (the window is built in `main.rs`, so the popup hook can be attached). Change window size/title/url here.
- `src/main.rs`: builds the main window from that config and handles `window.open` popups by opening them as extra Tauri windows with `window_features(...)`, which keeps `window.opener` (needed for Google SSO to report back to Figma).
- Downloads (`on_download` in `main.rs`): WebKitGTK has no save prompt, so a download goes to `~/.cache/dev.figma-tauri.client/downloads/<n>/` first; when it finishes a native save dialog opens (non-blocking), and the file is moved to the chosen path. Cancel deletes it; errors show a dialog. `tauri-plugin-dialog` is used from Rust only (no capability granted, so pages can't open dialogs).
- `build.rs`: `tauri_build::build()`.
- `icons/icon.png`: solid-colour placeholder, not Figma's logo. Replace with a real icon.
- `Cargo.lock` is committed (this is an app). `target/` and `gen/` are ignored.

## System dependency (Linux)

Needs `webkit2gtk-4.1` (Arch: `sudo pacman -S --needed webkit2gtk-4.1`). Without it `cargo build` dies in the `webkit2gtk-sys` build script.

## Commands

- Dev: `cargo run`
- Release binary: `cargo build --release` -> `target/release/figma-tauri`
- Installers (deb/AppImage/...): `bunx @tauri-apps/cli build`. **Not tried yet**; only `cargo build --release` has been run.

No tests: there is no logic to test.

## Rules

- **Remote content gets no IPC.** figma.com runs in this webview, so do not add a `capabilities/` entry with `remote.urls`, and do not enable `app.withGlobalTauri`. Tauri's default-deny ACL then blocks every command from the page.
- Popup windows (`popup-N`) are also remote content with no capabilities; keep it so.
- Don't add `devUrl`/`frontendDist`; the explicit window `url` makes them unnecessary.

## Verified state (Linux, WebKitGTK 2.54, Tauri 2.12)

- Builds (debug + release) and launches; figma.com marketing page loads and renders, no unsupported-browser page.
- Google login failed before the popup handler was added (popups were dropped). The handler builds and the main window still launches, but **the Google login flow itself is not verified** by an agent.
- Downloads: temp download + dialog request verified against a local `Content-Disposition: attachment` server (file landed with the right name, dialog requested with the right default). **Not seen on screen** (screen locked during testing): the dialog UI, saving, cancel.
- **Not verified:** email/password login, opening a file (WebGL canvas), popup closing after login.

## Known risks / troubleshooting

- The Linux engine is **WebKitGTK, not Chromium**. Figma targets Chromium, so expect rough edges the Electron app did not have. Don't promise parity.
- Blank/white canvas or flicker: try `WEBKIT_DISABLE_DMABUF_RENDERER=1 cargo run`, then `WEBKIT_DISABLE_COMPOSITING_MODE=1`. Only bake one into the app if it is confirmed necessary.
- If figma.com serves an unsupported-browser page, set a Safari-style `userAgent` on the window in `tauri.conf.json`.
- Google SSO may still refuse embedded webviews ("This browser or app may not be secure"). If so, set a Safari/Firefox-style `userAgent` on the window in `tauri.conf.json`. Email/password login is the fallback.
- All `window.open`/`target=_blank` links open as bare in-app windows with no URL bar (including external sites). Route non-Figma/Google URLs to the system browser only if that becomes annoying.
- Startup logs `GStreamer element autoaudiosink not found` (and a harmless-looking GLib-GObject CRITICAL). Audio/video in the webview likely needs `gst-plugins-good`; unconfirmed.
