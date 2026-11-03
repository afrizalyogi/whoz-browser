//! Whoz: minimal browser. The page fills the window; header (window buttons) and tabs auto-hide until hovered.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    cell::{Cell, RefCell},
    fmt::Write,
    path::PathBuf,
    rc::Rc,
};
use tao::{
    dpi::{LogicalSize, PhysicalPosition},
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy},
    platform::windows::WindowBuilderExtWindows,
    window::{Fullscreen, ResizeDirection, Window, WindowBuilder},
};
use webview2_com::{
    AcceleratorKeyPressedEventHandler, ContainsFullScreenElementChangedEventHandler,
    Microsoft::Web::WebView2::Win32::{
        ICoreWebView2Environment, COREWEBVIEW2_KEY_EVENT_KIND,
        COREWEBVIEW2_KEY_EVENT_KIND_KEY_DOWN, COREWEBVIEW2_KEY_EVENT_KIND_SYSTEM_KEY_DOWN,
    },
    WindowCloseRequestedEventHandler,
};
use windows::{
    core::{BOOL, HSTRING},
    Win32::{
        Graphics::Gdi::{CombineRgn, CreateRectRgn, DeleteObject, SetWindowRgn, RGN_OR},
        UI::{
            Input::KeyboardAndMouse::{GetKeyState, VIRTUAL_KEY, VK_CONTROL, VK_MENU, VK_SHIFT},
            WindowsAndMessaging::{
                MessageBoxW, SetWindowPos, HWND_TOP, MB_ICONERROR, SWP_NOACTIVATE, SWP_NOMOVE,
                SWP_NOSIZE,
            },
        },
    },
};
use wry::{
    MemoryUsageLevel, NewWindowResponse, Rect, WebContext, WebView, WebViewBuilder,
    WebViewBuilderExtWindows, WebViewExtWindows,
};

const HOME: &str = "https://www.google.com";
const SEARCH: &str = "https://www.google.com/search?q=";
const UI: &str = include_str!("ui.html");
// Alt+drag anywhere moves the window.
const DRAG_JS: &str = "addEventListener('mousedown',e=>{if(e.altKey&&e.button===0&&window.ipc){e.preventDefault();e.stopImmediatePropagation();ipc.postMessage('drag')}},true)";

#[derive(Debug, Clone, Copy, PartialEq)]
enum Cmd {
    NewTab,
    CloseTab,
    Next,
    Prev,
    Nth(usize),
    Last,
    Palette,
    Fullscreen,
}

enum Ev {
    Key(Cmd),
    Drag,
    Title(u32, String),
    Closed(u32),
    Fullscreen(bool),
    Adopt,
    Ui(String),
}

/// State the WebView callbacks need; lives behind an Rc so popups can build new tabs.
struct Shared {
    window: Window,
    proxy: EventLoopProxy<Ev>,
    pending: RefCell<Vec<(u32, WebView)>>,
    next_id: Cell<u32>,
    dir: PathBuf,
}

struct Tab {
    id: u32,
    title: String,
    wv: WebView,
}

struct App {
    sh: Rc<Shared>,
    env: ICoreWebView2Environment,
    tabs: Vec<Tab>,
    active: usize,
    last: u32,
    /// One overlay view for header, tab list and address bar; clipped to what is visible.
    ui: WebView,
    ui_ready: bool,
    pal_open: bool,
    pal_new: bool,
    tabs_left: bool,
    quit: bool,
}

