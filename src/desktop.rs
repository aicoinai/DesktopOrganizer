use crate::shapes;
use crate::shortcut;
use windows::core::w;
use windows::Win32::Foundation::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::System::Threading::*;
use windows::Win32::System::Memory::*;
use windows::Win32::System::Diagnostics::Debug::*;

const LVM_FIRST: u32 = 0x1000;
const LVM_GETITEMCOUNT: u32 = LVM_FIRST + 4;
const LVM_SETITEMPOSITION: u32 = LVM_FIRST + 15;
const LVM_GETITEMTEXTW: u32 = LVM_FIRST + 115;
const LVM_ARRANGE: u32 = LVM_FIRST + 22;
const LVA_DEFAULT: WPARAM = WPARAM(0);
const LVS_AUTOARRANGE: i32 = 0x0100;

#[repr(C)]
struct LVITEM {
    mask: u32,
    i_item: i32,
    i_sub_item: i32,
    state: u32,
    state_mask: u32,
    psz_text: *mut u16,
    cch_text_max: i32,
    i_image: i32,
    l_param: isize,
    i_indent: i32,
    i_group_id: i32,
    c_columns: u32,
    pu_columns: *mut u32,
    pi_col_fmt: *mut i32,
    i_group: i32,
}

const LVIF_TEXT: u32 = 0x0001;

#[allow(dead_code)]
pub unsafe fn find_desktop_listview() -> Option<DesktopListView> {
    // Generic search — returns the first ListView found (fallback)
    _find_listview_internal(None)
}

/// Find the ListView that belongs to the monitor at (mon_x, mon_y).
pub(crate) struct DesktopListView {
    hwnd: HWND,
    parent_left: i32,
    parent_top: i32,
}

/// Matches WorkerW window rect against the target monitor coordinates.
pub unsafe fn find_desktop_listview_for_monitor(mon_x: i32, mon_y: i32) -> Option<DesktopListView> {
    _find_listview_internal(Some((mon_x, mon_y)))
}

unsafe fn _find_listview_internal(target: Option<(i32, i32)>) -> Option<DesktopListView> {
    let progman = FindWindowW(w!("Progman"), w!("Program Manager")).ok()?;
    SendMessageW(progman, 0x052C, WPARAM(0xD), LPARAM(0));
    SendMessageW(progman, 0x052C, WPARAM(0xD), LPARAM(1));

    // Path 1: Progman → SHELLDLL_DefView → SysListView32 (primary monitor only)
    let defview = FindWindowExW(progman, HWND::default(), w!("SHELLDLL_DefView"), w!("")).ok();
    if let Some(dv) = defview {
        let lv = FindWindowExW(dv, HWND::default(), w!("SysListView32"), w!("FolderView")).ok();
        if let Some(lv) = lv {
            // Progman ListView only serves the primary monitor; only return if target matches
            if let Some((tx, ty)) = target {
                let mut pr = RECT::default();
                if GetWindowRect(progman, &mut pr).is_ok() {
                    if tx >= pr.left && ty >= pr.top && tx < pr.right && ty < pr.bottom {
                        println!("[desktop] Progman({},{},{},{}) 包含目标({},{}), hwnd={:?}",
                            pr.left, pr.top, pr.right, pr.bottom, tx, ty, lv);
                        return Some(DesktopListView { hwnd: lv, parent_left: pr.left, parent_top: pr.top });
                    }
                    println!("[desktop] Progman 不包含目标({},{}), 继续搜索 WorkerW", tx, ty);
                }
            } else {
                println!("[desktop] Progman 路径找到 ListView (无目标): {:?}", lv);
                let mut pr = RECT::default();
                let _ = GetWindowRect(progman, &mut pr);
                return Some(DesktopListView { hwnd: lv, parent_left: pr.left, parent_top: pr.top });
            }
        }
    }

    // Path 2: WorkerW → SHELLDLL_DefView → SysListView32 (multi-monitor)
    let mut prev = HWND::default();
    loop {
        let w = FindWindowExW(HWND::default(), prev, w!("WorkerW"), w!("")).ok();
        match w {
            Some(hw) => {
                if let Some(dlv) = extract_from_workerw(hw, target) {
                    return Some(dlv);
                }
                prev = hw;
            }
            None => break,
        }
    }
    None
}

