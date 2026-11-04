//! Whoz: a minimal Windows browser on WebView2. The page fills the window; the toolbar and tabs
//! live in one transparent overlay view that is clipped to what is visible.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod ext;
mod store;
mod win;

use serde_json::{json, Value};
use std::{
    borrow::Cow,
    cell::{Cell, RefCell},
    path::{Path, PathBuf},
    rc::Rc,
    sync::Mutex,
};
use store::{now, Download, ExtInfo, SessTab, Settings, Store, ENGINES};
use tao::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy},
    platform::windows::{WindowBuilderExtWindows, WindowExtWindows},
    window::{Fullscreen, ResizeDirection, Window, WindowBuilder},
};
use webview2_com::{
    take_pwstr, AcceleratorKeyPressedEventHandler, BrowserExtensionEnableCompletedHandler,
    BrowserExtensionRemoveCompletedHandler, BytesReceivedChangedEventHandler,
    ClearBrowsingDataCompletedHandler, ContainsFullScreenElementChangedEventHandler,
    DownloadStartingEventHandler, FaviconChangedEventHandler, FocusChangedEventHandler,
    HistoryChangedEventHandler, IsDocumentPlayingAudioChangedEventHandler,
    Microsoft::Web::WebView2::Win32::*, ProfileAddBrowserExtensionCompletedHandler,
    ProfileGetBrowserExtensionsCompletedHandler, SourceChangedEventHandler,
    StateChangedEventHandler, WindowCloseRequestedEventHandler, ZoomFactorChangedEventHandler,
};
use windows::{
    core::{w, Interface, BOOL, HSTRING, PWSTR},
    Win32::{
        Foundation::{GetLastError, E_POINTER, ERROR_ALREADY_EXISTS, HWND},
        System::Threading::CreateMutexW,
        UI::{
            Input::KeyboardAndMouse::{GetKeyState, VIRTUAL_KEY, VK_CONTROL, VK_MENU, VK_SHIFT},
            WindowsAndMessaging::{MessageBoxW, MB_ICONERROR},
        },
    },
};
use wry::{
    http::{header::CONTENT_TYPE, Request, Response},
    MemoryUsageLevel, NewWindowResponse, PageLoadEvent, Rect, RequestAsyncResponder, WebContext,
    WebView, WebViewBuilder, WebViewBuilderExtWindows, WebViewExtWindows,
};

const UI: &str = include_str!("ui.html");
const PAGES: &str = include_str!("pages.html");
/// whoz://settings and friends are served from here by the "whoz" custom protocol.
const ORIGIN: &str = "https://whoz.localhost/";
/// Toolbar height, and toolbar plus tab row when tabs are on top (logical px; matches ui.html).
const HEAD: f64 = 40.0;
const HEAD_TOP: f64 = 76.0;
const ZOOMS: [f64; 15] = [0.25, 0.33, 0.5, 0.67, 0.75, 0.8, 0.9, 1.0, 1.1, 1.25, 1.5, 1.75, 2.0, 2.5, 3.0];
// Alt+drag anywhere moves the window.
const DRAG_JS: &str = "addEventListener('mousedown',e=>{if(e.altKey&&e.button===0&&window.ipc){e.preventDefault();e.stopImmediatePropagation();ipc.postMessage('drag')}},true)";
// Extension popups report their content size, like Chrome sizes its popups to fit.
const POPUP_JS: &str = "addEventListener('DOMContentLoaded',()=>{const b=document.body;if(!b)return;const r=()=>{const s=getComputedStyle(b);ipc.postMessage('size\\n'+Math.ceil(b.scrollWidth+parseFloat(s.marginLeft)+parseFloat(s.marginRight))+','+Math.ceil(document.documentElement.scrollHeight))};new ResizeObserver(r).observe(b);r()})";

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
    NewWindow,
    Private,
    Reopen,
    Bookmark,
    Page(&'static str),
    Compact,
    Home,
    DevTools,
}

enum Ev {
    Key(Cmd),
    Drag,
    Title(u32, String),
    Url(u32, String),
    Nav(u32, bool, bool),
    Loading(u32),
    Loaded(u32, String),
    Icon(u32, String),
    Audio(u32, bool),
    Closed(u32),
    Fullscreen(bool),
    Adopt,
    Ui(String),
    Api(String, Vec<u8>, RequestAsyncResponder),
    Exts(Vec<ExtInfo>),
    ExtsChanged,
    ExtAdded(String, Result<String, String>, bool),
    ExtInstalled(Result<PathBuf, String>),
    LoadExt(PathBuf),
    DlStart(Download),
    DlProg(u64, i64, i64),
    DlDone(u64, bool, String, i64),
    PopupSize(f64, f64),
    PopupClose,
    Redraw,
}

/// State the WebView callbacks need; lives behind an Rc so popups can build new tabs.
struct Shared {
    window: Window,
    proxy: EventLoopProxy<Ev>,
    pending: RefCell<Vec<(u32, WebView)>>,
    next_id: Cell<u32>,
    dir: PathBuf,
    private: bool,
    /// The first normal window owns the saved session and window placement.
    primary: bool,
    ops: RefCell<Vec<(u64, ICoreWebView2DownloadOperation)>>,
    next_dl: Cell<u64>,
}

impl Shared {
    fn id(&self) -> u32 {
        let i = self.next_id.get();
        self.next_id.set(i + 1);
        i
    }
}

/// `wv` is None until the tab is first shown, so a restored session costs nothing up front.
#[derive(Default)]
struct Tab {
    id: u32,
    title: String,
    url: String,
    icon: String,
    wv: Option<WebView>,
    pinned: bool,
    loading: bool,
    audio: bool,
    muted: bool,
    back: bool,
    fwd: bool,
    visit: u64,
}

struct App {
    sh: Rc<Shared>,
    env: ICoreWebView2Environment,
    st: Store,
    tabs: Vec<Tab>,
    active: usize,
    last: u32,
    ui: WebView,
    ui_ready: bool,
    pal_open: bool,
    pal_new: bool,
    closed: Vec<SessTab>,
    popup: Option<WebView>,
    /// Right edge and top of the extension popup, in logical px.
    popup_at: (f64, f64),
    visits: u32,
    engine: String,
    quit: bool,
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let flag = |f: &str| args.iter().any(|a| a == f);
    let private = flag("--private") || flag("--incognito");
    let dir = PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap_or_else(|| ".".into())).join("WhozBrowser");
    let _ = std::fs::create_dir_all(dir.join("extensions"));
    let mut st = Store::load(dir.clone(), private);
    let arg_url = args.iter().find(|a| !a.starts_with("--")).and_then(|a| resolve(a, &st.settings.search));
    // The handle stays open for the life of the process; later windows see it exists.
    let primary = !private
        && !flag("--new-window")
        && unsafe { CreateMutexW(None, false, w!("WhozBrowser.Primary")).is_ok() && GetLastError() != ERROR_ALREADY_EXISTS };

    st.primary = primary;
    let mut tabs: Vec<Tab> = vec![];
    if primary && st.settings.startup == "restore" {
        tabs = st.session.iter().map(|s| Tab { url: s.url.clone(), title: s.title.clone(), pinned: s.pinned, ..Default::default() }).collect();
    }
    let mut active = st.session_active.min(tabs.len().saturating_sub(1));
    if let Some(u) = arg_url {
        tabs.push(Tab { url: u, ..Default::default() });
        active = tabs.len() - 1;
    }
    if tabs.is_empty() {
        tabs.push(Tab { url: st.settings.home.clone(), ..Default::default() });
        active = 0;
    }

    let event_loop = EventLoopBuilder::<Ev>::with_user_event().build();
    let mut wb = WindowBuilder::new()
        .with_title(if private { "Whoz (Private)" } else { "Whoz" })
        .with_decorations(false)
        .with_undecorated_shadow(true)
        .with_min_inner_size(PhysicalSize::new(480, 320))
        .with_inner_size(tao::dpi::LogicalSize::new(1280.0, 800.0));
    if let (true, Some([x, y, w, h, max])) = (primary, st.window) {
        wb = wb
            .with_position(PhysicalPosition::new(x, y))
            .with_inner_size(PhysicalSize::new(w.max(480) as u32, h.max(320) as u32))
            .with_maximized(max != 0);
    }
    let window = wb.build(&event_loop).unwrap_or_else(|e| fail(&e.to_string()));
    let sh = Rc::new(Shared {
        window,
        proxy: event_loop.create_proxy(),
        pending: RefCell::default(),
        next_id: Cell::new(0),
        dir,
        private,
        primary,
        ops: RefCell::default(),
        next_dl: Cell::new(now()),
    });
    for t in &mut tabs {
        t.id = sh.id();
    }

    let first = new_tab(&sh, None, tabs[active].id, Some(&tabs[active].url))
        .unwrap_or_else(|e| fail(&format!("Microsoft Edge WebView2 Runtime could not start:\n{e}")));
    let env = first.environment();
    tabs[active].wv = Some(first);
    let p = sh.proxy.clone();
    let ui = WebViewBuilder::new()
        .with_environment(env.clone())
        .with_html(UI)
        .with_bounds(full_rect(&sh.window))
        .with_transparent(true)
        .with_focused(false)
        .with_devtools(false)
        .with_ipc_handler(move |r| {
            let _ = p.send_event(Ev::Ui(r.into_body()));
        })
        .build_as_child(&sh.window)
        .unwrap_or_else(|e| fail(&e.to_string()));
    let _ = hook(&ui, &sh, None);
    // Nothing of the overlay is shown (or takes clicks) until ui.html reports its regions.
    win::set_region(hwnd(&ui), "");

    let mut engine = PWSTR::null();
    let engine = unsafe { env.BrowserVersionString(&mut engine).map(|_| take_pwstr(engine)).unwrap_or_default() };
    let mut app = App {
        env,
        sh,
        st,
        tabs,
        active: usize::MAX,
        last: u32::MAX,
        ui,
        ui_ready: false,
        pal_open: false,
        pal_new: false,
        closed: vec![],
        popup: None,
        popup_at: (0.0, 0.0),
        visits: 0,
        engine,
        quit: false,
    };
    app.switch(active);
    if !app.st.settings.sync_dir.is_empty() && !private {
        app.sync();
    }
    app.apply_settings();
    app.load_exts();

    event_loop.run(move |event, _, cf| {
        *cf = ControlFlow::Wait;
        match event {
            Event::UserEvent(e) => app.on(e),
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => app.quit = true,
                WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => app.resize(),
                WindowEvent::Focused(true) => app.focused(),
                _ => {}
            },
            _ => {}
        }
        if app.quit {
            app.shutdown();
            *cf = ControlFlow::Exit;
        }
    });
}

