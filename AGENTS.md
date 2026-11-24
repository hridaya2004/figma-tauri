# AGENTS.md

Desktop Figma client: a Tauri 2 window that loads `https://www.figma.com/`. Replaces the outdated, buggy Electron wrapper. There is no frontend and no Node project; the only custom Rust is the popup and download handlers in `src/main.rs`. Keep it that way unless asked.

## Layout

- `tauri.conf.json`: one window, `url` = figma.com, `create: false` (the window is built in `main.rs`, so the popup hook can be attached). Change window size/title/url here.
- `src/main.rs`: builds the main window from that config and handles `window.open` popups by opening them as extra Tauri windows with `window_features(...)`, which keeps `window.opener` (needed for Google SSO to report back to Figma). wry's own `window.close()` handler destroys the webview but leaves the window, so we also close the window from the webview's GTK `destroy` signal (a later `close` handler never runs: wry's destroys the widget first). Hence the Linux-only `gtk` + `webkit2gtk` deps.
- Downloads (`on_download` in `main.rs`): WebKitGTK has no save prompt, so a download goes to `~/.cache/dev.figma-tauri.client/downloads/<n>/` first; when it finishes a native save dialog opens (non-blocking), and the file is moved to the chosen path. Cancel deletes it; errors show a dialog. `tauri-plugin-dialog` is used from Rust only (no capability granted, so pages can't open dialogs).
- `build.rs`: `tauri_build::build()`.
- `icons/icon.png`: copied from the locally installed `figma-linux` package (512x512).
- `figma-tauri.desktop`: template desktop entry. On Wayland the taskbar/titlebar icon comes from a desktop file matching the app id, which is the binary name (`enableGTKAppId` is off), so the file must stay named `figma-tauri.desktop`. Install locally (icon display unconfirmed by an agent):
  `install -Dm644 icons/icon.png ~/.local/share/icons/hicolor/512x512/apps/figma-tauri.png`
  `sed "s|^Exec=.*|Exec=$PWD/target/release/figma-tauri|" figma-tauri.desktop > ~/.local/share/applications/figma-tauri.desktop`
- `LICENSE`: MIT, covers this wrapper's code only (not Figma, its trademarks, or its web app).
- `aur/`: package files, see Packaging below.
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
- Logged in and the Recents page loads inside the app (the user's session persisted), so login through the popup works.
- Popup: with a local test page, `window.open` opens a popup with `window.opener` set, `postMessage` reaches the opener, and `window.close()` removes the window. (The test needed a temporary patch: WebKitGTK blocks `window.open` without a user gesture.)
- Downloads: temp download + dialog request verified against a local `Content-Disposition: attachment` server (file landed with the right name, dialog requested with the right default). **Not seen on screen** (screen locked during testing): the dialog UI, saving, cancel.
- Design file opens (Intel+NVIDIA laptop, Wayland, no env vars): WebGL2 canvas boots, multiplayer joins, `loadtime=9.66s`. Two earlier launches sat on the loading bar (about 95%) with the skeleton UI. Cause not found: `WEBKIT_DISABLE_DMABUF_RENDERER=1` did not help, and the run that worked had no env vars. In that run Figma's multiplayer websocket closed and reconnected 3 times before joining, so a stall can be that retry loop. Reproducing it needs the page console: set `enable_write_console_messages_to_stdout` via `with_webview` (debug only) and read stdout.
- **Not verified:** email/password login.

## Known risks / troubleshooting

- The Linux engine is **WebKitGTK, not Chromium**. Figma targets Chromium, so expect rough edges the Electron app did not have. Don't promise parity.
- Blank/white canvas or flicker: try `WEBKIT_DISABLE_DMABUF_RENDERER=1 cargo run`, then `WEBKIT_DISABLE_COMPOSITING_MODE=1`. Only bake one into the app if it is confirmed necessary.
- If figma.com serves an unsupported-browser page, set a Safari-style `userAgent` on the window in `tauri.conf.json`.
- If Google SSO ever refuses the embedded webview ("This browser or app may not be secure"), set a Safari/Firefox-style `userAgent` on the window in `tauri.conf.json`. It has not been needed so far.
- All `window.open`/`target=_blank` links open as bare in-app windows with no URL bar (including external sites). Route non-Figma/Google URLs to the system browser only if that becomes annoying.
- Startup logs `GStreamer element autoaudiosink not found` (and a harmless-looking GLib-GObject CRITICAL). Audio/video in the webview likely needs `gst-plugins-good`; unconfirmed.

## Packaging (paru / AUR)

- `aur/PKGBUILD` (package `figma-tauri-git`) and `aur/.SRCINFO`. Source is `git+https://github.com/hridaya2004/figma-tauri.git`, i.e. the remote's **default branch** (the code must be merged to `main` and pushed before paru can build it). Lives in `aur/`, not the repo root, because makepkg uses `./src` as scratch space and would collide with the Rust `src/`.
- Depends: `webkit2gtk-4.1`, `gtk3` (optional: `gst-plugins-good`). `options=('!lto')` because it is a Rust build. Installs the binary, `figma-tauri.desktop`, the 512px icon and `LICENSE` (MIT).
- Install from a checkout: `cd aur && makepkg -si`, or `paru -B aur` (the paru form is untested; it needs sudo).
- After editing the PKGBUILD: `cd aur && makepkg --printsrcinfo > .SRCINFO`.
- Test without pushing: copy `PKGBUILD` to a temp dir, change `source` to `git+file:///path/to/this/repo#branch=<branch>` (changes must be committed; makepkg clones), run `makepkg -f`. Verified that way: builds, package holds binary/desktop/icon/license, no missing shared libs, packaged binary launches.
- `makepkg` warns `Package contains reference to $srcdir`: benign, Tauri's `generate_context!` bakes in the manifest dir as a literal. `--remap-path-prefix` does not remove it.
- Publishing to the real AUR (so anyone can `paru -S figma-tauri-git`): copy `PKGBUILD` + `.SRCINFO` into a clone of `ssh://aur@aur.archlinux.org/figma-tauri-git.git` and push. Needs an AUR account; **not done**.
- The icon is Figma's logo (trademark), taken from the figma-linux package. Think about that before publishing it anywhere.

## Parked: tab bar

A tab bar was built and tested, then parked at the user's request (they want "Figma's own" tabs, meaning unclear; the custom strip was rejected). It lives on the local branch `wip/tab-bar`, not in this branch. Findings worth keeping:

- Tauri multiwebview needs the `unstable` feature, and `custom-protocol` so `cargo run` serves the local strip.
- **On Linux Tauri packs child webviews into the window's `GtkBox` (expand=true) and ignores bounds** (`set_bounds` is a no-op there), so children split the window equally. The branch pins the strip's height via `with_webview` + GTK `set_child_packing`.
- The branch's strip IPC was locked down with an `AppManifest` in `build.rs` plus `capabilities/tabs.json` scoped to the strip webview; pages in tabs were verified to be denied. If tabs return, keep that, and update the "no IPC" and `frontendDist` rules above.
- Not built: Ctrl+T/W/Tab shortcuts, intercepting same-tab file navigations (only `window.open` of figma file URLs became tabs).