/// Extract SysListView32 from a WorkerW window, checking monitor containment
unsafe fn extract_from_workerw(worker: HWND, target: Option<(i32, i32)>) -> Option<DesktopListView> {
    let dv = FindWindowExW(worker, HWND::default(), w!("SHELLDLL_DefView"), w!("")).ok()?;
    let lv = FindWindowExW(dv, HWND::default(), w!("SysListView32"), w!("FolderView")).ok()?;
    let mut wr = RECT::default();
    let _ = GetWindowRect(worker, &mut wr);
    if let Some((tx, ty)) = target {
        if tx >= wr.left && ty >= wr.top && tx < wr.right && ty < wr.bottom {
            println!("[desktop] WorkerW({},{},{},{}) 包含目标显示器({},{}), hwnd={:?}", 
                wr.left, wr.top, wr.right, wr.bottom, tx, ty, lv);
        } else {
            println!("[desktop] WorkerW({},{},{},{}) 不包含目标显示器({},{}), 跳过", 
                wr.left, wr.top, wr.right, wr.bottom, tx, ty);
            return None;
        }
    }
    Some(DesktopListView { hwnd: lv, parent_left: wr.left, parent_top: wr.top })
}

pub unsafe fn get_item_count(listview: HWND) -> i32 {
    SendMessageW(listview, LVM_GETITEMCOUNT, WPARAM::default(), LPARAM::default()).0 as i32
}

pub unsafe fn get_item_text(listview: HWND, index: i32) -> Option<String> {
    let mut pid: u32 = 0;
    GetWindowThreadProcessId(listview, Some(&mut pid));
    let hproc = OpenProcess(
        PROCESS_VM_OPERATION | PROCESS_VM_READ | PROCESS_VM_WRITE,
        false,
        pid,
    ).ok()?;

    let buf_size: usize = 1024;
    let remote_buf = VirtualAllocEx(
        hproc,
        None,
        buf_size,
        MEM_COMMIT | MEM_RESERVE,
        PAGE_READWRITE,
    );
    if remote_buf.is_null() {
        let _ = CloseHandle(hproc);
        return None;
    }

    let local_lvi = LVITEM {
        mask: LVIF_TEXT,
        i_item: index,
        i_sub_item: 0,
        state: 0,
        state_mask: 0,
        psz_text: remote_buf as *mut u16,
        cch_text_max: (buf_size / 2) as i32,
        i_image: 0,
        l_param: 0,
        i_indent: 0,
        i_group_id: 0,
        c_columns: 0,
        pu_columns: std::ptr::null_mut(),
        pi_col_fmt: std::ptr::null_mut(),
        i_group: 0,
    };

    let lvi_size = std::mem::size_of::<LVITEM>();
    let remote_lvi = VirtualAllocEx(hproc, None, lvi_size, MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE);
    if remote_lvi.is_null() {
        let _ = VirtualFreeEx(hproc, remote_buf, 0, MEM_RELEASE);
        let _ = CloseHandle(hproc);
        return None;
    }

    let mut written: usize = 0;
    let _ = WriteProcessMemory(
        hproc,
        remote_lvi,
        &local_lvi as *const _ as *const std::ffi::c_void,
        lvi_size,
        Some(&mut written),
    );

    SendMessageW(listview, LVM_GETITEMTEXTW, WPARAM(index as usize), LPARAM(remote_lvi as isize));

    let mut local_buf: Vec<u16> = vec![0u16; buf_size / 2];
    let mut read: usize = 0;
    let _ = ReadProcessMemory(
        hproc,
        remote_buf,
        local_buf.as_mut_ptr() as *mut std::ffi::c_void,
        buf_size,
        Some(&mut read),
    );

    let txt = String::from_utf16_lossy(&local_buf);
    let txt = txt.trim_end_matches('\0').to_string();

    let _ = VirtualFreeEx(hproc, remote_lvi, 0, MEM_RELEASE);
    let _ = VirtualFreeEx(hproc, remote_buf, 0, MEM_RELEASE);
    let _ = CloseHandle(hproc);

    if txt.is_empty() { None } else { Some(txt) }
}

pub unsafe fn set_item_position(listview: HWND, index: i32, x: i32, y: i32) {
    let lparam = ((y as isize) << 16) | ((x as isize) & 0xFFFF);
    SendMessageW(listview, LVM_SETITEMPOSITION, WPARAM(index as usize), LPARAM(lparam));
}

pub struct DesktopIcon {
    pub index: i32,
    pub name: String,
}

#[allow(dead_code)]
pub unsafe fn get_all_icons() -> std::result::Result<Vec<DesktopIcon>, String> {
    let listview = find_desktop_listview().ok_or("Cannot find desktop icon list")?.hwnd;
    get_all_icons_from(listview)
}

pub unsafe fn get_all_icons_from(listview: HWND) -> std::result::Result<Vec<DesktopIcon>, String> {
    let count = get_item_count(listview);
    let mut icons = Vec::new();
    for i in 0..count {
        if let Some(name) = get_item_text(listview, i) {
            icons.push(DesktopIcon { index: i, name });
        }
    }
    Ok(icons)
}

