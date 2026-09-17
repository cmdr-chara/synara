use gpui::{
    App, Bounds, Context, Window, WindowBounds, WindowOptions, div, prelude::*, px, rgb, size,
};
struct Shell;
impl Render for Shell {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(0x11151c))
            .text_color(rgb(0xe4eaf3))
            .p_8()
            .gap_4()
            .child(div().text_3xl().child("Synara"))
            .child("Native coding-agent workspace")
            .child("Select a workspace and configure an ACP agent to begin.")
    }
}
fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("synara=info")
        .init();
    gpui_platform::application().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1280.0), px(840.0)), cx);
        if let Err(error) = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| cx.new(|_| Shell),
        ) {
            tracing::error!(%error,"Could not open the application window");
            cx.quit();
            return;
        }
        cx.activate(true);
    });
}
