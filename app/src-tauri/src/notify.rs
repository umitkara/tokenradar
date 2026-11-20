//! Desktop notifications: shows what the core's notifier decided (ADR-033). Rules, mute and
//! quiet hours live in the core; this module only puts the text on screen.
//!
//! On Windows the toast is built here instead of by `tauri-plugin-notification` (ADR-040).
//! Windows reports a click from the Action Center only to an app with a COM activator, and the
//! plugin has no way to name one. Our toast uses protocol activation instead: a click, on screen
//! or in the Action Center, starts `tokenradar-app.exe tokenradar://panel` (a summary:
//! `tokenradar://main/overview`, Step 17b). The installer
//! registers the scheme (`plugins > deep-link` in `tauri.conf.json`). The single-instance plugin
//! hands that launch to the running app, and a cold start opens the panel itself (`init`).
//! A portable copy registers no scheme, so there a click does nothing. The same launch path
//! carries `tokenradar://main` from `tokenradar open` (Step 15, `tokenradar_core::deeplink`).
//!
//! Windows names the sender by the `AppUserModelID`. Builds run from `target\debug` or
//! `target\release` show as PowerShell; any other copy uses the app identifier, which only the
//! installer's Start menu shortcut registers. A portable copy may show no toast at all, without
//! an error.

use tauri::AppHandle;
use tokenradar_core::deeplink::{Link, MainRoute};
use tokenradar_core::notifier::{Notice, NoticeTarget};

#[cfg(not(windows))]
use crate::panel;

/// What a limit alert's click launches. Registered by the installer only.
#[cfg(test)]
const PANEL_URI: &str = "tokenradar://panel";

/// Opens what a `tokenradar://` launch argument names (a toast click, `tokenradar open`). On
/// other platforms it also registers the plugin's click handler; toasts shown before this report
/// nothing.
pub fn init(app: &AppHandle) {
    if let Some(link) = Link::find(std::env::args().skip(1)) {
        crate::open_link(app, link);
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

/// Shows `notice` as a toast; a click opens what its target names.
pub fn show(app: &AppHandle, notice: &Notice) {
    let link = match notice.target {
        NoticeTarget::Panel => Link::Panel,
        NoticeTarget::Overview => Link::Main(Some(MainRoute::Overview)),
    };
    show_linked(app, &notice.title, &notice.body, link);
}

/// Shows a toast that no notifier rule decided (the widget's "could not dock", Step 18b).
pub fn show_text(app: &AppHandle, title: &str, body: &str) {
    show_linked(app, title, body, Link::Panel);
}

fn show_linked(app: &AppHandle, title: &str, body: &str, link: Link) {
    #[cfg(windows)]
    {
        let app_id = toast::app_id(&app.config().identifier);
        let (title, body, launch) = (title.to_owned(), body.to_owned(), link.uri());
        // WinRT activation can take a moment on first use; keep it off the event loop.
        tauri::async_runtime::spawn_blocking(move || {
            if let Err(error) = toast::show(&app_id, &title, &body, &launch) {
                tracing::warn!(%error, "could not show a notification");
            }
        });
    }
    #[cfg(not(windows))]
    {
        // The plugin's click opens the panel whatever the link (Step 21).
        let _ = link;
        use tauri_plugin_notification::NotificationExt as _;
        let result = app.notification().builder().title(title).body(body).show();
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
    /// activation of `launch` (a `tokenradar://` link). Text goes in through the DOM, so it
    /// cannot change the markup.
    pub fn document(title: &str, body: &str, launch: &str) -> windows::core::Result<XmlDocument> {
        let doc = XmlDocument::new()?;
        let toast = doc.CreateElement(h!("toast"))?;
        toast.SetAttribute(h!("duration"), h!("short"))?;
        toast.SetAttribute(h!("activationType"), h!("protocol"))?;
        toast.SetAttribute(h!("launch"), &HSTRING::from(launch))?;
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

    pub fn show(app_id: &str, title: &str, body: &str, launch: &str) -> windows::core::Result<()> {
        let toast = ToastNotification::CreateToastNotification(&document(title, body, launch)?)?;
        ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(app_id))?.Show(&toast)
    }

    #[cfg(test)]
    mod tests {
        use super::super::PANEL_URI;
        use super::*;

        #[test]
        fn toast_launches_the_panel_uri_and_keeps_text_as_text() {
            let xml = document("5h <90%> & \"up\"", "body", PANEL_URI)
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
        fn a_summary_toast_launches_the_overview() {
            let launch = tokenradar_core::deeplink::Link::Main(Some(
                tokenradar_core::deeplink::MainRoute::Overview,
            ))
            .uri();
            let xml = document("t", "b", &launch)
                .expect("toast document")
                .GetXml()
                .expect("toast xml")
                .to_string();
            assert!(
                xml.contains("launch=\"tokenradar://main/overview\""),
                "{xml}"
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toast_uri_is_the_panel_link() {
        assert_eq!(Link::parse(PANEL_URI), Some(Link::Panel));
        assert_eq!(Link::Panel.uri(), PANEL_URI);
    }
}