/// Classification result for a single desktop icon
#[derive(Debug, Clone)]
pub struct IconClass {
    pub index: i32,
    pub name: String,
    pub category: String,
    /// Full path on disk (e.g. C:\Users\...\Desktop\Chrome.lnk), used for icon extraction
    pub full_path: Option<String>,
}

/// Classify all icons on a monitor by extension / keyword (does NOT move them).
pub unsafe fn classify_icons_for_monitor(
    mon_x: i32, mon_y: i32,
) -> std::result::Result<Vec<IconClass>, String> {
    let dlv = find_desktop_listview_for_monitor(mon_x, mon_y)
        .ok_or("在此显示器上找不到桌面图标列表")?;
    let icons = get_all_icons_from(dlv.hwnd)?;
    let desktop_dir = dirs::desktop_dir().unwrap_or_default();
    let mut ext_lookup = std::collections::HashMap::new();
    let mut name_to_path = std::collections::HashMap::new();
    if let Ok(entries) = std::fs::read_dir(&desktop_dir) {
        for entry in entries.flatten() {
            if entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                let path = entry.path();
                let base = path.file_stem().unwrap_or_default().to_string_lossy().to_lowercase();
                let fname = entry.file_name().to_string_lossy().to_string();
                let ext = path.extension().unwrap_or_default().to_string_lossy().to_lowercase();
                name_to_path.insert(fname.to_lowercase(), path.to_string_lossy().to_string());
                if !ext.is_empty() {
                    ext_lookup.entry(base).or_insert(format!(".{}", ext));
                }
            }
        }
    }
    Ok(classify_icon_list(&icons, &ext_lookup, &name_to_path, &desktop_dir))
}

/// Classify a slice of desktop icons into categories (shared between preview and organize).
/// Check if an icon is a known system CLSID item (This PC, Recycle Bin, etc.)
fn is_system_icon(name: &str) -> bool {
    // Strip .lnk extension if present
    let base = name.strip_suffix(".lnk").unwrap_or(name);
    let lower = base.to_lowercase();
    // Exact match for system CLSID names (prevents false match like "NVIDIA 控制面板")
    lower == "回收站" || lower == "recycle bin" || lower == "recycle"
        || lower == "此电脑" || lower == "我的电脑" || lower == "this pc" || lower == "my computer" || lower == "computer"
        || lower == "网络" || lower == "network"
        || lower == "控制面板" || lower == "control panel"
        || lower == "用户文件夹" || lower == "user's files" || lower == "users files"
}

/// Check if an icon is a network location (UNC path shortcut, FTP, etc.)
fn is_network_location(name: &str, target_path: &Option<String>) -> bool {
    let lower = name.to_lowercase();
    // Name-based: explicitly "网络位置"
    if lower.contains("网络位置") || lower.contains("network location") {
        return true;
    }
    // Target-based: check if the .lnk target is a network path
    if let Some(t) = target_path {
        let tl = t.to_lowercase();
        if tl.starts_with("\\\\") || tl.starts_with("//") {
            return true; // UNC path
        }
        if tl.starts_with("ftp://") || tl.starts_with("ftps://") || tl.starts_with("sftp://") {
            return true;
        }
    }
    false
}

/// Check whether the icon name matches an existing desktop directory.
fn is_folder(name: &str, desktop_dir: &std::path::Path) -> bool {
    // Strip .lnk extension if present
    let base = name.strip_suffix(".lnk").unwrap_or(name);
    // Match exact folder name OR case-insensitive
    let path = desktop_dir.join(base);
    if path.is_dir() { return true; }
    // Also try case-insensitive
    if let Ok(entries) = std::fs::read_dir(desktop_dir) {
        for e in entries.flatten() {
            if e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                if e.file_name().to_string_lossy().to_lowercase() == base.to_lowercase() {
                    return true;
                }
            }
        }
    }
    false
}

