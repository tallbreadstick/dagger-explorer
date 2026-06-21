use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClipboardOp {
    Copy,
    Move,
}

/// Copy real filesystem paths to the OS clipboard the way desktop file managers do.
pub fn set_system_clipboard(paths: Vec<PathBuf>, op: ClipboardOp) -> Result<(), String> {
    if paths.is_empty() {
        return Err("No paths to place on clipboard".into());
    }

    #[cfg(windows)]
    {
        return windows::set_system_clipboard(paths, op);
    }

    #[cfg(target_os = "linux")]
    {
        return linux::set_system_clipboard(paths, op);
    }

    #[cfg(not(any(windows, target_os = "linux")))]
    {
        let _ = (paths, op);
        Err("OS clipboard is not supported on this platform".into())
    }
}

/// Read file paths and operation from the OS clipboard.
pub fn get_system_clipboard() -> Result<(Vec<PathBuf>, ClipboardOp), String> {
    #[cfg(windows)]
    {
        return windows::get_system_clipboard();
    }

    #[cfg(target_os = "linux")]
    {
        return linux::get_system_clipboard();
    }

    #[cfg(not(any(windows, target_os = "linux")))]
    {
        Err("OS clipboard is not supported on this platform".into())
    }
}

#[cfg(windows)]
mod windows {
    use std::{
        ffi::OsString,
        os::windows::ffi::OsStringExt,
        path::PathBuf,
        ptr,
        thread,
        time::Duration,
    };