fn main() {
    let dir = PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap_or_else(|| ".".into()))
        .join("WhozBrowser");
    // wry reads this folder on startup and fails if it is missing.
    let _ = std::fs::create_dir_all(dir.join("extensions"));
    let tabs_left = std::fs::read_to_string(dir.join("tabs")).map_or(true, |s| s.trim() != "top");

    let event_loop = EventLoopBuilder::<Ev>::with_user_event().build();
    let window = WindowBuilder::new()
        .with_title("Whoz")
        .with_decorations(false)
        .with_undecorated_shadow(true)
        .with_inner_size(LogicalSize::new(1280.0, 800.0))
        .build(&event_loop)
        .unwrap_or_else(|e| fail(&e.to_string()));
    let sh = Rc::new(Shared {
        window,
        proxy: event_loop.create_proxy(),
        pending: RefCell::default(),
        next_id: Cell::new(0),
        dir,
    });

    let url = std::env::args().nth(1).and_then(|a| resolve(&a)).unwrap_or_else(|| HOME.into());
    let (id, wv) = new_tab(&sh, None, Some(&url)).unwrap_or_else(|e| {
        fail(&format!("Microsoft Edge WebView2 Runtime tidak bisa dimulai:\n{e}"))
    });
    let env = wv.environment();
    let p = sh.proxy.clone();
    let ui = WebViewBuilder::new()
        .with_environment(env.clone())
        .with_html(UI)
        .with_bounds(full_rect(&sh.window))
        .with_transparent(true)
        .with_focused(false)
        .with_ipc_handler(move |r| {
            let _ = p.send_event(Ev::Ui(r.into_body()));
        })
        .build_as_child(&sh.window)
        .unwrap_or_else(|e| fail(&e.to_string()));
    let _ = hook(&ui, &sh.proxy, None);
    // Nothing of the overlay is shown (or takes clicks) until ui.html reports its regions.
    set_region(&ui, "");

    let mut app = App {
        env,
        sh,
        tabs: vec![Tab { id, title: String::new(), wv }],
        active: usize::MAX,
        last: u32::MAX,
        ui,
        ui_ready: false,
        pal_open: false,
        pal_new: false,
        tabs_left,
        quit: false,
    };
    app.switch(0);

    event_loop.run(move |event, _, cf| {
        *cf = ControlFlow::Wait;
        match event {
            Event::UserEvent(e) => app.on(e),
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => app.quit = true,
                WindowEvent::Resized(_) => app.resize(),
                WindowEvent::Focused(true) => app.focus(),
                _ => {}
            },
            _ => {}
        }
        if app.quit {
            *cf = ControlFlow::Exit;
        }
    });
}

fn fail(msg: &str) -> ! {
    unsafe { MessageBoxW(None, &HSTRING::from(msg), &HSTRING::from("Whoz"), MB_ICONERROR) };
    std::process::exit(1)
}

fn full_rect(w: &Window) -> Rect {
    Rect { position: PhysicalPosition::new(0, 0).into(), size: w.inner_size().into() }
}

