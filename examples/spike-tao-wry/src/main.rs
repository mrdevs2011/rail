// Minimal tao + wry spike
// Sinov: bitta oyna ochiladimi, ichida oddiy HTML ko'rinadimi.
// Ishga tushirish: cd examples/spike-tao-wry && cargo run

use tao::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use wry::WebViewBuilder;

fn main() -> wry::Result<()> {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Rail spike — tao + wry")
        .build(&event_loop)
        .unwrap();

    let _webview = WebViewBuilder::new()
        .with_url("https://example.com")
        // yoki data URL:
        // .with_html(r#"<html><body style='background:#111;color:#fff;font-family:sans-serif'><h1>Rail spike</h1><p>tao + wry ishlayapti</p></body></html>"#)
        .build(&window)?;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        if let Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } = event
        {
            *control_flow = ControlFlow::Exit;
        }
    });
}
