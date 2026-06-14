//! System tray icon + desktop file-system watcher.
//! - Tray thread: invisible window with proper style for TrackPopupMenu.
//! - Icon: raw 32×32 RGBA pixels, black background + white "E" — matches eframe window icon.
//! - Right-click menu: Show / Organize / — / Minimize to Tray [✓] / Auto Organize [✓] / — / Exit.
//!   Menu strings follow current language via AtomicU8 (0=Zh, 1=En, 2=Ja, 3=Ko, 4=Hi).
//! - Watcher thread: ReadDirectoryChangesW on desktop, zero-CPU when idle.
//! Communicates with the main egui thread via atomic flags.

use std::sync::atomic::{AtomicBool, AtomicU8, AtomicIsize, Ordering};
use std::sync::Arc;

/// Handle to background threads. Poll flags each frame in `update()`.
pub struct TrayHandle {
    /// Set by tray: user clicked "Show Window"
    pub show_flag: Arc<AtomicBool>,
    /// Set by tray: user clicked "Exit"
    pub exit_flag: Arc<AtomicBool>,
    /// Set by tray: "Auto Organize" checkbox state (persistent)
    pub auto_flag: Arc<AtomicBool>,
    /// Set by tray: "Minimize to Tray" checkbox state (persistent)
    pub minimize_to_tray_flag: Arc<AtomicBool>,
    /// Set by tray: "One-click Organize" was clicked (event flag, clear after read)
    pub organize_flag: Arc<AtomicBool>,
    /// Set by watcher: a file was created/renamed on the desktop
    pub desktop_changed: Arc<AtomicBool>,
    /// Tray menu language: 0=Zh, 1=En, 2=Ja, 3=Ko, 4=Hi
    pub lang: Arc<AtomicU8>,
    /// Main window HWND (set by main thread after window creation).
    /// Tray thread reads this and calls PostMessageW to wake the main event loop.
    pub main_hwnd: Arc<AtomicIsize>,
}

/// Public entry point. Returns a handle on Windows; None on other platforms.
pub fn start() -> Option<TrayHandle> {
    #[cfg(target_os = "windows")]
    {
        Some(win::start())
    }
    #[cfg(not(target_os = "windows"))]
    {
        None
    }
}

/// Returns (rgba, size) for the 32×32 black/white "E" icon.
/// Used by main.rs to set the eframe window icon so it matches the tray icon.
pub fn icon_rgba() -> (Vec<u8>, u32) {
    #[cfg(target_os = "windows")]
    {
        win::build_icon_rgba()
    }
    #[cfg(not(target_os = "windows"))]
    {
        (Vec::new(), 0)
    }
}

// ── Windows implementation ────────────────────────────────────────────
#[cfg(target_os = "windows")]
mod win {
    use super::*;
    use windows::core::{w, PCWSTR};
    use windows::Win32::Foundation::*;
    use windows::Win32::Storage::FileSystem::{
        CreateFileW, ReadDirectoryChangesW, FILE_ACTION_ADDED, FILE_ACTION_RENAMED_NEW_NAME,
        FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OVERLAPPED, FILE_LIST_DIRECTORY,
        FILE_NOTIFY_CHANGE_FILE_NAME, FILE_NOTIFY_INFORMATION,
        FILE_SHARE_READ, FILE_SHARE_WRITE, FILE_SHARE_DELETE,
        OPEN_EXISTING,
    };
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::System::Threading::{CreateEventW, WaitForSingleObject};
    use windows::Win32::System::IO::OVERLAPPED;
    use windows::Win32::System::Com::CoTaskMemFree;
    use windows::Win32::UI::Shell::*;
    use windows::Win32::UI::WindowsAndMessaging::*;

    // ── Tray constants ─────────────────────────────────────────────
    const WM_TRAY: u32 = WM_APP;
    const UID_TRAY: u32 = 100;
    const IDM_SHOW: usize = 1000;
    const IDM_ORGANIZE: usize = 1001;
    const IDM_MINIMIZE_TRAY: usize = 1002;
    const IDM_AUTO_ORG: usize = 1003;
    const IDM_EXIT: usize = 1004;

