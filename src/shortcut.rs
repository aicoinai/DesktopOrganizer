use std::path::Path;
use windows::core::*;
use windows::Win32::System::Com::*;
use windows::Win32::UI::Shell::*;

const SLGP_RAWPATH: u32 = 0x04;

/// Resolve .lnk shortcut, returning the target file path. Returns None on failure.
pub unsafe fn resolve_lnk_target(lnk_path: &Path) -> Option<String> {
    unsafe {
        // Initialize COM (ignore if already initialized; S_FALSE = already initialized)
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
    }
    let result = (|| -> Option<String> {
        let sl: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).ok()?;
        let pf: IPersistFile = sl.cast().ok()?;
        let abs = std::fs::canonicalize(lnk_path).ok()?;
        let wide: Vec<u16> = abs
            .to_string_lossy()
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        pf.Load(PCWSTR::from_raw(wide.as_ptr()), STGM_READ).ok()?;
        let mut buf = vec![0u16; 520];
        if sl.GetPath(&mut buf, std::ptr::null_mut(), SLGP_RAWPATH).is_ok() {
            let s = String::from_utf16_lossy(&buf);
            let s = s.trim_end_matches('\0').to_string();
            if s.is_empty() { None } else { Some(s) }
        } else {
            None
        }
    })();
    // Don't CoUninitialize — subsequent calls may still need COM
    result
}
