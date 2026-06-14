use serde::{Deserialize, Serialize};
use crate::i18n::{self, preset_name, shape_mode_label, Lang};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ShapeMode {
    #[serde(rename = "rect")]
    Rectangle,
    #[serde(rename = "heart")]
    Heart,
    #[serde(rename = "circle")]
    Circle,
    #[serde(rename = "triangle")]
    Triangle,
    #[serde(rename = "diamond")]
    Diamond,
    #[serde(rename = "star")]
    Star,
    #[serde(rename = "spiral")]
    Spiral,
    #[serde(rename = "sine")]
    Sine,
    #[serde(rename = "vshape")]
    VShape,
    #[serde(rename = "arc")]
    Arc,
    #[serde(rename = "cross")]
    Cross,
    #[serde(rename = "text")]
    Text,
}

impl Default for ShapeMode {
    fn default() -> Self { ShapeMode::Rectangle }
}

impl ShapeMode {
    pub fn label(&self) -> &'static str {
        shape_mode_label(crate::i18n::Lang::Zh, self)
    }

    pub fn all() -> &'static [ShapeMode] {
        use ShapeMode::*;
        &[Rectangle, Heart, Circle, Triangle, Diamond, Star, Spiral, Sine, VShape, Arc, Cross, Text]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Zone {
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub file_types: Vec<String>,
    #[serde(default = "default_spacing")]
    pub icon_spacing_x: i32,
    #[serde(default = "default_spacing")]
    pub icon_spacing_y: i32,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_sort_mode")]
    pub sort_mode: String,
    #[serde(default)]
    pub is_other: bool,
    #[serde(default)]
    pub children: Vec<Zone>,
    #[serde(default)]
    pub shape: ShapeMode,
    #[serde(default)]
    pub shape_text: String,
}

fn default_spacing() -> i32 { 80 }
fn default_sort_mode() -> String { "name".to_string() }
fn default_true() -> bool { true }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub version: u32,
    pub monitor_index: usize,
    pub zones: Vec<Zone>,
}

const CURRENT_CONFIG_VERSION: u32 = 3;

pub fn load_config(lang: Lang, monitor_idx: usize, mon_w: i32, mon_h: i32) -> Vec<Zone> {
    let path = config_path();
    if path.exists() {
        if let Ok(data) = std::fs::read_to_string(&path) {
            if let Ok(cfg) = serde_json::from_str::<Config>(&data) {
                if cfg.version == CURRENT_CONFIG_VERSION
                    && cfg.monitor_index == monitor_idx
                    && !cfg.zones.is_empty()
                {
                    return cfg.zones;
                }
                if cfg.version != CURRENT_CONFIG_VERSION {
                    println!("[config] 配置版本 {} → {} (重置为默认布局)", cfg.version, CURRENT_CONFIG_VERSION);
                }
            }
        }
    }
    default_zones(lang, mon_w, mon_h)
}

pub fn save_config(zones: &[Zone], monitor_idx: usize) {
    let cfg = Config {
        version: CURRENT_CONFIG_VERSION,
        monitor_index: monitor_idx,
        zones: zones.to_vec(),
    };
    if let Ok(json) = serde_json::to_string_pretty(&cfg) {
        let _ = std::fs::write(config_path(), json);
    }
}

fn config_path() -> std::path::PathBuf {
    std::env::current_exe()
        .unwrap_or_default()
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("config.json")
}

pub fn default_zones(lang: Lang, mon_w: i32, mon_h: i32) -> Vec<Zone> {
    default_zones_with_padding(lang, mon_w, mon_h, 30)
}

