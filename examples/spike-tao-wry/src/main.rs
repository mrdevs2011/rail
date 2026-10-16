// Tao + wry spike: 3 ta webview yonma-yon
// Linux: default_vbox + gtk::Box, expand, show after build

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
        let hbox = gtk::Box::new(gtk::Orientation::Horizontal, 2);
        hbox.set_homogeneous(true);
        hbox.set_hexpand(true);
        hbox.set_vexpand(true);
        vbox.pack_start(&hbox, true, true, 0);

        let mut webviews = Vec::new();
        for (i, color) in ["#1a1a2e", "#16213e", "#0f3460"].iter().enumerate() {
            let fixed = gtk::Fixed::new();
            fixed.set_hexpand(true);
            fixed.set_vexpand(true);
            fixed.set_size_request(300, 400);
            hbox.pack_start(&fixed, true, true, 0);

            let html = format!(
                r#"<!doctype html><html><head><meta charset="utf-8"></head><body style="margin:0;background:{};color:#fff;font-family:sans-serif;display:flex;align-items:center;justify-content:center;height:100vh;font-size:24px"><div>Ustun {}</div></body></html>"#,
                color, i + 1
            );
            let wv = WebViewBuilder::new().with_html(&html).build_gtk(&fixed)?;
            webviews.push(wv);
            fixed.show_all();
        }
        hbox.show_all();
        vbox.show_all();
        // keep webviews alive
        std::mem::forget(webviews);
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