fn fail(msg: &str) -> ! {
    unsafe { MessageBoxW(None, &HSTRING::from(msg), &HSTRING::from("Whoz"), MB_ICONERROR) };
    std::process::exit(1)
}

fn hwnd(wv: &WebView) -> HWND {
    wv.hwnd()
}

fn full_rect(w: &Window) -> Rect {
    Rect { position: PhysicalPosition::new(0, 0).into(), size: w.inner_size().into() }
}

fn reply(r: RequestAsyncResponder, status: u16, mime: &str, body: Cow<'static, [u8]>) {
    // Whoz pages must not be framed by other sites (clickjacking on settings).
    if let Ok(res) = Response::builder().status(status).header(CONTENT_TYPE, mime).header("X-Frame-Options", "DENY").body(body) {
        r.respond(res);
    }
}

fn reply_json(r: RequestAsyncResponder, res: Result<Value, String>) {
    let (status, v) = match res {
        Ok(v) => (200, v),
        Err(e) => (400, json!({ "error": e })),
    };
    reply(r, status, "application/json", v.to_string().into_bytes().into());
}

/// whoz:// pages, and their JSON API. The API needs a custom header, so other sites
/// cannot reach it: the header forces a CORS preflight, which this handler never approves.
fn serve(px: &Mutex<EventLoopProxy<Ev>>, req: Request<Vec<u8>>, resp: RequestAsyncResponder) {
    let local = req.uri().host() == Some("localhost");
    let path = req.uri().path().trim_start_matches('/').to_string();
    if !local {
        // shortcut: wry intercepts every https://whoz.* host, so real whoz.* sites can't load.
        return reply(resp, 404, "text/html", Cow::Borrowed(b"<h1>whoz.* addresses are reserved for Whoz pages.</h1>"));
    }
    let Some(api) = path.strip_prefix("api/") else {
        return reply(resp, 200, "text/html; charset=utf-8", Cow::Borrowed(PAGES.as_bytes()));
    };
    let h = req.headers();
    let same = h.get("origin").is_none_or(|o| o.as_bytes() == b"https://whoz.localhost");
    if !h.contains_key("x-whoz") || !same {
        return reply(resp, 403, "text/plain", Cow::Borrowed(b"forbidden"));
    }
    let key = format!("{api}?{}", req.uri().query().unwrap_or(""));
    if let Ok(p) = px.lock() {
        let _ = p.send_event(Ev::Api(key, req.into_body(), resp));
    }
}

/// Builds a hidden tab. `env == None` only for the first tab: it creates the shared profile.
fn new_tab(sh: &Rc<Shared>, env: Option<&ICoreWebView2Environment>, id: u32, url: Option<&str>) -> wry::Result<WebView> {
    let mut ctx;
    let builder = match env {
        Some(e) => WebViewBuilder::new().with_environment(e.clone()),
        None => {
            ctx = WebContext::new(Some(sh.dir.join("profile")));
            WebViewBuilder::new_with_web_context(&mut ctx).with_browser_extensions_enabled(true)
        }
    };
    let (p1, p2, p3) = (sh.proxy.clone(), sh.proxy.clone(), sh.proxy.clone());
    let api = Mutex::new(sh.proxy.clone());
    let sh2 = sh.clone();
    let mut b = builder
        .with_bounds(full_rect(&sh.window))
        .with_visible(false)
        .with_focused(false)
        .with_hotkeys_zoom(true)
        .with_devtools(true)
        .with_incognito(sh.private)
        .with_https_scheme(true)
        .with_initialization_script(DRAG_JS)
        // Pages can only ask for a window drag; every other UI message comes from ui.html.
        .with_ipc_handler(move |r| {
            if r.body() == "drag" {
                let _ = p1.send_event(Ev::Drag);
            }
        })
        .with_document_title_changed_handler(move |t| {
            let _ = p2.send_event(Ev::Title(id, t));
        })
        .with_on_page_load_handler(move |e, url| {
            let _ = p3.send_event(match e {
                PageLoadEvent::Started => Ev::Loading(id),
                PageLoadEvent::Finished => Ev::Loaded(id, url),
            });
        })
        .with_asynchronous_custom_protocol("whoz".into(), move |_, req, resp| serve(&api, req, resp))
        .with_new_window_req_handler(move |_, f| adopt(&sh2, &f.opener.environment));
    if let Some(u) = url {
        b = b.with_url(u);
    }
    let wv = b.build_as_child(&sh.window)?;
    hook(&wv, sh, Some(id)).map_err(|e| wry::Error::WebView2Error(e.into()))?;
    Ok(wv)
}

/// Popups become tabs but keep window.opener, so OAuth popups work.
fn adopt(sh: &Rc<Shared>, env: &ICoreWebView2Environment) -> NewWindowResponse {
    let id = sh.id();
    match new_tab(sh, Some(env), id, None) {
        Ok(wv) => {
            let webview = wv.webview();
            sh.pending.borrow_mut().push((id, wv));
            let _ = sh.proxy.send_event(Ev::Adopt);
            NewWindowResponse::Create { webview }
        }
        Err(_) => NewWindowResponse::Deny,
    }
}