pub fn classify_icon_list(
    icons: &[DesktopIcon],
    ext_lookup: &std::collections::HashMap<String, String>,
    name_to_path: &std::collections::HashMap<String, String>,
    desktop_dir: &std::path::Path,
) -> Vec<IconClass> {
    icons.iter().map(|icon| {
        // ── Pre-classification: special icon types (system, folder, network) ──
        let pre_category = {
            let base = icon.name.strip_suffix(".lnk").unwrap_or(&icon.name);
            // 1) System icons (CLSID items like 回收站, 此电脑, 网络, 控制面板)
            if is_system_icon(base) {
                Some("系统图标".to_string())
            }
            // 2) Folders (check filesystem before lnk resolution)
            else if is_folder(&icon.name, desktop_dir) {
                Some("文件夹".to_string())
            }
            // 3) Network locations — need lnk resolution first, so check after
            else {
                None
            }
        };

        // Check network location via lnk target (must resolve .lnk first)
        let network_check = if pre_category.is_none() {
            let lnk_target = if icon.name.to_lowercase().ends_with(".lnk")
                || resolve_ext(icon, ext_lookup) == ".lnk"
            {
                let lnk_name = if icon.name.to_lowercase().ends_with(".lnk") {
                    icon.name.clone()
                } else {
                    format!("{}.lnk", icon.name)
                };
                unsafe { shortcut::resolve_lnk_target(&desktop_dir.join(&lnk_name)) }
            } else {
                None
            };
            if is_network_location(&icon.name, &lnk_target) {
                Some("网络位置".to_string())
            } else {
                None
            }
        } else {
            None
        };

        if let Some(cat) = pre_category.or(network_check) {
            let full_path = name_to_path.get(&icon.name.to_lowercase()).cloned();
            return IconClass { index: icon.index, name: icon.name.clone(), category: cat, full_path };
        }

        // ── Regular classification (extension-based + keyword matching) ──
        let ext = resolve_ext(icon, ext_lookup);
        let category = if ext == ".lnk" {
            let lnk_name = if icon.name.to_lowercase().ends_with(".lnk") {
                icon.name.clone()
            } else {
                format!("{}.lnk", icon.name)
            };
            let target = unsafe { shortcut::resolve_lnk_target(&desktop_dir.join(&lnk_name)) };
            if let Some(ref t) = target {
                let t_ext = std::path::Path::new(t)
                    .extension()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_lowercase();
                if t_ext == "exe" || t_ext == "dll" || t_ext == "com" {
                    classify_exe_by_keyword(&icon.name, &target)
                } else if !t_ext.is_empty() {
                    format!(".{}", t_ext)
                } else {
                    ".lnk".to_string()
                }
            } else {
                ".lnk".to_string()
            }
        } else if matches!(ext.as_str(), "" | ".exe" | ".dll" | ".com") {
            classify_exe_by_keyword(&icon.name, &None)
        } else {
            ext
        };
        let full_path = name_to_path.get(&icon.name.to_lowercase()).cloned();
        IconClass { index: icon.index, name: icon.name.clone(), category, full_path }
    }).collect()
}

/// Resolve the effective file extension for an icon, using ext_lookup for hidden extensions.
fn resolve_ext(icon: &DesktopIcon, ext_lookup: &std::collections::HashMap<String, String>) -> String {
    if let Some(dot) = icon.name.rfind('.') {
        icon.name[dot..].to_lowercase()
    } else {
        let base = icon.name.to_lowercase();
        ext_lookup.get(&base).cloned().unwrap_or_default()
    }
}

