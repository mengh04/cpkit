use gpui::*;
use gpui_component::{
    input::{InputState, TextInput},
    *,
};

struct HelloWorld {
    input: Entity<InputState>,
}

impl HelloWorld {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| InputState::new(window, cx));
        Self { input }
    }
}

impl Render for HelloWorld {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        v_flex().child(TextInput::new(&self.input))
    }
}

fn main() {
    let app = Application::new();

    app.run(move |cx| {
        gpui_component::init(cx);

        let window_size = size(px(400.0), px(850.0));
        let window_bounds = Bounds::centered(None, window_size, cx);

        cx.spawn(async move |cx| {
            let window_option = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(window_bounds)),
                ..Default::default()
            };

            cx.open_window(window_option, |window, cx| {
                let view = cx.new(|cx| HelloWorld::new(window, cx));
                cx.new(|cx| Root::new(view.into(), window, cx))
            })?;

            Ok::<_, anyhow::Error>(())
        })
        .detach();
    })
}