/// Shortcuts for every WebView; page state and downloads only for tabs.
fn hook(wv: &WebView, sh: &Rc<Shared>, tab: Option<u32>) -> windows::core::Result<()> {
    let mut tk = 0;
    let p = sh.proxy.clone();
    unsafe {
        wv.controller().add_AcceleratorKeyPressed(
            &AcceleratorKeyPressedEventHandler::create(Box::new(move |_, args| {
                let Some(args) = args else { return Ok(()) };
                let (mut kind, mut vk) = (COREWEBVIEW2_KEY_EVENT_KIND::default(), 0);
                args.KeyEventKind(&mut kind)?;
                args.VirtualKey(&mut vk)?;
                if kind == COREWEBVIEW2_KEY_EVENT_KIND_KEY_DOWN || kind == COREWEBVIEW2_KEY_EVENT_KIND_SYSTEM_KEY_DOWN {
                    let down = |k: VIRTUAL_KEY| GetKeyState(k.0 as i32) < 0;
                    if let Some(c) = key_cmd(vk, down(VK_CONTROL), down(VK_SHIFT), down(VK_MENU)) {
                        args.SetHandled(true)?;
                        let _ = p.send_event(Ev::Key(c));
                    }
                }
                Ok(())
            })),
            &mut tk,
        )?;
        let Some(id) = tab else { return Ok(()) };
        let p = sh.proxy.clone();
        // Ctrl+wheel zoom changes the zoom badge.
        wv.controller().add_ZoomFactorChanged(
            &ZoomFactorChangedEventHandler::create(Box::new(move |_, _| {
                let _ = p.send_event(Ev::Redraw);
                Ok(())
            })),
            &mut tk,
        )?;
        let core = wv.webview();
        let p = sh.proxy.clone();
        core.add_ContainsFullScreenElementChanged(
            &ContainsFullScreenElementChangedEventHandler::create(Box::new(move |s, _| {
                let mut on = BOOL::default();
                if let Some(s) = s {
                    s.ContainsFullScreenElement(&mut on)?;
                }
                let _ = p.send_event(Ev::Fullscreen(on.as_bool()));
                Ok(())
            })),
            &mut tk,
        )?;
        let p = sh.proxy.clone();
        core.add_WindowCloseRequested(
            &WindowCloseRequestedEventHandler::create(Box::new(move |_, _| {
                let _ = p.send_event(Ev::Closed(id));
                Ok(())
            })),
            &mut tk,
        )?;
        let p = sh.proxy.clone();
        core.add_SourceChanged(
            &SourceChangedEventHandler::create(Box::new(move |s, _| {
                if let Some(s) = s {
                    let mut u = PWSTR::null();
                    s.Source(&mut u)?;
                    let _ = p.send_event(Ev::Url(id, take_pwstr(u)));
                }
                Ok(())
            })),
            &mut tk,
        )?;
        let p = sh.proxy.clone();
        core.add_HistoryChanged(
            &HistoryChangedEventHandler::create(Box::new(move |s, _| {
                if let Some(s) = s {
                    let (mut b, mut f) = (BOOL::default(), BOOL::default());
                    s.CanGoBack(&mut b)?;
                    s.CanGoForward(&mut f)?;
                    let _ = p.send_event(Ev::Nav(id, b.as_bool(), f.as_bool()));
                }
                Ok(())
            })),
            &mut tk,
        )?;
        if let Ok(c) = core.cast::<ICoreWebView2_15>() {
            let p = sh.proxy.clone();
            c.add_FaviconChanged(
                &FaviconChangedEventHandler::create(Box::new(move |s, _| {
                    if let Some(s) = s.and_then(|s| s.cast::<ICoreWebView2_15>().ok()) {
                        let mut u = PWSTR::null();
                        s.FaviconUri(&mut u)?;
                        let _ = p.send_event(Ev::Icon(id, take_pwstr(u)));
                    }
                    Ok(())
                })),
                &mut tk,
            )?;
        }
        if let Ok(c) = core.cast::<ICoreWebView2_8>() {
            let p = sh.proxy.clone();
            c.add_IsDocumentPlayingAudioChanged(
                &IsDocumentPlayingAudioChangedEventHandler::create(Box::new(move |s, _| {
                    if let Some(s) = s.and_then(|s| s.cast::<ICoreWebView2_8>().ok()) {
                        let mut on = BOOL::default();
                        s.IsDocumentPlayingAudio(&mut on)?;
                        let _ = p.send_event(Ev::Audio(id, on.as_bool()));
                    }
                    Ok(())
                })),
                &mut tk,
            )?;
        }
        if let Ok(c) = core.cast::<ICoreWebView2_4>() {
            let sh = sh.clone();
            c.add_DownloadStarting(
                &DownloadStartingEventHandler::create(Box::new(move |_, args| {
                    let Some(args) = args else { return Ok(()) };
                    // Whoz shows its own download list instead of the WebView2 flyout.
                    args.SetHandled(true)?;
                    let op = args.DownloadOperation()?;
                    let dl = sh.next_dl.get();
                    sh.next_dl.set(dl + 1);
                    let (mut uri, mut path, mut total) = (PWSTR::null(), PWSTR::null(), 0i64);
                    op.Uri(&mut uri)?;
                    args.ResultFilePath(&mut path)?;
                    op.TotalBytesToReceive(&mut total)?;
                    let _ = sh.proxy.send_event(Ev::DlStart(Download {
                        id: dl,
                        url: take_pwstr(uri),
                        path: take_pwstr(path),
                        state: "progress".into(),
                        bytes: 0,
                        total,
                        time: now(),
                    }));
                    let mut tk = 0;
                    let (p, last) = (sh.proxy.clone(), Cell::new(0u64));
                    op.add_BytesReceivedChanged(
                        &BytesReceivedChangedEventHandler::create(Box::new(move |o, _| {
                            let t = now();
                            if let (Some(o), true) = (o, t > last.get() + 250) {
                                last.set(t);
                                let (mut b, mut n) = (0, 0);
                                o.BytesReceived(&mut b)?;
                                o.TotalBytesToReceive(&mut n)?;
                                let _ = p.send_event(Ev::DlProg(dl, b, n));
                            }
                            Ok(())
                        })),
                        &mut tk,
                    )?;
                    let p = sh.proxy.clone();
                    op.add_StateChanged(
                        &StateChangedEventHandler::create(Box::new(move |o, _| {
                            let Some(o) = o else { return Ok(()) };
                            let mut s = COREWEBVIEW2_DOWNLOAD_STATE::default();
                            o.State(&mut s)?;
                            if s != COREWEBVIEW2_DOWNLOAD_STATE_IN_PROGRESS {
                                let (mut path, mut b) = (PWSTR::null(), 0);
                                o.ResultFilePath(&mut path)?;
                                o.BytesReceived(&mut b)?;
                                let ok = s == COREWEBVIEW2_DOWNLOAD_STATE_COMPLETED;
                                let _ = p.send_event(Ev::DlDone(dl, ok, take_pwstr(path), b));
                            }
                            Ok(())
                        })),
                        &mut tk,
                    )?;
                    sh.ops.borrow_mut().push((dl, op));
                    Ok(())
                })),
                &mut tk,
            )?;
        }
    }
    Ok(())
}

/// whoz.localhost URLs show as whoz:// in the address bar.
fn display(url: &str) -> String {
    match url.strip_prefix(ORIGIN) {
        Some(rest) => format!("whoz://{rest}"),
        None => url.into(),
    }
}

fn file_name(path: &str) -> &str {
    path.rsplit(['\\', '/']).next().unwrap_or(path)
}

impl App {
    fn on(&mut self, e: Ev) {
        match e {
            Ev::Key(c) => self.run(c),
            Ev::Drag => {
                let _ = self.sh.window.drag_window();
            }
            Ev::Title(id, t) => {
                if let Some(i) = self.find(id) {
                    if i == self.active {
                        self.sh.window.set_title(&t);
                    }
                    if self.tabs[i].visit != 0 {
                        self.st.retitle(self.tabs[i].visit, &t);
                    }
                    self.tabs[i].title = t;
                    self.render();
                }
            }
            Ev::Url(id, u) => {
                if let Some(i) = self.find(id) {
                    self.tabs[i].url = u;
                    self.tabs[i].visit = 0;
                    self.render();
                }
            }
            Ev::Nav(id, b, f) => {
                if let Some(i) = self.find(id) {
                    (self.tabs[i].back, self.tabs[i].fwd) = (b, f);
                    self.render();
                }
            }
            Ev::Loading(id) => {
                if let Some(i) = self.find(id) {
                    self.tabs[i].loading = true;
                    self.render();
                }
            }
            Ev::Loaded(id, url) => {
                if let Some(i) = self.find(id) {
                    let t = &mut self.tabs[i];
                    t.loading = false;
                    if !self.st.private && url.starts_with("http") && !url.starts_with(ORIGIN) && t.visit == 0 {
                        t.visit = self.st.visit(&url, &t.title);
                        self.visits += 1;
                        if self.visits.is_multiple_of(20) {
                            self.st.save_history();
                        }
                    }
                    self.render();
                }
            }
            Ev::Icon(id, u) => {
                if let Some(i) = self.find(id) {
                    self.tabs[i].icon = u;
                    self.render();
                }
            }
            Ev::Audio(id, on) => {
                if let Some(i) = self.find(id) {
                    self.tabs[i].audio = on;
                    self.render();
                }
            }
            Ev::Closed(id) => {
                if let Some(i) = self.find(id) {
                    self.close(i);
                }
            }
            Ev::Fullscreen(on) => self.set_fullscreen(on),
            Ev::Adopt => {
                let new: Vec<_> = self.sh.pending.borrow_mut().drain(..).collect();
                for (id, wv) in new {
                    let at = (self.active + 1).min(self.tabs.len());
                    self.tabs.insert(at, Tab { id, wv: Some(wv), ..Default::default() });
                    self.switch(at);
                }
            }
            Ev::Ui(m) => self.on_ui(&m),
            Ev::Api(key, body, resp) => self.api_call(&key, &body, resp),
            Ev::Exts(list) => {
                self.st.ext_live = list;
                self.render();
            }
            Ev::ExtsChanged => self.refresh_exts(),
            Ev::ExtAdded(path, r, announce) => match r {
                Ok(id) => {
                    self.st.exts.insert(id.clone(), path);
                    self.st.save_local();
                    self.refresh_exts();
                    if announce {
                        let name = self.ext_name(&id);
                        self.toast(&format!("Added {name}"));
                    }
                }
                Err(e) if announce => self.toast(&format!("Could not add the extension: {e}")),
                Err(_) => {}
            },
            Ev::ExtInstalled(r) => match r {
                Ok(p) => self.add_ext(&p, true),
                Err(e) => self.toast(&format!("Could not install: {e}")),
            },
            Ev::LoadExt(p) => self.add_ext(&p, true),
            Ev::DlStart(d) => {
                self.toast(&format!("Downloading {}", file_name(&d.path)));
                self.st.downloads.push(d);
                self.render();
            }
            Ev::DlProg(id, b, n) => {
                if let Some(d) = self.st.downloads.iter_mut().find(|d| d.id == id) {
                    (d.bytes, d.total) = (b, n);
                    self.render();
                }
            }
            Ev::DlDone(id, ok, path, b) => {
                self.sh.ops.borrow_mut().retain(|(i, _)| *i != id);
                if let Some(d) = self.st.downloads.iter_mut().find(|d| d.id == id) {
                    d.state = if ok { "done" } else { "failed" }.into();
                    d.bytes = b;
                    if !path.is_empty() {
                        d.path = path;
                    }
                    let msg = if ok { format!("Downloaded {}", file_name(&d.path)) } else { format!("Download failed: {}", file_name(&d.path)) };
                    self.st.save_downloads();
                    self.toast(&msg);
                }
                self.render();
            }
            Ev::PopupSize(w, h) => self.place_popup(w, h),
            Ev::PopupClose => self.popup = None,
            Ev::Redraw => self.render(),
        }
    }

