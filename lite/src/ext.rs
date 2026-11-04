//! Chrome Web Store installs and manifest reading for WebView2 browser extensions.
use serde_json::Value;
use std::{
    fs,
    os::windows::process::CommandExt,
    path::{Path, PathBuf},
    process::Command,
};

pub const NO_WINDOW: u32 = 0x0800_0000;

/// Extension ID from a Chrome Web Store link or a bare 32-letter ID.
pub fn store_id(s: &str) -> Option<String> {
    let ok = |p: &str| p.len() == 32 && p.bytes().all(|b| (b'a'..=b'p').contains(&b));
    let s = s.trim();
    if ok(s) {
        return Some(s.into());
    }
    s.split(['/', '?', '#']).find(|p| ok(p)).map(Into::into)
}

/// A Chrome Web Store detail page, where Whoz offers "Add to Whoz".
pub fn store_page(url: &str) -> bool {
    (url.starts_with("https://chromewebstore.google.com/detail/")
        || url.starts_with("https://chrome.google.com/webstore/detail/"))
        && store_id(url).is_some()
}

/// The zip inside a CRX2/CRX3 package (or the bytes themselves if it is a plain zip).
pub fn crx_zip(b: &[u8]) -> Option<&[u8]> {
    if b.get(..4)? != b"Cr24" {
        return (b.get(..2)? == b"PK").then_some(b);
    }
    let u = |i: usize| -> Option<usize> { Some(u32::from_le_bytes(b.get(i..i + 4)?.try_into().ok()?) as usize) };
    let off = match u(4)? {
        3 => 12 + u(8)?,
        2 => 16 + u(8)? + u(12)?,
        _ => return None,
    };
    b.get(off..)
}

fn varint(b: &[u8], i: &mut usize) -> Option<u64> {
    let mut v = 0u64;
    for s in 0..10 {
        let c = *b.get(*i)?;
        *i += 1;
        v |= ((c & 0x7f) as u64) << (7 * s);
        if c < 0x80 {
            return Some(v);
        }
    }
    None
}

/// Length-delimited protobuf fields with this number.
fn pb_fields(b: &[u8], want: u64) -> Vec<&[u8]> {
    let (mut i, mut out) = (0, vec![]);
    while i < b.len() {
        let Some(tag) = varint(b, &mut i) else { break };
        match tag & 7 {
            0 => {
                if varint(b, &mut i).is_none() {
                    break;
                }
            }
            1 => i += 8,
            5 => i += 4,
            2 => {
                let Some(n) = varint(b, &mut i) else { break };
                let Some(f) = b.get(i..i + n as usize) else { break };
                if tag >> 3 == want {
                    out.push(f);
                }
                i += n as usize;
            }
            _ => break,
        }
    }
    out
}

/// Chrome extension ID of a public key: the first 16 bytes of its SHA-256, as letters a-p.
pub fn key_id(pubkey: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(pubkey)[..16].iter().flat_map(|b| [b >> 4, b & 15]).map(|n| (b'a' + n) as char).collect()
}

/// The public key in a CRX header whose ID is `id`.
pub fn crx_key(b: &[u8], id: &str) -> Option<Vec<u8>> {
    let u = |i: usize| -> Option<usize> { Some(u32::from_le_bytes(b.get(i..i + 4)?.try_into().ok()?) as usize) };
    match u(4)? {
        2 => Some(b.get(16..16 + u(8)?)?.to_vec()).filter(|k| key_id(k) == id),
        3 => {
            let header = b.get(12..12 + u(8)?)?;
            // CrxFileHeader.sha256_with_rsa (2) -> AsymmetricKeyProof.public_key (1)
            pb_fields(header, 2).into_iter().flat_map(|p| pb_fields(p, 1)).find(|k| key_id(k) == id).map(<[u8]>::to_vec)
        }
        _ => None,
    }
}