    use super::ClipboardOp;
    use windows::Win32::{
        Foundation::{HANDLE, HGLOBAL, POINT},
        System::{
            DataExchange::{
                CloseClipboard, EmptyClipboard, GetClipboardData, IsClipboardFormatAvailable,
                OpenClipboard, RegisterClipboardFormatW, SetClipboardData,
            },
            Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE, GMEM_ZEROINIT},
            Ole::CF_HDROP,
        },
        UI::Shell::{DragQueryFileW, DROPFILES, HDROP},
    };
    use windows_core::{w, BOOL};

    pub fn set_system_clipboard(paths: Vec<PathBuf>, op: ClipboardOp) -> Result<(), String> {
        let paths: Vec<String> = paths
            .iter()
            .map(|path| path.to_string_lossy().into_owned())
            .collect();
        unsafe { set_system_clipboard_inner(paths, op) }
    }

    pub fn get_system_clipboard() -> Result<(Vec<PathBuf>, ClipboardOp), String> {
        unsafe { get_system_clipboard_inner() }
    }

    unsafe fn set_system_clipboard_inner(
        paths: Vec<String>,
        op: ClipboardOp,
    ) -> Result<(), String> {
        if paths.is_empty() {
            return Err("No valid paths provided".into());
        }

        let canonical: Vec<PathBuf> = paths.iter().map(PathBuf::from).collect();

        let mut wide_units: Vec<u16> = Vec::new();
        for path in &canonical {
            let wide: Vec<u16> = path
                .display()
                .to_string()
                .encode_utf16()
                .chain(Some(0))
                .collect();
            wide_units.extend_from_slice(&wide);
        }
        wide_units.push(0);

        let dropfiles_size = std::mem::size_of::<DROPFILES>();
        let total_size = dropfiles_size + wide_units.len() * 2;

        let hdrop = GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, total_size)
            .map_err(|e| format!("GlobalAlloc failed: {e:?}"))?;
        let ptr = GlobalLock(hdrop) as *mut u8;
        if ptr.is_null() {
            return Err("GlobalLock returned null for CF_HDROP".into());
        }

        let dropfiles = DROPFILES {
            pFiles: dropfiles_size as u32,
            pt: POINT { x: 0, y: 0 },
            fNC: BOOL(0),
            fWide: BOOL(1),
        };
        ptr::copy_nonoverlapping(
            &dropfiles as *const DROPFILES as *const u8,
            ptr,
            dropfiles_size,
        );
        ptr::copy_nonoverlapping(
            wide_units.as_ptr() as *const u8,
            ptr.add(dropfiles_size),
            wide_units.len() * 2,
        );
        GlobalUnlock(hdrop).ok();

        let effect_val: u32 = match op {
            ClipboardOp::Copy => 5,
            ClipboardOp::Move => 2,
        };

        let heffect = GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, 4)
            .map_err(|e| format!("GlobalAlloc (DropEffect) failed: {e:?}"))?;
        let eptr = GlobalLock(heffect) as *mut u32;
        *eptr = effect_val;
        GlobalUnlock(heffect).ok();

        let mut opened = false;
        for _ in 0..10 {
            if OpenClipboard(None).is_ok() {
                opened = true;
                break;
            }
            thread::sleep(Duration::from_millis(50));
        }
        if !opened {
            return Err("Failed to open clipboard after retries".into());
        }

        let result = (|| -> Result<(), String> {
            EmptyClipboard().map_err(|e| format!("EmptyClipboard failed: {e:?}"))?;

            let preferred_fmt = RegisterClipboardFormatW(w!("Preferred DropEffect"));
            let drop_fmt = RegisterClipboardFormatW(w!("DropEffect"));

            SetClipboardData(CF_HDROP.0 as u32, Some(HANDLE(hdrop.0)))
                .map_err(|e| format!("SetClipboardData CF_HDROP failed: {e:?}"))?;
            SetClipboardData(drop_fmt, Some(HANDLE(heffect.0)))
                .map_err(|e| format!("SetClipboardData DropEffect failed: {e:?}"))?;
            SetClipboardData(preferred_fmt, Some(HANDLE(heffect.0)))
                .map_err(|e| format!("SetClipboardData Preferred DropEffect failed: {e:?}"))?;

            Ok(())
        })();

        CloseClipboard().ok();
        result
    }

    unsafe fn get_system_clipboard_inner() -> Result<(Vec<PathBuf>, ClipboardOp), String> {
        OpenClipboard(None).map_err(|e| format!("OpenClipboard failed: {e}"))?;

        let mut file_list = Vec::new();
        if IsClipboardFormatAvailable(CF_HDROP.0 as u32).is_ok() {
            let handle = GetClipboardData(CF_HDROP.0 as u32)
                .map_err(|e| format!("GetClipboardData failed: {e}"))?;
            let hdrop = HDROP(handle.0);
            let count = DragQueryFileW(hdrop, 0xFFFF_FFFF, None);
            for i in 0..count {
                let mut buffer = vec![0u16; 260];
                let len = DragQueryFileW(hdrop, i, Some(&mut buffer));
                let s = OsString::from_wide(&buffer[..len as usize]);
                file_list.push(PathBuf::from(s));
            }
        }

        let mut op = ClipboardOp::Copy;
        let fmt = RegisterClipboardFormatW(w!("Preferred DropEffect"));
        if fmt != 0 && IsClipboardFormatAvailable(fmt).is_ok() {
            if let Ok(handle) = GetClipboardData(fmt) {
                let ptr = GlobalLock(HGLOBAL(handle.0)) as *const u32;
                if !ptr.is_null() {
                    op = match *ptr {
                        2 => ClipboardOp::Move,
                        _ => ClipboardOp::Copy,
                    };
                    GlobalUnlock(HGLOBAL(handle.0)).ok();
                }
            }
        }

        CloseClipboard().map_err(|e| format!("CloseClipboard failed: {e}"))?;
        Ok((file_list, op))
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use std::process::{Command, Stdio};

    use super::ClipboardOp;

    pub fn set_system_clipboard(paths: Vec<PathBuf>, op: ClipboardOp) -> Result<(), String> {
        let uris: Vec<String> = paths.iter().map(|path| path_to_uri(path)).collect();
        let uri_list = uris.join("\n");
        let action = match op {
            ClipboardOp::Copy => "copy",
            ClipboardOp::Move => "cut",
        };
        let gnome = format!("{action}\n{}\n", uris.join("\n"));

        copy_to_clipboard("text/uri-list", &uri_list)?;
        copy_to_clipboard("x-special/gnome-copied-files", &gnome)?;
        Ok(())
    }

    pub fn get_system_clipboard() -> Result<(Vec<PathBuf>, ClipboardOp), String> {
        if let Ok(gnome) = paste_from_clipboard("x-special/gnome-copied-files") {
            let mut lines = gnome.lines();
            let op = match lines.next().unwrap_or("copy") {
                "cut" => ClipboardOp::Move,
                _ => ClipboardOp::Copy,
            };
            let paths = lines.filter_map(parse_uri_line).collect::<Vec<_>>();
            if !paths.is_empty() {
                return Ok((paths, op));
            }
        }

        let uri_list = paste_from_clipboard("text/uri-list")?;
        let paths = uri_list
            .lines()
            .filter_map(parse_uri_line)
            .collect::<Vec<_>>();
        if paths.is_empty() {
            return Err("Clipboard does not contain file paths".into());
        }
        Ok((paths, ClipboardOp::Copy))
    }

    fn path_to_uri(path: &Path) -> String {
        let abs = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let encoded = abs
            .display()
            .to_string()
            .chars()
            .map(|ch| match ch {
                ' ' => "%20".to_string(),
                '%' => "%25".to_string(),
                '#' => "%23".to_string(),
                '?' => "%3F".to_string(),
                _ if ch.is_ascii() => ch.to_string(),
                _ => ch.to_string(),
            })
            .collect::<String>();
        format!("file://{encoded}")
    }

    fn parse_uri_line(line: &str) -> Option<PathBuf> {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return None;
        }
        let path = trimmed.strip_prefix("file://")?;
        Some(PathBuf::from(percent_decode(path)))
    }

    fn percent_decode(input: &str) -> String {
        let mut out = String::with_capacity(input.len());
        let bytes = input.as_bytes();
        let mut index = 0;
        while index < bytes.len() {
            if bytes[index] == b'%' && index + 2 < bytes.len() {
                if let Ok(value) = u8::from_str_radix(
                    std::str::from_utf8(&bytes[index + 1..index + 3]).unwrap_or(""),
                    16,
                ) {
                    out.push(value as char);
                    index += 3;
                    continue;
                }
            }
            out.push(bytes[index] as char);
            index += 1;
        }
        out
    }

    fn copy_to_clipboard(mime: &str, data: &str) -> Result<(), String> {
        if std::env::var("WAYLAND_DISPLAY").is_ok() {
            let mut child = Command::new("wl-copy")
                .arg("-t")
                .arg(mime)
                .stdin(Stdio::piped())
                .spawn()
                .map_err(|e| format!("wl-copy failed to start: {e}"))?;
            if let Some(mut stdin) = child.stdin.take() {
                stdin
                    .write_all(data.as_bytes())
                    .map_err(|e| format!("wl-copy write failed: {e}"))?;
            }
            let status = child
                .wait()
                .map_err(|e| format!("wl-copy wait failed: {e}"))?;
            if status.success() {
                return Ok(());
            }
        }

        let mut child = Command::new("xclip")
            .args(["-selection", "clipboard", "-t", mime, "-i"])
            .stdin(Stdio::piped())
            .spawn()
            .map_err(|e| format!("xclip failed to start: {e}"))?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(data.as_bytes())
                .map_err(|e| format!("xclip write failed: {e}"))?;
        }
        let status = child
            .wait()
            .map_err(|e| format!("xclip wait failed: {e}"))?;
        if status.success() {
            Ok(())
        } else {
            Err("Failed to write to clipboard".into())
        }
    }

    fn paste_from_clipboard(mime: &str) -> Result<String, String> {
        if std::env::var("WAYLAND_DISPLAY").is_ok() {
            if let Ok(output) = Command::new("wl-paste").arg("-t").arg(mime).output() {
                if output.status.success() {
                    return String::from_utf8(output.stdout)
                        .map_err(|e| format!("Invalid UTF-8 from wl-paste: {e}"));
                }
            }
        }

        let output = Command::new("xclip")
            .args(["-selection", "clipboard", "-t", mime, "-o"])
            .output()
            .map_err(|e| format!("xclip read failed: {e}"))?;
        if !output.status.success() {
            return Err("Failed to read from clipboard".into());
        }
        String::from_utf8(output.stdout).map_err(|e| format!("Invalid UTF-8 from xclip: {e}"))
    }
}