    fn find(&self, id: u32) -> Option<usize> {
        self.tabs.iter().position(|t| t.id == id)
    }

    fn wv(&self) -> Option<&WebView> {
        self.tabs.get(self.active).and_then(|t| t.wv.as_ref())
    }

    fn cur_url(&self) -> &str {
        self.tabs.get(self.active).map_or("", |t| t.url.as_str())
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
            Cmd::NewWindow => spawn(&["--new-window"]),
            Cmd::Private => spawn(&["--private"]),
            Cmd::Reopen => {
                if let Some(t) = self.closed.pop() {
                    self.add(&t.url);
                    self.tabs[self.active].pinned = t.pinned;
                }
            }
            Cmd::Bookmark => self.star(),
            Cmd::Page(p) => self.open_page(p),
            Cmd::Compact => {
                let mut s = self.st.settings.clone();
                s.compact = !s.compact;
                self.set_settings(s);
            }
            Cmd::Home => {
                let h = self.st.settings.home.clone();
                self.navigate(&h);
            }
            Cmd::DevTools => {
                if let Some(w) = self.wv() {
                    w.open_devtools();
                }
            }
        }
    }

    fn set_fullscreen(&mut self, on: bool) {
        self.sh.window.set_fullscreen(on.then_some(Fullscreen::Borderless(None)));
        self.resize();
    }

    fn navigate(&mut self, url: &str) {
        if let Some(w) = self.wv() {
            let _ = w.load_url(url);
        }
        self.hide_palette();
    }

    fn add(&mut self, url: &str) {
        let id = self.sh.id();
        match new_tab(&self.sh, Some(&self.env), id, Some(url)) {
            Ok(wv) => {
                let at = (self.active + 1).min(self.tabs.len());
                let at = at.max(self.tabs.iter().take_while(|t| t.pinned).count());
                self.tabs.insert(at, Tab { id, url: url.into(), wv: Some(wv), ..Default::default() });
                self.switch(at);
            }
            Err(e) => self.toast(&format!("Could not open a tab: {e}")),
        }
    }

    fn open_page(&mut self, page: &str) {
        let base = page.split('#').next().unwrap_or(page);
        let url = format!("{ORIGIN}{page}");
        match self.tabs.iter().position(|t| t.url.starts_with(&format!("{ORIGIN}{base}"))) {
            Some(i) => {
                self.switch(i);
                if let Some(w) = self.wv() {
                    let _ = w.load_url(&url);
                }
            }
            None => self.add(&url),
        }
    }

    fn switch(&mut self, i: usize) {
        if i >= self.tabs.len() {
            return;
        }
        if i != self.active {
            if let Some(t) = self.tabs.get(self.active) {
                self.last = t.id;
                if let Some(w) = &t.wv {
                    let _ = w.set_visible(false);
                    // Hidden tabs give memory back to the OS.
                    let _ = w.set_memory_usage_level(MemoryUsageLevel::Low);
                }
            }
        }
        self.active = i;
        self.hide_palette();
        if self.tabs[i].wv.is_none() {
            let t = &self.tabs[i];
            match new_tab(&self.sh, Some(&self.env), t.id, Some(&t.url)) {
                Ok(w) => self.tabs[i].wv = Some(w),
                Err(e) => self.toast(&format!("Could not open the tab: {e}")),
            }
        }
        let r = self.page_rect();
        let t = &self.tabs[i];
        if let Some(w) = &t.wv {
            let _ = w.set_bounds(r);
            let _ = w.set_memory_usage_level(MemoryUsageLevel::Normal);
            let _ = w.set_visible(true);
            let _ = w.focus();
        }
        let title = if t.title.is_empty() { "Whoz".to_string() } else { t.title.clone() };
        self.sh.window.set_title(&title);
        self.render();
        self.save_session();
    }

    fn close(&mut self, i: usize) {
        if i >= self.tabs.len() {
            return;
        }
        let gone = self.tabs.remove(i);
        if !gone.url.is_empty() && !self.sh.private {
            self.closed.push(SessTab { url: gone.url.clone(), title: gone.title.clone(), pinned: gone.pinned });
        }
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
            self.save_session();
        }
        drop(gone); // after the next tab is shown, so there is no blank frame
    }

    fn move_tab(&mut self, from: usize, to: usize) {
        if from >= self.tabs.len() || to >= self.tabs.len() || from == to {
            return;
        }
        let id = self.tabs[self.active].id;
        let t = self.tabs.remove(from);
        self.tabs.insert(to, t);
        self.active = self.find(id).unwrap_or(0);
        self.render();
        self.save_session();
    }

    fn tab_action(&mut self, action: &str, i: usize) {
        if i >= self.tabs.len() {
            return;
        }
        match action {
            "reload" => {
                if let Some(w) = &self.tabs[i].wv {
                    let _ = w.reload();
                }
            }
            "duplicate" => {
                let u = self.tabs[i].url.clone();
                self.switch(i);
                self.add(&u);
            }
            "pin" => {
                let pinned = !self.tabs[i].pinned;
                self.tabs[i].pinned = pinned;
                let to = self.tabs.iter().filter(|t| t.pinned).count() - usize::from(pinned);
                self.move_tab(i, to.min(self.tabs.len() - 1));
                self.render();
            }
            "mute" => {
                let t = &mut self.tabs[i];
                t.muted = !t.muted;
                if let Some(c) = t.wv.as_ref().and_then(|w| w.webview().cast::<ICoreWebView2_8>().ok()) {
                    let _ = unsafe { c.SetIsMuted(t.muted) };
                }
                self.render();
            }
            "close" => self.close(i),
            "others" | "below" => {
                let keep = self.tabs[i].id;
                let below = action == "below";
                let mut j = self.tabs.len();
                while j > 0 {
                    j -= 1;
                    let t = &self.tabs[j];
                    if t.id != keep && !t.pinned && (!below || j > i) {
                        self.close(j);
                    }
                }
            }
            _ => {}
        }
    }

    fn page_rect(&self) -> Rect {
        let w = &self.sh.window;
        let s = &self.st.settings;
        if s.compact || w.fullscreen().is_some() {
            return full_rect(w);
        }
        let k = w.scale_factor();
        let size = w.inner_size();
        let top = ((if s.tabs_left { HEAD } else { HEAD_TOP }) * k).round() as u32;
        let left = if s.tabs_left { (s.side_w as f64 * k).round() as u32 } else { 0 };
        Rect {
            position: PhysicalPosition::new(left as i32, top as i32).into(),
            size: PhysicalSize::new(size.width.saturating_sub(left), size.height.saturating_sub(top)).into(),
        }
    }

    fn resize(&mut self) {
        let _ = self.ui.set_bounds(full_rect(&self.sh.window));
        let r = self.page_rect();
        if let Some(w) = self.wv() {
            let _ = w.set_bounds(r);
        }
        self.render(); // maximized state may have changed
    }

    fn focused(&mut self) {
        // Another window may have changed settings or bookmarks.
        let (settings, bookmarks) = Store::peek(&self.sh.dir);
        if settings.mtime > self.st.settings.mtime {
            self.st.settings = settings;
            self.apply_settings();
        }
        store::merge_bookmarks(&mut self.st.bookmarks, bookmarks);
        let wv = if self.pal_open { Some(&self.ui) } else { self.wv() };
        if let Some(w) = wv {
            let _ = w.focus();
        }
        self.render();
    }

    fn profile(&self) -> Option<ICoreWebView2Profile> {
        let w = self.wv().unwrap_or(&self.ui);
        unsafe { w.webview().cast::<ICoreWebView2_13>().ok()?.Profile().ok() }
    }

    /// Extensions live in the regular profile, which the overlay view always uses.
    fn ext_profile(&self) -> Option<ICoreWebView2Profile7> {
        unsafe { self.ui.webview().cast::<ICoreWebView2_13>().ok()?.Profile().ok()?.cast().ok() }
    }

    fn apply_settings(&mut self) {
        let s = self.st.settings.clone();
        let scheme = match s.theme.as_str() {
            "light" => COREWEBVIEW2_PREFERRED_COLOR_SCHEME_LIGHT,
            "dark" => COREWEBVIEW2_PREFERRED_COLOR_SCHEME_DARK,
            _ => COREWEBVIEW2_PREFERRED_COLOR_SCHEME_AUTO,
        };
        let level = match s.tracking.as_str() {
            "none" => COREWEBVIEW2_TRACKING_PREVENTION_LEVEL_NONE,
            "basic" => COREWEBVIEW2_TRACKING_PREVENTION_LEVEL_BASIC,
            "strict" => COREWEBVIEW2_TRACKING_PREVENTION_LEVEL_STRICT,
            _ => COREWEBVIEW2_TRACKING_PREVENTION_LEVEL_BALANCED,
        };
        // The tab profile differs from the overlay's in private windows; theme both.
        let ui_profile = unsafe { self.ui.webview().cast::<ICoreWebView2_13>().ok().and_then(|c| c.Profile().ok()) };
        if let Some(p) = ui_profile {
            let _ = unsafe { p.SetPreferredColorScheme(scheme) };
        }
        if let Some(p) = self.profile() {
            unsafe {
                let _ = p.SetPreferredColorScheme(scheme);
                if let Ok(p3) = p.cast::<ICoreWebView2Profile3>() {
                    let _ = p3.SetPreferredTrackingPreventionLevel(level);
                }
                if let Ok(p6) = p.cast::<ICoreWebView2Profile6>() {
                    let _ = p6.SetIsPasswordAutosaveEnabled(s.passwords);
                    let _ = p6.SetIsGeneralAutofillEnabled(s.autofill);
                }
                let dl = if s.download_dir.is_empty() { win::downloads_dir() } else { PathBuf::from(&s.download_dir) };
                let _ = p.SetDefaultDownloadFolderPath(&HSTRING::from(dl.as_os_str()));
            }
        }
        self.resize();
    }

    fn set_settings(&mut self, mut s: Settings) {
        s.side_w = s.side_w.clamp(180, 480);
        if !s.search.contains("%s") {
            s.search = self.st.settings.search.clone();
        }
        if !s.home.contains(':') {
            s.home = resolve(&s.home, &s.search).unwrap_or_else(|| self.st.settings.home.clone());
        }
        s.mtime = now();
        let resync = s.sync_dir != self.st.settings.sync_dir && !s.sync_dir.is_empty();
        self.st.settings = s;
        self.st.save_local();
        self.apply_settings();
        if resync {
            self.sync();
        }
    }

    fn sync(&mut self) {
        self.st.sync_error = self.st.sync().err().unwrap_or_default();
        self.render();
    }

    fn save_session(&mut self) {
        if !self.sh.primary {
            return;
        }
        self.st.session = self
            .tabs
            .iter()
            .filter(|t| !t.url.is_empty())
            .map(|t| SessTab { url: t.url.clone(), title: t.title.clone(), pinned: t.pinned })
            .collect();
        self.st.session_active = self.active.min(self.tabs.len().saturating_sub(1));
        self.st.save_local();
    }

    fn shutdown(&mut self) {
        let w = &self.sh.window;
        if self.sh.primary && w.fullscreen().is_none() {
            let max = w.is_maximized();
            let [mut x, mut y, mut ww, mut hh, _] = self.st.window.unwrap_or([100, 100, 1280, 800, 0]);
            // A maximized window keeps the size it restores to.
            if !max {
                if let Ok(p) = w.outer_position() {
                    (x, y) = (p.x, p.y);
                }
                let s = w.inner_size();
                (ww, hh) = (s.width as i32, s.height as i32);
            }
            self.st.window = Some([x, y, ww, hh, max as i32]);
        }
        self.save_session();
        self.st.save_history();
        self.st.save_downloads();
        if !self.st.settings.sync_dir.is_empty() && !self.sh.private {
            let _ = self.st.sync();
        }
    }

    /// Pushes everything the overlay shows to ui.html.
    fn render(&self) {
        if !self.ui_ready {
            return;
        }
        let w = &self.sh.window;
        let s = &self.st.settings;
        let cur = self.cur_url();
        let t = self.tabs.get(self.active);
        let zoom = self.wv().map_or(1.0, |w| {
            let mut z = 1.0;
            let _ = unsafe { w.controller().ZoomFactor(&mut z) };
            z
        });
        let tabs: Vec<Value> = self
            .tabs
            .iter()
            .enumerate()
            .map(|(i, t)| {
                json!({ "t": tab_title(t), "u": display(&t.url), "i": t.icon, "a": i == self.active,
                        "p": t.pinned, "l": t.loading, "au": t.audio, "m": t.muted })
            })
            .collect();
        let live: Vec<&Download> = self.st.downloads.iter().filter(|d| d.state == "progress").collect();
        let (got, total) = live.iter().fold((0i64, 0i64), |a, d| (a.0 + d.bytes, a.1 + d.total.max(0)));
        let recent: Vec<Value> = self
            .st
            .downloads
            .iter()
            .rev()
            .take(5)
            .map(|d| json!({ "id": d.id, "n": file_name(&d.path), "s": d.state, "b": d.bytes, "t": d.total }))
            .collect();
        let mut folders: Vec<&str> = self.st.bookmarks.iter().filter(|b| !b.deleted && !b.folder.is_empty()).map(|b| b.folder.as_str()).collect();
        folders.sort_unstable();
        folders.dedup();
        let exts: Vec<Value> = self
            .st
            .ext_live
            .iter()
            .filter(|e| e.enabled)
            .map(|e| json!({ "id": e.id, "n": e.name, "i": e.icon, "p": !e.popup.is_empty() || !e.options.is_empty() }))
            .collect();
        let store = ext::store_id(cur).filter(|id| ext::store_page(cur) && !self.st.ext_live.iter().any(|e| &e.id == id));
        let state = json!({
            "tabs": tabs, "left": s.tabs_left, "compact": s.compact, "side": s.side_w, "theme": s.theme,
            "max": w.is_maximized(), "full": w.fullscreen().is_some(), "private": self.sh.private,
            "back": t.is_some_and(|t| t.back), "fwd": t.is_some_and(|t| t.fwd), "loading": t.is_some_and(|t| t.loading),
            "url": display(cur), "secure": cur.starts_with("https://"), "internal": cur.starts_with(ORIGIN),
            "star": self.st.bookmark(cur).map(|b| json!({ "title": b.title, "folder": b.folder })),
            "folders": folders, "zoom": (zoom * 100.0).round(),
            "dl": { "n": live.len(), "p": if total > 0 { got as f64 / total as f64 } else { -1.0 }, "list": recent },
            "exts": exts, "store": store,
        });
        let _ = self.ui.evaluate_script(&format!("render({state})"));
    }

    fn toast(&self, msg: &str) {
        if self.ui_ready {
            let _ = self.ui.evaluate_script(&format!("toast({})", json!(msg)));
        }
    }

    fn open_palette(&mut self, new: bool) {
        if !self.ui_ready {
            return;
        }
        self.pal_new = new;
        self.pal_open = true;
        let cur = if new { String::new() } else { display(self.cur_url()) };
        // Focus the view first: a programmatic MoveFocus resets focus inside it and would blur the input.
        let _ = self.ui.focus();
        let _ = self.ui.evaluate_script(&format!("pal({},{new},{})", json!(cur), self.suggest("")));
    }

    fn hide_palette(&mut self) {
        if self.pal_open {
            self.pal_open = false;
            let _ = self.ui.evaluate_script("closePal()");
            if let Some(w) = self.wv() {
                let _ = w.focus();
            }
        }
    }

    /// Address bar rows: what you typed, open tabs, bookmarks, history.
    fn suggest(&self, q: &str) -> Value {
        let mut rows = vec![];
        let q = q.trim();
        let ql = q.to_lowercase();
        if let Some(u) = resolve(q, &self.st.settings.search) {
            let engine = ENGINES.iter().find(|e| e.1 == self.st.settings.search).map_or("the web", |e| e.0);
            let searching = u == search(q, &self.st.settings.search);
            rows.push(if searching {
                json!({ "k": "search", "t": q, "u": format!("Search {engine}") })
            } else {
                json!({ "k": "go", "t": display(&u), "u": "Open" })
            });
        }
        for (i, t) in self.tabs.iter().enumerate() {
            if rows.len() >= 6 {
                break;
            }
            let title = tab_title(t);
            if i != self.active && (ql.is_empty() || title.to_lowercase().contains(&ql) || t.url.to_lowercase().contains(&ql)) {
                rows.push(json!({ "k": "tab", "t": title, "u": display(&t.url), "i": i, "icon": t.icon }));
            }
        }
        for (k, t, u) in self.st.suggest(q, 12 - rows.len().min(6)) {
            if !rows.iter().any(|r| r["u"] == u.as_str()) {
                rows.push(json!({ "k": k, "t": if t.is_empty() { u.clone() } else { t }, "u": u }));
            }
        }
        Value::Array(rows)
    }

    fn star(&mut self) {
        let url = self.cur_url().to_string();
        if url.is_empty() {
            return;
        }
        if self.st.bookmark(&url).is_none() {
            let title = self.tabs.get(self.active).map(tab_title).unwrap_or_default();
            self.st.set_bookmark(&url, &title, "");
        }
        self.render();
        let _ = self.ui.focus();
        let _ = self.ui.evaluate_script("openStar()");
    }

    fn zoom(&self, dir: i32) {
        let Some(w) = self.wv() else { return };
        let c = w.controller();
        let mut z = 1.0;
        unsafe {
            let _ = c.ZoomFactor(&mut z);
            let n = match dir {
                0 => 1.0,
                d if d > 0 => *ZOOMS.iter().find(|&&s| s > z + 0.001).unwrap_or(&z),
                _ => *ZOOMS.iter().rev().find(|&&s| s < z - 0.001).unwrap_or(&z),
            };
            let _ = c.SetZoomFactor(n);
        }
        self.render();
    }

    fn on_ui(&mut self, msg: &str) {
        let w = &self.sh.window;
        let mut parts = msg.splitn(3, '\n');
        let cmd = parts.next().unwrap_or_default();
        let arg = parts.next().unwrap_or_default();
        let arg2 = parts.next().unwrap_or_default();
        let idx = arg.parse::<usize>().ok().filter(|&i| i < self.tabs.len());
        match cmd {
            "ready" => {
                self.ui_ready = true;
                self.render();
            }
            "rgn" => win::set_region(hwnd(&self.ui), arg),
            "hide" => {
                self.pal_open = false;
                if let Some(w) = self.wv() {
                    let _ = w.focus();
                }
            }
            "tab" => {
                if let Some(i) = idx {
                    self.switch(i);
                }
            }
            "x" => {
                if let Some(i) = idx {
                    self.close(i);
                }
            }
            "mv" => {
                if let (Some(i), Ok(j)) = (idx, arg2.parse::<usize>()) {
                    self.move_tab(i, j.min(self.tabs.len() - 1));
                }
            }
            "tabctx" => {
                if let Ok(i) = arg2.parse() {
                    self.tab_action(arg, i);
                }
            }
            "q" => {
                let _ = self.ui.evaluate_script(&format!("sug({},{})", json!(arg), self.suggest(arg)));
            }
            "go" => {
                let Some(url) = resolve(arg, &self.st.settings.search) else { return };
                if self.pal_new {
                    self.add(&url);
                } else {
                    self.navigate(&url);
                }
            }
            "new" => self.open_palette(true),
            "pal" => self.open_palette(false),
            "back" | "fwd" | "stop" => {
                if let Some(c) = self.wv().map(|w| w.webview()) {
                    let _ = unsafe {
                        match cmd {
                            "back" => c.GoBack(),
                            "fwd" => c.GoForward(),
                            _ => c.Stop(),
                        }
                    };
                }
            }
            "reload" => {
                if let Some(w) = self.wv() {
                    let _ = w.reload();
                }
            }
            "home" => self.run(Cmd::Home),
            "star" => self.star(),
            "bm" => {
                let v: Value = serde_json::from_str(arg).unwrap_or_default();
                let url = self.cur_url().to_string();
                self.st.set_bookmark(&url, v["title"].as_str().unwrap_or_default(), v["folder"].as_str().unwrap_or_default());
                self.render();
            }
            "unbm" => {
                let url = self.cur_url().to_string();
                self.st.del_bookmark(&url);
                self.render();
            }
            "layout" | "compact" => {
                let mut s = self.st.settings.clone();
                if cmd == "layout" {
                    s.tabs_left = !s.tabs_left;
                } else {
                    s.compact = !s.compact;
                }
                self.set_settings(s);
            }
            "side" => {
                // Live while dragging the sidebar edge; saved on release.
                if let Ok(v) = arg.parse::<u32>() {
                    self.st.settings.side_w = v.clamp(180, 480);
                    let r = self.page_rect();
                    if let Some(w) = self.wv() {
                        let _ = w.set_bounds(r);
                    }
                    if arg2 == "save" {
                        let s = self.st.settings.clone();
                        self.set_settings(s);
                    }
                }
            }
            "page" => {
                let p = match arg {
                    "history" => "history",
                    "downloads" => "downloads",
                    "bookmarks" => "bookmarks",
                    "extensions" => "extensions",
                    _ => "settings",
                };
                self.open_page(p);
            }
            "newwin" => self.run(Cmd::NewWindow),
            "reopen" => self.run(Cmd::Reopen),
            "private" => self.run(Cmd::Private),
            "print" => {
                if let Some(w) = self.wv() {
                    let _ = w.print();
                }
            }
            "zoom" => self.zoom(arg.parse().unwrap_or(0)),
            "full" => self.run(Cmd::Fullscreen),
            "devtools" => self.run(Cmd::DevTools),
            "ext" => {
                let xy: Vec<f64> = arg2.split(',').filter_map(|v| v.parse().ok()).collect();
                if let [x, y] = xy[..] {
                    self.open_popup(arg, x, y);
                }
            }
            "addext" => {
                if let Some(id) = ext::store_id(self.cur_url()) {
                    self.install_ext(id);
                }
            }
            "dl" => self.download_action(arg, arg2.parse().unwrap_or(0)),
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

    fn download_action(&mut self, action: &str, id: u64) {
        let Some(i) = self.st.downloads.iter().position(|d| d.id == id) else {
            if action == "clear" {
                self.st.downloads.retain(|d| d.state == "progress");
                self.st.save_downloads();
                self.render();
            }
            return;
        };
        let path = self.st.downloads[i].path.clone();
        match action {
            "open" => win::open(&path),
            "show" => win::reveal(&path),
            "cancel" | "remove" => {
                if let Some((_, op)) = self.sh.ops.borrow().iter().find(|(d, _)| *d == id) {
                    let _ = unsafe { op.Cancel() };
                }
                if action == "remove" {
                    self.st.downloads.remove(i);
                    self.st.save_downloads();
                }
            }
            "retry" => {
                let u = self.st.downloads[i].url.clone();
                if let Some(w) = self.wv() {
                    let _ = w.load_url(&u);
                }
            }
            _ => {}
        }
        self.render();
    }

    // ---- extensions ----

    fn ext_name(&self, id: &str) -> String {
        self.st.ext_live.iter().find(|e| e.id == id).map_or_else(|| "the extension".into(), |e| e.name.clone())
    }

    /// Adds every known extension folder; the profile keeps extensions, so most of these are
    /// already there and only refresh the id -> folder map Whoz reads popups and icons from.
    fn load_exts(&mut self) {
        let mut dirs: Vec<String> = self.st.exts.values().cloned().collect();
        if let Ok(rd) = std::fs::read_dir(self.sh.dir.join("extensions")) {
            for e in rd.flatten() {
                let p = e.path().to_string_lossy().into_owned();
                if e.path().join("manifest.json").is_file() && !dirs.contains(&p) {
                    dirs.push(p);
                }
            }
        }
        for d in dirs {
            self.add_ext(Path::new(&d), false);
        }
        self.refresh_exts();
    }

    fn add_ext(&self, path: &Path, announce: bool) {
        let Some(p7) = self.ext_profile() else {
            if announce {
                self.toast("This WebView2 Runtime has no extension support. Update Microsoft Edge WebView2.");
            }
            return;
        };
        let px = self.sh.proxy.clone();
        let ps = path.to_string_lossy().into_owned();
        unsafe {
            let _ = p7.AddBrowserExtension(
                &HSTRING::from(path.as_os_str()),
                &ProfileAddBrowserExtensionCompletedHandler::create(Box::new(move |hr, e| {
                    let r = (|| {
                        hr?;
                        let e = e.ok_or(windows::core::Error::from(E_POINTER))?;
                        let mut id = PWSTR::null();
                        e.Id(&mut id)?;
                        Ok::<_, windows::core::Error>(take_pwstr(id))
                    })();
                    let _ = px.send_event(Ev::ExtAdded(ps, r.map_err(|e| e.message()), announce));
                    Ok(())
                })),
            );
        }
    }

    fn refresh_exts(&self) {
        let Some(p7) = self.ext_profile() else { return };
        let (map, px) = (self.st.exts.clone(), self.sh.proxy.clone());
        unsafe {
            let _ = p7.GetBrowserExtensions(&ProfileGetBrowserExtensionsCompletedHandler::create(Box::new(move |hr, list| {
                hr?;
                let mut out = vec![];
                if let Some(l) = list {
                    let mut n = 0;
                    l.Count(&mut n)?;
                    for i in 0..n {
                        let e = l.GetValueAtIndex(i)?;
                        let (mut id, mut name, mut on) = (PWSTR::null(), PWSTR::null(), BOOL::default());
                        e.Id(&mut id)?;
                        e.Name(&mut name)?;
                        e.IsEnabled(&mut on)?;
                        let id = take_pwstr(id);
                        let (popup, options, icon) = map.get(&id).map(|d| ext::manifest(Path::new(d))).unwrap_or_default();
                        out.push(ExtInfo { id, name: take_pwstr(name), enabled: on.as_bool(), popup, options, icon });
                    }
                }
                out.sort_by_key(|e| e.name.to_lowercase());
                let _ = px.send_event(Ev::Exts(out));
                Ok(())
            })));
        }
    }

    /// Runs `f` on the extension with this id, then refreshes the list.
    fn with_ext(&self, id: &str, f: impl FnOnce(&ICoreWebView2BrowserExtension, EventLoopProxy<Ev>) -> windows::core::Result<()> + 'static) {
        let Some(p7) = self.ext_profile() else { return };
        let (id, px) = (id.to_string(), self.sh.proxy.clone());
        unsafe {
            let _ = p7.GetBrowserExtensions(&ProfileGetBrowserExtensionsCompletedHandler::create(Box::new(move |hr, list| {
                hr?;
                let Some(l) = list else { return Ok(()) };
                let mut n = 0;
                l.Count(&mut n)?;
                for i in 0..n {
                    let e = l.GetValueAtIndex(i)?;
                    let mut eid = PWSTR::null();
                    e.Id(&mut eid)?;
                    if take_pwstr(eid) == id {
                        return f(&e, px);
                    }
                }
                Ok(())
            })));
        }
    }

    fn install_ext(&self, id: String) {
        self.toast("Installing from the Chrome Web Store…");
        let (dir, ver, px) = (self.sh.dir.join("extensions"), self.engine.clone(), self.sh.proxy.clone());
        std::thread::spawn(move || {
            let _ = px.send_event(Ev::ExtInstalled(ext::install(&id, &dir, &ver)));
        });
    }

    fn open_popup(&mut self, id: &str, x: f64, y: f64) {
        self.popup = None;
        let Some(e) = self.st.ext_live.iter().find(|e| e.id == id) else { return };
        if e.popup.is_empty() {
            if !e.options.is_empty() {
                let u = format!("chrome-extension://{id}/{}", e.options.trim_start_matches('/'));
                self.add(&u);
            } else {
                self.toast(&format!("{} has no popup.", e.name));
            }
            return;
        }
        let url = format!("chrome-extension://{id}/{}", e.popup.trim_start_matches('/'));
        let k = self.sh.window.scale_factor();
        self.popup_at = (x, y);
        let px = self.sh.proxy.clone();
        let sh = self.sh.clone();
        let built = WebViewBuilder::new()
            .with_environment(self.env.clone())
            .with_url(&url)
            // A usable size until the page reports its own (or if it never does).
            .with_bounds(Rect {
                position: PhysicalPosition::new(((x - 380.0).max(0.0) * k) as i32, (y * k) as i32).into(),
                size: PhysicalSize::new((380.0 * k) as u32, (480.0 * k) as u32).into(),
            })
            .with_devtools(true)
            .with_initialization_script(POPUP_JS)
            .with_ipc_handler(move |r| {
                let b = r.body();
                if let Some((w, h)) = b.strip_prefix("size\n").and_then(|s| s.split_once(',')) {
                    if let (Ok(w), Ok(h)) = (w.parse(), h.parse()) {
                        let _ = px.send_event(Ev::PopupSize(w, h));
                    }
                }
            })
            .with_new_window_req_handler(move |_, f| adopt(&sh, &f.opener.environment))
            .build_as_child(&self.sh.window);
        let Ok(p) = built else { return self.toast("Could not open the extension popup.") };
        let _ = hook(&p, &self.sh, None);
        unsafe {
            let (mut tk, px) = (0, self.sh.proxy.clone());
            let _ = p.controller().add_LostFocus(
                &FocusChangedEventHandler::create(Box::new(move |_, _| {
                    let _ = px.send_event(Ev::PopupClose);
                    Ok(())
                })),
                &mut tk,
            );
            let px = self.sh.proxy.clone();
            let _ = p.webview().add_WindowCloseRequested(
                &WindowCloseRequestedEventHandler::create(Box::new(move |_, _| {
                    let _ = px.send_event(Ev::PopupClose);
                    Ok(())
                })),
                &mut tk,
            );
        }
        win::raise(hwnd(&p));
        let _ = p.focus();
        self.popup = Some(p);
    }

    fn place_popup(&self, w: f64, h: f64) {
        let Some(p) = &self.popup else { return };
        let k = self.sh.window.scale_factor();
        let win = self.sh.window.inner_size();
        let (w, h) = (w.clamp(120.0, 800.0), h.clamp(40.0, 600.0));
        let (x, y) = self.popup_at;
        let left = ((x - w) * k).max(0.0).min(win.width as f64 - w * k);
        let _ = p.set_bounds(Rect {
            position: PhysicalPosition::new(left as i32, (y * k) as i32).into(),
            size: PhysicalSize::new((w * k) as u32, (h * k) as u32).into(),
        });
    }

    // ---- whoz:// API ----

    fn api_call(&mut self, key: &str, body: &[u8], resp: RequestAsyncResponder) {
        let (path, q) = key.split_once('?').unwrap_or((key, ""));
        // The folder picker is modal; run it off the event loop and answer from there.
        if path == "pick" || path == "extensions/load" {
            let (owner, px, load) = (self.sh.window.hwnd(), self.sh.proxy.clone(), path != "pick");
            std::thread::spawn(move || {
                let p = win::pick_folder(owner);
                if let (true, Some(p)) = (load, &p) {
                    let _ = px.send_event(Ev::LoadExt(p.clone()));
                }
                reply_json(resp, Ok(json!(p.map(|p| p.to_string_lossy().into_owned()))));
            });
            return;
        }
        let r = self.api(path, q, body);
        reply_json(resp, r);
    }

    fn api(&mut self, path: &str, q: &str, body: &[u8]) -> Result<Value, String> {
        let p = |k: &str| store::query_param(q, k);
        let body_json = || serde_json::from_slice::<Value>(body).map_err(|e| e.to_string());
        Ok(match path {
            "state" => json!({
                "settings": self.st.settings, "engines": ENGINES, "version": env!("CARGO_PKG_VERSION"),
                "engine": self.engine, "private": self.sh.private, "lastSync": self.st.last_sync,
                "syncError": self.st.sync_error, "downloadsDefault": win::downloads_dir(), "data": self.sh.dir,
            }),
            "settings" => {
                let s: Settings = serde_json::from_slice(body).map_err(|e| e.to_string())?;
                self.set_settings(s);
                json!(self.st.settings)
            }
            "sync" => {
                self.sync();
                if !self.st.sync_error.is_empty() {
                    return Err(self.st.sync_error.clone());
                }
                json!(self.st.last_sync)
            }
            "history" => json!(self.st.history_page(&p("q"), p("n").parse().unwrap_or(500))),
            "history/delete" => {
                self.st.del_visit(p("time").parse().unwrap_or(0));
                json!(true)
            }
            "clear" => {
                self.clear(&p("range"), &p("what"));
                json!(true)
            }
            "bookmarks" => json!(self.st.bookmarks.iter().filter(|b| !b.deleted).collect::<Vec<_>>()),
            "bookmarks/set" => {
                let v = body_json()?;
                let s = |k: &str| v[k].as_str().unwrap_or_default().trim().to_string();
                let url = resolve(&s("url"), &self.st.settings.search).ok_or("Enter an address.")?;
                let old = s("old");
                if !old.is_empty() && old != url {
                    self.st.del_bookmark(&old);
                }
                self.st.set_bookmark(&url, &s("title"), &s("folder"));
                self.render();
                json!(true)
            }
            "bookmarks/delete" => {
                self.st.del_bookmark(&p("url"));
                self.render();
                json!(true)
            }
            "bookmarks/import" => {
                let from = p("from");
                let file = store::chromium_bookmarks_path(&from).ok_or("Unknown browser.")?;
                let text = std::fs::read_to_string(&file).map_err(|_| format!("No bookmarks found for {from} on this PC."))?;
                let n = self.st.import(store::parse_chromium_bookmarks(&text));
                self.render();
                json!(n)
            }
            "bookmarks/export" => json!(store::export_html(&self.st.bookmarks)),
            "downloads" => json!(self.st.downloads.iter().rev().collect::<Vec<_>>()),
            "downloads/action" => {
                self.download_action(&p("do"), p("id").parse().unwrap_or(0));
                json!(true)
            }
            "extensions" => json!(self.st.ext_live),
            "extensions/install" => {
                let id = ext::store_id(&p("id")).ok_or("Paste a Chrome Web Store link or a 32-letter extension ID.")?;
                self.install_ext(id);
                json!(true)
            }
            "extensions/toggle" => {
                let on = p("on") == "1";
                self.with_ext(&p("id"), move |e, px| unsafe {
                    e.Enable(on, &BrowserExtensionEnableCompletedHandler::create(Box::new(move |_| {
                        let _ = px.send_event(Ev::ExtsChanged);
                        Ok(())
                    })))
                });
                json!(true)
            }
            "extensions/remove" => {
                let id = p("id");
                if let Some(dir) = self.st.exts.remove(&id) {
                    // Only folders Whoz downloaded are deleted; "Load unpacked" folders stay.
                    if Path::new(&dir).starts_with(self.sh.dir.join("extensions")) {
                        let _ = std::fs::remove_dir_all(&dir);
                    }
                    self.st.save_local();
                }
                self.with_ext(&id, |e, px| unsafe {
                    e.Remove(&BrowserExtensionRemoveCompletedHandler::create(Box::new(move |_| {
                        let _ = px.send_event(Ev::ExtsChanged);
                        Ok(())
                    })))
                });
                json!(true)
            }
            "extensions/options" => {
                let id = p("id");
                let e = self.st.ext_live.iter().find(|e| e.id == id).ok_or("Extension not found.")?;
                if e.options.is_empty() {
                    return Err("This extension has no options page.".into());
                }
                let u = format!("chrome-extension://{id}/{}", e.options.trim_start_matches('/'));
                self.add(&u);
                json!(true)
            }
            "default-browser" => {
                win::open("ms-settings:defaultapps?registeredAppUser=Whoz");
                json!(true)
            }
            "open" => {
                win::open(&p("path"));
                json!(true)
            }
            _ => return Err(format!("Unknown request: {path}")),
        })
    }

    fn clear(&mut self, range: &str, what: &str) {
        let hour = 3_600_000;
        let span = match range {
            "hour" => hour,
            "day" => 24 * hour,
            "week" => 7 * 24 * hour,
            "month" => 28 * 24 * hour,
            _ => u64::MAX,
        };
        let since = now().saturating_sub(span);
        let since = if span == u64::MAX { 0 } else { since };
        let has = |k: &str| what.split(',').any(|w| w == k);
        let mut kinds = 0;
        if has("history") {
            self.st.clear_history(since);
            kinds |= COREWEBVIEW2_BROWSING_DATA_KINDS_BROWSING_HISTORY.0;
        }
        if has("downloads") {
            self.st.downloads.retain(|d| d.state == "progress" || d.time < since);
            self.st.save_downloads();
            kinds |= COREWEBVIEW2_BROWSING_DATA_KINDS_DOWNLOAD_HISTORY.0;
        }
        if has("cookies") {
            kinds |= COREWEBVIEW2_BROWSING_DATA_KINDS_ALL_SITE.0;
        }
        if has("cache") {
            kinds |= COREWEBVIEW2_BROWSING_DATA_KINDS_DISK_CACHE.0 | COREWEBVIEW2_BROWSING_DATA_KINDS_CACHE_STORAGE.0;
        }
        if has("passwords") {
            kinds |= COREWEBVIEW2_BROWSING_DATA_KINDS_PASSWORD_AUTOSAVE.0;
        }
        if has("autofill") {
            kinds |= COREWEBVIEW2_BROWSING_DATA_KINDS_GENERAL_AUTOFILL.0;
        }
        if let Some(p2) = self.profile().and_then(|p| p.cast::<ICoreWebView2Profile2>().ok()) {
            let handler = ClearBrowsingDataCompletedHandler::create(Box::new(|_| Ok(())));
            unsafe {
                let _ = p2.ClearBrowsingDataInTimeRange(COREWEBVIEW2_BROWSING_DATA_KINDS(kinds), since as f64 / 1000.0, now() as f64 / 1000.0, &handler);
            }
        }
        self.render();
    }
}

fn tab_title(t: &Tab) -> String {
    if !t.title.is_empty() {
        return t.title.clone();
    }
    let u = display(&t.url);
    let host = u.split("://").nth(1).and_then(|r| r.split('/').next()).unwrap_or("");
    if host.is_empty() { "New tab".into() } else { host.into() }
}

fn spawn(args: &[&str]) {
    if let Ok(exe) = std::env::current_exe() {
        let _ = std::process::Command::new(exe).args(args).spawn();
    }
}

fn key_cmd(vk: u32, ctrl: bool, shift: bool, alt: bool) -> Option<Cmd> {
    Some(match (ctrl, shift, alt, vk) {
        (true, false, false, 0x54) => Cmd::NewTab,        // T
        (true, true, false, 0x54) => Cmd::Reopen,         // Shift+T
        (true, false, false, 0x4E) => Cmd::NewWindow,     // N
        (true, true, false, 0x4E) => Cmd::Private,        // Shift+N
        (true, _, false, 0x57 | 0x73) => Cmd::CloseTab,   // W, F4
        (true, true, false, 0x09) => Cmd::Prev,           // Tab
        (true, false, false, 0x09 | 0x22) => Cmd::Next,   // Tab, PageDown
        (true, false, false, 0x21) => Cmd::Prev,          // PageUp
        (true, false, false, 0x31..=0x38) => Cmd::Nth((vk - 0x31) as usize),
        (true, false, false, 0x39) => Cmd::Last,
        (true, false, false, 0x4C | 0x20) | (false, false, true, 0x44) | (false, false, false, 0x75) => Cmd::Palette, // Ctrl+L, Ctrl+Space, Alt+D, F6
        (true, false, false, 0x44) => Cmd::Bookmark,      // D
        (true, false, false, 0x48) => Cmd::Page("history"),     // H
        (true, false, false, 0x4A) => Cmd::Page("downloads"),   // J
        (true, true, false, 0x4F) => Cmd::Page("bookmarks"),    // Shift+O
        (true, true, false, 0x2E) => Cmd::Page("settings#privacy"), // Shift+Delete
        (true, true, false, 0x42) => Cmd::Compact,        // Shift+B
        (false, false, true, 0x24) => Cmd::Home,          // Alt+Home
        (true, true, false, 0x49) | (false, false, false, 0x7B) => Cmd::DevTools, // Ctrl+Shift+I, F12
        (false, false, false, 0x7A) => Cmd::Fullscreen,   // F11
        _ => return None,
    })
}

/// Address-bar text to URL: explicit URLs and paths pass through, host-like text gets https, the rest is searched.
fn resolve(input: &str, engine: &str) -> Option<String> {
    let s = input.trim();
    if s.is_empty() {
        return None;
    }
    let lower = s.to_ascii_lowercase();
    if let Some(rest) = lower.strip_prefix("whoz://") {
        return Some(format!("{ORIGIN}{rest}"));
    }
    let b = s.as_bytes();
    Some(if s.contains("://")
        || ["about:", "data:", "file:", "mailto:", "view-source:", "edge:"].iter().any(|p| lower.starts_with(p))
    {
        s.into()
    } else if b.len() > 2 && b[0].is_ascii_alphabetic() && b[1] == b':' && matches!(b[2], b'\\' | b'/') {
        format!("file:///{}", s.replace('\\', "/").replace(' ', "%20"))
    } else if s.contains(char::is_whitespace) {
        search(s, engine)
    } else if lower == "localhost" || lower.starts_with("localhost:") || lower.starts_with("localhost/") || lower.starts_with("127.") {
        format!("http://{s}")
    } else if s.contains('.') && !s.starts_with('.') && !s.ends_with('.') {
        // shortcut: any dotted word is treated as a host ("node.js" opens https://node.js); type "? node.js" to search.
        format!("https://{s}")
    } else {
        search(s, engine)
    })
}

fn search(q: &str, engine: &str) -> String {
    let mut enc = String::new();
    for b in q.trim().bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => enc.push(b as char),
            b' ' => enc.push('+'),
            _ => enc.push_str(&format!("%{b:02X}")),
        }
    }
    engine.replace("%s", &enc)
}

