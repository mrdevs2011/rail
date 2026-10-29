//! In-page continuous scroll screenshot with element picker & brutalist toolbar.
//!
//! Selection lives directly inside the webpage DOM (document coordinates),
//! not as a floating GTK overlay. When the page scrolls (e.g. YouTube),
//! the selection and element borders scroll together with the page.

use crate::*;

/// JavaScript injected into the webpage for in-page selection.
const IN_PAGE_SCREENSHOT_JS: &str = r##"
(function() {
  if (window.__railScreenshotCleanup) {
    try { window.__railScreenshotCleanup(); } catch(e) {}
  }

  const docEl = document.documentElement;
  const body = document.body || docEl;

  const root = document.createElement('div');
  root.id = '__rail_ss_root';
  root.style.cssText = 'position: absolute; left: 0; top: 0; width: 100%; height: 100%; pointer-events: none; z-index: 2147483647; overflow: visible; margin: 0; padding: 0;';

  const style = document.createElement('style');
  style.textContent = `
    #__rail_ss_root * { box-sizing: border-box; font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif; user-select: none; }
    .rail-ss-mask { position: absolute; background: rgba(0, 0, 0, 0.45); pointer-events: none; display: none; z-index: 2147483645; }
    #__rail_ss_hover {
      position: absolute;
      border: 2px solid #60a5fa;
      background: rgba(96, 165, 250, 0.22);
      border-radius: 4px;
      pointer-events: none;
      display: none;
      z-index: 2147483646;
      transition: all 0.04s ease-out;
    }
    #__rail_ss_selection {
      position: absolute;
      border: 2px solid #ffffff;
      box-shadow: 0 0 0 1px rgba(0, 0, 0, 0.65);
      pointer-events: auto;
      display: none;
      z-index: 2147483647;
      cursor: move;
    }
    .rail-ss-handle {
      position: absolute;
      width: 9px;
      height: 9px;
      background: #ffffff;
      border: 1.5px solid #18191c;
      border-radius: 2px;
      pointer-events: auto;
    }
    .rail-ss-h-nw { top: -5px; left: -5px; cursor: nwse-resize; }
    .rail-ss-h-ne { top: -5px; right: -5px; cursor: nesw-resize; }
    .rail-ss-h-sw { bottom: -5px; left: -5px; cursor: nesw-resize; }
    .rail-ss-h-se { bottom: -5px; right: -5px; cursor: nwse-resize; }
    .rail-ss-h-n  { top: -5px; left: calc(50% - 4.5px); cursor: ns-resize; }
    .rail-ss-h-s  { bottom: -5px; left: calc(50% - 4.5px); cursor: ns-resize; }
    .rail-ss-h-w  { top: calc(50% - 4.5px); left: -5px; cursor: ew-resize; }
    .rail-ss-h-e  { top: calc(50% - 4.5px); right: -5px; cursor: ew-resize; }
    #__rail_ss_size {
      position: absolute;
      left: 0;
      top: -24px;
      background: rgba(18, 19, 22, 0.9);
      color: #ffffff;
      padding: 2px 7px;
      border-radius: 4px;
      font-size: 11px;
      font-weight: 500;
      white-space: nowrap;
      pointer-events: none;
      border: 1px solid rgba(255, 255, 255, 0.2);
    }
    #__rail_ss_topbar {
      position: fixed;
      top: 14px;
      left: 50%;
      transform: translateX(-50%);
      background: #18191c;
      border: 1px solid rgba(255, 255, 255, 0.16);
      color: #e4e4e7;
      padding: 6px 14px;
      border-radius: 8px;
      font-size: 12px;
      font-weight: 500;
      display: flex;
      align-items: center;
      gap: 12px;
      z-index: 2147483647;
      box-shadow: 0 4px 20px rgba(0, 0, 0, 0.5);
      pointer-events: auto;
    }
    #__rail_ss_toolbar {
      position: absolute;
      z-index: 2147483647;
      display: none;
      background: #18191c;
      border: 1px solid rgba(255, 255, 255, 0.15);
      border-radius: 8px;
      padding: 4px;
      gap: 6px;
      align-items: center;
      box-shadow: 0 8px 30px rgba(0, 0, 0, 0.65);
      pointer-events: auto;
    }
    .rail-ss-btn {
      border: none;
      border-radius: 6px;
      cursor: pointer;
      display: inline-flex;
      align-items: center;
      justify-content: center;
      gap: 6px;
      font-size: 13px;
      font-weight: 500;
      transition: background 0.12s ease;
    }
    .rail-ss-btn-close {
      background: #2b2d30;
      color: #e4e4e7;
      width: 32px;
      height: 32px;
      font-size: 14px;
    }
    .rail-ss-btn-close:hover { background: #383a40; }
    .rail-ss-btn-copy {
      background: #2b2d30;
      color: #ffffff;
      padding: 6px 14px;
      height: 32px;
    }
    .rail-ss-btn-copy:hover { background: #383a40; }
    .rail-ss-btn-dl {
      background: #93c5fd;
      color: #0f172a;
      border: 1.5px solid #60a5fa;
      padding: 6px 16px;
      height: 32px;
      font-weight: 700;
    }
    .rail-ss-btn-dl:hover { background: #bfdbfe; }
    .rail-ss-top-btn {
      background: #2b2d30;
      color: #fff;
      border: 1px solid rgba(255, 255, 255, 0.15);
      border-radius: 5px;
      padding: 3px 10px;
      font-size: 11px;
      cursor: pointer;
    }
    .rail-ss-top-btn:hover { background: #383a40; }
  `;
  root.appendChild(style);

  const maskTop = document.createElement('div'); maskTop.className = 'rail-ss-mask';
  const maskBottom = document.createElement('div'); maskBottom.className = 'rail-ss-mask';
  const maskLeft = document.createElement('div'); maskLeft.className = 'rail-ss-mask';
  const maskRight = document.createElement('div'); maskRight.className = 'rail-ss-mask';
  root.appendChild(maskTop);
  root.appendChild(maskBottom);
  root.appendChild(maskLeft);
  root.appendChild(maskRight);

  const hoverBox = document.createElement('div');
  hoverBox.id = '__rail_ss_hover';
  root.appendChild(hoverBox);

  const selBox = document.createElement('div');
  selBox.id = '__rail_ss_selection';

  const sizeLbl = document.createElement('div');
  sizeLbl.id = '__rail_ss_size';
  selBox.appendChild(sizeLbl);

  ['nw', 'ne', 'sw', 'se', 'n', 's', 'w', 'e'].forEach(dir => {
    const h = document.createElement('div');
    h.className = `rail-ss-handle rail-ss-h-${dir}`;
    h.dataset.dir = dir;
    selBox.appendChild(h);
  });
  root.appendChild(selBox);

  const topbar = document.createElement('div');
  topbar.id = '__rail_ss_topbar';
  topbar.innerHTML = `
    <span>Maydonni belgilang yoki element ustiga bosing</span>
    <button class="rail-ss-top-btn" id="__rail_ss_top_full">To'liq sahifa</button>
    <span style="opacity: 0.6; font-size: 11px;">ESC: bekor qilish</span>
  `;
  root.appendChild(topbar);

  const toolbar = document.createElement('div');
  toolbar.id = '__rail_ss_toolbar';
  toolbar.innerHTML = `
    <button class="rail-ss-btn rail-ss-btn-close" id="__rail_ss_close" title="Bekor qilish">✕</button>
    <button class="rail-ss-btn rail-ss-btn-copy" id="__rail_ss_copy" title="Nusxalash">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>
      <span>Copy</span>
    </button>
    <button class="rail-ss-btn rail-ss-btn-dl" id="__rail_ss_dl" title="Yuklab olish">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="#0f172a" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path><polyline points="7 10 12 15 17 10"></polyline><line x1="12" y1="15" x2="12" y2="3"></line></svg>
      <span>Download</span>
    </button>
  `;
  root.appendChild(toolbar);

  (document.body || docEl).appendChild(root);

  let hasSelection = false;
  let sel = { left: 0, top: 0, width: 0, height: 0 };
  let isMouseDown = false;
  let isDragging = false;
  let isMoving = false;
  let isResizing = false;
  let resizeHandle = null;
  let startX = 0, startY = 0;
  let moveOffsetX = 0, moveOffsetY = 0;
  let resizeInitial = null;
  let hoveredRect = null;
  let mouseDownPos = { x: 0, y: 0 };

  const origCursor = document.body ? document.body.style.cursor : '';
  if (document.body) document.body.style.cursor = 'crosshair';

  function updateBackdrop() {
    if (!hasSelection || sel.width <= 0 || sel.height <= 0) {
      maskTop.style.display = 'none';
      maskBottom.style.display = 'none';
      maskLeft.style.display = 'none';
      maskRight.style.display = 'none';
      return;
    }
    const docW = Math.max(docEl.scrollWidth, window.innerWidth);
    const docH = Math.max(docEl.scrollHeight, window.innerHeight);

    maskTop.style.left = '0px';
    maskTop.style.top = '0px';
    maskTop.style.width = docW + 'px';
    maskTop.style.height = Math.max(0, sel.top) + 'px';
    maskTop.style.display = 'block';

    maskBottom.style.left = '0px';
    maskBottom.style.top = (sel.top + sel.height) + 'px';
    maskBottom.style.width = docW + 'px';
    maskBottom.style.height = Math.max(0, docH - (sel.top + sel.height)) + 'px';
    maskBottom.style.display = 'block';

    maskLeft.style.left = '0px';
    maskLeft.style.top = sel.top + 'px';
    maskLeft.style.width = Math.max(0, sel.left) + 'px';
    maskLeft.style.height = sel.height + 'px';
    maskLeft.style.display = 'block';

    maskRight.style.left = (sel.left + sel.width) + 'px';
    maskRight.style.top = sel.top + 'px';
    maskRight.style.width = Math.max(0, docW - (sel.left + sel.width)) + 'px';
    maskRight.style.height = sel.height + 'px';
    maskRight.style.display = 'block';
  }

  function updateSelectionView() {
    if (!hasSelection) {
      selBox.style.display = 'none';
      toolbar.style.display = 'none';
      updateBackdrop();
      return;
    }
    selBox.style.left = sel.left + 'px';
    selBox.style.top = sel.top + 'px';
    selBox.style.width = sel.width + 'px';
    selBox.style.height = sel.height + 'px';
    selBox.style.display = 'block';

    sizeLbl.textContent = `${Math.round(sel.width)} × ${Math.round(sel.height)}`;
    if (sel.top - window.scrollY < 26) {
      sizeLbl.style.top = '4px';
      sizeLbl.style.left = '4px';
    } else {
      sizeLbl.style.top = '-24px';
      sizeLbl.style.left = '0px';
    }

    updateBackdrop();
    if (!isMouseDown && !isDragging && !isMoving && !isResizing) {
      updateToolbarPos();
    } else {
      toolbar.style.display = 'none';
    }
  }

  function updateToolbarPos() {
    if (!hasSelection || isMouseDown || isDragging || isMoving || isResizing) {
      toolbar.style.display = 'none';
      return;
    }
    toolbar.style.display = 'inline-flex';
    const tw = 226;
    const th = 42;
    const margin = 10;

    let tx = sel.left + sel.width - tw;
    let ty = sel.top + sel.height + 8;

    const minX = window.scrollX + margin;
    const maxX = window.scrollX + window.innerWidth - tw - margin;
    tx = Math.max(minX, Math.min(tx, maxX));

    const minY = window.scrollY + margin;
    const maxY = window.scrollY + window.innerHeight - th - margin;
    if (ty > maxY) {
      if (sel.top - th - 8 >= minY) {
        ty = sel.top - th - 8;
      } else {
        ty = maxY;
      }
    }
    toolbar.style.left = tx + 'px';
    toolbar.style.top = ty + 'px';
  }

  function postMsg(action) {
    root.style.display = 'none';
    const payload = {
      action: action,
      x: Math.round(sel.left),
      y: Math.round(sel.top),
      w: Math.round(sel.width),
      h: Math.round(sel.height),
      docW: Math.round(docEl.scrollWidth),
      docH: Math.round(docEl.scrollHeight),
      scrollX: Math.round(window.scrollX),
      scrollY: Math.round(window.scrollY),
      viewportW: Math.round(window.innerWidth),
      viewportH: Math.round(window.innerHeight),
      devicePixelRatio: window.devicePixelRatio || 1
    };
    try {
      if (window.webkit && window.webkit.messageHandlers && window.webkit.messageHandlers.railScreenshot) {
        window.webkit.messageHandlers.railScreenshot.postMessage(JSON.stringify(payload));
      }
    } catch(e) {}
  }

  document.getElementById('__rail_ss_close').onclick = (e) => {
    e.stopPropagation();
    cleanup();
    try {
      if (window.webkit && window.webkit.messageHandlers && window.webkit.messageHandlers.railScreenshot) {
        window.webkit.messageHandlers.railScreenshot.postMessage(JSON.stringify({ action: "cancel" }));
      }
    } catch(e) {}
  };

  document.getElementById('__rail_ss_copy').onclick = (e) => {
    e.stopPropagation();
    postMsg('copy');
  };

  document.getElementById('__rail_ss_dl').onclick = (e) => {
    e.stopPropagation();
    postMsg('download');
  };

  document.getElementById('__rail_ss_top_full').onclick = (e) => {
    e.stopPropagation();
    postMsg('full_page');
  };

  function onMouseMove(e) {
    if (isDragging) {
      const currX = e.pageX;
      const currY = e.pageY;
      sel.left = Math.min(startX, currX);
      sel.top = Math.min(startY, currY);
      sel.width = Math.abs(currX - startX);
      sel.height = Math.abs(currY - startY);
      hasSelection = true;
      updateSelectionView();

      if (e.clientY > window.innerHeight - 45) {
        window.scrollBy({ top: 32, behavior: 'instant' });
      } else if (e.clientY < 45) {
        window.scrollBy({ top: -32, behavior: 'instant' });
      }
      return;
    }

    if (isMoving) {
      const docW = Math.max(docEl.scrollWidth, window.innerWidth);
      const docH = Math.max(docEl.scrollHeight, window.innerHeight);
      sel.left = Math.max(0, Math.min(e.pageX - moveOffsetX, docW - sel.width));
      sel.top = Math.max(0, Math.min(e.pageY - moveOffsetY, docH - sel.height));
      updateSelectionView();
      return;
    }

    if (isResizing) {
      const dx = e.pageX - resizeInitial.pageX;
      const dy = e.pageY - resizeInitial.pageY;
      let { left, top, width, height } = resizeInitial.sel;
      const d = resizeHandle;
      if (d.includes('e')) width += dx;
      if (d.includes('s')) height += dy;
      if (d.includes('w')) { left += dx; width -= dx; }
      if (d.includes('n')) { top += dy; height -= dy; }
      if (width < 10) { width = 10; }
      if (height < 10) { height = 10; }
      sel = { left, top, width, height };
      updateSelectionView();
      return;
    }

    if (hasSelection) {
      hoverBox.style.display = 'none';
      return;
    }

    hoverBox.style.display = 'none';
    const el = document.elementFromPoint(e.clientX, e.clientY);
    if (!el || el === document.body || el === docEl || (el.closest && el.closest('#__rail_ss_root'))) {
      hoverBox.style.display = 'none';
      hoveredRect = null;
      return;
    }

    let curr = el;
    while (curr && curr.parentElement && curr.parentElement !== document.body && curr.parentElement !== docEl) {
      const r = curr.getBoundingClientRect();
      if (r.width >= 24 && r.height >= 18) break;
      curr = curr.parentElement;
    }
    const r = (curr || el).getBoundingClientRect();
    if (r.width < 10 || r.height < 10) {
      hoverBox.style.display = 'none';
      hoveredRect = null;
      return;
    }

    hoveredRect = {
      left: r.left + window.scrollX,
      top: r.top + window.scrollY,
      width: r.width,
      height: r.height
    };
    hoverBox.style.left = hoveredRect.left + 'px';
    hoverBox.style.top = hoveredRect.top + 'px';
    hoverBox.style.width = hoveredRect.width + 'px';
    hoverBox.style.height = hoveredRect.height + 'px';
    hoverBox.style.display = 'block';
  }

  function onMouseDown(e) {
    if (e.button !== 0) return;
    if (e.target.closest && (e.target.closest('#__rail_ss_toolbar') || e.target.closest('#__rail_ss_topbar'))) {
      return;
    }

    toolbar.style.display = 'none';

    mouseDownPos = { x: e.pageX, y: e.pageY };

    const handle = e.target.closest ? e.target.closest('.rail-ss-handle') : null;
    if (handle && hasSelection) {
      isResizing = true;
      resizeHandle = handle.dataset.dir;
      resizeInitial = { pageX: e.pageX, pageY: e.pageY, sel: { ...sel } };
      e.preventDefault();
      e.stopPropagation();
      return;
    }

    if (hasSelection && e.target.closest && e.target.closest('#__rail_ss_selection')) {
      isMoving = true;
      moveOffsetX = e.pageX - sel.left;
      moveOffsetY = e.pageY - sel.top;
      e.preventDefault();
      e.stopPropagation();
      return;
    }

    isMouseDown = true;
    startX = e.pageX;
    startY = e.pageY;
    e.preventDefault();
  }

  function onMouseUp(e) {
    if (e.button !== 0) return;

    if (isResizing) {
      isResizing = false;
      resizeHandle = null;
      updateToolbarPos();
      return;
    }

    if (isMoving) {
      isMoving = false;
      updateToolbarPos();
      return;
    }

    if (isMouseDown) {
      isMouseDown = false;
      const dist = Math.hypot(e.pageX - mouseDownPos.x, e.pageY - mouseDownPos.y);

      if (dist < 6 && hoveredRect && !isDragging) {
        sel = { ...hoveredRect };
        hasSelection = true;
        hoverBox.style.display = 'none';
        updateSelectionView();
        updateToolbarPos();
        return;
      }

      isDragging = false;
      if (sel.width > 12 && sel.height > 12) {
        hasSelection = true;
        hoverBox.style.display = 'none';
        updateSelectionView();
        updateToolbarPos();
      } else {
        hasSelection = false;
        sel = { left: 0, top: 0, width: 0, height: 0 };
        updateSelectionView();
        toolbar.style.display = 'none';
      }
    }
  }

  function onKeyDown(e) {
    if (e.key === 'Escape') {
      cleanup();
      try {
        if (window.webkit && window.webkit.messageHandlers && window.webkit.messageHandlers.railScreenshot) {
          window.webkit.messageHandlers.railScreenshot.postMessage(JSON.stringify({ action: "cancel" }));
        }
      } catch(e) {}
    }
  }

  function onScroll() {
    updateToolbarPos();
  }

  window.addEventListener('mousemove', onMouseMove, true);
  window.addEventListener('mousedown', onMouseDown, true);
  window.addEventListener('mouseup', onMouseUp, true);
  window.addEventListener('keydown', onKeyDown, true);
  window.addEventListener('scroll', onScroll, true);

  function cleanup() {
    window.removeEventListener('mousemove', onMouseMove, true);
    window.removeEventListener('mousedown', onMouseDown, true);
    window.removeEventListener('mouseup', onMouseUp, true);
    window.removeEventListener('keydown', onKeyDown, true);
    window.removeEventListener('scroll', onScroll, true);
    if (document.body) document.body.style.cursor = origCursor;
    root.remove();
    window.__railScreenshotCleanup = null;
  }

  window.__railScreenshotCleanup = cleanup;
})();
"##;

