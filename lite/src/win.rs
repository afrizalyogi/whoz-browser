//! Small Win32 helpers: overlay clipping, shell open/reveal, folder picker.
use std::{os::windows::process::CommandExt, path::PathBuf, process::Command};
use windows::{
    core::{w, HSTRING},
    Win32::{
        Foundation::HWND,
        Graphics::Gdi::{CombineRgn, CreateRectRgn, CreateRoundRectRgn, DeleteObject, SetWindowRgn, RGN_OR},
        System::Com::{CoCreateInstance, CoInitializeEx, CoTaskMemFree, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED},
        UI::{
            Shell::{FileOpenDialog, IFileOpenDialog, ShellExecuteW, FOS_FORCEFILESYSTEM, FOS_PICKFOLDERS, SIGDN_FILESYSPATH},
            WindowsAndMessaging::{SetWindowPos, HWND_TOP, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SW_SHOWNORMAL},
        },
    },
};

/// Clips a child window to "x,y,w,h[,radius];..." (device px) and raises it.
pub fn set_region(hwnd: HWND, spec: &str) {
    unsafe {
        let rgn = CreateRectRgn(0, 0, 0, 0);
        for r in spec.split(';') {
            let v: Vec<i32> = r.split(',').filter_map(|n| n.parse().ok()).collect();
            let part = match v[..] {
                [x, y, w, h] | [x, y, w, h, 0] => CreateRectRgn(x, y, x + w, y + h),
                [x, y, w, h, r] => CreateRoundRectRgn(x, y, x + w + 1, y + h + 1, r * 2, r * 2),
                _ => continue,
            };
            CombineRgn(Some(rgn), Some(rgn), Some(part), RGN_OR);
            let _ = DeleteObject(part.into());
        }
        // The window owns the region after this call.
        if SetWindowRgn(hwnd, Some(rgn), true) == 0 {
            let _ = DeleteObject(rgn.into());
        }
        raise(hwnd);
    }
}

pub fn raise(hwnd: HWND) {
    unsafe {
        let _ = SetWindowPos(hwnd, Some(HWND_TOP), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
    }
}

/// Opens a file, folder or URI with its default app.
pub fn open(target: &str) {
    unsafe {
        ShellExecuteW(None, w!("open"), &HSTRING::from(target), None, None, SW_SHOWNORMAL);
    }
}

/// Shows a file selected in Explorer.
pub fn reveal(path: &str) {
    let _ = Command::new("explorer.exe").raw_arg(format!("/select,\"{path}\"")).spawn();
}

/// Modal folder picker. Run it on its own thread: it pumps messages until closed.
pub fn pick_folder(owner: isize) -> Option<PathBuf> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let d: IFileOpenDialog = CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER).ok()?;
        d.SetOptions(d.GetOptions().ok()? | FOS_PICKFOLDERS | FOS_FORCEFILESYSTEM).ok()?;
        d.Show(Some(HWND(owner as _))).ok()?;
        let p = d.GetResult().ok()?.GetDisplayName(SIGDN_FILESYSPATH).ok()?;
        let s = p.to_string().ok();
        CoTaskMemFree(Some(p.0 as _));
        s.map(PathBuf::from)
    }
}

pub fn downloads_dir() -> PathBuf {
    PathBuf::from(std::env::var_os("USERPROFILE").unwrap_or_default()).join("Downloads")
}
