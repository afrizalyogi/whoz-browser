//! Settings, bookmarks, history and downloads: JSON files in the data folder, plus folder sync.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fmt::Write,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

pub const HIST_MAX: usize = 20_000;

pub const ENGINES: [(&str, &str); 5] = [
    ("Google", "https://www.google.com/search?q=%s"),
    ("Bing", "https://www.bing.com/search?q=%s"),
    ("DuckDuckGo", "https://duckduckgo.com/?q=%s"),
    ("Brave", "https://search.brave.com/search?q=%s"),
    ("Ecosia", "https://www.ecosia.org/search?q=%s"),
];

pub fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(default)]
pub struct Settings {
    /// "system", "light" or "dark".
    pub theme: String,
    pub tabs_left: bool,
    pub side_w: u32,
    /// Header and tabs hide until hovered; off keeps them on screen and shrinks the page.
    pub compact: bool,
    /// "newtab", "restore" or "home".
    pub startup: String,
    pub home: String,
    /// Search URL with %s for the query.
    pub search: String,
    /// "none", "basic", "balanced" or "strict".
    pub tracking: String,
    pub passwords: bool,
    pub autofill: bool,
    pub download_dir: String,
    pub sync_dir: String,
    pub mtime: u64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: "system".into(),
            tabs_left: true,
            side_w: 260,
            compact: true,
            startup: "restore".into(),
            home: "https://www.google.com".into(),
            search: ENGINES[0].1.into(),
            tracking: "balanced".into(),
            passwords: true,
            autofill: true,
            download_dir: String::new(),
            sync_dir: String::new(),
            mtime: 0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Default)]
#[serde(default)]
pub struct Bookmark {
    pub url: String,
    pub title: String,
    pub folder: String,
    pub time: u64,
    /// Last change; the newer copy wins when merging.
    pub mtime: u64,
    /// Kept as a tombstone so a delete survives a sync.
    pub deleted: bool,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Default)]
#[serde(default)]
pub struct Visit {
    pub url: String,
    pub title: String,
    pub time: u64,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Default)]
