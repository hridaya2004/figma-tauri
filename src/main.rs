use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
};
use tauri::{
    webview::{DownloadEvent, NewWindowResponse, WebviewWindowBuilder},
    AppHandle, Manager, Runtime, Webview, WebviewUrl,
};
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};

static DOWNLOADS: AtomicUsize = AtomicUsize::new(0);

fn error_dialog<R: Runtime>(app: &AppHandle<R>, msg: impl Into<String>) {
    app.dialog()
        .message(msg)
        .kind(MessageDialogKind::Error)
        .title("Figma Tauri")
        .show(|_| {});
}

fn move_file(from: &Path, to: &Path) -> std::io::Result<()> {
    // rename fails across filesystems; fall back to copy + delete
    fs::rename(from, to).or_else(|_| {
        fs::copy(from, to)?;
        fs::remove_file(from)
    })
}

// WebKitGTK has no save prompt and wry only sets a destination if we do. Download to a
// private temp path, then ask where to save once it finishes (never blocks the GTK loop).
fn on_download<R: Runtime>(wv: Webview<R>, event: DownloadEvent<'_>) -> bool {
    let app = wv.app_handle().clone();
    match event {
        DownloadEvent::Requested { destination, .. } => {
            let (Some(name), Ok(cache)) = (destination.file_name(), app.path().app_cache_dir())
            else {
                return false;
            };
            let id = DOWNLOADS.fetch_add(1, Ordering::Relaxed);
            let dir = cache.join("downloads").join(id.to_string());
            if fs::create_dir_all(&dir).is_err() {
                return false;
            }
            *destination = dir.join(name);
            true
        }
        DownloadEvent::Finished { path: Some(tmp), success: true, .. } => {
            save_dialog(app, tmp);
            true
        }
        _ => {
            error_dialog(&app, "Download failed.");
            true
        }
    }
}

fn save_dialog<R: Runtime>(app: AppHandle<R>, tmp: PathBuf) {
    let name = tmp.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let mut dialog = app.dialog().file().set_file_name(name);
    if let Ok(dir) = app.path().download_dir() {
        dialog = dialog.set_directory(dir);
    }
    dialog.save_file(move |dest| match dest.and_then(|p| p.into_path().ok()) {
        Some(dest) => {
            if let Err(e) = move_file(&tmp, &dest) {
                error_dialog(
                    &app,
                    format!("Couldn't save to {}: {e}\nThe download is still at {}", dest.display(), tmp.display()),
                );
            }
        }
        None => {
            let _ = fs::remove_file(&tmp); // cancelled
        }
    });
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // drop temp files left by a crash mid-download
            if let Ok(cache) = app.path().app_cache_dir() {
                let _ = fs::remove_dir_all(cache.join("downloads"));
            }
            let handle = app.handle().clone();
            let popups = AtomicUsize::new(0);
            // Built in code (config has `create: false`) to hook popups: Figma's
            // "Continue with Google" uses window.open and needs window.opener back.
            WebviewWindowBuilder::from_config(app.handle(), &app.config().app.windows[0])?
                .on_download(on_download)
                .on_new_window(move |url, features| {
                    let label = format!("popup-{}", popups.fetch_add(1, Ordering::Relaxed));
                    let window = WebviewWindowBuilder::new(
                        &handle,
                        label,
                        WebviewUrl::External("about:blank".parse().unwrap()),
                    )
                    .window_features(features)
                    .title(url.as_str())
                    .build()
                    .expect("failed to open popup window");
                    // wry's window.close() handler destroys the webview but leaves the
                    // empty window behind (and disconnects any later `close` handler),
                    // so close the window when the webview is destroyed.
                    #[cfg(target_os = "linux")]
                    {
                        let w = window.clone();
                        let _ = window.with_webview(move |pw| {
                            use gtk::prelude::WidgetExt;
                            pw.inner().connect_destroy(move |_| {
                                let _ = w.close();
                            });
                        });
                    }
                    NewWindowResponse::Create { window }
                })
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