/// Clips the overlay to "x,y,w,h;..." (device px). Outside it the page shows and gets the mouse.
fn set_region(wv: &WebView, spec: &str) {
    unsafe {
        let rgn = CreateRectRgn(0, 0, 0, 0);
        for r in spec.split(';') {
            let v: Vec<i32> = r.split(',').filter_map(|n| n.parse().ok()).collect();
            if let [x, y, w, h] = v[..] {
                let part = CreateRectRgn(x, y, x + w, y + h);
                CombineRgn(Some(rgn), Some(rgn), Some(part), RGN_OR);
                let _ = DeleteObject(part.into());
            }
        }
        // The window owns the region after this call.
        if SetWindowRgn(wv.hwnd(), Some(rgn), true) == 0 {
            let _ = DeleteObject(rgn.into());
        }
        let _ = SetWindowPos(wv.hwnd(), Some(HWND_TOP), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
    }
}

/// Builds a hidden tab. `env == None` only for the first tab: it creates the shared profile.
fn new_tab(
    sh: &Rc<Shared>,
    env: Option<&ICoreWebView2Environment>,
    url: Option<&str>,
) -> wry::Result<(u32, WebView)> {
    let id = sh.next_id.get();
    sh.next_id.set(id + 1);
    let mut ctx;
    let builder = match env {
        Some(e) => WebViewBuilder::new().with_environment(e.clone()),
        None => {
            ctx = WebContext::new(Some(sh.dir.join("profile")));
            WebViewBuilder::new_with_web_context(&mut ctx)
                .with_browser_extensions_enabled(true)
                .with_extensions_path(sh.dir.join("extensions"))
        }
    };
    let (p_ipc, p_title, sh2) = (sh.proxy.clone(), sh.proxy.clone(), sh.clone());
    let mut builder = builder
        .with_bounds(full_rect(&sh.window))
        .with_visible(false)
        .with_focused(false)
        .with_hotkeys_zoom(true)
        .with_initialization_script(DRAG_JS)
        // Pages can only ask for a window drag; every other UI message comes from ui.html.
        .with_ipc_handler(move |r| {
            if r.body() == "drag" {
                let _ = p_ipc.send_event(Ev::Drag);
            }
        })
        .with_document_title_changed_handler(move |t| {
            let _ = p_title.send_event(Ev::Title(id, t));
        })
        // Popups become tabs but keep window.opener, so OAuth popups work.
        .with_new_window_req_handler(move |_, f| {
            match new_tab(&sh2, Some(&f.opener.environment), None) {
                Ok((id, wv)) => {
                    let webview = wv.webview();
                    sh2.pending.borrow_mut().push((id, wv));
                    let _ = sh2.proxy.send_event(Ev::Adopt);
                    NewWindowResponse::Create { webview }
                }
                Err(_) => NewWindowResponse::Deny,
            }
        });
    if let Some(u) = url {
        builder = builder.with_url(u);
    }
    let wv = builder.build_as_child(&sh.window)?;
    hook(&wv, &sh.proxy, Some(id)).map_err(|e| wry::Error::WebView2Error(e.into()))?;
    Ok((id, wv))
}

/// Shortcuts for every WebView; fullscreen and window.close() only for tabs.
fn hook(wv: &WebView, proxy: &EventLoopProxy<Ev>, tab: Option<u32>) -> windows::core::Result<()> {
    let mut token = 0;
    let p = proxy.clone();
    unsafe {
        wv.controller().add_AcceleratorKeyPressed(
            &AcceleratorKeyPressedEventHandler::create(Box::new(move |_, args| {
                let Some(args) = args else { return Ok(()) };
                let (mut kind, mut vk) = (COREWEBVIEW2_KEY_EVENT_KIND::default(), 0);
                args.KeyEventKind(&mut kind)?;
                args.VirtualKey(&mut vk)?;
                if kind == COREWEBVIEW2_KEY_EVENT_KIND_KEY_DOWN
                    || kind == COREWEBVIEW2_KEY_EVENT_KIND_SYSTEM_KEY_DOWN
                {
                    let down = |k: VIRTUAL_KEY| GetKeyState(k.0 as i32) < 0;
                    if let Some(c) = key_cmd(vk, down(VK_CONTROL), down(VK_SHIFT), down(VK_MENU)) {
                        args.SetHandled(true)?;
                        let _ = p.send_event(Ev::Key(c));
                    }
                }
                Ok(())
            })),
            &mut token,
        )?;
        let Some(id) = tab else { return Ok(()) };
        let core = wv.webview();
        let p = proxy.clone();
        core.add_ContainsFullScreenElementChanged(
            &ContainsFullScreenElementChangedEventHandler::create(Box::new(move |s, _| {
                let mut on = BOOL::default();
                if let Some(s) = s {
                    s.ContainsFullScreenElement(&mut on)?;
                }
                let _ = p.send_event(Ev::Fullscreen(on.as_bool()));
                Ok(())
            })),
            &mut token,
        )?;
        let p = proxy.clone();
        core.add_WindowCloseRequested(
            &WindowCloseRequestedEventHandler::create(Box::new(move |_, _| {
                let _ = p.send_event(Ev::Closed(id));
                Ok(())
            })),
            &mut token,
        )?;
    }
    Ok(())
}

impl App {
    fn on(&mut self, e: Ev) {
        match e {
            Ev::Key(c) => self.run(c),
            Ev::Drag => {
                let _ = self.sh.window.drag_window();
            }
            Ev::Title(id, t) => {
                if let Some(i) = self.tabs.iter().position(|t| t.id == id) {
                    if i == self.active {
                        self.sh.window.set_title(&t);
                    }
                    self.tabs[i].title = t;
                    self.render();
                }
            }
            Ev::Closed(id) => {
                if let Some(i) = self.tabs.iter().position(|t| t.id == id) {
                    self.close(i);
                }
            }
            Ev::Fullscreen(on) => self.set_fullscreen(on),
            Ev::Adopt => {
                let new: Vec<_> = self.sh.pending.borrow_mut().drain(..).collect();
                for (id, wv) in new {
                    self.tabs.push(Tab { id, title: String::new(), wv });
                }
                self.switch(self.tabs.len() - 1);
            }
            Ev::Ui(m) => self.on_ui(&m),
        }
    }

    fn run(&mut self, c: Cmd) {
        let n = self.tabs.len();
        match c {
            Cmd::NewTab => self.open_palette(true),
            Cmd::Palette => self.open_palette(false),
            Cmd::CloseTab => self.close(self.active),
            Cmd::Next => self.switch((self.active + 1) % n),
            Cmd::Prev => self.switch((self.active + n - 1) % n),
            Cmd::Nth(i) if i < n => self.switch(i),
            Cmd::Nth(_) => {}
            Cmd::Last => self.switch(n - 1),
            Cmd::Fullscreen => self.set_fullscreen(self.sh.window.fullscreen().is_none()),
        }
    }

    fn set_fullscreen(&mut self, on: bool) {
        self.sh.window.set_fullscreen(on.then_some(Fullscreen::Borderless(None)));
        self.render();
    }

    fn add(&mut self, url: &str) {
        match new_tab(&self.sh, Some(&self.env), Some(url)) {
            Ok((id, wv)) => {
                self.tabs.push(Tab { id, title: String::new(), wv });
                self.switch(self.tabs.len() - 1);
            }
            Err(e) => eprintln!("new tab failed: {e}"),
        }
    }

    fn switch(&mut self, i: usize) {
        if i != self.active {
            if let Some(t) = self.tabs.get(self.active) {
                self.last = t.id;
                let _ = t.wv.set_visible(false);
                // Hidden tabs give memory back to the OS.
                let _ = t.wv.set_memory_usage_level(MemoryUsageLevel::Low);
            }
        }
        self.active = i;
        self.hide_palette();
        let t = &self.tabs[i];
        let _ = t.wv.set_bounds(full_rect(&self.sh.window));
        let _ = t.wv.set_memory_usage_level(MemoryUsageLevel::Normal);
        let _ = t.wv.set_visible(true);
        let _ = t.wv.focus();
        self.sh.window.set_title(if t.title.is_empty() { "Whoz" } else { &t.title });
        self.render();
    }

    fn close(&mut self, i: usize) {
        let gone = self.tabs.remove(i);
        if self.tabs.is_empty() {
            self.quit = true;
            return;
        }
        if i == self.active {
            // Back to the previously used tab (the opener, for popups), else the neighbour.
            let n = self.tabs.len();
            let j = self.tabs.iter().position(|t| t.id == self.last).unwrap_or(i.min(n - 1));
            self.active = usize::MAX;
            self.switch(j);
        } else {
            if i < self.active {
                self.active -= 1;
            }
            self.render();
        }
        drop(gone); // after the next tab is shown, so there is no blank frame
    }

    fn resize(&mut self) {
        let r = full_rect(&self.sh.window);
        let _ = self.ui.set_bounds(r);
        if let Some(t) = self.tabs.get(self.active) {
            let _ = t.wv.set_bounds(r);
        }
        self.render(); // maximized state may have changed
    }

    fn focus(&self) {
        let wv = if self.pal_open { &self.ui } else { &self.tabs[self.active].wv };
        let _ = wv.focus();
    }

    /// Pushes tab titles and window state to ui.html.
    fn render(&self) {
        if !self.ui_ready {
            return;
        }
        let mut js = String::from("render([");
        for (i, t) in self.tabs.iter().enumerate() {
            let title = if t.title.is_empty() { "New tab" } else { &t.title };
            let _ = write!(js, "{{t:{},a:{}}},", js_str(title), i == self.active);
        }
        let w = &self.sh.window;
        let _ = write!(js, "],{},{},{})", self.tabs_left, w.is_maximized(), w.fullscreen().is_some());
        let _ = self.ui.evaluate_script(&js);
    }

    fn open_palette(&mut self, new: bool) {
        if !self.ui_ready {
            return;
        }
        self.pal_new = new;
        self.pal_open = true;
        // URLs are read only here, not on every render: url() is a COM round-trip per tab.
        let mut js = String::from("pal(");
        let cur = if new { String::new() } else { self.tabs[self.active].wv.url().unwrap_or_default() };
        js.push_str(&js_str(&cur));
        js.push_str(",[");
        for t in &self.tabs {
            js.push_str(&js_str(&t.wv.url().unwrap_or_default()));
            js.push(',');
        }
        js.push_str("])");
        // Focus the view first: a programmatic MoveFocus resets focus inside it and would blur the input.
        let _ = self.ui.focus();
        let _ = self.ui.evaluate_script(&js);
    }

    fn hide_palette(&mut self) {
        if self.pal_open {
            self.pal_open = false;
            let _ = self.ui.evaluate_script("closePal()");
            let _ = self.tabs[self.active].wv.focus();
        }
    }

    fn on_ui(&mut self, msg: &str) {
        let w = &self.sh.window;
        let (cmd, arg) = msg.split_once('\n').unwrap_or((msg, ""));
        let idx = || arg.parse::<usize>().ok().filter(|&i| i < self.tabs.len());
        match cmd {
            "ready" => {
                self.ui_ready = true;
                self.render();
            }
            "rgn" => set_region(&self.ui, arg),
            "hide" => {
                self.pal_open = false;
                let _ = self.tabs[self.active].wv.focus();
            }
            "tab" => {
                if let Some(i) = idx() {
                    self.switch(i);
                }
            }
            "x" => {
                if let Some(i) = idx() {
                    self.close(i);
                }
            }
            "go" => {
                let Some(url) = resolve(arg) else { return };
                if self.pal_new {
                    self.add(&url);
                } else {
                    let _ = self.tabs[self.active].wv.load_url(&url);
                    self.hide_palette();
                }
            }
            "new" => self.open_palette(true),
            "pal" => self.open_palette(false),
            "layout" => {
                self.tabs_left = !self.tabs_left;
                let _ = std::fs::write(self.sh.dir.join("tabs"), if self.tabs_left { "left" } else { "top" });
                self.render();
            }
            "min" => w.set_minimized(true),
            "max" => w.set_maximized(!w.is_maximized()),
            "close" => self.quit = true,
            "drag" => {
                let _ = w.drag_window();
            }
            "resize" => {
                let _ = w.drag_resize_window(ResizeDirection::North);
            }
            _ => {}
        }
    }
}

fn key_cmd(vk: u32, ctrl: bool, shift: bool, alt: bool) -> Option<Cmd> {
    Some(match (ctrl, alt, vk) {
        (true, false, 0x54) if !shift => Cmd::NewTab, // T
        (true, false, 0x57 | 0x73) => Cmd::CloseTab,   // W, F4
        (true, false, 0x09) if shift => Cmd::Prev,     // Tab
        (true, false, 0x09 | 0x22) => Cmd::Next,       // Tab, PageDown
        (true, false, 0x21) => Cmd::Prev,              // PageUp
        (true, false, 0x31..=0x38) => Cmd::Nth((vk - 0x31) as usize),
        (true, false, 0x39) => Cmd::Last,
        (true, false, 0x4C | 0x20) | (false, true, 0x44) | (false, false, 0x75) => Cmd::Palette, // Ctrl+L, Ctrl+Space, Alt+D, F6
        (false, false, 0x7A) => Cmd::Fullscreen, // F11
        _ => return None,
    })
}

/// Address-bar text to URL: explicit URLs and paths pass through, host-like text gets https, the rest is searched.
fn resolve(input: &str) -> Option<String> {
    let s = input.trim();
    if s.is_empty() {
        return None;
    }
    let lower = s.to_ascii_lowercase();
    let b = s.as_bytes();
    Some(if s.contains("://")
        || ["about:", "data:", "file:", "mailto:", "view-source:"].iter().any(|p| lower.starts_with(p))
    {
        s.into()
    } else if b.len() > 2 && b[0].is_ascii_alphabetic() && b[1] == b':' && matches!(b[2], b'\\' | b'/') {
        format!("file:///{}", s.replace('\\', "/").replace(' ', "%20"))
    } else if s.contains(char::is_whitespace) {
        search(s)
    } else if lower == "localhost" || lower.starts_with("localhost:") || lower.starts_with("localhost/") || lower.starts_with("127.") {
        format!("http://{s}")
    } else if s.contains('.') && !s.starts_with('.') && !s.ends_with('.') {
        // shortcut: any dotted word is treated as a host ("node.js" opens https://node.js); type "? node.js" to search.
        format!("https://{s}")
    } else {
        search(s)
    })
}

fn search(q: &str) -> String {
    let mut u = String::from(SEARCH);
    for b in q.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => u.push(b as char),
            b' ' => u.push('+'),
            _ => {
                let _ = write!(u, "%{b:02X}");
            }
        }
    }
    u
}