pub fn default_zones_with_padding(lang: Lang, mon_w: i32, mon_h: i32, pad: i32) -> Vec<Zone> {
    let cols: usize = 3;
    let cw = mon_w / cols as i32;
    let p = pad.max(5).min(150);

    // Weighted row heights — top rows larger (hero zones), bottom rows compact (catch-all)
    // Inspired by Fences / DesktopOK UX patterns: frequent-use items get more space
    let row_weights: [i32; 4] = [3, 3, 2, 2]; // total=10 → 30%/30%/20%/20%
    let total_weight: i32 = row_weights.iter().sum();
    let mut row_ys: [i32; 4] = [0; 4];
    let mut row_hs: [i32; 4] = [0; 4];
    let mut cum = 0;
    for i in 0..4 {
        let rh = mon_h * row_weights[i] / total_weight;
        row_ys[i] = cum;
        row_hs[i] = rh;
        cum += rh;
    }

    // 4×3 grid: arranged by UX priority — top→bottom = hot→cold
    // Row 0 (30%): most frequent — 浏览器 / 编程开发 / 办公学习
    // Row 1 (30%): secondary — 社交聊天 / 文件夹 / 影音娱乐
    // Row 2 (20%): occasional — 游戏 / 系统工具 / 程序(未分类)
    // Row 3 (20%): rarely accessed — 系统图标 / 网络位置 / 其他文件
    let grid: [(&str, &[&str], bool); 12] = [
        (&preset_name(lang, "浏览器"),   &["浏览器"],   false),
        (&preset_name(lang, "编程开发"), &["编程开发"], false),
        (&preset_name(lang, "办公学习"), &["办公学习"], false),
        (&preset_name(lang, "社交聊天"), &["社交聊天"], false),
        (&preset_name(lang, "文件夹"),   &["文件夹"],   false),
        (&preset_name(lang, "影音娱乐"), &["影音娱乐"], false),
        (&preset_name(lang, "游戏"),     &["游戏"],     false),
        (&preset_name(lang, "系统工具"), &["系统工具"], false),
        (&i18n::t(lang, "zone_uncategorized"), &["程序"], false),
        (&preset_name(lang, "系统图标"), &["系统图标"], false),
        (&preset_name(lang, "网络位置"), &["网络位置"], false),
        (&i18n::t(lang, "zone_other_files"), &[], true),
    ];

    let mut zones = Vec::with_capacity(12);
    for (i, (name, types, is_other)) in grid.iter().enumerate() {
        let col = (i % cols) as i32;
        let row_idx = (i / cols) as usize;
        zones.push(Zone {
            name: name.to_string(),
            x: col * cw + p,
            y: row_ys[row_idx] + p,
            width: cw - p * 2,
            height: (row_hs[row_idx] - p * 2).max(40),
            file_types: types.iter().map(|s| s.to_string()).collect(),
            icon_spacing_x: 80,
            icon_spacing_y: 80,
            enabled: true,
            sort_mode: "name".to_string(),
            is_other: *is_other,
            children: vec![],
            shape: ShapeMode::Rectangle,
            shape_text: String::new(),
        });
    }
    zones
}

pub const FILE_TYPE_PRESETS: &[(&str, &[&str])] = &[
    // Keyword-based categories (for .lnk shortcuts resolving to .exe)
    ("编程开发", &["编程开发"]),
    ("设计创作", &["设计创作"]),
    ("办公学习", &["办公学习"]),
    ("浏览器",   &["浏览器"]),
    ("社交聊天", &["社交聊天"]),
    ("影音娱乐", &["影音娱乐"]),
    ("游戏",     &["游戏"]),
    ("系统工具", &["系统工具"]),
    ("程序",     &["程序"]),
    // File extension-based presets
    ("文档",      &[".txt",".doc",".docx",".pdf",".xls",".xlsx",".ppt",".pptx",".csv",".md",".rtf",".odt"]),
    ("图片",    &[".jpg",".jpeg",".png",".gif",".bmp",".svg",".webp",".ico",".tiff",".psd"]),
    ("视频",    &[".mp4",".avi",".mkv",".mov",".wmv",".flv",".webm",".m4v",".mpg"]),
    ("音频",     &[".mp3",".wav",".flac",".aac",".wma",".ogg",".m4a"]),
    ("代码",      &[".py",".js",".ts",".html",".css",".json",".xml",".yaml",".yml",".java",".cpp",".c",".h",".cs",".go",".rs",".rb",".sh",".bat",".ps1"]),
    ("压缩包",  &[".zip",".rar",".7z",".tar",".gz",".bz2",".xz"]),
    ("快捷方式", &[".lnk",".url"]),
    ("文件夹",   &["文件夹"]),
    ("系统图标", &["系统图标"]),
    ("网络位置", &["网络位置"]),
];
