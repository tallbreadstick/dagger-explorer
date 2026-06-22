use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClipboardOp {
    Copy,
    Move,
}

const MIME_URI_LIST: &str = "text/uri-list";
const MIME_GNOME: &str = "x-special/gnome-copied-files";
const MIME_KDE_CUT: &str = "application/x-kde-cutselection";

/// Copy real filesystem paths to the OS clipboard the way Qt/KDE file managers do.
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

/// Read file paths and copy/move intent from the OS clipboard.
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

/// Whether the OS clipboard currently contains file URLs (cheap check for UI enablement).
pub fn has_file_clipboard() -> bool {
    #[cfg(windows)]
    {
        return windows::has_file_clipboard();
    }

    #[cfg(target_os = "linux")]
    {
        return linux::has_file_clipboard();
    }

    #[cfg(not(any(windows, target_os = "linux")))]
    {
        false
    }
}

struct FileClipboardPayload {
    uri_list: Vec<u8>,
    gnome: Vec<u8>,
    kde_cut: Option<Vec<u8>>,
}

fn build_payload(paths: &[PathBuf], op: ClipboardOp) -> Result<FileClipboardPayload, String> {
    let uris: Vec<String> = paths
        .iter()
        .map(|path| path_to_file_uri(path))
        .collect::<Result<_, _>>()?;
    let uri_list = uris.join("\r\n");
    let action = match op {
        ClipboardOp::Copy => "copy",
        ClipboardOp::Move => "cut",
    };
    let gnome = format!("{action}\r\n{}\r\n", uris.join("\r\n"));
    let kde_cut = match op {
        ClipboardOp::Move => Some(vec![b'1']),
        ClipboardOp::Copy => None,
    };

    Ok(FileClipboardPayload {
        uri_list: uri_list.into_bytes(),
        gnome: gnome.into_bytes(),
        kde_cut,
    })
}

fn path_to_file_uri(path: &Path) -> Result<String, String> {
    let abs = path.canonicalize().unwrap_or_else(|_| {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("/"))
                .join(path)
        }
    });
    let encoded = percent_encode(abs.display().to_string().as_bytes());
    Ok(format!("file://{encoded}"))
}

fn percent_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len());
    for byte in bytes {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'/' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*byte as char);
            }
            _ => {
                out.push('%');
                out.push(hex_digit(byte >> 4));
                out.push(hex_digit(byte & 0x0f));
            }
        }
    }
    out
}

fn hex_digit(value: u8) -> char {
    match value {
        0..=9 => (b'0' + value) as char,
        _ => (b'A' + (value - 10)) as char,
    }
}

fn parse_uri_list(data: &str) -> Vec<PathBuf> {
    data.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(parse_uri_line)
        .collect()
}

fn parse_uri_line(line: &str) -> Option<PathBuf> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.starts_with("file://") {
        return Some(PathBuf::from(percent_decode(trimmed.strip_prefix("file://")?)));
    }
    Some(PathBuf::from(trimmed))
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = String::with_capacity(bytes.len());
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

fn parse_gnome(data: &str) -> Option<(ClipboardOp, Vec<PathBuf>)> {
    let mut lines = data.lines();
    let op = match lines.next()?.trim() {
        "cut" => ClipboardOp::Move,
        _ => ClipboardOp::Copy,
    };
    let paths = lines.filter_map(parse_uri_line).collect::<Vec<_>>();
    if paths.is_empty() {
        None
    } else {
        Some((op, paths))
    }
}

