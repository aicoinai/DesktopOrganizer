// Icon extraction from Windows desktop files (windows 0.58 API)
#![cfg(windows)]

use windows::Win32::UI::Shell::{
    SHGetFileInfoW, SHFILEINFOW, SHGFI_FLAGS, SHGFI_ICON, SHGFI_LARGEICON,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DrawIconEx, DestroyIcon, DI_NORMAL,
};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, DeleteDC, CreateCompatibleBitmap,
    DeleteObject, SelectObject, GetDIBits,
    BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
    HDC, HGDIOBJ, HBRUSH,
};
use windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES;
use windows::core::PCWSTR;

const ICON_SIZE: u32 = 32;

/// Extract a 32x32 RGBA icon from a file path. Returns (w, h, RGBA_bytes).
pub fn extract_icon_rgba(path: &str) -> Option<(u32, u32, Vec<u8>)> {
    unsafe {
        let wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
        let mut info = SHFILEINFOW::default();
        let result = SHGetFileInfoW(
            PCWSTR::from_raw(wide.as_ptr()),
            FILE_FLAGS_AND_ATTRIBUTES(0),
            Some(&mut info),
            std::mem::size_of::<SHFILEINFOW>() as u32,
            SHGFI_FLAGS(SHGFI_ICON.0 | SHGFI_LARGEICON.0),
        );
        if result == 0 || info.hIcon.is_invalid() {
            return None;
        }

        let dc = CreateCompatibleDC(HDC::default());
        if dc.is_invalid() {
            let _ = DestroyIcon(info.hIcon);
            return None;
        }

        let bmp = CreateCompatibleBitmap(dc, ICON_SIZE as i32, ICON_SIZE as i32);
        if bmp.is_invalid() {
            let _ = DeleteDC(dc);
            let _ = DestroyIcon(info.hIcon);
            return None;
        }

        let old_bmp = SelectObject(dc, HGDIOBJ(bmp.0));

        // Draw the icon at 32x32
        let _ = DrawIconEx(
            dc, 0, 0, info.hIcon,
            ICON_SIZE as i32, ICON_SIZE as i32,
            0,
            HBRUSH::default(),
            DI_NORMAL,
        );

        // Get pixel data (BGRA → convert to RGBA)
        let header = BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: ICON_SIZE as i32,
            biHeight: -(ICON_SIZE as i32), // top-down
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        };
        let mut bmi = BITMAPINFO {
            bmiHeader: header,
            ..Default::default()
        };

        let data_size = (ICON_SIZE * ICON_SIZE * 4) as usize;
        let mut pixels: Vec<u8> = vec![0u8; data_size];
        let scan_lines = GetDIBits(
            dc,
            bmp,
            0,
            ICON_SIZE,
            Some(pixels.as_mut_ptr() as *mut _),
            &mut bmi,
            DIB_RGB_COLORS,
        );
        if scan_lines == 0 {
            let _ = SelectObject(dc, old_bmp);
            let _ = DeleteObject(HGDIOBJ(bmp.0));
            let _ = DeleteDC(dc);
            let _ = DestroyIcon(info.hIcon);
            return None;
        }

        // BGRA → RGBA
        for chunk in pixels.chunks_exact_mut(4) {
            chunk.swap(0, 2); // swap B and R
        }

        // Cleanup
        let _ = SelectObject(dc, old_bmp);
        let _ = DeleteObject(HGDIOBJ(bmp.0));
        let _ = DeleteDC(dc);
        let _ = DestroyIcon(info.hIcon);

        Some((ICON_SIZE, ICON_SIZE, pixels))
    }
}
