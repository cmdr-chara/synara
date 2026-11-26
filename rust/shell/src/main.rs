use gpui_kit::component::{
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{Input, InputEvent, InputState},
    menu::{DropdownMenu as _, PopupMenuItem},
    v_flex, ActiveTheme as _, IconName,
};
use gpui_kit::*;
use gpui_webview::WebView;

mod backdrop;
mod backend;
mod browser_host;
mod cua;
mod native;
mod updater;
mod voice;
mod window_material;
mod window_state;
mod ws_client;

const DEFAULT_URL: &str = "http://127.0.0.1:4577";

pub struct Shell {
    focus_handle: FocusHandle,
    webview: Option<Entity<WebView>>,
    native: Entity<native::Native>,
    address_input: Entity<InputState>,
    last_window_state_write: std::time::Instant,
    desktop_backgrounded_at: Option<std::time::Instant>,
    window_state_subscription: Option<Subscription>,
}

impl Shell {
    pub fn new(window: &mut Window, cx: &mut App) -> Entity<Self> {
        // Native GPUI UI is the default; SYNARA_WEBVIEW=1 re-enables the
        // legacy webview shell during the transition away from it.
        let native_ui = std::env::var("SYNARA_WEBVIEW").is_err();
        let webview = if native_ui {
            // In native mode the WKWebView subview is never rendered; creating
            // it anyway leaves an invisible native view over the window that
            // intercepts scroll-wheel events (scroll stops working).
            None
        } else {
            let webview = cx.new(|cx| {
                let builder = wry::WebViewBuilder::new();
                #[cfg(debug_assertions)]
                let builder = builder.with_devtools(true);

                WebView::build(builder, window, cx).expect("Failed to create WebView")
            });
            Some(webview)
        };

        let url = std::env::var("SYNARA_URL").unwrap_or_else(|_| DEFAULT_URL.to_string());
        let address_input = cx.new(|cx| InputState::new(window, cx).default_value(url.as_str()));

        if let Some(webview) = &webview {
            webview.update(cx, |view, _| {
                view.load_url(&url);
            });
        }

        cx.new(|cx| {
            let mut this = Self {
                focus_handle: cx.focus_handle(),
                webview,
                native: native::Native::new(window, cx),
                address_input: address_input.clone(),
                last_window_state_write: std::time::Instant::now()
                    - std::time::Duration::from_secs(1),
                desktop_backgrounded_at: None,
                window_state_subscription: None,
            };
            this.window_state_subscription = Some(cx.observe_window_bounds(
                window,
                |this: &mut Self, window: &mut Window, _cx: &mut Context<Self>| {
                    if this.last_window_state_write.elapsed()
                        >= std::time::Duration::from_millis(300)
                    {
                        window_state::save_from_window(window);
                        this.last_window_state_write = std::time::Instant::now();
                    }
                },
            ));

            // Native owns most workspace state and changes it from background
            // websocket/updater tasks. GPUI does not automatically invalidate
            // the parent merely because a child entity was passed through, so
            // forward Native notifications into the root view's render pass.
            cx.observe(&this.native, |_, _, cx| cx.notify()).detach();
            cx.observe_window_activation(window, |this, window, _| {
                if window.is_window_active() {
                    if let Some(backgrounded_at) = this.desktop_backgrounded_at.take() {
                        updater::note_desktop_foregrounded(backgrounded_at.elapsed());
                    }
                } else if this.desktop_backgrounded_at.is_none() {
                    this.desktop_backgrounded_at = Some(std::time::Instant::now());
                }
            })
            .detach();
            cx.subscribe(
                &address_input,
                |this: &mut Self, input, event: &InputEvent, cx| {
                    if let InputEvent::PressEnter { .. } = event {
                        let url = input.read(cx).value();
                        if let Some(webview) = &this.webview {
                            webview.update(cx, |view, _| {
                                view.load_url(&url);
                            });
                        }
                    }
                },
            )
            .detach();

            this
        })
    }

