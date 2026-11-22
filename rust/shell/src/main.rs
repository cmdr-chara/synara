use gpui_kit::component::{
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{Input, InputEvent, InputState},
    menu::{DropdownMenu as _, PopupMenuItem},
    v_flex, ActiveTheme as _, IconName,
};
use gpui_kit::*;
use gpui_webview::WebView;

const DEFAULT_URL: &str = "http://127.0.0.1:4577";

pub struct Shell {
    focus_handle: FocusHandle,
    webview: Entity<WebView>,
    address_input: Entity<InputState>,
}

impl Shell {
    pub fn new(window: &mut Window, cx: &mut App) -> Entity<Self> {
        let webview = cx.new(|cx| {
            let builder = wry::WebViewBuilder::new();
            #[cfg(debug_assertions)]
            let builder = builder.with_devtools(true);

            WebView::build(builder, window, cx).expect("Failed to create WebView")
        });

        let url = std::env::var("SYNARA_URL").unwrap_or_else(|_| DEFAULT_URL.to_string());
        let address_input = cx.new(|cx| InputState::new(window, cx).default_value(url.as_str()));

        webview.update(cx, |view, _| {
            view.load_url(&url);
        });

        cx.new(|cx| {
            let this = Self {
                focus_handle: cx.focus_handle(),
                webview,
                address_input: address_input.clone(),
            };

            cx.subscribe(
                &address_input,
                |this: &mut Self, input, event: &InputEvent, cx| {
                    if let InputEvent::PressEnter { .. } = event {
                        let url = input.read(cx).value();
                        this.webview.update(cx, |view, _| {
                            view.load_url(&url);
                        });
                    }
                },
            )
            .detach();

            this
        })
    }

    fn go_back(&mut self, _: &ClickEvent, _: &mut Window, cx: &mut Context<Self>) {
        self.webview.update(cx, |webview, _| {
            let _ = webview.back();
        });
    }

    fn go_forward(&mut self, _: &ClickEvent, _: &mut Window, cx: &mut Context<Self>) {
        self.webview.update(cx, |webview, _| {
            let _ = webview.forward();
        });
    }
}

impl Focusable for Shell {
    fn focus_handle(&self, _cx: &gpui_kit::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for Shell {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let webview = self.webview.clone();

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
                                let webview = self.webview.clone();
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

    app.with_assets(gpui_kit::assets::Assets).run(move |cx| {
        gpui_kit::init(cx);

        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                None,
                size(px(1440.), px(900.)),
                cx,
            ))),
            ..Default::default()
        };

        gpui_kit::open_window(options, cx, Shell::new).expect("Failed to open window");
    });
}