const CLEANUP_JS: &str = r#"
(function() {
  if (window.__railScreenshotCleanup) {
    try { window.__railScreenshotCleanup(); } catch(e) {}
  }
})();
"#;

#[derive(Clone, Copy)]
struct Rect {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

impl App {
    /// Launches the in-page screenshot selector on the given WebView.
    pub(crate) fn start_screenshot_mode(self: &Rc<Self>, view: &WebView) {
        self.end_screenshot_overlay();

        if let Some(ucm) = view.user_content_manager() {
            let _ = ucm.register_script_message_handler("railScreenshot", None);
        }

        view.evaluate_javascript(
            IN_PAGE_SCREENSHOT_JS,
            None,
            None,
            None::<&gio::Cancellable>,
            |res| {
                if let Err(e) = res {
                    eprintln!("[rail] failed to inject screenshot script: {e}");
                }
            },
        );
    }

    /// Handles messages from the in-page screenshot script.
    pub(crate) fn on_screenshot_message(self: &Rc<Self>, view: &WebView, payload: &str) {
        let action = get_json_str(payload, "action").unwrap_or_default();
        if action == "cancel" {
            cleanup_screenshot(view);
            return;
        }

        if action == "full_page" {
            let app = self.clone();
            let view_c = view.clone();
            view.snapshot(
                webkit6::SnapshotRegion::FullDocument,
                webkit6::SnapshotOptions::empty(),
                None::<&gio::Cancellable>,
                move |res| {
                    match res {
                        Ok(tex) => finish_capture(&app, &tex, false),
                        Err(e) => eprintln!("[rail] full page screenshot error: {e}"),
                    }
                    cleanup_screenshot(&view_c);
                },
            );
            return;
        }

        let x = get_json_num(payload, "x").unwrap_or(0.0);
        let y = get_json_num(payload, "y").unwrap_or(0.0);
        let w = get_json_num(payload, "w").unwrap_or(0.0);
        let h = get_json_num(payload, "h").unwrap_or(0.0);
        let doc_w = get_json_num(payload, "docW").unwrap_or(1920.0).max(1.0);
        let doc_h = get_json_num(payload, "docH").unwrap_or(1080.0).max(1.0);
        let scroll_x = get_json_num(payload, "scrollX").unwrap_or(0.0);
        let scroll_y = get_json_num(payload, "scrollY").unwrap_or(0.0);
        let vp_w = get_json_num(payload, "viewportW").unwrap_or(1920.0).max(1.0);
        let vp_h = get_json_num(payload, "viewportH").unwrap_or(1080.0).max(1.0);

        if w < 2.0 || h < 2.0 {
            cleanup_screenshot(view);
            return;
        }

        let to_clipboard = action == "copy";
        let is_visible = y >= scroll_y
            && (y + h) <= (scroll_y + vp_h + 1.5)
            && x >= scroll_x
            && (x + w) <= (scroll_x + vp_w + 1.5);

        let app = self.clone();
        let view_c = view.clone();

        if is_visible {
            // Fast viewport capture
            view.snapshot(
                webkit6::SnapshotRegion::Visible,
                webkit6::SnapshotOptions::empty(),
                None::<&gio::Cancellable>,
                move |res| {
                    match res {
                        Ok(tex) => {
                            let tw = tex.width() as f64;
                            let th = tex.height() as f64;
                            let sx = tw / vp_w;
                            let sy = th / vp_h;
                            let crop_r = Rect {
                                x: (x - scroll_x) * sx,
                                y: (y - scroll_y) * sy,
                                w: w * sx,
                                h: h * sy,
                            };
                            let final_tex = crop_texture(&tex, crop_r).unwrap_or(tex);
                            finish_capture(&app, &final_tex, to_clipboard);
                        }
                        Err(e) => eprintln!("[rail] visible snapshot error: {e}"),
                    }
                    cleanup_screenshot(&view_c);
                },
            );
        } else {
            // Continuous scroll capture across the document
            view.snapshot(
                webkit6::SnapshotRegion::FullDocument,
                webkit6::SnapshotOptions::empty(),
                None::<&gio::Cancellable>,
                move |res| {
                    match res {
                        Ok(tex) => {
                            let tw = tex.width() as f64;
                            let th = tex.height() as f64;
                            let sx = tw / doc_w;
                            let sy = th / doc_h;
                            let crop_r = Rect {
                                x: x * sx,
                                y: y * sy,
                                w: w * sx,
                                h: h * sy,
                            };
                            let final_tex = crop_texture(&tex, crop_r).unwrap_or(tex);
                            finish_capture(&app, &final_tex, to_clipboard);
                        }
                        Err(e) => {
                            eprintln!("[rail] full document snapshot error: {e}; falling back");
                        }
                    }
                    cleanup_screenshot(&view_c);
                },
            );
        }
    }

