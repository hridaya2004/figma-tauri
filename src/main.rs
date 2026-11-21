use std::sync::atomic::{AtomicUsize, Ordering};
use tauri::{
    webview::{NewWindowResponse, WebviewWindowBuilder},
    WebviewUrl,
};

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let handle = app.handle().clone();
            let popups = AtomicUsize::new(0);
            // Built in code (config has `create: false`) to hook popups: Figma's
            // "Continue with Google" uses window.open and needs window.opener back.
            WebviewWindowBuilder::from_config(app.handle(), &app.config().app.windows[0])?
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
                    NewWindowResponse::Create { window }
                })
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
