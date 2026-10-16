// Tao + wry spike: 3 ta webview yonma-yon (lenta sinovi)
// Linux: default_vbox + gtk::Box
// Ishga tushirish: cd examples/spike-tao-wry && cargo run

use tao::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use wry::WebViewBuilder;

#[cfg(target_os = "linux")]
use tao::platform::unix::WindowExtUnix;
#[cfg(target_os = "linux")]
use wry::WebViewBuilderExtUnix;

fn main() -> wry::Result<()> {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Rail spike — 3 webview")
        .with_inner_size(tao::dpi::LogicalSize::new(1200.0, 700.0))
        .build(&event_loop)
        .expect("oyna ochilmadi");

    #[cfg(target_os = "linux")]
    {
        use gtk::prelude::*;
        let vbox = window.default_vbox().expect("default_vbox");
        let hbox = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        hbox.set_homogeneous(true);
        vbox.pack_start(&hbox, true, true, 0);
        hbox.show_all();

        for (i, color) in ["#1a1a2e", "#16213e", "#0f3460"].iter().enumerate() {
            let fixed = gtk::Fixed::new();
            fixed.set_size_request(380, 600);
            hbox.pack_start(&fixed, true, true, 0);
            fixed.show_all();

            let html = format!(
                r#"<!doctype html><html><body style="margin:0;background:{};color:#eee;font-family:system-ui;display:flex;align-items:center;justify-content:center;height:100vh"><div style="text-align:center"><h2>Ustun {}</h2><p>tao + wry</p></div></body></html>"#,
                color, i + 1
            );
            let _wv = WebViewBuilder::new().with_html(&html).build_gtk(&fixed)?;
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        let html = r#"<!doctype html><html><body style="margin:0;background:#111;color:#eee;display:flex;align-items:center;justify-content:center;height:100vh"><h1>Rail spike</h1></body></html>"#;
        let _wv = WebViewBuilder::new().with_html(html).build(&window)?;
    }

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