#[cfg(test)]
mod tests {
    use super::*;
    const G: &str = "https://www.google.com/search?q=%s";

    #[test]
    fn resolves_input() {
        let s = |q: &str| G.replace("%s", q);
        assert_eq!(resolve("  ", G), None);
        assert_eq!(resolve("https://a.b/c", G).unwrap(), "https://a.b/c");
        assert_eq!(resolve("about:blank", G).unwrap(), "about:blank");
        assert_eq!(resolve("example.com", G).unwrap(), "https://example.com");
        assert_eq!(resolve("localhost:3000/x", G).unwrap(), "http://localhost:3000/x");
        assert_eq!(resolve(r"C:\a b\x.html", G).unwrap(), "file:///C:/a%20b/x.html");
        assert_eq!(resolve("rust lang", G).unwrap(), s("rust+lang"));
        assert_eq!(resolve("c++", G).unwrap(), s("c%2B%2B"));
        assert_eq!(resolve("? node.js", G).unwrap(), s("%3F+node.js"));
        assert_eq!(resolve("javascript:alert(1)", G).unwrap(), s("javascript%3Aalert%281%29"));
        assert_eq!(resolve("whoz://settings", G).unwrap(), "https://whoz.localhost/settings");
        assert_eq!(resolve("hi", "https://duckduckgo.com/?q=%s").unwrap(), "https://duckduckgo.com/?q=hi");
    }

    #[test]
    fn shows_internal_urls() {
        assert_eq!(display("https://whoz.localhost/history"), "whoz://history");
        assert_eq!(display("https://a.b"), "https://a.b");
    }

    #[test]
    fn maps_keys() {
        assert_eq!(key_cmd(0x54, true, false, false), Some(Cmd::NewTab));
        assert_eq!(key_cmd(0x54, true, true, false), Some(Cmd::Reopen));
        assert_eq!(key_cmd(0x09, true, false, false), Some(Cmd::Next));
        assert_eq!(key_cmd(0x09, true, true, false), Some(Cmd::Prev));
        assert_eq!(key_cmd(0x33, true, false, false), Some(Cmd::Nth(2)));
        assert_eq!(key_cmd(0x39, true, false, false), Some(Cmd::Last));
        assert_eq!(key_cmd(0x44, false, false, true), Some(Cmd::Palette));
        assert_eq!(key_cmd(0x44, true, false, false), Some(Cmd::Bookmark));
        assert_eq!(key_cmd(0x4C, false, false, false), None);
        assert_eq!(key_cmd(0x7A, false, false, false), Some(Cmd::Fullscreen));
        assert_eq!(key_cmd(0x48, true, false, false), Some(Cmd::Page("history")));
    }
}