    // ── i18n: tray menu strings by language (0=Zh, 1=En, 2=Ja, 3=Ko, 4=Hi) ──
    fn menu_str(lang: u8, key: &str) -> &'static str {
        match lang {
            0 => match key { // 中文
                "show" => "显示窗口",
                "organize" => "一键整理",
                "minimize_tray" => "关闭到托盘",
                "auto_org" => "自动整理",
                "exit" => "退出",
                _ => "",
            },
            2 => match key { // 日本語
                "show" => "表示",
                "organize" => "整理",
                "minimize_tray" => "トレイに格納",
                "auto_org" => "自動整理",
                "exit" => "終了",
                _ => "",
            },
            3 => match key { // 한국어
                "show" => "표시",
                "organize" => "정리",
                "minimize_tray" => "트레이로 최소화",
                "auto_org" => "자동 정리",
                "exit" => "종료",
                _ => "",
            },
            4 => match key { // हिन्दी (Devanagari)
                "show" => "दिखाएं",
                "organize" => "व्यवस्थित करें",
                "minimize_tray" => "ट्रे में छुपाएं",
                "auto_org" => "स्वतः व्यवस्थित",
                "exit" => "बाहर",
                _ => "",
            },
            _ => match key { // English (default)
                "show" => "Show Window",
                "organize" => "Organize Now",
                "minimize_tray" => "Minimize to Tray",
                "auto_org" => "Auto Organize",
                "exit" => "Exit",
                _ => "",
            },
        }
    }

    // ── build_icon_rgba: shared 32×32 black/white "E" icon ─────────
    /// Build raw RGBA pixel data for the 32×32 icon (black bg, white "E").
    /// Used by both the tray icon and the eframe window icon to keep them
    /// pixel-identical. Returns (rgba, size).
    pub fn build_icon_rgba() -> (Vec<u8>, u32) {
        const S: i32 = 32;
        let mut rgba = vec![0u8; (S * S * 4) as usize];

        // Helper: set pixel at (x,y) to white
        let set_white = |pixels: &mut [u8], x: i32, y: i32| {
            if x >= 0 && x < S && y >= 0 && y < S {
                let i = ((y * S + x) * 4) as usize;
                pixels[i] = 0xFF;     // B
                pixels[i + 1] = 0xFF; // G
                pixels[i + 2] = 0xFF; // R
                pixels[i + 3] = 0xFF; // A (opaque)
            }
        };

        // Fill all pixels with opaque black
        for i in (0..rgba.len()).step_by(4) {
            rgba[i] = 0x00;     // B
            rgba[i + 1] = 0x00; // G
            rgba[i + 2] = 0x00; // R
            rgba[i + 3] = 0xFF; // A
        }

        // Draw bold uppercase "E" letter
        // Letter fits in rectangle: left=6, right=27, top=5, bottom=27
        // Stroke width: 5px horizontal, 5px vertical
        for y in 5..=9 {
            for x in 6..=26 { set_white(&mut rgba, x, y); }
        }
        for y in 13..=17 {
            for x in 6..=24 { set_white(&mut rgba, x, y); }
        }
        for y in 21..=25 {
            for x in 6..=26 { set_white(&mut rgba, x, y); }
        }
        for x in 6..=10 {
            for y in 5..=25 { set_white(&mut rgba, x, y); }
        }

        (rgba, S as u32)
    }

    fn make_tray_icon() -> HICON {
        let (rgba, s) = build_icon_rgba();
        unsafe {
            CreateIcon(
                HINSTANCE(GetModuleHandleW(PCWSTR::null()).unwrap_or_default().0),
                s as i32, s as i32,
                1, 32,
                std::ptr::null(),
                rgba.as_ptr(),
            )
            .unwrap_or_else(|_| {
                eprintln!("[tray] CreateIcon failed: {}", GetLastError().0);
                HICON::default()
            })
        }
    }

    // ── Tray add / remove ─────────────────────────────────────────
    fn add_tray(hwnd: HWND) {
        unsafe {
            let icon = make_tray_icon();
            let mut nid = NOTIFYICONDATAW {
                cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
                hWnd: hwnd,
                uID: UID_TRAY,
                uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
                uCallbackMessage: WM_TRAY,
                hIcon: icon,
                ..Default::default()
            };
            let tip: Vec<u16> = "Desktop Organizer\0".encode_utf16().collect();
            let len = tip.len().min(127);
            nid.szTip[..len].copy_from_slice(&tip[..len]);

            let ok = Shell_NotifyIconW(NIM_ADD, &nid);
            if !ok.as_bool() {
                eprintln!("[tray] Shell_NotifyIconW(NIM_ADD) FAILED: {}", GetLastError().0);
            } else {
                println!("[tray] NIM_ADD OK, hIcon={:?}", icon.0);
            }
            // Don't destroy icon here — Shell owns it until NIM_DELETE
        }
    }

    fn remove_tray(hwnd: HWND) {
        unsafe {
            let nid = NOTIFYICONDATAW {
                cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
                hWnd: hwnd,
                uID: UID_TRAY,
                ..Default::default()
            };
            let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
        }
    }

    // ── Context menu ──────────────────────────────────────────────
    unsafe fn show_context_menu(
        hwnd: HWND,
        auto: &Arc<AtomicBool>,
        minimize_tray: &Arc<AtomicBool>,
        lang: &AtomicU8,
    ) {
        let Ok(menu) = CreatePopupMenu() else {
            eprintln!("[tray] CreatePopupMenu failed");
            return;
        };

        let l = lang.load(Ordering::SeqCst);

        // Show Window
        let s: Vec<u16> = format!("{}\0", menu_str(l, "show")).encode_utf16().collect();
        let _ = AppendMenuW(menu, MF_STRING, IDM_SHOW, PCWSTR(s.as_ptr()));

        // Organize Now
        let s: Vec<u16> = format!("{}\0", menu_str(l, "organize")).encode_utf16().collect();
        let _ = AppendMenuW(menu, MF_STRING, IDM_ORGANIZE, PCWSTR(s.as_ptr()));

        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());

        // Minimize to Tray (checkbox)
        let mt = minimize_tray.load(Ordering::SeqCst);
        let flags_mt = if mt { MF_STRING | MF_CHECKED } else { MF_STRING };
        let s: Vec<u16> = format!("{}\0", menu_str(l, "minimize_tray")).encode_utf16().collect();
        let _ = AppendMenuW(menu, flags_mt, IDM_MINIMIZE_TRAY, PCWSTR(s.as_ptr()));

        // Auto Organize (checkbox)
        let ao = auto.load(Ordering::SeqCst);
        let flags_ao = if ao { MF_STRING | MF_CHECKED } else { MF_STRING };
        let s: Vec<u16> = format!("{}\0", menu_str(l, "auto_org")).encode_utf16().collect();
        let _ = AppendMenuW(menu, flags_ao, IDM_AUTO_ORG, PCWSTR(s.as_ptr()));

        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());

        // Exit
        let s: Vec<u16> = format!("{}\0", menu_str(l, "exit")).encode_utf16().collect();
        let _ = AppendMenuW(menu, MF_STRING, IDM_EXIT, PCWSTR(s.as_ptr()));

        let mut pt = POINT::default();
        let _ = GetCursorPos(&mut pt);

        // SetForegroundWindow: best-effort (makes the menu dismiss on click-outside).
        // Even if it fails, TrackPopupMenu's OWN modal loop dispatches
        // WM_COMMAND to this window when items are clicked.
        let _ = SetForegroundWindow(hwnd);

        // Standard TrackPopupMenu — NO TPM_RETURNCMD.
        // Clicks on menu items → WM_COMMAND dispatched by the menu's internal
        // modal loop → wnd_proc sets atomic flags + wakes main thread.
        let _ = TrackPopupMenu(
            menu, TPM_LEFTALIGN | TPM_TOPALIGN,
            pt.x, pt.y, 0, hwnd, None,
        );

        let _ = PostMessageW(hwnd, WM_NULL, WPARAM(0), LPARAM(0));
        let _ = DestroyMenu(menu);
    }

    // ── Tray window data ──────────────────────────────────────────
    struct TrayData {
        show_flag: Arc<AtomicBool>,
        exit_flag: Arc<AtomicBool>,
        auto_flag: Arc<AtomicBool>,
        minimize_to_tray_flag: Arc<AtomicBool>,
        organize_flag: Arc<AtomicBool>,
        lang: Arc<AtomicU8>,
        main_hwnd: Arc<AtomicIsize>,
    }

    unsafe extern "system" fn wnd_proc(
        hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM,
    ) -> LRESULT {
        if msg == WM_CREATE {
            let cs = &*(lparam.0 as *const CREATESTRUCTW);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, cs.lpCreateParams as isize);
            return LRESULT(0);
        }

        let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const TrayData;
        if ptr.is_null() {
            return DefWindowProcW(hwnd, msg, wparam, lparam);
        }
        let data = &*ptr;

        match msg {
            WM_TRAY => {
                // lparam low-word = the actual notification code (WM_RBUTTONUP, etc.)
                // e.g. WM_RBUTTONUP = 0x0202 = 514
                let lo = (lparam.0 & 0xFFFF) as u32;
                match lo {
                    0x0201 | 0x0202 | 0x0203 | 0x0204 | 0x0205 => {
                        // Any mouse button event → show context menu
                        show_context_menu(
                            hwnd,
                            &data.auto_flag,
                            &data.minimize_to_tray_flag,
                            &data.lang,
                        );
                    }
                    _ => {
                        // NIN_* notifications from Shell_NotifyIcon
                        // (NIN_SELECT = 0x0600, NIN_KEYSELECT = 0x0601, etc.)
                        // Silently ignore — left-click is handled by mouse events above.
                    }
                }
                LRESULT(0)
            }
            // Menu clicks dispatch WM_COMMAND from TrackPopupMenu's own
            // modal loop → set flags + RESTORE the main window from minimize.
            // (Window stays in minimized state (not hidden) so winit/eframe
            //  event loop continues running; we just bring it back.)
            WM_COMMAND => {
                let id = (wparam.0 & 0xFFFF) as usize;
                let mh = data.main_hwnd.load(Ordering::Relaxed);
                let needs_restore = matches!(id, IDM_SHOW | IDM_ORGANIZE | IDM_EXIT);
                eprintln!("[tray] WM_COMMAND id={id}, main_hwnd={mh:#x}, needs_restore={needs_restore}");
                if needs_restore && mh != 0 {
                    let hwnd = HWND(mh as *mut _);
                    let sw = ShowWindow(hwnd, SW_RESTORE);
                    let SF = SetForegroundWindow(hwnd);
                    eprintln!("[tray] ShowWindow→{:#x}, SetForegroundWindow→{:#x}", sw.0, SF.0);
                }
                match id {
                    IDM_SHOW => data.show_flag.store(true, Ordering::SeqCst),
                    IDM_ORGANIZE => data.organize_flag.store(true, Ordering::SeqCst),
                    IDM_MINIMIZE_TRAY => {
                        let cur = data.minimize_to_tray_flag.load(Ordering::SeqCst);
                        data.minimize_to_tray_flag.store(!cur, Ordering::SeqCst);
                    }
                    IDM_AUTO_ORG => {
                        let cur = data.auto_flag.load(Ordering::SeqCst);
                        data.auto_flag.store(!cur, Ordering::SeqCst);
                    }
                    IDM_EXIT => data.exit_flag.store(true, Ordering::SeqCst),
                    _ => {}
                }
                // PostMessageW as a wake hint
                let mh = data.main_hwnd.load(Ordering::Relaxed);
                if mh != 0 {
                    let _ = PostMessageW(HWND(mh as *mut _), WM_NULL, WPARAM(0), LPARAM(0));
                }
                LRESULT(0)
            }
            WM_DESTROY => {
                remove_tray(hwnd);
                PostQuitMessage(0);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }

    // ── Desktop file-system watcher ───────────────────────────────
    fn start_desktop_watcher(changed_flag: Arc<AtomicBool>, main_hwnd: Arc<AtomicIsize>) {
        std::thread::Builder::new()
            .name("desktop-watcher".into())
            .spawn(move || unsafe {
                let pwstr = match SHGetKnownFolderPath(
                    &FOLDERID_Desktop,
                    KF_FLAG_DEFAULT,
                    None,
                ) {
                    Ok(p) => p,
                    Err(e) => {
                        eprintln!("[watcher] SHGetKnownFolderPath failed: {e:?}");
                        return;
                    }
                };
                let path_str: String = pwstr.to_string().unwrap_or_default();
                CoTaskMemFree(Some(pwstr.0 as *const std::ffi::c_void));

                if path_str.is_empty() {
                    eprintln!("[watcher] empty desktop path");
                    return;
                }

                let path_wide: Vec<u16> = format!("{}\0", path_str).encode_utf16().collect();
                let dir_handle = CreateFileW(
                    PCWSTR(path_wide.as_ptr()),
                    FILE_LIST_DIRECTORY.0,
                    FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                    None,
                    OPEN_EXISTING,
                    FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OVERLAPPED,
                    None,
                );
                match dir_handle {
                    Ok(h) if h.0 != INVALID_HANDLE_VALUE.0 => {
                        println!("[watcher] watching {}", path_str);
                        watch_loop(h, changed_flag, main_hwnd);
                    }
                    Ok(_) => eprintln!("[watcher] CreateFile returned INVALID_HANDLE_VALUE"),
                    Err(e) => eprintln!("[watcher] CreateFile failed: {e:?}"),
                }
            })
            .ok();
    }

    unsafe fn watch_loop(dir: HANDLE, changed: Arc<AtomicBool>, main_hwnd: Arc<AtomicIsize>) {
        let event = CreateEventW(None, false, false, None).unwrap_or_default();
        let mut ov = OVERLAPPED::default();
        ov.hEvent = event;

        const BUF_SIZE: usize = 4096;
        let mut buf: Vec<u8> = vec![0u8; BUF_SIZE];

        loop {
            let mut bytes_returned: u32 = 0;
            let Ok(()) = ReadDirectoryChangesW(
                dir,
                buf.as_mut_ptr() as *mut std::ffi::c_void,
                BUF_SIZE as u32,
                false,
                FILE_NOTIFY_CHANGE_FILE_NAME,
                Some(&mut bytes_returned),
                Some(&mut ov),
                None,
            ) else {
                eprintln!("[watcher] ReadDirectoryChangesW failed");
                break;
            };

            let wait = WaitForSingleObject(event, 5000);
            if wait == WAIT_OBJECT_0 {
                let mut offset: usize = 0;
                loop {
                    let fni = &*(buf.as_ptr().add(offset) as *const FILE_NOTIFY_INFORMATION);
                    match fni.Action {
                        FILE_ACTION_ADDED | FILE_ACTION_RENAMED_NEW_NAME => {
                            changed.store(true, Ordering::SeqCst);
                            let mh = main_hwnd.load(Ordering::Relaxed);
                            if mh != 0 {
                                let _ = PostMessageW(HWND(mh as *mut _), WM_NULL, WPARAM(0), LPARAM(0));
                            }
                        }
                        _ => {}
                    }
                    if fni.NextEntryOffset == 0 {
                        break;
                    }
                    offset += fni.NextEntryOffset as usize;
                }
            }
        }
        let _ = CloseHandle(event);
    }

    // ── Start everything ──────────────────────────────────────────
    pub fn start() -> TrayHandle {
        let _ = std::fs::write("C:\\temp_tray_1.txt", "[start] 1\n");
        let show = Arc::new(AtomicBool::new(false));
        let exit = Arc::new(AtomicBool::new(false));
        let auto = Arc::new(AtomicBool::new(true));   // auto-organize ON by default
        let mt = Arc::new(AtomicBool::new(true));     // minimize-to-tray ON by default
        let org = Arc::new(AtomicBool::new(false));
        let changed = Arc::new(AtomicBool::new(false));
        let lang = Arc::new(AtomicU8::new(0));        // default: Zh (matches Lang::default())
        let main_hwnd = Arc::new(AtomicIsize::new(0));

        let show_c = show.clone();
        let exit_c = exit.clone();
        let auto_c = auto.clone();
        let mt_c = mt.clone();
        let org_c = org.clone();
        let lang_c = lang.clone();
        let main_hwnd_c = main_hwnd.clone();

        // Start desktop watcher
        start_desktop_watcher(changed.clone(), main_hwnd.clone());

        let _ = std::fs::write("C:\\temp_tray_2.txt", "[start] 2\n");
        // Start tray icon thread
        std::thread::Builder::new()
            .name("tray-icon".into())
            .spawn(move || unsafe {
                let hinst = GetModuleHandleW(PCWSTR::null()).unwrap_or_default();

                let wc = WNDCLASSW {
                    lpfnWndProc: Some(wnd_proc),
                    hInstance: hinst.into(),
                    lpszClassName: w!("DesktopOrgTrayCls"),
                    ..Default::default()
                };
                let _ = std::fs::write("C:\\temp_tray_3.txt", "[thread] before RegisterClassW\n");
                if RegisterClassW(&wc) == 0 {
                    let err = GetLastError().0;
                    let _ = std::fs::write("C:\\temp_tray_3.txt", format!("[thread] RegisterClassW failed: {}\n", err));
                    return;
                }
                let _ = std::fs::write("C:\\temp_tray_4.txt", "[thread] after RegisterClassW\n");

                let data = TrayData {
                    show_flag: show_c,
                    exit_flag: exit_c,
                    auto_flag: auto_c,
                    minimize_to_tray_flag: mt_c,
                    organize_flag: org_c,
                    lang: lang_c,
                    main_hwnd: main_hwnd_c,
                };

                // Tiny visible popup window (1×1 px at 0,0 = effectively invisible).
                // MUST be visible for SetForegroundWindow to succeed, which is
                // required for TrackPopupMenu to work. WS_EX_TOOLWINDOW keeps it
                // out of the taskbar.
                let _ = std::fs::write("C:\\temp_tray_5.txt", "[thread] before CreateWindowExW\n");
                let Ok(hwnd) = CreateWindowExW(
                    WS_EX_TOOLWINDOW,
                    w!("DesktopOrgTrayCls"),
                    w!("DesktopOrgTray"),
                    WS_POPUP | WS_VISIBLE,
                    0, 0, 1, 1,
                    None,
                    None,
                    hinst,
                    Some(&data as *const TrayData as *const std::ffi::c_void),
                ) else {
                    let _ = std::fs::write("C:\\temp_tray_5.txt", "[thread] CreateWindowExW Err\n");
                    return;
                };

                if hwnd.0.is_null() {
                    let err = GetLastError().0;
                    let _ = std::fs::write("C:\\temp_tray_5.txt", format!("[thread] CreateWindowExW NULL: {}\n", err));
                    return;
                }
                let _ = std::fs::write("C:\\temp_tray_6.txt", format!("[thread] window created: {:?}\n", hwnd.0));

                add_tray(hwnd);
                let _ = std::fs::write("C:\\temp_tray_7.txt", "[thread] add_tray done\n");

                let mut msg = MSG::default();
                loop {
                    let ret = GetMessageW(&mut msg, HWND::default(), 0, 0);
                    if ret.0 <= 0 {
                        break;
                    }
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }

                println!("[tray] thread exited");
            })
            .ok();

        TrayHandle {
            show_flag: show,
            exit_flag: exit,
            auto_flag: auto,
            minimize_to_tray_flag: mt,
            organize_flag: org,
            desktop_changed: changed,
            lang,
            main_hwnd,
        }
    }
}