#[serde(default)]
pub struct Download {
    pub id: u64,
    pub url: String,
    pub path: String,
    /// "progress", "done" or "failed".
    pub state: String,
    pub bytes: i64,
    pub total: i64,
    pub time: u64,
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct ExtInfo {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    /// Paths inside the extension, from its manifest.
    pub popup: String,
    pub options: String,
    /// data: URL of the icon.
    pub icon: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct SessTab {
    pub url: String,
    pub title: String,
    pub pinned: bool,
}

/// Small, often-written state.
#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
struct Local {
    settings: Settings,
    /// Extension id -> folder it was loaded from.
    exts: BTreeMap<String, String>,
    session: Vec<SessTab>,
    active: usize,
    window: Option<[i32; 5]>,
    hist_clear: u64,
    last_sync: u64,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
struct SyncFile {
    settings: Settings,
    bookmarks: Vec<Bookmark>,
    history: Vec<Visit>,
    hist_clear: u64,
}

#[derive(Default)]
pub struct Store {
    pub dir: PathBuf,
    pub private: bool,
    /// Only the primary window writes the session and window placement.
    pub primary: bool,
    pub settings: Settings,
    pub bookmarks: Vec<Bookmark>,
    pub history: Vec<Visit>,
    pub downloads: Vec<Download>,
    pub exts: BTreeMap<String, String>,
    pub ext_live: Vec<ExtInfo>,
    pub session: Vec<SessTab>,
    pub session_active: usize,
    /// x, y, width, height, maximized (device px).
    pub window: Option<[i32; 5]>,
    /// History up to this time was cleared; synced so other devices drop it too.
    pub hist_clear: u64,
    pub last_sync: u64,
    pub sync_error: String,
    loaded_at: u64,
}

fn read<T: for<'a> Deserialize<'a> + Default>(p: &Path) -> T {
    fs::read(p).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}

/// Write to a temp file and rename, so a crash never leaves half a file.
fn write<T: Serialize>(p: &Path, v: &T) -> std::io::Result<()> {
    let tmp = p.with_extension("tmp");
    fs::write(&tmp, serde_json::to_vec(v)?)?;
    fs::rename(tmp, p)
}

impl Store {
    pub fn load(dir: PathBuf, private: bool) -> Self {
        let l: Local = read(&dir.join("settings.json"));
        let mut s = Store {
            private,
            bookmarks: read(&dir.join("bookmarks.json")),
            history: read(&dir.join("history.json")),
            downloads: read(&dir.join("downloads.json")),
            settings: l.settings,
            exts: l.exts,
            session: l.session,
            session_active: l.active,
            window: l.window,
            hist_clear: l.hist_clear,
            last_sync: l.last_sync,
            loaded_at: now(),
            dir,
            ..Default::default()
        };
        if s.settings.search.is_empty() || !s.settings.search.contains("%s") {
            s.settings.search = ENGINES[0].1.into();
        }
        s.settings.side_w = s.settings.side_w.clamp(180, 480);
        if private {
            s.downloads.clear();
        }
        s
    }

    /// Settings and bookmarks as another window last saved them (skips the large history file).
    pub fn peek(dir: &Path) -> (Settings, Vec<Bookmark>) {
        let l: Local = read(&dir.join("settings.json"));
        (l.settings, read(&dir.join("bookmarks.json")))
    }

    pub fn save_local(&self) {
        let disk: Local = if self.primary { Local::default() } else { read(&self.dir.join("settings.json")) };
        let (session, active, window) =
            if self.primary { (self.session.clone(), self.session_active, self.window) } else { (disk.session, disk.active, disk.window) };
        let l = Local {
            settings: self.settings.clone(),
            exts: self.exts.clone(),
            session,
            active,
            window,
            hist_clear: self.hist_clear,
            last_sync: self.last_sync,
        };
        let _ = write(&self.dir.join("settings.json"), &l);
    }

    pub fn save_bookmarks(&self) {
        let _ = write(&self.dir.join("bookmarks.json"), &self.bookmarks);
    }

    pub fn save_downloads(&self) {
        if !self.private {
            let _ = write(&self.dir.join("downloads.json"), &self.downloads);
        }
    }

    /// Another window may have added visits since we loaded; keep those too.
    pub fn save_history(&mut self) {
        if self.private {
            return;
        }
        let disk: Vec<Visit> = read(&self.dir.join("history.json"));
        let since = self.loaded_at;
        merge_history(&mut self.history, disk.into_iter().filter(|v| v.time > since).collect(), self.hist_clear);
        let _ = write(&self.dir.join("history.json"), &self.history);
        self.loaded_at = now();
    }

    pub fn del_visit(&mut self, time: u64) {
        self.history.retain(|v| v.time != time);
        if !self.private {
            let _ = write(&self.dir.join("history.json"), &self.history);
            self.loaded_at = now();
        }
    }

    /// Adds bookmarks that are not there yet; returns how many.
    pub fn import(&mut self, list: Vec<(String, String, String)>) -> usize {
        let mut n = 0;
        for (url, title, folder) in list {
            if self.bookmark(&url).is_none() {
                let t = now();
                self.bookmarks.retain(|b| b.url != url);
                self.bookmarks.push(Bookmark { url, title, folder, time: t, mtime: t, deleted: false });
                n += 1;
            }
        }
        self.save_bookmarks();
        n
    }

    pub fn visit(&mut self, url: &str, title: &str) -> u64 {
        let mut time = now();
        if let Some(l) = self.history.last() {
            time = time.max(l.time + 1);
        }
        self.history.push(Visit { url: url.into(), title: title.into(), time });
        if self.history.len() > HIST_MAX + 1000 {
            self.history.drain(..self.history.len() - HIST_MAX);
        }
        time
    }

    pub fn retitle(&mut self, time: u64, title: &str) {
        if let Some(v) = self.history.iter_mut().rev().take(50).find(|v| v.time == time) {
            v.title = title.into();
        }
    }

    pub fn bookmark(&self, url: &str) -> Option<&Bookmark> {
        self.bookmarks.iter().find(|b| b.url == url && !b.deleted)
    }

    pub fn set_bookmark(&mut self, url: &str, title: &str, folder: &str) {
        let t = now();
        match self.bookmarks.iter_mut().find(|b| b.url == url) {
            Some(b) => {
                if b.deleted {
                    b.time = t;
                }
                b.title = title.into();
                b.folder = folder.into();
                b.deleted = false;
                b.mtime = t;
            }
            None => self.bookmarks.push(Bookmark {
                url: url.into(),
                title: title.into(),
                folder: folder.into(),
                time: t,
                mtime: t,
                deleted: false,
            }),
        }
        self.save_bookmarks();
    }

    pub fn del_bookmark(&mut self, url: &str) {
        if let Some(b) = self.bookmarks.iter_mut().find(|b| b.url == url) {
            b.deleted = true;
            b.mtime = now();
        }
        self.save_bookmarks();
    }

    pub fn clear_history(&mut self, since: u64) {
        self.history.retain(|v| v.time < since);
        if since == 0 {
            self.hist_clear = now();
        }
        if !self.private {
            let _ = write(&self.dir.join("history.json"), &self.history);
            self.loaded_at = now();
        }
        self.save_local();
    }

    /// Newest first, optionally filtered; at most `n`.
    pub fn history_page(&self, q: &str, n: usize) -> Vec<&Visit> {
        let q = q.to_lowercase();
        self.history
            .iter()
            .rev()
            .filter(|v| q.is_empty() || v.url.to_lowercase().contains(&q) || v.title.to_lowercase().contains(&q))
            .take(n)
            .collect()
    }

    /// Address bar matches: bookmarks first, then the most visited history entries.
    pub fn suggest(&self, q: &str, n: usize) -> Vec<(&'static str, String, String)> {
        let q = q.trim().to_lowercase();
        if q.is_empty() {
            return vec![];
        }
        let hit = |u: &str, t: &str| u.to_lowercase().contains(&q) || t.to_lowercase().contains(&q);
        let mut out: Vec<(&'static str, String, String)> = self
            .bookmarks
            .iter()
            .filter(|b| !b.deleted && hit(&b.url, &b.title))
            .take(n)
            .map(|b| ("bm", b.title.clone(), b.url.clone()))
            .collect();
        let mut seen: BTreeMap<&str, (usize, u64, &str)> = BTreeMap::new();
        for v in self.history.iter().rev().take(5000).filter(|v| hit(&v.url, &v.title)) {
            let e = seen.entry(&v.url).or_insert((0, v.time, &v.title));
            e.0 += 1;
        }
        let mut hist: Vec<_> = seen.into_iter().collect();
        hist.sort_by_key(|h| std::cmp::Reverse((h.1 .0, h.1 .1)));
        for (url, (_, _, title)) in hist {
            if out.len() >= n {
                break;
            }
            if !out.iter().any(|o| o.2 == url) {
                out.push(("hist", title.to_string(), url.to_string()));
            }
        }
        out
    }

    /// Merge with `whoz-sync.json` in the sync folder (OneDrive, Google Drive, Dropbox...).
    pub fn sync(&mut self) -> Result<(), String> {
        let dir = PathBuf::from(self.settings.sync_dir.trim());
        if self.settings.sync_dir.trim().is_empty() {
            return Err("Choose a sync folder first.".into());
        }
        if !dir.is_dir() {
            return Err(format!("Folder not found: {}", dir.display()));
        }
        let file = dir.join("whoz-sync.json");
        let remote: SyncFile = match fs::read(&file) {
            Ok(b) => serde_json::from_slice(&b).map_err(|e| format!("whoz-sync.json is damaged: {e}"))?,
            Err(_) => SyncFile::default(),
        };
        merge_bookmarks(&mut self.bookmarks, remote.bookmarks);
        self.hist_clear = self.hist_clear.max(remote.hist_clear);
        merge_history(&mut self.history, remote.history, self.hist_clear);
        if remote.settings.mtime > self.settings.mtime {
            // Folders are per device.
            let (sync_dir, download_dir) = (self.settings.sync_dir.clone(), self.settings.download_dir.clone());
            self.settings = Settings { sync_dir, download_dir, ..remote.settings };
        }
        let out = SyncFile {
            settings: self.settings.clone(),
            bookmarks: self.bookmarks.clone(),
            history: self.history.clone(),
            hist_clear: self.hist_clear,
        };
        write(&file, &out).map_err(|e| format!("Could not write {}: {e}", file.display()))?;
        self.last_sync = now();
        self.save_bookmarks();
        let _ = write(&self.dir.join("history.json"), &self.history);
        self.save_local();
        Ok(())
    }
}

pub fn merge_bookmarks(a: &mut Vec<Bookmark>, b: Vec<Bookmark>) {
    for x in b {
        match a.iter_mut().find(|y| y.url == x.url) {
            Some(y) if x.mtime > y.mtime => *y = x,
            Some(_) => {}
            None => a.push(x),
        }
    }
}

pub fn merge_history(a: &mut Vec<Visit>, b: Vec<Visit>, cleared: u64) {
    a.extend(b);
    a.retain(|v| v.time > cleared);
    a.sort_by_key(|v| v.time);
    a.dedup_by(|x, y| x.time == y.time && x.url == y.url);
    if a.len() > HIST_MAX {
        a.drain(..a.len() - HIST_MAX);
    }
}

/// Bookmarks file of a Chromium browser on this machine ("chrome", "edge", "brave").
pub fn chromium_bookmarks_path(browser: &str) -> Option<PathBuf> {
    let base = PathBuf::from(std::env::var_os("LOCALAPPDATA")?);
    let sub = match browser {
        "chrome" => r"Google\Chrome\User Data\Default\Bookmarks",
        "edge" => r"Microsoft\Edge\User Data\Default\Bookmarks",
        "brave" => r"BraveSoftware\Brave-Browser\User Data\Default\Bookmarks",
        _ => return None,
    };
    Some(base.join(sub))
}

/// Flattens a Chromium `Bookmarks` JSON file into (url, title, folder path).
pub fn parse_chromium_bookmarks(json: &str) -> Vec<(String, String, String)> {
    fn walk(n: &Value, folder: &str, out: &mut Vec<(String, String, String)>) {
        let name = n["name"].as_str().unwrap_or_default();
        match n["type"].as_str() {
            Some("url") => {
                if let Some(u) = n["url"].as_str() {
                    out.push((u.into(), name.into(), folder.into()));
                }
            }
            Some("folder") => {
                let f = if folder.is_empty() { name.to_string() } else { format!("{folder}/{name}") };
                for c in n["children"].as_array().into_iter().flatten() {
                    walk(c, &f, out);
                }
            }
            _ => {}
        }
    }
    let mut out = vec![];
    if let Ok(v) = serde_json::from_str::<Value>(json) {
        if let Some(roots) = v["roots"].as_object() {
            for (k, r) in roots {
                // The bookmarks bar becomes the top level, like in Chrome's bookmark manager.
                let top = if k == "bookmark_bar" { "" } else { r["name"].as_str().unwrap_or(k) };
                for c in r["children"].as_array().into_iter().flatten() {
                    walk(c, top, &mut out);
                }
            }
        }
    }
    out
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// Netscape bookmark file: every browser can import it.
pub fn export_html(bm: &[Bookmark]) -> String {
    let mut o = String::from("<!DOCTYPE NETSCAPE-Bookmark-file-1>\n<META HTTP-EQUIV=\"Content-Type\" CONTENT=\"text/html; charset=UTF-8\">\n<TITLE>Bookmarks</TITLE>\n<H1>Bookmarks</H1>\n<DL><p>\n");
    let mut folders: BTreeMap<&str, Vec<&Bookmark>> = BTreeMap::new();
    for b in bm.iter().filter(|b| !b.deleted) {
        folders.entry(&b.folder).or_default().push(b);
    }
    for (f, items) in folders {
        if !f.is_empty() {
            let _ = writeln!(o, "<DT><H3>{}</H3>\n<DL><p>", html_escape(f));
        }
        for b in items {
            let _ = writeln!(o, "<DT><A HREF=\"{}\" ADD_DATE=\"{}\">{}</A>", html_escape(&b.url), b.time / 1000, html_escape(&b.title));
        }
        if !f.is_empty() {
            o.push_str("</DL><p>\n");
        }
    }
    o.push_str("</DL><p>\n");
    o
}

pub fn pct_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < b.len() => {
                match u8::from_str_radix(std::str::from_utf8(&b[i + 1..i + 3]).unwrap_or("zz"), 16) {
                    Ok(v) => {
                        out.push(v);
                        i += 2;
                    }
                    Err(_) => out.push(b'%'),
                }
            }
            c => out.push(c),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Value of `key` in a URL query string.
pub fn query_param(q: &str, key: &str) -> String {
    q.split('&')
        .filter_map(|kv| kv.split_once('='))
        .find(|(k, _)| *k == key)
        .map(|(_, v)| pct_decode(v))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bm(url: &str, mtime: u64, deleted: bool) -> Bookmark {
        Bookmark { url: url.into(), mtime, deleted, ..Default::default() }
    }

    #[test]
    fn merges_bookmarks_newest_wins() {
        let mut a = vec![bm("a", 1, false), bm("b", 5, false)];
        merge_bookmarks(&mut a, vec![bm("a", 2, true), bm("b", 4, true), bm("c", 1, false)]);
        assert!(a[0].deleted, "newer delete wins");
        assert!(!a[1].deleted, "older delete loses");
        assert_eq!(a.len(), 3);
    }

    #[test]
    fn merges_history() {
        let v = |t| Visit { url: "u".into(), title: String::new(), time: t };
        let mut a = vec![v(1), v(3)];
        merge_history(&mut a, vec![v(2), v(3)], 1);
        assert_eq!(a.iter().map(|v| v.time).collect::<Vec<_>>(), [2, 3]);
    }

    #[test]
    fn parses_chromium_bookmarks() {
        let j = r#"{"roots":{"bookmark_bar":{"children":[{"type":"url","name":"A","url":"https://a"},{"type":"folder","name":"Dev","children":[{"type":"url","name":"B","url":"https://b"}]}],"name":"Bookmarks bar","type":"folder"},"other":{"children":[{"type":"url","name":"C","url":"https://c"}],"name":"Other bookmarks","type":"folder"}}}"#;
        let r = parse_chromium_bookmarks(j);
        assert!(r.contains(&("https://a".into(), "A".into(), "".into())));
        assert!(r.contains(&("https://b".into(), "B".into(), "Dev".into())));
        assert!(r.contains(&("https://c".into(), "C".into(), "Other bookmarks".into())));
    }

    #[test]
    fn decodes_query() {
        assert_eq!(query_param("q=a%20b+c&n=5", "q"), "a b c");
        assert_eq!(query_param("q=100%", "q"), "100%");
        assert_eq!(query_param("x=1", "q"), "");
    }

    #[test]
    fn exports_html() {
        let mut b = bm("https://a?x=1&y=2", 1, false);
        b.title = "<A>".into();
        let h = export_html(&[b, bm("gone", 1, true)]);
        assert!(h.contains("HREF=\"https://a?x=1&amp;y=2\""));
        assert!(h.contains("&lt;A&gt;"));
        assert!(!h.contains("gone"));
    }
}
