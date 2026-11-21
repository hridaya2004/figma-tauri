# AGENTS.md

Desktop Figma client: a Tauri 2 window that loads `https://www.figma.com/`. Replaces the outdated, buggy Electron wrapper. There is no frontend, no Node project and no custom Rust logic; keep it that way unless asked.

## Layout

- `tauri.conf.json`: the whole app. One window, `url` = figma.com. Change window behaviour here first.
- `src/main.rs`: stock `tauri::Builder` boilerplate.
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
- Don't add `devUrl`/`frontendDist`; the explicit window `url` makes them unnecessary.

## Verified state (Linux, WebKitGTK 2.54, Tauri 2.12)

- Builds (debug + release) and launches; figma.com marketing page loads and renders, no unsupported-browser page.
- **Not verified:** logging in, opening a file (WebGL canvas), Google SSO, popups.

## Known risks / troubleshooting

- The Linux engine is **WebKitGTK, not Chromium**. Figma targets Chromium, so expect rough edges the Electron app did not have. Don't promise parity.
- Blank/white canvas or flicker: try `WEBKIT_DISABLE_DMABUF_RENDERER=1 cargo run`, then `WEBKIT_DISABLE_COMPOSITING_MODE=1`. Only bake one into the app if it is confirmed necessary.
- If figma.com serves an unsupported-browser page, set a Safari-style `userAgent` on the window in `tauri.conf.json`.
- Google SSO often refuses embedded webviews and Tauri does not open `window.open` popups by default. Email/password login should work. Popup handling (`on_new_window`) is not implemented; add it only if login/links are confirmed broken.
- Startup logs `GStreamer element autoaudiosink not found` (and a harmless-looking GLib-GObject CRITICAL). Audio/video in the webview likely needs `gst-plugins-good`; unconfirmed.
