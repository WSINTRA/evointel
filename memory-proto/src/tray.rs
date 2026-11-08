//! Menu-bar tray icon — the daemon's visible presence.
//!
//! Three states: idle, dreaming, notification. The icon swaps when
//! the shared atomic state changes. Menu: status, open folder, quit.
//!
//! On macOS the tray must be created on the main thread after the
//! Cocoa event loop has started. `tao` owns that loop; menu events
//! are forwarded into it and the shared state is read on each tick.

/// Tray icon states, stored in the shared `AtomicU8`.
pub const ICON_IDLE: u8 = 0;
pub const ICON_DREAMING: u8 = 1;
pub const ICON_NOTIFY: u8 = 2;

#[cfg(target_os = "macos")]
mod platform {
    use super::{ICON_DREAMING, ICON_IDLE, ICON_NOTIFY};
    use std::process::Command;
    use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    use tao::event::{Event, StartCause};
    use tao::event_loop::{ControlFlow, EventLoopBuilder};
    use tao::platform::macos::{ActivationPolicy, EventLoopExtMacOS};
    use tao::platform::run_return::EventLoopExtRunReturn;
    use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
    use tray_icon::{TrayIcon, TrayIconBuilder};

    /// Embedded icon PNGs.
    const IDLE_PNG: &[u8] = include_bytes!("../assets/tray-idle.png");
    const DREAMING_PNG: &[u8] = include_bytes!("../assets/tray-dreaming.png");
    const NOTIFY_PNG: &[u8] = include_bytes!("../assets/tray-notify.png");

    const STATE_POLL_INTERVAL: Duration = Duration::from_millis(500);

    enum UserEvent {
        Menu(MenuEvent),
    }

    /// Run the tray icon on the current thread. Returns after Quit or
    /// shutdown so the caller can join the daemon thread.
    ///
    /// - `state`: shared atomic for the icon state (idle/dreaming/notify).
    /// - `shutdown`: set to `true` when the user clicks Quit.
    pub fn run_tray(state: &Arc<AtomicU8>, shutdown: &Arc<AtomicBool>) {
        let idle_icon = load_icon(IDLE_PNG);
        let dreaming_icon = load_icon(DREAMING_PNG);
        let notify_icon = load_icon(NOTIFY_PNG);

        // Build the menu before entering the event loop, following the
        // upstream tray-icon tao example.
        let status_item = MenuItem::new("Wunicorm — Running", false, None);
        let open_item = MenuItem::new("Open Wunicorm Folder", true, None);
        let quit_item = MenuItem::new("Quit", true, None);
        let menu = Menu::new();
        menu.append_items(&[
            &status_item,
            &PredefinedMenuItem::separator(),
            &open_item,
            &PredefinedMenuItem::separator(),
            &quit_item,
        ])
        .expect("build tray menu");

        let mut event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
        event_loop.set_activation_policy(ActivationPolicy::Accessory);

        // Forward native menu events into tao so its run loop wakes up.
        let proxy = event_loop.create_proxy();
        MenuEvent::set_event_handler(Some(move |event| {
            let _ = proxy.send_event(UserEvent::Menu(event));
        }));

        let open_id = open_item.id().clone();
        let quit_id = quit_item.id().clone();
        let state = Arc::clone(state);
        let shutdown = Arc::clone(shutdown);
        let mut tray: Option<TrayIcon> = None;
        let mut current_state = ICON_IDLE;

        event_loop.run_return(move |event, _, control_flow| {
            *control_flow = ControlFlow::WaitUntil(Instant::now() + STATE_POLL_INTERVAL);

            match event {
                Event::NewEvents(StartCause::Init) => {
                    // The event loop is now running, which is a macOS
                    // requirement for a reliable status item.
                    tray = Some(
                        TrayIconBuilder::new()
                            .with_menu(Box::new(menu.clone()))
                            .with_tooltip("Wunicorm")
                            .with_icon(idle_icon.clone())
                            .build()
                            .expect("create tray icon"),
                    );

                    // Ask Cocoa to draw the newly created status item.
                    let run_loop = objc2_core_foundation::CFRunLoop::main().expect("main run loop");
                    run_loop.wake_up();
                }
                Event::UserEvent(UserEvent::Menu(event)) => {
                    if event.id() == &open_id {
                        open_wunicorm_folder();
                    } else if event.id() == &quit_id {
                        shutdown.store(true, Ordering::Relaxed);
                    }
                }
                Event::MainEventsCleared => {
                    if let Some(tray) = &tray {
                        let new_state = state.load(Ordering::Relaxed);
                        if new_state != current_state {
                            let icon = match new_state {
                                ICON_DREAMING => dreaming_icon.clone(),
                                ICON_NOTIFY => notify_icon.clone(),
                                _ => idle_icon.clone(),
                            };
                            tray.set_icon(Some(icon)).expect("set tray icon");
                            current_state = new_state;
                        }
                    }
                }
                _ => {}
            }

            if shutdown.load(Ordering::Relaxed) {
                tray.take();
                *control_flow = ControlFlow::Exit;
            }
        });
    }

    /// Open the "Wunicorm" folder in Mac Notes.
    fn open_wunicorm_folder() {
        let _ = Command::new("osascript")
            .arg("-e")
            .arg(
                r#"tell application "Notes"
    activate
    reveal folder "Wunicorm"
end tell"#,
            )
            .output();
    }

    /// Load a PNG into a tray-icon `Icon`.
    fn load_icon(png_bytes: &[u8]) -> tray_icon::Icon {
        let img = image::load_from_memory(png_bytes).expect("decode tray icon PNG");
        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();
        tray_icon::Icon::from_rgba(rgba.into_raw(), width, height).expect("create tray icon")
    }
}

#[cfg(target_os = "macos")]
pub use platform::run_tray;

/// Non-macOS builds retain the library and tests without pulling in native
/// windowing dependencies. The daemon's tray UI is available only on macOS.
#[cfg(not(target_os = "macos"))]
pub fn run_tray(
    _state: &std::sync::Arc<std::sync::atomic::AtomicU8>,
    _shutdown: &std::sync::Arc<std::sync::atomic::AtomicBool>,
) {
    eprintln!("the Wunicorm tray is available only on macOS");
}