/// Classify executables by keyword matching on name and resolved target path
fn classify_exe_by_keyword(name: &str, target_path: &Option<String>) -> String {
    let mut haystack = name.to_lowercase();
    if let Some(t) = target_path {
        let tl = t.to_lowercase();
        haystack.push(' ');
        haystack.push_str(&tl);
        // Also add just the file name for better matching
        if let Some(sep) = tl.rfind('\\').or_else(|| tl.rfind('/')) {
            haystack.push(' ');
            haystack.push_str(&tl[sep+1..]);
        }
    }

    // Programming / dev tools
    if haystack.contains("code") || haystack.contains("visual studio") || haystack.contains("devenv")
        || haystack.contains("idea") || haystack.contains("pycharm") || haystack.contains("intellij")
        || haystack.contains("eclipse") || haystack.contains("rider") || haystack.contains("webstorm")
        || haystack.contains("clion") || haystack.contains("goland") || haystack.contains("android studio")
        || haystack.contains("cursor") || haystack.contains("sublime") || haystack.contains("notepad++")
        || haystack.contains("godot") || haystack.contains("blender") || haystack.contains("unity")
        || haystack.contains("unreal") || haystack.contains("github") || haystack.contains("docker")
        || haystack.contains("postman") || haystack.contains("xshell") || haystack.contains("putty")
        || haystack.contains("cmake") || haystack.contains("node") || haystack.contains("npm")
        || haystack.contains("powershell") || haystack.contains("git") || haystack.contains("wireshark")
        || haystack.contains("fiddler") || haystack.contains("navicat") || haystack.contains("dbeaver")
        || haystack.contains("datagrip") || haystack.contains("mobaxterm") || haystack.contains("winmerge")
        || haystack.contains("everything") || haystack.contains("snipaste") || haystack.contains("快贴")
        || haystack.contains("编程") || haystack.contains("开发") || haystack.contains("dev")
        || haystack.contains("pl/sql") || haystack.contains("mysql") || haystack.contains("postgres")
        || haystack.contains("mongodb") || haystack.contains("redis") || haystack.contains("rabbitmq")
        || haystack.contains("kubernetes") || haystack.contains("jenkins") || haystack.contains("nginx")
        || haystack.contains("api") || haystack.contains("sublime merge") || haystack.contains("chatgpt")
        || haystack.contains("openai") || haystack.contains("claude") || haystack.contains("copilot")
        || haystack.contains("ollama") || haystack.contains("lm studio") || haystack.contains("torch")
        || haystack.contains("anaconda") || haystack.contains("conda") || haystack.contains("python")
        || haystack.contains("hugging") || haystack.contains("wsl") || haystack.contains("cygwin")
        || haystack.contains("mingw") || haystack.contains("cmder") || haystack.contains("gitkraken")
        || haystack.contains("goland") || haystack.contains("qt creator") || haystack.contains("vcpkg")
        || haystack.contains("gitee") || haystack.contains("gitlab") || haystack.contains("bitbucket")
    {
        return "编程开发".to_string();
    }

    // Design & creative
    if haystack.contains("photoshop") || haystack.contains("premiere") || haystack.contains("after effects")
        || haystack.contains("illustrator") || haystack.contains("indesign") || haystack.contains("figma")
        || haystack.contains("autocad") || haystack.contains("maya") || haystack.contains("sketch")
        || haystack.contains("davinci") || haystack.contains("capcut") || haystack.contains("剪映")
        || haystack.contains("lightroom") || haystack.contains("corel") || haystack.contains("audition")
        || haystack.contains("3d") || haystack.contains("渲染") || haystack.contains("建模")
        || haystack.contains("oculus") || haystack.contains("vr") || haystack.contains("ar")
    {
        return "设计创作".to_string();
    }

    // Office / productivity
    if haystack.contains("word") || haystack.contains("excel") || haystack.contains("powerpoint")
        || haystack.contains("outlook") || haystack.contains("onenote") || haystack.contains("winword")
        || haystack.contains("wps") || haystack.contains("office") || haystack.contains("notion")
        || haystack.contains("obsidian") || haystack.contains("typora") || haystack.contains("evernote")
        || haystack.contains("pdf") || haystack.contains("foxit") || haystack.contains("adobe acrobat")
        || haystack.contains("access") || haystack.contains("publisher") || haystack.contains("visio")
        || haystack.contains("office") || haystack.contains("永中") || haystack.contains("office")
        || haystack.contains("笔记") || haystack.contains("便签") || haystack.contains("日历")
        || haystack.contains("todo") || haystack.contains("trello") || haystack.contains("asana")
    {
        return "办公学习".to_string();
    }

    // Browsers
    if haystack.contains("chrome") || haystack.contains("msedge") || haystack.contains("firefox")
        || haystack.contains("opera") || haystack.contains("brave") || haystack.contains("vivaldi")
        || haystack.contains("浏览器") || haystack.contains("browser") || haystack.contains("百分")
        || haystack.contains("星愿") || haystack.contains("搜狗") || haystack.contains("遨游")
        || haystack.contains("uc") || haystack.contains("qq浏览器") || haystack.contains("360")
        || haystack.contains("极速") || haystack.contains("夸克") || haystack.contains("edge")
        || haystack.contains("safari") || haystack.contains("豆包")
    {
        return "浏览器".to_string();
    }

    // Gaming
    if haystack.contains("steam") || haystack.contains("epic") || haystack.contains("battle.net")
        || haystack.contains("原神") || haystack.contains("genshin") || haystack.contains("minecraft")
        || haystack.contains("valorant") || haystack.contains("英雄联盟") || haystack.contains("lol")
        || haystack.contains("pubg") || haystack.contains("dota") || haystack.contains("wow")
        || haystack.contains("overwatch") || haystack.contains("cs") || haystack.contains("apex")
        || haystack.contains("ubisoft") || haystack.contains("游戏") || haystack.contains("game")
        || haystack.contains("wegame") || haystack.contains("腾讯手游") || haystack.contains("雷电")
        || haystack.contains("模拟器") || haystack.contains("emulator") || haystack.contains("bluestacks")
        || haystack.contains("mumu") || haystack.contains("逍遥") || haystack.contains("夜神")
        || haystack.contains("王者荣耀") || haystack.contains("和平精英") || haystack.contains("崩坏")
        || haystack.contains("星铁") || haystack.contains("绝区零") || haystack.contains("永劫")
        || haystack.contains("nvidia") || haystack.contains("geforce") || haystack.contains("游戏中心")
    {
        return "游戏".to_string();
    }

    // Social & communication
    if haystack.contains("wechat") || haystack.contains("微信") || haystack.contains("qq")
        || haystack.contains("telegram") || haystack.contains("discord") || haystack.contains("slack")
        || haystack.contains("钉钉") || haystack.contains("飞书") || haystack.contains("teams")
        || haystack.contains("zoom") || haystack.contains("腾讯会议") || haystack.contains("whatsapp")
        || haystack.contains("微博") || haystack.contains("skype") || haystack.contains("line")
        || haystack.contains("tim") || haystack.contains("yy") || haystack.contains("陌陌")
        || haystack.contains("探探") || haystack.contains("soul") || haystack.contains("signal")
        || haystack.contains("小红书") || haystack.contains("知乎") || haystack.contains("百度贴吧")
    {
        return "社交聊天".to_string();
    }

    // Media & entertainment
    if haystack.contains("qq音乐") || haystack.contains("spotify") || haystack.contains("netflix")
        || haystack.contains("vlc") || haystack.contains("potplayer") || haystack.contains("播放器")
        || haystack.contains("player") || haystack.contains("bilibili") || haystack.contains("mpv")
        || haystack.contains("网易云") || haystack.contains("酷狗") || haystack.contains("抖音")
        || haystack.contains("kodi") || haystack.contains("plex") || haystack.contains("xbox")
        || haystack.contains("爱奇艺") || haystack.contains("优酷") || haystack.contains("腾讯视频")
        || haystack.contains("芒果") || haystack.contains("youtube") || haystack.contains("twitch")
        || haystack.contains("media") || haystack.contains("foobar") || haystack.contains("aimp")
        || haystack.contains("网易云音乐") || haystack.contains("喜马拉雅") || haystack.contains("播客")
        || haystack.contains("音乐") || haystack.contains("视频") || haystack.contains("影视")
        || haystack.contains("直播") || haystack.contains("录屏") || haystack.contains("obs")
    {
        return "影音娱乐".to_string();
    }

    // System tools
    if haystack.contains("cmd") || haystack.contains("powershell") || haystack.contains("terminal")
        || haystack.contains("regedit") || haystack.contains("控制面板") || haystack.contains("control")
        || haystack.contains("任务管理器") || haystack.contains("taskmgr") || haystack.contains("mmc")
        || haystack.contains("设备管理器") || haystack.contains("防火墙") || haystack.contains("远程桌面")
        || haystack.contains("mstsc") || haystack.contains("explorer") || haystack.contains("notepad")
        || haystack.contains("磁盘管理") || haystack.contains("msinfo") || haystack.contains("msconfig")
        || haystack.contains("clean") || haystack.contains("清理") || haystack.contains("管家")
        || haystack.contains("安全") || haystack.contains("杀毒") || haystack.contains("驱动")
        || haystack.contains("压缩") || haystack.contains("winrar") || haystack.contains("winzip")
        || haystack.contains("bandizip") || haystack.contains("peazip") || haystack.contains("ultraiso")
        || haystack.contains("daemon") || haystack.contains("virtualbox") || haystack.contains("vmware")
        || haystack.contains("rufus") || haystack.contains("cpuz") || haystack.contains("gpuz")
        || haystack.contains("hwinfo") || haystack.contains("crystal") || haystack.contains("aida")
        || haystack.contains("afterburner") || haystack.contains("rtss") || haystack.contains("fan")
        || haystack.contains("键盘") || haystack.contains("鼠标") || haystack.contains("外设")
        || haystack.contains("ghub") || haystack.contains("razer") || haystack.contains("logitech")
        || haystack.contains("steelseries") || haystack.contains("corsair")
    {
        return "系统工具".to_string();
    }

    "程序".to_string()
}