fn parse_kde_cut(data: &[u8]) -> ClipboardOp {
    if data.first() == Some(&b'1') {
        ClipboardOp::Move
    } else {
        ClipboardOp::Copy
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn build_payload_from_tmp() {
        let paths = vec![PathBuf::from("/tmp")];
        let payload = build_payload(&paths, ClipboardOp::Copy).expect("payload");
        assert!(payload.uri_list.starts_with(b"file://"));
        assert!(payload.gnome.starts_with(b"copy"));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn set_and_get_system_clipboard() {
        let paths = vec![PathBuf::from("/tmp")];
        set_system_clipboard(paths, ClipboardOp::Copy).expect("set clipboard");
        std::thread::sleep(std::time::Duration::from_millis(300));
        let (got, op) = get_system_clipboard().expect("get clipboard");
        assert!(!got.is_empty());
        assert_eq!(op, ClipboardOp::Copy);
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

    pub fn has_file_clipboard() -> bool {
        unsafe {
            OpenClipboard(None).is_ok()
                && IsClipboardFormatAvailable(CF_HDROP.0 as u32).is_ok()
                && CloseClipboard().is_ok()
        }
    }

    unsafe fn set_system_clipboard_inner(
        paths: Vec<String>,
        op: ClipboardOp,
    ) -> Result<(), String> {
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
                let mut buffer = vec![0u16; 1024];
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

        if file_list.is_empty() {
            return Err("Clipboard does not contain file paths".into());
        }

        Ok((file_list, op))
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use std::path::PathBuf;

    use super::{
        build_payload, parse_gnome, parse_kde_cut, parse_uri_list, ClipboardOp,
    };

    pub fn set_system_clipboard(paths: Vec<PathBuf>, op: ClipboardOp) -> Result<(), String> {
        let payload = build_payload(&paths, op)?;
        if is_wayland() {
            wayland::set(&payload).or_else(|wayland_error| {
                x11::set(&payload).map_err(|x11_error| {
                    format!("Wayland clipboard failed ({wayland_error}); X11 fallback failed ({x11_error})")
                })
            })
        } else {
            x11::set(&payload)
        }
    }

    pub fn get_system_clipboard() -> Result<(Vec<PathBuf>, ClipboardOp), String> {
        if is_wayland() {
            wayland::get().or_else(|_| x11::get())
        } else {
            x11::get()
        }
    }

    pub fn has_file_clipboard() -> bool {
        if is_wayland() {
            wayland::has_files().unwrap_or(false) || x11::has_files().unwrap_or(false)
        } else {
            x11::has_files().unwrap_or(false)
        }
    }

    fn is_wayland() -> bool {
        std::env::var("WAYLAND_DISPLAY").is_ok()
    }

    mod wayland {
        use std::io::Read;
        use std::path::PathBuf;

        use wl_clipboard_rs::copy::{MimeSource, MimeType, Options, Source};
        use wl_clipboard_rs::paste::{
            get_contents, get_mime_types, ClipboardType, Error as PasteError, MimeType as PasteMime,
            Seat,
        };

        use super::super::{FileClipboardPayload, MIME_GNOME, MIME_KDE_CUT, MIME_URI_LIST};
        use super::{parse_gnome, parse_kde_cut, parse_uri_list, ClipboardOp};

        pub fn set(payload: &FileClipboardPayload) -> Result<(), String> {
            let mut sources = vec![
                MimeSource {
                    source: Source::Bytes(payload.uri_list.clone().into()),
                    mime_type: MimeType::Specific(MIME_URI_LIST.into()),
                },
                MimeSource {
                    source: Source::Bytes(payload.gnome.clone().into()),
                    mime_type: MimeType::Specific(MIME_GNOME.into()),
                },
            ];
            if let Some(kde_cut) = &payload.kde_cut {
                sources.push(MimeSource {
                    source: Source::Bytes(kde_cut.clone().into()),
                    mime_type: MimeType::Specific(MIME_KDE_CUT.into()),
                });
            }

            let mut opts = Options::new();
            opts.omit_additional_text_mime_types(true);
            opts.copy_multi(sources)
                .map_err(|error| format!("Wayland clipboard write failed: {error}"))
        }

        pub fn get() -> Result<(Vec<PathBuf>, ClipboardOp), String> {
            if let Ok(data) = read_mime(MIME_GNOME) {
                if let Ok(text) = String::from_utf8(data) {
                    if let Some((op, paths)) = parse_gnome(&text) {
                        return Ok((paths, op));
                    }
                }
            }

            let uri_data = read_mime(MIME_URI_LIST)?;
            let uri_text = String::from_utf8(uri_data)
                .map_err(|error| format!("Invalid UTF-8 in text/uri-list: {error}"))?;
            let paths = parse_uri_list(&uri_text);
            if paths.is_empty() {
                return Err("Clipboard does not contain file paths".into());
            }

            let op = read_mime(MIME_KDE_CUT)
                .map(|data| parse_kde_cut(&data))
                .unwrap_or(ClipboardOp::Copy);

            Ok((paths, op))
        }

        pub fn has_files() -> Result<bool, String> {
            let types = get_mime_types(ClipboardType::Regular, Seat::Unspecified)
                .map_err(|error| format!("Wayland clipboard query failed: {error}"))?;
            Ok(types.contains(MIME_URI_LIST)
                || types.contains(MIME_GNOME)
                || types.contains(MIME_KDE_CUT))
        }

        fn read_mime(mime: &str) -> Result<Vec<u8>, String> {
            let (mut pipe, _) =
                get_contents(ClipboardType::Regular, Seat::Unspecified, PasteMime::Specific(mime))
                    .map_err(map_paste_error)?;
            let mut data = Vec::new();
            pipe.read_to_end(&mut data)
                .map_err(|error| format!("Wayland clipboard read failed: {error}"))?;
            Ok(data)
        }

        fn map_paste_error(error: PasteError) -> String {
            match error {
                PasteError::ClipboardEmpty | PasteError::NoMimeType | PasteError::NoSeats => {
                    "Clipboard does not contain file paths".into()
                }
                other => format!("Wayland clipboard read failed: {other}"),
            }
        }
    }

    mod x11 {
        use std::path::PathBuf;
        use std::time::Duration;

        use x11_clipboard::Clipboard;
        use x11rb::protocol::xproto::ConnectionExt;

        use super::super::{FileClipboardPayload, MIME_GNOME, MIME_KDE_CUT, MIME_URI_LIST};
        use super::{parse_gnome, parse_kde_cut, parse_uri_list, ClipboardOp};

        pub fn set(payload: &FileClipboardPayload) -> Result<(), String> {
            let clipboard =
                Clipboard::new().map_err(|error| format!("X11 clipboard unavailable: {error}"))?;
            let selection = clipboard.getter.atoms.clipboard;
            let uri_atom = intern_atom(&clipboard, MIME_URI_LIST)?;
            let gnome_atom = intern_atom(&clipboard, MIME_GNOME)?;

            clipboard
                .store(selection, uri_atom, payload.uri_list.clone())
                .map_err(|error| format!("X11 clipboard write failed for uri-list: {error}"))?;
            clipboard
                .store(selection, gnome_atom, payload.gnome.clone())
                .map_err(|error| format!("X11 clipboard write failed for gnome format: {error}"))?;

            if let Some(kde_cut) = &payload.kde_cut {
                let kde_atom = intern_atom(&clipboard, MIME_KDE_CUT)?;
                clipboard.store(selection, kde_atom, kde_cut.clone()).map_err(|error| {
                    format!("X11 clipboard write failed for KDE cut marker: {error}")
                })?;
            }

            Ok(())
        }

        pub fn get() -> Result<(Vec<PathBuf>, ClipboardOp), String> {
            let clipboard =
                Clipboard::new().map_err(|error| format!("X11 clipboard unavailable: {error}"))?;
            let selection = clipboard.getter.atoms.clipboard;
            let property = clipboard.getter.atoms.property;
            let timeout = Duration::from_millis(100);

            if let Ok(data) = clipboard.load(
                selection,
                intern_atom(&clipboard, MIME_GNOME)?,
                property,
                timeout,
            ) {
                if let Ok(text) = String::from_utf8(data) {
                    if let Some((op, paths)) = parse_gnome(&text) {
                        return Ok((paths, op));
                    }
                }
            }

            let uri_data = clipboard
                .load(
                    selection,
                    intern_atom(&clipboard, MIME_URI_LIST)?,
                    property,
                    timeout,
                )
                .map_err(|error| format!("X11 clipboard read failed: {error}"))?;
            let uri_text = String::from_utf8(uri_data)
                .map_err(|error| format!("Invalid UTF-8 in text/uri-list: {error}"))?;
            let paths = parse_uri_list(&uri_text);
            if paths.is_empty() {
                return Err("Clipboard does not contain file paths".into());
            }

            let op = clipboard
                .load(
                    selection,
                    intern_atom(&clipboard, MIME_KDE_CUT)?,
                    property,
                    timeout,
                )
                .map(|data| parse_kde_cut(&data))
                .unwrap_or(ClipboardOp::Copy);

            Ok((paths, op))
        }

        pub fn has_files() -> Result<bool, String> {
            let clipboard =
                Clipboard::new().map_err(|error| format!("X11 clipboard unavailable: {error}"))?;
            let selection = clipboard.getter.atoms.clipboard;
            let property = clipboard.getter.atoms.property;
            let timeout = Duration::from_millis(50);

            for mime in [MIME_URI_LIST, MIME_GNOME] {
                if intern_atom(&clipboard, mime)
                    .ok()
                    .is_some_and(|atom| clipboard.load(selection, atom, property, timeout).is_ok())
                {
                    return Ok(true);
                }
            }
            Ok(false)
        }

        fn intern_atom(clipboard: &Clipboard, name: &str) -> Result<x11_clipboard::Atom, String> {
            clipboard
                .getter
                .connection
                .intern_atom(false, name.as_bytes())
                .map_err(|error| format!("Failed to intern atom {name}: {error}"))?
                .reply()
                .map(|reply| reply.atom)
                .map_err(|error| format!("Failed to intern atom {name}: {error}"))
        }
    }
}
