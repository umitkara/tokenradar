//! Desktop notifications: shows what the core's notifier decided (ADR-033). Rules, mute and
//! quiet hours live in the core; this module only puts the text on screen.
//!
//! On Windows the toast is built here instead of by `tauri-plugin-notification` (ADR-040).
//! Windows reports a click from the Action Center only to an app with a COM activator, and the
//! plugin has no way to name one. Our toast uses protocol activation instead: a click, on screen
//! or in the Action Center, starts `tokenradar-app.exe tokenradar://panel`. The installer
//! registers the scheme (`plugins > deep-link` in `tauri.conf.json`). The single-instance plugin
//! hands that launch to the running app, and a cold start opens the panel itself (`init`).
//! A portable copy registers no scheme, so there a click does nothing.
//!
//! Windows names the sender by the `AppUserModelID`. Builds run from `target\debug` or
//! `target\release` show as PowerShell; any other copy uses the app identifier, which only the
//! installer's Start menu shortcut registers. A portable copy may show no toast at all, without
//! an error.

use tauri::AppHandle;
use tokenradar_core::notifier::Notice;

use crate::panel;

/// What a toast click launches. Registered by the installer only.
pub const PANEL_URI: &str = "tokenradar://panel";

/// Opens the panel when this process was started by a toast click. On other platforms it
/// registers the plugin's click handler; toasts shown before this report nothing.
pub fn init(app: &AppHandle) {
    #[cfg(windows)]
    if std::env::args().skip(1).any(|arg| arg == PANEL_URI) {
        panel::report(panel::show(app));
    }
    #[cfg(not(windows))]
    {
        use tauri_plugin_notification::NotificationExt as _;
        let handle = app.clone();
        let _ = app.notification().on_action(move |performed| {
            if performed.action_id() == "tap" {
                panel::report(panel::show(&handle));
            }
        });
    }
}

/// Shows `notice` as a toast.
pub fn show(app: &AppHandle, notice: &Notice) {
    #[cfg(windows)]
    {
        let app_id = toast::app_id(&app.config().identifier);
        let (title, body) = (notice.title.clone(), notice.body.clone());
        // WinRT activation can take a moment on first use; keep it off the event loop.
        tauri::async_runtime::spawn_blocking(move || {
            if let Err(error) = toast::show(&app_id, &title, &body) {
                tracing::warn!(%error, "could not show a notification");
            }
        });
    }
    #[cfg(not(windows))]
    {
        use tauri_plugin_notification::NotificationExt as _;
        let result = app
            .notification()
            .builder()
            .title(&notice.title)
            .body(&notice.body)
            .show();
        if let Err(error) = result {
            tracing::warn!(%error, "could not show a notification");
        }
    }
}

#[cfg(windows)]
mod toast {
    use std::path::{MAIN_SEPARATOR, Path};

    use windows::Data::Xml::Dom::XmlDocument;
    use windows::UI::Notifications::{ToastNotification, ToastNotificationManager};
    use windows::core::{HSTRING, h};

    use super::PANEL_URI;

    /// The sender Windows shows for builds that are not installed, as the plugin does.
    const POWERSHELL_APP_ID: &str =
        r"{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}\WindowsPowerShell\v1.0\powershell.exe";

    /// The `AppUserModelID` to send under: the app identifier, except for cargo builds, whose
    /// identifier Windows does not know (the plugin's rule, ADR-033).
    pub fn app_id(identifier: &str) -> String {
        let exe = std::env::current_exe().ok();
        let dir = exe.as_deref().and_then(Path::parent);
        if dir.is_some_and(is_cargo_target) {
            POWERSHELL_APP_ID.to_owned()
        } else {
            identifier.to_owned()
        }
    }

    fn is_cargo_target(dir: &Path) -> bool {
        let dir = dir.to_string_lossy();
        ["debug", "release"].iter().any(|profile| {
            dir.ends_with(&format!("{MAIN_SEPARATOR}target{MAIN_SEPARATOR}{profile}"))
        })
    }

    /// The toast the plugin used to build (silent, short, `ToastGeneric`), plus protocol
    /// activation. Text goes in through the DOM, so it cannot change the markup.
    pub fn document(title: &str, body: &str) -> windows::core::Result<XmlDocument> {
        let doc = XmlDocument::new()?;
        let toast = doc.CreateElement(h!("toast"))?;
        toast.SetAttribute(h!("duration"), h!("short"))?;
        toast.SetAttribute(h!("activationType"), h!("protocol"))?;
        toast.SetAttribute(h!("launch"), &HSTRING::from(PANEL_URI))?;
        let visual = doc.CreateElement(h!("visual"))?;
        let binding = doc.CreateElement(h!("binding"))?;
        binding.SetAttribute(h!("template"), h!("ToastGeneric"))?;
        for (id, text) in [("1", title), ("2", body)] {
            let element = doc.CreateElement(h!("text"))?;
            element.SetAttribute(h!("id"), &HSTRING::from(id))?;
            element.SetInnerText(&HSTRING::from(text))?;
            binding.AppendChild(&element)?;
        }
        visual.AppendChild(&binding)?;
        toast.AppendChild(&visual)?;
        let audio = doc.CreateElement(h!("audio"))?;
        audio.SetAttribute(h!("silent"), h!("true"))?;
        toast.AppendChild(&audio)?;
        doc.AppendChild(&toast)?;
        Ok(doc)
    }

    pub fn show(app_id: &str, title: &str, body: &str) -> windows::core::Result<()> {
        let toast = ToastNotification::CreateToastNotification(&document(title, body)?)?;
        ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(app_id))?.Show(&toast)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn toast_launches_the_panel_uri_and_keeps_text_as_text() {
            let xml = document("5h <90%> & \"up\"", "body")
                .expect("toast document")
                .GetXml()
                .expect("toast xml");
            assert_eq!(
                xml.to_string(),
                "<toast duration=\"short\" activationType=\"protocol\" \
                 launch=\"tokenradar://panel\"><visual><binding template=\"ToastGeneric\">\
                 <text id=\"1\">5h &lt;90%&gt; &amp; \"up\"</text><text id=\"2\">body</text>\
                 </binding></visual><audio silent=\"true\"/></toast>"
            );
        }

        #[test]
        fn cargo_builds_send_as_powershell() {
            let sep = MAIN_SEPARATOR;
            assert!(is_cargo_target(Path::new(&format!(
                r"C:{sep}repo{sep}target{sep}release"
            ))));
            assert!(is_cargo_target(Path::new(&format!(
                r"C:{sep}repo{sep}target{sep}debug"
            ))));
            assert!(!is_cargo_target(Path::new(
                r"C:\Users\u\AppData\Local\Programs\TokenRadar"
            )));
        }
    }
}