pub unsafe fn organize_icons(zones: &[crate::config::Zone], mon_x: i32, mon_y: i32, spacing_scale: f32) -> std::result::Result<usize, String> {
    println!("[organize] ========== 开始整理 ==========");
    println!("[organize] 目标显示器: ({}, {}), 区域数量: {}", mon_x, mon_y, zones.len());
    for (i, z) in zones.iter().enumerate() {
        println!("[organize]   区域[{}] name={} x={} y={} w={} h={} enabled={} types={:?} sort={} other={}",
            i, z.name, z.x, z.y, z.width, z.height, z.enabled, z.file_types, z.sort_mode, z.is_other);
    }
    
    // Find the ListView for THIS specific monitor, not the first one system-wide
    println!("[organize] 查找 ListView for ({}, {})...", mon_x, mon_y);
    let dlv = find_desktop_listview_for_monitor(mon_x, mon_y)
        .ok_or("在当前显示器上找不到桌面图标列表\n请确认该显示器已启用'显示桌面图标'")?;
    let listview = dlv.hwnd;
    let offset_x = mon_x - dlv.parent_left;
    let offset_y = mon_y - dlv.parent_top;
    println!("[organize] 找到 ListView: {:?}, parent=({},{}), 坐标偏移=({},{})",
        listview, dlv.parent_left, dlv.parent_top, offset_x, offset_y);
    
    // Detect & suppress auto-arrange (Windows overrides LVM_SETITEMPOSITION when enabled)
    let orig_style = GetWindowLongW(listview, WINDOW_LONG_PTR_INDEX(-16i32)) as u32;
    let auto_arrange_was_on = (orig_style & LVS_AUTOARRANGE as u32) != 0;
    println!("[organize] ListView style: 0x{:x}, auto_arrange={}", orig_style, auto_arrange_was_on);
    if auto_arrange_was_on {
        println!("[organize] 暂时关闭 LVS_AUTOARRANGE");
        SetWindowLongW(listview, WINDOW_LONG_PTR_INDEX(-16i32), (orig_style & !(LVS_AUTOARRANGE as u32)) as i32);
    }

    let icons = get_all_icons_from(listview)?;
    println!("[organize] 桌面图标数量: {}", icons.len());
    if icons.is_empty() {
        if auto_arrange_was_on { SetWindowLongW(listview, WINDOW_LONG_PTR_INDEX(-16i32), orig_style as i32); }
        return Ok(0);
    }

    let active: Vec<_> = zones.iter().filter(|z| z.enabled).collect();
    println!("[organize] 启用的区域: {}", active.len());
    if active.is_empty() {
        if auto_arrange_was_on { SetWindowLongW(listview, WINDOW_LONG_PTR_INDEX(-16i32), orig_style as i32); }
        return Ok(0);
    }

    let desktop_dir = dirs::desktop_dir().unwrap_or_default();
    let mut ext_lookup = std::collections::HashMap::new();
    let mut name_to_path = std::collections::HashMap::new();
    if let Ok(entries) = std::fs::read_dir(&desktop_dir) {
        for entry in entries.flatten() {
            if entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                let path = entry.path();
                let base = path.file_stem().unwrap_or_default().to_string_lossy().to_lowercase();
                let fname = entry.file_name().to_string_lossy().to_string();
                let ext = path.extension().unwrap_or_default().to_string_lossy().to_lowercase();
                name_to_path.insert(fname.to_lowercase(), path.to_string_lossy().to_string());
                if !ext.is_empty() {
                    ext_lookup.entry(base).or_insert(format!(".{}", ext));
                }
            }
        }
    }

    let all_classes = classify_icon_list(&icons, &ext_lookup, &name_to_path, &desktop_dir);

    // Separate non-other active zones from the "other" catch-all zone
    let other_zone_idx = active.iter().position(|z| z.is_other);
    let non_other: Vec<&crate::config::Zone> = active.iter().filter(|z| !z.is_other).copied().collect();

    let mut classified: Vec<Vec<&DesktopIcon>> = vec![Vec::new(); non_other.len()];
    let mut unmatched = Vec::new();

    println!("[organize] 开始分类 {} 个图标...", icons.len());
    for cls in &all_classes {
        let icon = &icons[cls.index as usize];
        let mut found = false;
        for (ni, z) in non_other.iter().enumerate() {
            // Empty file_types = match any unmatched type (catch-all behaviour)
            if z.file_types.is_empty() || z.file_types.iter().any(|t| t.to_lowercase() == cls.category) {
                classified[ni].push(icon);
                found = true;
                break;
            }
        }
        if !found { unmatched.push(icon); }
    }

    println!("[organize] 分类结果: non_other zones={}, unmatched_icons={}", non_other.len(), unmatched.len());
    if !unmatched.is_empty() {
        println!("[organize] 未匹配图标 (前20个):");
        for icon in unmatched.iter().take(20) {
            let cls = all_classes.iter().find(|c| c.index == icon.index);
            let detail = if let Some(c) = cls {
                format!("category={}", c.category)
            } else {
                "无分类".to_string()
            };
            println!("[organize]   '{}' ({})", icon.name, detail);
        }
    }

    // Helper: get the file extension string for sorting (raw from icon name)
    let icon_sort_ext = |name: &str| -> String {
        name.rfind('.')
            .map(|i| name[i..].to_lowercase())
            .unwrap_or_default()
    };

    // Sort icons within each non-other zone
    for (ni, z) in non_other.iter().enumerate() {
        let icons_in_zone = &mut classified[ni];
        match z.sort_mode.as_str() {
            "name" => {
                icons_in_zone.sort_by(|a, b|
                    a.name.to_lowercase().cmp(&b.name.to_lowercase())
                );
            }
            "type" => {
                icons_in_zone.sort_by(|a, b| {
                    let ea = icon_sort_ext(&a.name);
                    let eb = icon_sort_ext(&b.name);
                    ea.cmp(&eb).then_with(||
                        a.name.to_lowercase().cmp(&b.name.to_lowercase())
                    )
                });
            }
            _ => {} // "none" or unknown — keep original order
        }
    }

    let mut moved = 0usize;
    let mut zone_msgs: Vec<String> = Vec::new();

    // Place icons in non-other zones (zone coords are monitor-relative, add monitor offset)
    println!("[organize] 开始放置图标...");
    for (ni, z) in non_other.iter().enumerate() {
        let icons_in_zone = &classified[ni];
        if icons_in_zone.is_empty() { continue; }
        let sx = z.x + z.icon_spacing_x / 2;
        let sy = z.y + z.icon_spacing_y / 2;
        
        // Generate shape positions (relative to zone top-left)
        let sp = shapes::ShapeParams {
            icon_count: icons_in_zone.len(),
            zone_w: z.width,
            zone_h: z.height,
            spacing_x: z.icon_spacing_x,
            spacing_y: z.icon_spacing_y,
            spacing_scale,
        };
        let positions = shapes::generate_positions(&z.shape, &z.shape_text, &sp);
        println!("[organize]   放置区域[{}] '{}': {} 图标, 形状={}",
            ni, z.name, icons_in_zone.len(), z.shape.label());
        for (j, icon) in icons_in_zone.iter().enumerate() {
            let (px, py) = positions.get(j).copied().unwrap_or((sx, sy));
            let tx = z.x + px;
            let ty = z.y + py;
            if j < 3 { println!("[organize]     icon[{}] '{}' -> ({},{})", icon.index, icon.name, tx, ty); }
            set_item_position(listview, icon.index, tx + offset_x, ty + offset_y);
            moved += 1;
        }
        zone_msgs.push(format!("{}: {}", z.name, icons_in_zone.len()));
    }

    // Handle unmatched icons with the "other" catch-all zone
    let mut other_line = String::new();
    if let Some(oi) = other_zone_idx {
        let oz = active[oi];
        // Sort unmatched by the other zone's sort_mode
        match oz.sort_mode.as_str() {
            "name" => {
                unmatched.sort_by(|a, b|
                    a.name.to_lowercase().cmp(&b.name.to_lowercase())
                );
            }
            "type" => {
                unmatched.sort_by(|a, b| {
                    let ea = icon_sort_ext(&a.name);
                    let eb = icon_sort_ext(&b.name);
                    ea.cmp(&eb).then_with(||
                        a.name.to_lowercase().cmp(&b.name.to_lowercase())
                    )
                });
            }
            _ => {}
        }

        if !unmatched.is_empty() {
            let sp_o = shapes::ShapeParams {
                icon_count: unmatched.len(),
                zone_w: oz.width, zone_h: oz.height,
                spacing_x: oz.icon_spacing_x, spacing_y: oz.icon_spacing_y,
                spacing_scale,
            };
            let pos_o = shapes::generate_positions(&oz.shape, &oz.shape_text, &sp_o);
            for (j, icon) in unmatched.iter().enumerate() {
                let (px, py) = pos_o.get(j).copied().unwrap_or((oz.x, oz.y));
                let tx = oz.x + px;
                let ty = oz.y + py;
                set_item_position(listview, icon.index, tx + offset_x, ty + offset_y);
                moved += 1;
            }
            zone_msgs.push(format!("{}: {}", oz.name, unmatched.len()));
            other_line = format!(
                "\n{} 个未匹配 -> {} 区 ({})",
                unmatched.len(), oz.name, unmatched.len()
            );
        }
    } else if !unmatched.is_empty() {
        other_line = format!(
            "\n{} 个未分类图标留在原位",
            unmatched.len()
        );
    }

    let mut msg = format!("已移动 {} 个图标", moved);
    if !zone_msgs.is_empty() {
        msg.push_str(&format!(" ({})", zone_msgs.join(", ")));
    }
    msg.push_str(&other_line);

    // Refresh layout while auto-arrange is still suppressed (so Explorer doesn't undo our work)
    println!("[organize] 刷新布局 (LVM_ARRANGE)...");
    SendMessageW(listview, LVM_ARRANGE, LVA_DEFAULT, LPARAM::default());

    // Restore auto-arrange style
    if auto_arrange_was_on {
        println!("[organize] 恢复 LVS_AUTOARRANGE");
        SetWindowLongW(listview, WINDOW_LONG_PTR_INDEX(-16i32), orig_style as i32);
    }

    println!("[organize] 完成: {}", msg);
    Ok(moved)
}