/// A JS string literal that is safe to splice into evaluate_script.
fn js_str(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            c if (c as u32) < 0x20 || c == '\u{2028}' || c == '\u{2029}' => {
                let _ = write!(o, "\\u{:04x}", c as u32);
            }
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_input() {
        assert_eq!(resolve("  "), None);
        assert_eq!(resolve("https://a.b/c").unwrap(), "https://a.b/c");
        assert_eq!(resolve("about:blank").unwrap(), "about:blank");
        assert_eq!(resolve("example.com").unwrap(), "https://example.com");
        assert_eq!(resolve("localhost:3000/x").unwrap(), "http://localhost:3000/x");
        assert_eq!(resolve(r"C:\a b\x.html").unwrap(), "file:///C:/a%20b/x.html");
        assert_eq!(resolve("rust lang").unwrap(), format!("{SEARCH}rust+lang"));
        assert_eq!(resolve("c++").unwrap(), format!("{SEARCH}c%2B%2B"));
        assert_eq!(resolve("? node.js").unwrap(), format!("{SEARCH}%3F+node.js"));
        assert_eq!(resolve("javascript:alert(1)").unwrap(), format!("{SEARCH}javascript%3Aalert%281%29"));
    }

    #[test]
    fn escapes_js() {
        assert_eq!(js_str(r#"a"b\c"#), r#""a\"b\\c""#);
        assert_eq!(js_str("x\ny\u{2028}"), r#""x\u000ay\u2028""#);
        assert_eq!(js_str("</script>"), r#""</script>""#);
    }

    #[test]
    fn maps_keys() {
        assert_eq!(key_cmd(0x54, true, false, false), Some(Cmd::NewTab));
        assert_eq!(key_cmd(0x54, true, true, false), None);
        assert_eq!(key_cmd(0x09, true, false, false), Some(Cmd::Next));
        assert_eq!(key_cmd(0x09, true, true, false), Some(Cmd::Prev));
        assert_eq!(key_cmd(0x33, true, false, false), Some(Cmd::Nth(2)));
        assert_eq!(key_cmd(0x39, true, false, false), Some(Cmd::Last));
        assert_eq!(key_cmd(0x44, false, false, true), Some(Cmd::Palette));
        assert_eq!(key_cmd(0x4C, false, false, false), None);
        assert_eq!(key_cmd(0x7A, false, false, false), Some(Cmd::Fullscreen));
    }
}