    pub(crate) fn end_screenshot_overlay(&self) {
        if let Some(v) = self.focused_view() {
            cleanup_screenshot(&v);
        }
    }
}

fn cleanup_screenshot(view: &WebView) {
    view.evaluate_javascript(CLEANUP_JS, None, None, None::<&gio::Cancellable>, |_| {});
}

fn finish_capture(app: &Rc<App>, tex: &gdk::Texture, to_clipboard: bool) {
    if to_clipboard {
        app.win.clipboard().set_texture(tex);
        app.show_copied_toast(tr("Nusxalandi!"), tr("Screenshot buferga nusxalandi"));
    } else if let Some(path) = save_texture_png(tex) {
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        app.show_copied_toast(tr("Saqlandi!"), &format!("Downloads/Rail/{}", name));
    }
}

fn crop_texture(tex: &gdk::Texture, r: Rect) -> Option<gdk::Texture> {
    let tw = tex.width() as f64;
    let th = tex.height() as f64;
    let x = r.x.clamp(0.0, (tw - 1.0).max(0.0));
    let y = r.y.clamp(0.0, (th - 1.0).max(0.0));
    let w = r.w.clamp(1.0, (tw - x).max(1.0));
    let h = r.h.clamp(1.0, (th - y).max(1.0));
    let ix = x as i32;
    let iy = y as i32;
    let iw = w as i32;
    let ih = h as i32;
    if iw <= 0 || ih <= 0 {
        return None;
    }
    let stride = tw as i32 * 4;
    let mut buf = vec![0u8; (tw as usize) * (th as usize) * 4];
    tex.download(&mut buf, stride as usize);
    let mut out = vec![0u8; (iw as usize) * (ih as usize) * 4];
    for row in 0..ih as usize {
        let src_off = ((iy as usize + row) * stride as usize) + (ix as usize * 4);
        let dst_off = row * (iw as usize) * 4;
        if src_off + iw as usize * 4 <= buf.len() && dst_off + iw as usize * 4 <= out.len() {
            out[dst_off..dst_off + iw as usize * 4]
                .copy_from_slice(&buf[src_off..src_off + iw as usize * 4]);
        }
    }
    let bytes = glib::Bytes::from_owned(out);
    Some(
        gdk::MemoryTexture::new(iw, ih, gdk::MemoryFormat::R8g8b8a8, &bytes, (iw * 4) as usize)
            .upcast(),
    )
}

pub(crate) fn screenshot_dir() -> std::path::PathBuf {
    let dl_dir = glib::user_special_dir(glib::UserDirectory::Downloads)
        .unwrap_or_else(|| glib::home_dir().join("Downloads"));
    dl_dir.join("Rail")
}

fn save_texture_png(tex: &gdk::Texture) -> Option<std::path::PathBuf> {
    let dir = screenshot_dir();
    let _ = std::fs::create_dir_all(&dir);
    let ts = chrono_like_timestamp();
    let path = dir.join(format!("Screenshot {ts}.png"));
    if let Err(e) = tex.save_to_png(&path) {
        eprintln!("[rail] save png failed: {e}");
        None
    } else {
        eprintln!("[rail] screenshot saved → {}", path.display());
        Some(path)
    }
}

fn chrono_like_timestamp() -> String {
    if let Ok(out) = std::process::Command::new("date")
        .args(["+%Y-%m-%d %H-%M-%S"])
        .output()
    {
        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !s.is_empty() {
                return s;
            }
        }
    }
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{secs}")
}

fn get_json_num(s: &str, key: &str) -> Option<f64> {
    let pat = format!("\"{}\":", key);
    let idx = s.find(&pat)?;
    let rest = &s[idx + pat.len()..];
    let num: String = rest
        .chars()
        .skip_while(|c| c.is_whitespace())
        .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
        .collect();
    num.parse().ok()
}

fn get_json_str(s: &str, key: &str) -> Option<String> {
    let pat = format!("\"{}\":\"", key);
    let idx = s.find(&pat)?;
    let rest = &s[idx + pat.len()..];
    let end_idx = rest.find('"')?;
    Some(rest[..end_idx].to_string())
}
