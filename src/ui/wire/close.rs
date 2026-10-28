//! Saving state when the window closes.

use crate::*;

pub(crate) fn wire_close(app: &Rc<App>, win: gtk::ApplicationWindow) {
    let a = app.clone();
    win.connect_close_request(move |_| {
        a.save_session_immediate();
        a.history.borrow_mut().flush();
        // Kill every WebKit process now so closing the window never leaves RAM-eating orphans.
        a.shutdown_all_web_processes();
        if let Some(gapp) = a.win.application() {
            let gapp = gapp.clone();
            glib::idle_add_local_once(move || {
                gapp.quit();
            });
        }
        glib::Propagation::Proceed
    });

    {
        // Ko'rinmagan tablarni uxlatish (split da yonma-yon ko'rinib turgan tab uxlamaydi)
        let weak = Rc::downgrade(&app);
        glib::timeout_add_local(Duration::from_secs(15), move || match weak.upgrade() {
            Some(a) => {
                a.suspend_hidden_tabs();
                if let Ok(mut h) = a.history.try_borrow_mut() {
                    h.flush();
                }
                glib::ControlFlow::Continue
            }
            None => glib::ControlFlow::Break,
        });
    }

    {
        // Main-loop watchdog: a 100 ms timer that fires late means the UI thread was blocked.
        let weak = Rc::downgrade(&app);
        let last = Rc::new(Cell::new(std::time::Instant::now()));
        // Jank counter: late-fire buckets (>16/33/100 ms) summarised once a minute in jank.log.
        let jank = Rc::new(RefCell::new(jank::Jank::new()));
        glib::timeout_add_local(Duration::from_millis(100), move || {
            let Some(a) = weak.upgrade() else { return glib::ControlFlow::Break };
            let now = std::time::Instant::now();
            let gap = now.duration_since(last.replace(now));
            // Ignore suspend/resume style gaps; they are not UI-thread work.
            if gap < Duration::from_secs(30) {
                let mut j = jank.borrow_mut();
                j.record(gap.saturating_sub(Duration::from_millis(100)));
                j.maybe_report(&rail_data_dir());
            }
            if gap > Duration::from_millis(300) && gap < Duration::from_secs(30) {
                a.log_stall(gap);
            }
            glib::ControlFlow::Continue
        });
    }
}