    fn go_back(&mut self, _: &ClickEvent, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(webview) = &self.webview {
            webview.update(cx, |webview, _| {
                let _ = webview.back();
            });
        }
    }

    fn go_forward(&mut self, _: &ClickEvent, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(webview) = &self.webview {
            webview.update(cx, |webview, _| {
                let _ = webview.forward();
            });
        }
    }
}

impl Focusable for Shell {
    fn focus_handle(&self, _cx: &gpui_kit::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for Shell {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(webview) = self.webview.clone() else {
            return v_flex()
                .size_full()
                .child(self.native.clone());
        };

        v_flex()
            .size_full()
            .child(
                h_flex()
                    .p_2()
                    .gap_2()
                    .items_center()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        Button::new("back")
                            .ghost()
                            .icon(IconName::ChevronLeft)
                            .on_click(cx.listener(Self::go_back)),
                    )
                    .child(
                        Button::new("forward")
                            .ghost()
                            .icon(IconName::ChevronRight)
                            .on_click(cx.listener(Self::go_forward)),
                    )
                    .child(Input::new(&self.address_input))
                    .child(
                        Button::new("more")
                            .ghost()
                            .icon(IconName::Ellipsis)
                            .dropdown_menu({
                                let webview = webview.clone();
                                move |menu, _, _| {
                                    let reload = webview.clone();
                                    menu.item(PopupMenuItem::new("Reload").on_click(
                                        move |_, _, cx| {
                                            let _ = reload.read(cx).raw().reload();
                                        },
                                    ))
                                }
                            }),
                    ),
            )
            .child(div().flex_1().child(webview))
    }
}

fn main() {
    if let Err(error) = updater::start_desktop_update_bridge() {
        eprintln!("desktop updater bridge unavailable: {error}");
    }
    let backend_launch = backend::ensure_backend();
    eprintln!(
        "backend supervision: {}",
        backend::launch_summary(&backend_launch)
    );
    if let Some(supervisor) = backend_launch.supervisor() {
        backend::install_signal_shutdown(supervisor);
    }

    // Required for Windows to render the WebView.
    #[cfg(target_os = "windows")]
    unsafe {
        std::env::set_var("GPUI_DISABLE_DIRECT_COMPOSITION", "true");
    }

    // WebKitGTK embeds only into X11 windows, so use XWayland in a Wayland session.
    #[cfg(target_os = "linux")]
    let app = gpui_kit::platform::linux(WindowingModes::X11);
    #[cfg(not(target_os = "linux"))]
    let app = gpui_kit::application();

    // Upstream desktop role: host visible browser automation for the server.
    browser_host::start_browser_host();

    app.with_assets(gpui_kit::assets::AllAssets).run(move |cx| {
        gpui_kit::init(cx);
        // Upstream display face for the wordmark (--font-display-family:
        // "Cal Sans"; SIL OFL 1.1, see assets/fonts/CalSans-OFL.txt).
        let _ = cx.text_system().add_fonts(vec![std::borrow::Cow::Borrowed(
            include_bytes!("../assets/fonts/CalSans-Regular.ttf").as_slice(),
        )]);

        let persisted_window = window_state::load();
        let mut options = WindowOptions {
            window_background: window_material::background_for_pack(
                window_material::current_translucency(),
                &serde_json::json!({}),
            ),
            window_bounds: Some(WindowBounds::Maximized(Bounds::centered(
                None,
                size(px(1440.), px(900.)),
                cx,
            ))),
            ..Default::default()
        };
        if let Some(bounds) = persisted_window.window_bounds {
            options.window_bounds = Some(bounds);
        }

        let (window_handle, _) =
            gpui_kit::open_window(options, cx, Shell::new).expect("Failed to open window");
        if let Some(native_state) = persisted_window.native_state {
            let _ = window_handle.update(cx, |_, window, _| {
                window.restore_native_window_state(&native_state);
            });
        }
    });
}