fn run(cmd: &mut Command, what: &str) -> Result<(), String> {
    let out = cmd.creation_flags(NO_WINDOW).output().map_err(|e| format!("{what}: {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(format!("{what} failed: {}", String::from_utf8_lossy(&out.stderr).trim()))
    }
}

/// Downloads an extension from the Chrome Web Store and unpacks it to `dir/<id>`. Blocking.
pub fn install(id: &str, dir: &Path, browser_version: &str) -> Result<PathBuf, String> {
    let major = browser_version.split('.').next().filter(|v| !v.is_empty()).unwrap_or("130");
    let url = format!(
        "https://clients2.google.com/service/update2/crx?response=redirect&prodversion={major}.0&acceptformat=crx2,crx3&x=id%3D{id}%26uc"
    );
    let crx = dir.join(format!("{id}.crx"));
    run(
        Command::new("curl.exe").args(["-L", "-f", "-s", "-S", "--max-time", "120", "-o"]).arg(&crx).arg(&url),
        "Download",
    )?;
    let bytes = fs::read(&crx).map_err(|e| e.to_string())?;
    let _ = fs::remove_file(&crx);
    let zip = crx_zip(&bytes).ok_or("The download is not an extension package.")?;
    let zpath = dir.join(format!("{id}.zip"));
    fs::write(&zpath, zip).map_err(|e| e.to_string())?;
    let target = dir.join(id);
    let _ = fs::remove_dir_all(&target);
    fs::create_dir_all(&target).map_err(|e| e.to_string())?;
    // Windows 10+ ships bsdtar, which reads zip files.
    let r = run(Command::new("tar.exe").arg("-xf").arg(&zpath).arg("-C").arg(&target), "Unpacking");
    let _ = fs::remove_file(&zpath);
    r?;
    // Chromium refuses unpacked extensions that contain the store's reserved "_metadata" folder.
    let _ = fs::remove_dir_all(target.join("_metadata"));
    let mf = target.join("manifest.json");
    let text = fs::read_to_string(&mf).map_err(|_| "The package has no manifest.json.")?;
    // An unpacked extension's ID comes from its folder path unless the manifest has the store key.
    // With the key it keeps its Chrome Web Store ID, which sign-in and messaging often rely on.
    if let (Some(key), Ok(mut m)) = (crx_key(&bytes, id), serde_json::from_str::<Value>(text.trim_start_matches('\u{feff}'))) {
        if m.get("key").is_none() {
            m["key"] = Value::String(b64(&key));
            let _ = fs::write(&mf, m.to_string());
        }
    }
    Ok(target)
}

/// (popup page, options page, icon as a data: URL) from an unpacked extension's manifest.
pub fn manifest(dir: &Path) -> (String, String, String) {
    let Some(m) = fs::read_to_string(dir.join("manifest.json"))
        .ok()
        .and_then(|t| serde_json::from_str::<Value>(t.trim_start_matches('\u{feff}')).ok())
    else {
        return Default::default();
    };
    let s = |v: &Value| v.as_str().unwrap_or_default().to_string();
    let act = ["action", "browser_action", "page_action"].iter().map(|k| &m[*k]).find(|v| v.is_object());
    let popup = act.map(|a| s(&a["default_popup"])).unwrap_or_default();
    let options = Some(s(&m["options_ui"]["page"])).filter(|o| !o.is_empty()).unwrap_or_else(|| s(&m["options_page"]));
    let largest = |v: &Value| {
        v.as_object()
            .and_then(|o| o.iter().max_by_key(|(k, _)| k.parse::<u32>().unwrap_or(0)))
            .and_then(|(_, p)| p.as_str())
            .map(String::from)
    };
    let icon = act
        .and_then(|a| a["default_icon"].as_str().map(String::from).or_else(|| largest(&a["default_icon"])))
        .or_else(|| largest(&m["icons"]))
        .and_then(|p| {
            let b = fs::read(dir.join(p.trim_start_matches('/'))).ok()?;
            let mime = if p.ends_with(".svg") { "image/svg+xml" } else { "image/png" };
            Some(format!("data:{mime};base64,{}", b64(&b)))
        })
        .unwrap_or_default();
    (popup, options, icon)
}

pub fn b64(b: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut o = String::with_capacity(b.len().div_ceil(3) * 4);
    for c in b.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        for i in 0..4 {
            o.push(if i <= c.len() { T[(n >> (18 - 6 * i)) as usize & 63] as char } else { '=' });
        }
    }
    o
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_store_id() {
        let id = "cjpalhdlnbpafiamejdnhcphjbkeiagm";
        assert_eq!(store_id(id).as_deref(), Some(id));
        assert_eq!(store_id(&format!("https://chromewebstore.google.com/detail/ublock-origin/{id}?hl=en")).as_deref(), Some(id));
        assert_eq!(store_id("https://example.com/abc"), None);
        assert!(store_page(&format!("https://chromewebstore.google.com/detail/x/{id}")));
        assert!(!store_page(&format!("https://example.com/detail/x/{id}")));
    }

    #[test]
    fn strips_crx_header() {
        let mut v3 = b"Cr24".to_vec();
        v3.extend(3u32.to_le_bytes());
        v3.extend(2u32.to_le_bytes());
        v3.extend(b"hhPKzip");
        assert_eq!(crx_zip(&v3), Some(&b"PKzip"[..]));
        assert_eq!(crx_zip(b"PKplain"), Some(&b"PKplain"[..]));
        assert_eq!(crx_zip(b"<html>"), None);
    }

    #[test]
    fn reads_crx3_key() {
        let key = b"public-key-bytes";
        let id = key_id(key);
        assert_eq!(id.len(), 32);
        // AsymmetricKeyProof { public_key: key } inside CrxFileHeader field 2
        let mut proof = vec![0x0a, key.len() as u8];
        proof.extend(key);
        let mut header = vec![0x12, proof.len() as u8];
        header.extend(&proof);
        let mut crx = b"Cr24".to_vec();
        crx.extend(3u32.to_le_bytes());
        crx.extend((header.len() as u32).to_le_bytes());
        crx.extend(&header);
        assert_eq!(crx_key(&crx, &id).as_deref(), Some(&key[..]));
        assert_eq!(crx_key(&crx, "x"), None);
    }

    #[test]
    fn encodes_base64() {
        assert_eq!(b64(b""), "");
        assert_eq!(b64(b"f"), "Zg==");
        assert_eq!(b64(b"fo"), "Zm8=");
        assert_eq!(b64(b"foobar"), "Zm9vYmFy");
    }
}
