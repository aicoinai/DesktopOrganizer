#![windows_subsystem = "console"]
mod config;
mod desktop;
mod i18n;
mod icons;
mod shapes;
mod shortcut;

use config::*;
use egui::*;
use egui::epaint::StrokeKind;
use i18n::{t, preset_name, shape_mode_label, Lang};

#[derive(Debug, Clone, Copy, PartialEq)]
enum ShapeTool { Rect, Ellipse, Star, Polygon }

const GRID_SNAP: i32 = 80; // 形状绘制对齐到 80px 网格

impl ShapeTool {
    fn name(&self) -> &str {
        match self { Self::Rect => "\u{25a1}", Self::Ellipse => "\u{25cb}", Self::Star => "\u{2606}", Self::Polygon => "\u{2b20}" }
    }
    fn label(&self) -> &str {
        match self { Self::Rect => "shape_rect", Self::Ellipse => "shape_ellipse", Self::Star => "shape_star", Self::Polygon => "shape_polygon" }
    }
}

#[derive(Clone)]
struct Monitor {
    name: String,
    x: i32, y: i32, w: i32, h: i32,
    primary: bool,
}

fn get_monitors(lang: Lang) -> Vec<Monitor> {
    let mut monitors = Vec::new();
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::Graphics::Gdi::*;
        use windows::Win32::Foundation::*;
        const MONITORINFOF_PRIMARY: u32 = 1;
        struct Ctx { out: *mut Vec<Monitor>, lang: Lang }
        unsafe {
            extern "system" fn enum_proc(
                hmon: HMONITOR, _hdc: HDC, _rc: *mut RECT, lp: LPARAM,
            ) -> BOOL {
                unsafe {
                    let ctx = &mut *(lp.0 as *mut Ctx);
                    let out = &mut *ctx.out;
                    let lang = ctx.lang;
                    let mut info = MONITORINFOEXW::default();
                    info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
                    if GetMonitorInfoW(hmon, &mut info as *mut _ as *mut MONITORINFO).as_bool() {
                        let rc = info.monitorInfo.rcMonitor;
                        let wr = rc.right - rc.left;
                        let hr = rc.bottom - rc.top;
                        let primary = (info.monitorInfo.dwFlags & MONITORINFOF_PRIMARY) != 0;
                        let name = if primary {
                            t(lang, "monitor_main").replacen("{}", &format!("{}", out.len() + 1), 1).replacen("{}", &wr.to_string(), 1).replacen("{}", &hr.to_string(), 1)
                        } else {
                            t(lang, "monitor_aux").replacen("{}", &format!("{}", out.len() + 1), 1).replacen("{}", &wr.to_string(), 1).replacen("{}", &hr.to_string(), 1)
                        };
                        out.push(Monitor { name, x: rc.left, y: rc.top, w: wr, h: hr, primary });
                    }
                    BOOL(1)
                }
            }
            let mut ctx = Ctx { out: &raw mut monitors, lang };
            let _ = EnumDisplayMonitors(HDC::default(), None, Some(enum_proc), LPARAM(&raw mut ctx as isize));
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        monitors.push(Monitor { name: "Screen".into(), x: 0, y: 0, w: 1920, h: 1080, primary: true });
    }
    monitors
}

struct Drawing {
    tool: ShapeTool,
    start: Pos2,
    current: Pos2,
    vertices: Vec<Pos2>,
}

struct App {
    monitors: Vec<Monitor>,
    cur_mon: usize,   // which monitor's zones we're viewing/editing
    sel_mons: Vec<usize>, // which monitors to organize
    mon_x: i32, mon_y: i32, mon_w: i32, mon_h: i32,
    zones: Vec<Zone>,
    sel_zone: i32,
    sel_child: Option<usize>, // selected child index within sel_zone's children
    drawing: Option<Drawing>,
    resize_drag: Option<ResizeDrag>,
    zone_drag: Option<ZoneDrag>,
    tool: ShapeTool,
    edit_name: String,
    edit_types: String,
    edit_spx: String,
    edit_spy: String,
    edit_enabled: bool,
    edit_x: String,
    edit_y: String,
    edit_w: String,
    edit_h: String,
    edit_sort_mode: String,
    edit_is_other: bool,
    edit_shape: ShapeMode,
    edit_shape_text: String,
    shape_spacing: f32,
    grid_padding: i32,
    status: String,
    preview_icons: Vec<desktop::IconClass>,
    preview_dirty: bool,
    keybinds_enabled: bool,
    undo_stack: Vec<Vec<Zone>>,
    redo_stack: Vec<Vec<Zone>>,
    thumb_cache: std::collections::HashMap<String, (u32, u32, Vec<u8>)>, // path → (w,h,rgba)
    lang: i18n::Lang,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ResizeEdge { N, S, E, W, NE, NW, SE, SW }

#[derive(Debug, Clone)]
struct ResizeDrag {
    zone_idx: usize,
    child_idx: Option<usize>, // None = parent, Some(idx) = child of zones[zone_idx]
    edge: ResizeEdge,
    orig_x: i32, orig_y: i32, orig_w: i32, orig_h: i32,
    start_dx: i32, start_dy: i32,  // desktop coords at drag start
}

#[derive(Debug, Clone)]
struct ZoneDrag {
    zone_idx: usize,
    child_idx: Option<usize>,
    orig_x: i32, orig_y: i32,
    start_dx: i32, start_dy: i32,
}

impl App {
    fn new() -> Self {
        let lang = Lang::default();
        let monitors = get_monitors(lang);
        let cur = monitors.iter().position(|m| m.primary).unwrap_or(0);
        let m = &monitors[cur];
        let zones = load_config(cur, m.w, m.h);
        println!("[main] 启动: {} 个显示器, 主显示器=第{}个, sel_mons=[{}]", monitors.len(), cur, cur);
        for (i, mon) in monitors.iter().enumerate() {
            println!("[main]   显示器[{}]: '{}' origin=({},{}) {}x{} primary={}", 
                i, mon.name, mon.x, mon.y, mon.w, mon.h, mon.primary);
        }
        Self {
            mon_x: m.x, mon_y: m.y, mon_w: m.w, mon_h: m.h,
            monitors, cur_mon: cur, sel_mons: vec![cur], zones,
            sel_zone: -1,
            sel_child: None,
            drawing: None, resize_drag: None, zone_drag: None, tool: ShapeTool::Rect,
            edit_name: String::new(), edit_types: String::new(),
            edit_spx: String::from("80"), edit_spy: String::from("80"),
            edit_enabled: true,
            edit_x: String::new(), edit_y: String::new(),
            edit_w: String::new(), edit_h: String::new(),
            edit_sort_mode: String::from("none"),
            edit_is_other: false,
            edit_shape: ShapeMode::Rectangle,
            edit_shape_text: String::new(),
            shape_spacing: 1.0,
            grid_padding: 30,
            status: String::new(),
            preview_icons: Vec::new(),
            preview_dirty: true,
            keybinds_enabled: true,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            thumb_cache: std::collections::HashMap::new(),
            lang,
        }
    }

    fn tr<'a>(&self, key: &'a str) -> &'a str { t(self.lang, key) }

    fn trf1(&self, key: &str, a: impl std::fmt::Display) -> String {
        self.tr(key).replacen("{}", &a.to_string(), 1)
    }
    fn trf2(&self, key: &str, a: impl std::fmt::Display, b: impl std::fmt::Display) -> String {
        self.tr(key).replacen("{}", &a.to_string(), 1)
                   .replacen("{}", &b.to_string(), 1)
    }
    fn trf3(&self, key: &str, a: impl std::fmt::Display, b: impl std::fmt::Display, c: impl std::fmt::Display) -> String {
        self.tr(key).replacen("{}", &a.to_string(), 1)
                   .replacen("{}", &b.to_string(), 1)
                   .replacen("{}", &c.to_string(), 1)
    }
    fn trf4(&self, key: &str, a: impl std::fmt::Display, b: impl std::fmt::Display, c: impl std::fmt::Display, d: impl std::fmt::Display) -> String {
        self.tr(key).replacen("{}", &a.to_string(), 1)
                   .replacen("{}", &b.to_string(), 1)
                   .replacen("{}", &c.to_string(), 1)
                   .replacen("{}", &d.to_string(), 1)
    }
    fn trf5(&self, key: &str, a: impl std::fmt::Display, b: impl std::fmt::Display, c: impl std::fmt::Display, d: impl std::fmt::Display, e: impl std::fmt::Display) -> String {
        self.tr(key).replacen("{}", &a.to_string(), 1)
                   .replacen("{}", &b.to_string(), 1)
                   .replacen("{}", &c.to_string(), 1)
                   .replacen("{}", &d.to_string(), 1)
                   .replacen("{}", &e.to_string(), 1)
    }

    /// Switch the canvas view to a different monitor (save current zones first)
    fn switch_to_monitor(&mut self, idx: usize) {
        if idx == self.cur_mon || idx >= self.monitors.len() { return; }
        self.save_editor();
        save_config(&self.zones, self.cur_mon);
        self.cur_mon = idx;
        let m = &self.monitors[idx];
        self.mon_x = m.x; self.mon_y = m.y;
        self.mon_w = m.w; self.mon_h = m.h;
        self.zones = load_config(idx, m.w, m.h);
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.sel_zone = -1;
        self.load_editor(-1);
        self.preview_dirty = true;
    }

    fn refresh_preview(&mut self) {
        self.preview_icons.clear();
        self.thumb_cache.clear();
        if self.mon_w <= 0 || self.mon_h <= 0 { return; }
        match unsafe { desktop::classify_icons_for_monitor(self.mon_x, self.mon_y) } {
            Ok(icons) => {
                // Pre-extract icons for all classified entries
                for icon in &icons {
                    if let Some(ref path) = icon.full_path {
                        if let Some(rgba) = icons::extract_icon_rgba(path) {
                            self.thumb_cache.insert(path.clone(), rgba);
                        }
                    }
                }
                self.preview_icons = icons;
            }
            Err(e) => self.status = self.trf1("preview_fail", &e),
        }
        self.preview_dirty = false;
    }

    fn load_editor(&mut self, idx: i32) {
        self.preview_dirty = true;
        self.sel_child = None;
        if idx < 0 || idx >= self.zones.len() as i32 {
            self.edit_name.clear(); self.edit_types.clear();
            self.edit_spx = String::from("80"); self.edit_spy = String::from("80");
            self.edit_enabled = true;
            self.edit_x.clear(); self.edit_y.clear();
            self.edit_w.clear(); self.edit_h.clear();
            self.edit_sort_mode = "none".to_string();
            self.edit_is_other = false;
            self.edit_shape = ShapeMode::Rectangle;
            self.edit_shape_text.clear();
            return;
        }
        let z = &self.zones[idx as usize];
        self.edit_name = z.name.clone();
        self.edit_types = z.file_types.join(", ");
        self.edit_spx = z.icon_spacing_x.to_string();
        self.edit_spy = z.icon_spacing_y.to_string();
        self.edit_enabled = z.enabled;
        self.edit_x = z.x.to_string();
        self.edit_y = z.y.to_string();
        self.edit_w = z.width.to_string();
        self.edit_h = z.height.to_string();
        self.edit_sort_mode = z.sort_mode.clone();
        self.edit_is_other = z.is_other;
        self.edit_shape = z.shape.clone();
        self.edit_shape_text = z.shape_text.clone();
    }

    fn load_child_editor(&mut self, ci: usize) {
        let zi = self.sel_zone as usize;
        if zi >= self.zones.len() { return; }
        if let Some(c) = self.zones[zi].children.get(ci) {
            self.sel_child = Some(ci);
            self.preview_dirty = true;
            self.edit_name = c.name.clone();
            self.edit_types = c.file_types.join(", ");
            self.edit_spx = c.icon_spacing_x.to_string();
            self.edit_spy = c.icon_spacing_y.to_string();
            self.edit_enabled = c.enabled;
            self.edit_x = c.x.to_string();
            self.edit_y = c.y.to_string();
            self.edit_w = c.width.to_string();
            self.edit_h = c.height.to_string();
            self.edit_sort_mode = c.sort_mode.clone();
            self.edit_is_other = false; // children can't be catch-all
            self.edit_shape = c.shape.clone();
            self.edit_shape_text = c.shape_text.clone();
        }
    }

    fn save_editor(&mut self) {
        if self.sel_zone < 0 || self.sel_zone as usize >= self.zones.len() { return; }
        let name = if self.edit_name.is_empty() { self.trf1("zone_default", self.sel_zone + 1) } else { self.edit_name.clone() };
        let types: Vec<String> = self.edit_types.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
        let spx = self.edit_spx.parse().unwrap_or(80);
        let spy = self.edit_spy.parse().unwrap_or(80);
        let en = self.edit_enabled;
        let sm = self.edit_sort_mode.clone();
        // Get current values for diff
        let (cur_name, cur_types, cur_spx, cur_spy, cur_en, cur_ex, cur_ey, cur_ew, cur_eh, cur_sm, cur_other, cur_shape, cur_shape_text) = if let Some(ci) = self.sel_child {
            let zi = self.sel_zone as usize;
            if let Some(c) = self.zones[zi].children.get(ci) {
                (c.name.clone(), c.file_types.clone(), c.icon_spacing_x, c.icon_spacing_y,
                 c.enabled, c.x, c.y, c.width, c.height, c.sort_mode.clone(), false,
                 c.shape.clone(), c.shape_text.clone())
            } else { return; }
        } else {
            let z = &self.zones[self.sel_zone as usize];
            (z.name.clone(), z.file_types.clone(), z.icon_spacing_x, z.icon_spacing_y,
             z.enabled, z.x, z.y, z.width, z.height, z.sort_mode.clone(), z.is_other,
             z.shape.clone(), z.shape_text.clone())
        };
        let ex: i32 = self.edit_x.parse().unwrap_or(cur_ex);
        let ey: i32 = self.edit_y.parse().unwrap_or(cur_ey);
        let ew: i32 = self.edit_w.parse().unwrap_or(cur_ew);
        let eh: i32 = self.edit_h.parse().unwrap_or(cur_eh);
        let other = if self.sel_child.is_some() { false } else { self.edit_is_other };
        let changed = cur_name != name || cur_types != types
            || cur_spx != spx || cur_spy != spy
            || cur_en != en || cur_ex != ex || cur_ey != ey
            || cur_ew != ew || cur_eh != eh
            || cur_sm != sm || cur_other != other
            || cur_shape != self.edit_shape || cur_shape_text != self.edit_shape_text;
        if changed {
            self.push_undo();
        }
        if let Some(ci) = self.sel_child {
            let zi = self.sel_zone as usize;
            if let Some(c) = self.zones[zi].children.get_mut(ci) {
                c.name = name;
                c.file_types = types;
                c.icon_spacing_x = spx;
                c.icon_spacing_y = spy;
                c.enabled = en;
                c.x = ex;
                c.y = ey;
                c.width = ew;
                c.height = eh;
                c.sort_mode = sm;
                c.shape = self.edit_shape.clone();
                c.shape_text = self.edit_shape_text.clone();
            }
        } else {
            let zi = self.sel_zone as usize;
            let z = &mut self.zones[zi];
            z.name = name;
            z.file_types = types;
            z.icon_spacing_x = spx;
            z.icon_spacing_y = spy;
            z.enabled = en;
            z.x = ex;
            z.y = ey;
            z.width = ew;
            z.height = eh;
            z.sort_mode = sm;
            z.shape = self.edit_shape.clone();
            z.shape_text = self.edit_shape_text.clone();
            let make_other = other;
            z.is_other = make_other;
            if make_other {
                for (i, oz) in self.zones.iter_mut().enumerate() {
                    if i != zi {
                        oz.is_other = false;
                    }
                }
            }
        }
    }

    fn canvas_to_desktop(&self, p: Pos2, canvas_rect: Rect) -> Option<(i32, i32)> {
        let sx = canvas_rect.width() / self.mon_w as f32;
        let sy = canvas_rect.height() / self.mon_h as f32;
        let sc = sx.min(sy);
        let ox = (canvas_rect.width() - self.mon_w as f32 * sc) * 0.5;
        let oy = (canvas_rect.height() - self.mon_h as f32 * sc) * 0.5;
        let dx = ((p.x - canvas_rect.min.x - ox) / sc) as i32;
        let dy = ((p.y - canvas_rect.min.y - oy) / sc) as i32;
        if dx < 0 || dy < 0 || dx > self.mon_w || dy > self.mon_h { None } else { Some((dx, dy)) }
    }

    fn snap_to_grid(x: i32, y: i32) -> (i32, i32) {
        let sx = ((x as f32 / GRID_SNAP as f32).round() * GRID_SNAP as f32) as i32;
        let sy = ((y as f32 / GRID_SNAP as f32).round() * GRID_SNAP as f32) as i32;
        (sx.max(0), sy.max(0))
    }

    fn desktop_to_canvas(&self, dx: i32, dy: i32, canvas_rect: Rect) -> Pos2 {
        let sx = canvas_rect.width() / self.mon_w as f32;
        let sy = canvas_rect.height() / self.mon_h as f32;
        let sc = sx.min(sy);
        let ox = (canvas_rect.width() - self.mon_w as f32 * sc) * 0.5;
        let oy = (canvas_rect.height() - self.mon_h as f32 * sc) * 0.5;
        Pos2::new(canvas_rect.min.x + ox + dx as f32 * sc, canvas_rect.min.y + oy + dy as f32 * sc)
    }

    /// Get a reference to the currently selected zone (parent or child)
    fn selected_zone(&self) -> Option<&Zone> {
        let zi = self.sel_zone as usize;
        if zi >= self.zones.len() { return None; }
        if let Some(ci) = self.sel_child {
            self.zones[zi].children.get(ci)
        } else {
            Some(&self.zones[zi])
        }
    }

    /// Get a mutable reference to the currently selected zone
    fn selected_zone_mut(&mut self) -> Option<&mut Zone> {
        let zi = self.sel_zone as usize;
        if zi >= self.zones.len() { return None; }
        if let Some(ci) = self.sel_child {
            self.zones[zi].children.get_mut(ci)
        } else {
            Some(&mut self.zones[zi])
        }
    }

    /// Get the absolute screen position of the selected zone (adding parent offset if child)
    fn selected_zone_abs(&self) -> Option<(i32, i32, i32, i32)> {
        let zi = self.sel_zone as usize;
        if zi >= self.zones.len() { return None; }
        let p = &self.zones[zi];
        if let Some(ci) = self.sel_child {
            let c = p.children.get(ci)?;
            Some((p.x + c.x, p.y + c.y, c.width, c.height))
        } else {
            Some((p.x, p.y, p.width, p.height))
        }
    }

    /// Find which zone (or child) contains the given desktop point.
    /// Returns (zone_index, optional_child_index). Children checked first (on top).
    fn zone_at(&self, dx: i32, dy: i32) -> Option<(usize, Option<usize>)> {
        // Check children first (visual priority)
        for (zi, z) in self.zones.iter().enumerate() {
            if !z.enabled { continue; }
            for (ci, child) in z.children.iter().enumerate() {
                if !child.enabled { continue; }
                let cx = z.x + child.x;
                let cy = z.y + child.y;
                if dx >= cx && dx <= cx + child.width && dy >= cy && dy <= cy + child.height {
                    return Some((zi, Some(ci)));
                }
            }
        }
        // Then check parent zones
        for (zi, z) in self.zones.iter().enumerate() {
            if !z.enabled { continue; }
            if dx >= z.x && dx <= z.x + z.width && dy >= z.y && dy <= z.y + z.height {
                return Some((zi, None));
            }
        }
        None
    }

    /// Check if a point is on the resize edge of a zone (including children)
    fn resize_edge_at(&self, dx: i32, dy: i32) -> Option<(usize, Option<usize>, ResizeEdge)> {
        let edge_dist = 12i32;
        let corner_dist = 24i32;
        // Check children first
        for (zi, z) in self.zones.iter().enumerate() {
            if !z.enabled { continue; }
            for (ci, child) in z.children.iter().enumerate() {
                if !child.enabled { continue; }
                let cx = z.x + child.x;
                let cy = z.y + child.y;
                let cw = child.width;
                let ch = child.height;
                let near_l = (dx - cx).abs() <= edge_dist;
                let near_r = (dx - (cx + cw)).abs() <= edge_dist;
                let near_t = (dy - cy).abs() <= edge_dist;
                let near_b = (dy - (cy + ch)).abs() <= edge_dist;
                if near_l && near_t && (dx - cx).abs() <= corner_dist && (dy - cy).abs() <= corner_dist { return Some((zi, Some(ci), ResizeEdge::NW)); }
                if near_r && near_t && (dx - (cx + cw)).abs() <= corner_dist && (dy - cy).abs() <= corner_dist { return Some((zi, Some(ci), ResizeEdge::NE)); }
                if near_l && near_b && (dx - cx).abs() <= corner_dist && (dy - (cy + ch)).abs() <= corner_dist { return Some((zi, Some(ci), ResizeEdge::SW)); }
                if near_r && near_b && (dx - (cx + cw)).abs() <= corner_dist && (dy - (cy + ch)).abs() <= corner_dist { return Some((zi, Some(ci), ResizeEdge::SE)); }
                if near_l && dy >= cy + edge_dist && dy <= cy + ch - edge_dist { return Some((zi, Some(ci), ResizeEdge::W)); }
                if near_r && dy >= cy + edge_dist && dy <= cy + ch - edge_dist { return Some((zi, Some(ci), ResizeEdge::E)); }
                if near_t && dx >= cx + edge_dist && dx <= cx + cw - edge_dist { return Some((zi, Some(ci), ResizeEdge::N)); }
                if near_b && dx >= cx + edge_dist && dx <= cx + cw - edge_dist { return Some((zi, Some(ci), ResizeEdge::S)); }
            }
        }
        // Then check parent zones
        for (zi, z) in self.zones.iter().enumerate() {
            if !z.enabled { continue; }
            let l = z.x; let r = z.x + z.width;
            let t = z.y; let b = z.y + z.height;
            let near_l = (dx - l).abs() <= edge_dist;
            let near_r = (dx - r).abs() <= edge_dist;
            let near_t = (dy - t).abs() <= edge_dist;
            let near_b = (dy - b).abs() <= edge_dist;
            if near_l && near_t && (dx - l).abs() <= corner_dist && (dy - t).abs() <= corner_dist { return Some((zi, None, ResizeEdge::NW)); }
            if near_r && near_t && (dx - r).abs() <= corner_dist && (dy - t).abs() <= corner_dist { return Some((zi, None, ResizeEdge::NE)); }
            if near_l && near_b && (dx - l).abs() <= corner_dist && (dy - b).abs() <= corner_dist { return Some((zi, None, ResizeEdge::SW)); }
            if near_r && near_b && (dx - r).abs() <= corner_dist && (dy - b).abs() <= corner_dist { return Some((zi, None, ResizeEdge::SE)); }
            if near_l && dy >= t + edge_dist && dy <= b - edge_dist { return Some((zi, None, ResizeEdge::W)); }
            if near_r && dy >= t + edge_dist && dy <= b - edge_dist { return Some((zi, None, ResizeEdge::E)); }
            if near_t && dx >= l + edge_dist && dx <= r - edge_dist { return Some((zi, None, ResizeEdge::N)); }
            if near_b && dx >= l + edge_dist && dx <= r - edge_dist { return Some((zi, None, ResizeEdge::S)); }
            // Inside but not on edge
            if dx >= l && dx <= r && dy >= t && dy <= b { return None; }
        }
        None
    }

    fn draw_shape_preview(&self, painter: &Painter, _canvas_rect: Rect, d: &Drawing) {
        let col = Color32::from_rgb(255, 255, 255);
        match d.tool {
            ShapeTool::Rect | ShapeTool::Ellipse => {
                let min = Pos2::new(d.start.x.min(d.current.x), d.start.y.min(d.current.y));
                let max = Pos2::new(d.start.x.max(d.current.x), d.start.y.max(d.current.y));
                let r = Rect { min, max };
                let stroke = Stroke::new(2.0, col);
                painter.rect_stroke(r, 0.0, stroke, StrokeKind::Middle);
                if d.tool == ShapeTool::Ellipse {
                    let center = r.center();
                    let rx = r.width() * 0.5;
                    let ry = r.height() * 0.5;
                    let n = 48;
                    let pts: Vec<Pos2> = (0..=n).map(|i| {
                        let a = i as f32 * std::f32::consts::TAU / n as f32;
                        Pos2::new(center.x + rx * a.cos(), center.y + ry * a.sin())
                    }).collect();
                    painter.add(Shape::line(pts, stroke));
                }
            }
            ShapeTool::Star => {
                let dx = d.current.x - d.start.x;
                let dy = d.current.y - d.start.y;
                let outer_r = (dx * dx + dy * dy).sqrt();
                let angle = dy.atan2(dx) - std::f32::consts::FRAC_PI_2;
                let inner_r = outer_r * 0.4;
                let verts = Self::star_vertices(d.start, outer_r, inner_r, angle);
                painter.add(Shape::line(verts, Stroke::new(2.0, col)));
            }
            ShapeTool::Polygon => {
                let stroke = Stroke::new(2.0, col);
                let pts: Vec<Pos2> = d.vertices.iter().copied()
                    .chain(std::iter::once(d.current))
                    .collect();
                painter.add(Shape::line(pts, stroke));
                for v in &d.vertices {
                    painter.circle_filled(*v, 3.0, col);
                }
            }
        }
    }

    fn star_vertices(center: Pos2, outer_r: f32, inner_r: f32, angle_off: f32) -> Vec<Pos2> {
        let mut v = Vec::with_capacity(11);
        for i in 0..10 {
            let r = if i % 2 == 0 { outer_r } else { inner_r };
            let a = angle_off + i as f32 * std::f32::consts::PI / 5.0;
            v.push(Pos2::new(center.x + r * a.cos(), center.y + r * a.sin()));
        }
        v.push(v[0]);
        v
    }

    fn nudge_zone(&mut self, dx: i32, dy: i32) {
        if self.sel_zone < 0 || (self.sel_zone as usize) >= self.zones.len() { return; }
        self.push_undo();
        if let Some(ci) = self.sel_child {
            let p = &self.zones[self.sel_zone as usize];
            let pw = p.width;
            let ph = p.height;
            if let Some(c) = self.zones[self.sel_zone as usize].children.get_mut(ci) {
                let nx = (c.x + dx).max(0).min(pw - c.width);
                let ny = (c.y + dy).max(0).min(ph - c.height);
                c.x = nx;
                c.y = ny;
                self.edit_x = c.x.to_string();
                self.edit_y = c.y.to_string();
            }
        } else {
            let z = &mut self.zones[self.sel_zone as usize];
            let nx = (z.x + dx).max(0).min(self.mon_w - z.width);
            let ny = (z.y + dy).max(0).min(self.mon_h - z.height);
            z.x = nx;
            z.y = ny;
            self.edit_x = z.x.to_string();
            self.edit_y = z.y.to_string();
        }
        self.preview_dirty = true;
    }

    fn organize_selected(&mut self) {
        self.save_editor();
        save_config(&self.zones, self.cur_mon);
        let mut msgs = Vec::new();
        let total_mons = self.sel_mons.len();
        for (i, &mon_idx) in self.sel_mons.iter().enumerate() {
            if mon_idx >= self.monitors.len() { continue; }
            let m = &self.monitors[mon_idx];
            self.status = self.trf3("organizing", i + 1, total_mons, &m.name);
            let zones = if mon_idx == self.cur_mon {
                self.zones.clone()
            } else {
                load_config(mon_idx, m.w, m.h)
            };
            let zone_count = zones.iter().filter(|z| z.enabled).count();
            let total_zones = zones.len();
            unsafe {
                match desktop::organize_icons(&zones, m.x, m.y, self.shape_spacing) {
                    Ok(count) => {
                        let child_count: usize = zones.iter().map(|z| z.children.iter().filter(|c| c.enabled).count()).sum();
                        let detail = if child_count > 0 {
                            format!("[{}] ✅ 整理完成 — {} 图标 → {}/{} 区域 ({}/{} 父区域 + {} 子区域)", 
                                m.name, count, zone_count, total_zones, zone_count, total_zones, child_count)
                        } else {
                            format!("[{}] ✅ 整理完成 — {} 图标 → {}/{} 区域", m.name, count, zone_count, total_zones)
                        };
                        msgs.push(detail);
                    }
                    Err(e) => msgs.push(format!("[{}] ❌ 错误：{}", m.name, e)),
                }
            }
        }
        self.status = msgs.join("  |  ");
        self.preview_dirty = true;
    }

    fn push_undo(&mut self) {
        self.undo_stack.push(self.zones.clone());
        // Keep last 50 snapshots
        if self.undo_stack.len() > 50 {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }

    fn undo(&mut self) {
        if let Some(prev) = self.undo_stack.pop() {
            self.redo_stack.push(self.zones.clone());
            self.zones = prev;
            self.sel_zone = -1;
            self.load_editor(-1);
            self.preview_dirty = true;
            self.status = self.tr("undo_status").to_string();
        }
    }

    fn redo(&mut self) {
        if let Some(next) = self.redo_stack.pop() {
            self.undo_stack.push(self.zones.clone());
            self.zones = next;
            self.sel_zone = -1;
            self.load_editor(-1);
            self.preview_dirty = true;
            self.status = self.tr("redo_status").to_string();
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        let mon_w = self.mon_w;
        let mon_h = self.mon_h;

        // ── Status panel (top, above toolbar) ──
        egui::TopBottomPanel::top("status").min_height(0.0).show(ctx, |ui| {
            if !self.status.is_empty() {
                ui.horizontal(|ui| {
                    ui.colored_label(Color32::from_rgb(255, 215, 0), "●");
                    ui.colored_label(Color32::WHITE, &self.status);
                });
            }
        });

        egui::TopBottomPanel::top("toolbar").min_height(0.0).show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(self.tr("monitor_label"));
                let names: Vec<String> = self.monitors.iter().map(|m| m.name.clone()).collect();
                egui::ComboBox::from_id_salt("mon")
                    .selected_text(&names[self.cur_mon])
                    .show_ui(ui, |ui| {
                        for (i, n) in names.iter().enumerate() {
                            let mut checked = self.sel_mons.contains(&i);
                            if ui.checkbox(&mut checked, n).changed() {
                                println!("[main] checkbox 显示器[{}] '{}': {} -> {}", i, n, !checked, checked);
                                if checked {
                                    if !self.sel_mons.contains(&i) {
                                        self.sel_mons.push(i);
                                        self.sel_mons.sort();
                                        println!("[main]   sel_mons 新增 {}, 现在 = {:?}", i, self.sel_mons);
                                    }
                                    self.switch_to_monitor(i);
                                } else {
                                    self.sel_mons.retain(|&x| x != i);
                                    println!("[main]   sel_mons 移除 {}, 现在 = {:?}", i, self.sel_mons);
                                    if self.cur_mon == i && !self.sel_mons.is_empty() {
                                        self.switch_to_monitor(self.sel_mons[0]);
                                    }
                                }
                            }
                        }
                    });

                ui.separator();
                ui.label(self.tr("shape_label"));
                let current_tool = self.tool;
                for tool_item in &[ShapeTool::Rect, ShapeTool::Ellipse, ShapeTool::Star, ShapeTool::Polygon] {
                    let sl = current_tool == *tool_item;
                    if ui.selectable_label(sl, format!("{} {}", tool_item.name(), t(self.lang, tool_item.label()))).clicked() {
                        self.tool = *tool_item;
                    }
                }

                ui.separator();
                ui.label(self.tr("grid_label"));
                let densities: &[(&str, i32)] = &[
                    ("density_compact", 10), ("density_standard", 30), ("density_loose", 60), ("density_wide", 100)
                ];
                for &(key, pad) in densities {
                    if ui.selectable_label(self.grid_padding == pad, self.tr(key)).clicked() {
                        self.grid_padding = pad;
                        self.zones = default_zones_with_padding(self.mon_w, self.mon_h, pad);
                        self.sel_zone = -1;
                        self.load_editor(-1);
                        self.status = self.trf2("density_status", &self.tr(key), pad);
                    }
                }

                ui.separator();
                ui.label(self.tr("spacing_label"));
                if ui.add(egui::Slider::new(&mut self.shape_spacing, 0.5..=4.0).step_by(0.1).text("x")).changed() {
                    self.preview_dirty = true;
                }
                ui.label(format!("{:.1}x", self.shape_spacing));

                ui.label(self.tr("language"));
                egui::ComboBox::from_id_salt("lang_sel")
                    .selected_text(Lang::label(&self.lang))
                    .show_ui(ui, |ui| {
                        for &l in Lang::all() {
                            ui.selectable_value(&mut self.lang, l, l.label());
                        }
                    });
                if ui.button(self.tr("add_btn")).clicked() {
                    self.save_editor();
                    let (nx, ny) = if self.sel_zone >= 0 && (self.sel_zone as usize) < self.zones.len() {
                        let sz = &self.zones[self.sel_zone as usize];
                        let right = sz.x + sz.width + 10;
                        if right + 200 <= self.mon_w {
                            (right, sz.y)
                        } else {
                            let below = sz.y + sz.height + 10;
                            if below + 200 <= self.mon_h {
                                (sz.x, below)
                            } else {
                                (10, 10)
                            }
                        }
                    } else {
                        (self.mon_w / 2 - 200, self.mon_h / 2 - 200)
                    };
                    let z = Zone {
                        name: self.trf1("zone_default", self.zones.len() + 1),
                        x: nx, y: ny, width: 400, height: 400,
                        file_types: vec![], icon_spacing_x: 80, icon_spacing_y: 80, enabled: true,
                        sort_mode: "none".to_string(), is_other: false, children: vec![],
                        shape: ShapeMode::Rectangle, shape_text: String::new(),
                    };
                    self.push_undo();
                    self.zones.push(z);
                    self.sel_zone = self.zones.len() as i32 - 1;
                    self.load_editor(self.sel_zone);
                }
                if self.sel_zone >= 0 && ui.button(self.tr("del_btn")).clicked() {
                    self.push_undo();
                    self.zones.remove(self.sel_zone as usize);
                    self.sel_zone = -1;
                    self.load_editor(-1);
                }
                if !self.zones.is_empty() && ui.button(self.tr("clear_btn")).clicked() {
                    self.zones.clear();
                    self.sel_zone = -1;
                    self.load_editor(-1);
                    self.status = self.tr("clear_status").into();
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(self.tr("organize_btn")).clicked() {
                        self.save_editor();
                        save_config(&self.zones, self.cur_mon);
                        println!("[main] sel_mons = {:?}", self.sel_mons);
                        let mut msgs = Vec::new();
                        for &mon_idx in &self.sel_mons {
                            let m = &self.monitors[mon_idx];
                            println!("[main] 处理显示器 {}: name={} origin=({},{})", mon_idx, m.name, m.x, m.y);
                            let zones = if mon_idx == self.cur_mon {
                                self.zones.clone()
                            } else {
                                load_config(mon_idx, m.w, m.h)
                            };
                            println!("[main]   zones count: {}", zones.len());
                            unsafe {
                                match desktop::organize_icons(&zones, m.x, m.y, self.shape_spacing) {
                                    Ok(count) => {
                                        let zone_count = zones.iter().filter(|z| z.enabled).count();
                                        let total_zones = zones.len();
                                        let child_cnt: usize = zones.iter().map(|z| z.children.iter().filter(|c| c.enabled).count()).sum();
                                        let msg = if child_cnt > 0 {
                                            self.trf5("organize_msg", count, zone_count, total_zones, zone_count, child_cnt)
                                        } else {
                                            self.trf4("organize_msg_simple", count, zone_count, total_zones, total_zones)
                                        };
                                        println!("[main]   结果: {}", msg);
                                        msgs.push(format!("[{}] {}", m.name, msg));
                                    }
                                    Err(e) => { println!("[main]   错误: {}", e); msgs.push(format!("[{}] {}", m.name, self.trf1("organize_error", &e))); }
                                }
                            }
                        }
                        self.status = msgs.join("  |  ");
                    }
                    if ui.button(self.tr("save_btn")).clicked() {
                        self.save_editor();
                        save_config(&self.zones, self.cur_mon);
                        self.status = self.tr("save_status").into();
                    }
                    if ui.button(self.tr("reset_btn")).clicked() {
                        self.push_undo();
                        self.zones = default_zones(self.mon_w, self.mon_h);
                        self.sel_zone = -1;
                        self.load_editor(-1);
                        self.status = self.tr("reset_status").into();
                    }
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let avail = ui.available_size();
            let sidebar_w = 320.0;
            let canvas_w = (avail.x - sidebar_w - 10.0).max(200.0);
            let canvas_h = (avail.y * 0.55).max(200.0);

            let (canvas_resp, painter) = ui.allocate_painter(
                vec2(canvas_w, canvas_h),
                egui::Sense::click_and_drag(),
            );
            let cr = canvas_resp.rect;

            painter.rect_filled(cr, 0.0, Color32::from_gray(24));
            let mo = self.desktop_to_canvas(0, 0, cr);
            let me = self.desktop_to_canvas(mon_w, mon_h, cr);
            painter.rect_filled(Rect { min: mo, max: me }, 0.0, Color32::from_gray(42));

            let colors: &[Color32] = &[
                Color32::from_rgb(255, 69, 0), Color32::from_rgb(30, 144, 255),
                Color32::from_rgb(50, 205, 50), Color32::from_rgb(255, 215, 0),
                Color32::from_rgb(147, 112, 219), Color32::from_rgb(255, 127, 80),
                Color32::from_rgb(0, 255, 255), Color32::from_rgb(255, 105, 180),
                Color32::from_rgb(255, 165, 0), Color32::from_rgb(65, 105, 225),
                Color32::from_rgb(0, 255, 127), Color32::from_rgb(255, 255, 0),
                Color32::from_rgb(221, 160, 221), Color32::from_rgb(250, 128, 114),
                Color32::from_rgb(64, 224, 208), Color32::from_rgb(255, 20, 147),
            ];

            for (i, z) in self.zones.iter().enumerate() {
                if !z.enabled { continue; }
                let zmin = self.desktop_to_canvas(z.x, z.y, cr);
                let zmax = self.desktop_to_canvas(z.x + z.width, z.y + z.height, cr);
                let zr = Rect { min: zmin, max: zmax };
                let col = colors[i % colors.len()];
                let fill = Color32::from_rgba_premultiplied(col.r(), col.g(), col.b(), 50);
                painter.rect_filled(zr, 0.0, fill);

                let stroke = if i as i32 == self.sel_zone && self.sel_child.is_none() {
                    Stroke::new(3.0, Color32::WHITE)
                } else {
                    Stroke::new(1.0, col)
                };
                painter.rect_stroke(zr, 0.0, stroke, StrokeKind::Middle);
                // Zone title with shadow for contrast on any background
                let title_font = FontId::proportional(12.0);
                painter.text(
                    zmin + vec2(4.0, 3.0), Align2::LEFT_TOP, &z.name,
                    title_font.clone(), Color32::from_black_alpha(160),
                );
                painter.text(
                    zmin + vec2(3.0, 2.0), Align2::LEFT_TOP, &z.name,
                    title_font, Color32::WHITE,
                );
                let cx = zmin.x + zr.width() * 0.5;
                let cy = zmin.y + zr.height() * 0.5;
                let dash = Stroke::new(1.0, Color32::from_rgba_premultiplied(col.r(), col.g(), col.b(), 30));
                painter.line_segment([pos2(cx, zmin.y), pos2(cx, zmax.y)], dash);
                painter.line_segment([pos2(zmin.x, cy), pos2(zmax.x, cy)], dash);

                // Render children inside parent zone
                for (ci, child) in z.children.iter().enumerate() {
                    if !child.enabled { continue; }
                    let cmin = self.desktop_to_canvas(z.x + child.x, z.y + child.y, cr);
                    let cmax = self.desktop_to_canvas(z.x + child.x + child.width, z.y + child.y + child.height, cr);
                    let cr = Rect { min: cmin, max: cmax };
                    let cfill = Color32::from_rgba_premultiplied(col.r(), col.g(), col.b(), 30);
                    painter.rect_filled(cr, 2.0, cfill);
                    let cstroke = if i as i32 == self.sel_zone && self.sel_child == Some(ci) {
                        Stroke::new(2.0, Color32::WHITE)
                    } else {
                        Stroke::new(1.0, Color32::from_rgba_premultiplied(col.r(), col.g(), col.b(), 120))
                    };
                    // Dashed border for children
                    let dash_stroke = Stroke::new(cstroke.width, cstroke.color);
                    painter.rect_stroke(cr, 2.0, dash_stroke, StrokeKind::Middle);
                    // ">" prefix to distinguish from parent zones
                    let child_label = format!("└ {}", child.name);
                    let child_font = FontId::proportional(10.0);
                    painter.text(
                        cmin + vec2(4.0, 3.0), Align2::LEFT_TOP, &child_label,
                        child_font.clone(), Color32::from_black_alpha(140),
                    );
                    painter.text(
                        cmin + vec2(3.0, 2.0), Align2::LEFT_TOP, &child_label,
                        child_font, Color32::from_rgba_premultiplied(255, 255, 255, 220),
                    );
                }
            }

            if let Some(ref d) = self.drawing {
                self.draw_shape_preview(&painter, cr, d);
            }

            // ── Draw icon thumbnails on canvas ──
            if !self.preview_icons.is_empty() {
                let icon_canvas_w = 18.0;
                let icon_canvas_h = 18.0;
                for (_i, z) in self.zones.iter().enumerate() {
                    if !z.enabled { continue; }
                    let matched: Vec<&desktop::IconClass>;
                    if z.is_other {
                        let non_other: Vec<&crate::config::Zone> = self.zones.iter()
                            .filter(|zz| !zz.is_other && zz.enabled && !zz.file_types.is_empty())
                            .collect();
                        matched = self.preview_icons.iter()
                            .filter(|c| !non_other.iter().any(|zz|
                                zz.file_types.iter().any(|t| t.to_lowercase() == c.category.to_lowercase())
                            ))
                            .collect();
                    } else if z.file_types.is_empty() {
                        continue;
                    } else {
                        matched = self.preview_icons.iter()
                            .filter(|c| z.file_types.iter().any(|t| t.to_lowercase() == c.category.to_lowercase()))
                            .collect();
                    }
                    if matched.is_empty() { continue; }

                    let zmin = self.desktop_to_canvas(z.x, z.y, cr);
                    let zmax = self.desktop_to_canvas(z.x + z.width, z.y + z.height, cr);
                    let name_h = 14.0; // space below zone name

                    // Generate shape-based positions for the matched icons
                    let positions = {
                        let sp = shapes::ShapeParams {
                            icon_count: matched.len(),
                            zone_w: z.width,
                            zone_h: z.height,
                            spacing_x: z.icon_spacing_x,
                            spacing_y: z.icon_spacing_y,
                            spacing_scale: 1.0, // preview at default density
                        };
                        shapes::generate_positions(&z.shape, &z.shape_text, &sp)
                    };

                    for (idx, cls) in matched.iter().enumerate() {
                        if idx >= positions.len() { break; }
                        let (px, py) = positions[idx];
                        // Map shape position (zone-local) → desktop → canvas
                        let cp = self.desktop_to_canvas(z.x + px, z.y + py, cr);
                        let ix = cp.x - icon_canvas_w / 2.0; // center icon on shape position
                        let iy = cp.y - icon_canvas_h / 2.0 + name_h * 0.5;
                        // Clip to zone bounds
                        if ix < zmin.x + 2.0 || iy < zmin.y + name_h
                            || ix + icon_canvas_w > zmax.x - 2.0
                            || iy + icon_canvas_h > zmax.y - 2.0
                        {
                            continue;
                        }
                        let icon_rect = egui::Rect::from_min_size(pos2(ix, iy), vec2(icon_canvas_w, icon_canvas_h));
                        let alpha: u8 = 180;
                        let tint = Color32::from_rgba_unmultiplied(255, 255, 255, alpha);
                        let mut drawn = false;
                        if let Some(ref path) = cls.full_path {
                            if let Some((w, h, pixels)) = self.thumb_cache.get(path.as_str()) {
                                let color_img = egui::ColorImage::from_rgba_unmultiplied(
                                    [*w as usize, *h as usize], pixels,
                                );
                                let tex = ui.ctx().load_texture(
                                    &format!("cv_{}", path),
                                    color_img,
                                    egui::TextureOptions::default(),
                                );
                                let uv = egui::Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
                                painter.image(tex.id(), icon_rect, uv, tint);
                                drawn = true;
                            }
                        }
                        if !drawn {
                            painter.rect_filled(icon_rect, 2.0, Color32::from_rgba_unmultiplied(100, 100, 100, alpha));
                        }
                    }
                }
            }

            // ── Mouse interaction ──
            let pointer = ctx.input(|i| i.pointer.hover_pos());
            let button_down = ctx.input(|i| i.pointer.button_down(egui::PointerButton::Primary));
            let button_just_pressed = ctx.input(|i| i.pointer.button_pressed(egui::PointerButton::Primary));
            let button_just_released = ctx.input(|i| i.pointer.button_released(egui::PointerButton::Primary));
            let mut in_resize = false;

            if let Some(pos) = pointer {
                if cr.contains(pos) {
                    let dp = self.canvas_to_desktop(pos, cr);

                    // ── Resize: live dimension update while dragging ──
                    if self.resize_drag.is_some() {
                        let rd = self.resize_drag.as_ref().unwrap();
                        if button_down {
                            if let Some((dx, dy)) = dp {
                                let delta_x = dx - rd.start_dx;
                                let delta_y = dy - rd.start_dy;
                                let mut nx = rd.orig_x;
                                let mut ny = rd.orig_y;
                                let mut nw = rd.orig_w;
                                let mut nh = rd.orig_h;
                                let min_w = GRID_SNAP;
                                let min_h = GRID_SNAP;
                                match rd.edge {
                                    ResizeEdge::E  => { nw = (rd.orig_w + delta_x).max(min_w); }
                                    ResizeEdge::W  => { nw = (rd.orig_w - delta_x).max(min_w); nx = rd.orig_x + rd.orig_w - nw; }
                                    ResizeEdge::S  => { nh = (rd.orig_h + delta_y).max(min_h); }
                                    ResizeEdge::N  => { nh = (rd.orig_h - delta_y).max(min_h); ny = rd.orig_y + rd.orig_h - nh; }
                                    ResizeEdge::NE => { nw = (rd.orig_w + delta_x).max(min_w); nh = (rd.orig_h - delta_y).max(min_h); ny = rd.orig_y + rd.orig_h - nh; }
                                    ResizeEdge::NW => { nw = (rd.orig_w - delta_x).max(min_w); nx = rd.orig_x + rd.orig_w - nw; nh = (rd.orig_h - delta_y).max(min_h); ny = rd.orig_y + rd.orig_h - nh; }
                                    ResizeEdge::SE => { nw = (rd.orig_w + delta_x).max(min_w); nh = (rd.orig_h + delta_y).max(min_h); }
                                    ResizeEdge::SW => { nw = (rd.orig_w - delta_x).max(min_w); nx = rd.orig_x + rd.orig_w - nw; nh = (rd.orig_h + delta_y).max(min_h); }
                                }
                                if rd.zone_idx < self.zones.len() {
                                    if let Some(ci) = rd.child_idx {
                                        let (pw, ph) = {
                                            let p = &self.zones[rd.zone_idx];
                                            (p.width, p.height)
                                        };
                                        if let Some(c) = self.zones[rd.zone_idx].children.get_mut(ci) {
                                            c.x = nx.max(0).min(pw - nw);
                                            c.y = ny.max(0).min(ph - nh);
                                            c.width = nw;
                                            c.height = nh;
                                            self.edit_x = c.x.to_string();
                                            self.edit_y = c.y.to_string();
                                            self.edit_w = c.width.to_string();
                                            self.edit_h = c.height.to_string();
                                        }
                                    } else {
                                        let z = &mut self.zones[rd.zone_idx];
                                        z.x = nx.max(0).min(self.mon_w - nw);
                                        z.y = ny.max(0).min(self.mon_h - nh);
                                        z.width = nw;
                                        z.height = nh;
                                        self.edit_x = z.x.to_string();
                                        self.edit_y = z.y.to_string();
                                        self.edit_w = z.width.to_string();
                                        self.edit_h = z.height.to_string();
                                    }
                                }
                            }
                        }
                        if !button_down {
                            // Released → snap to grid and finalize
                            if let Some(rd) = &self.resize_drag {
                                if rd.zone_idx < self.zones.len() {
                                    if let Some(ci) = rd.child_idx {
                                        if let Some(c) = self.zones[rd.zone_idx].children.get_mut(ci) {
                                            let (sx, sy) = Self::snap_to_grid(c.x, c.y);
                                            let (sex, sey) = Self::snap_to_grid(c.x + c.width, c.y + c.height);
                                            c.x = sx;
                                            c.y = sy;
                                            c.width = (sex - sx).max(GRID_SNAP);
                                            c.height = (sey - sy).max(GRID_SNAP);
                                            self.edit_x = c.x.to_string();
                                            self.edit_y = c.y.to_string();
                                            self.edit_w = c.width.to_string();
                                            self.edit_h = c.height.to_string();
                                        }
                                    } else {
                                        let z = &mut self.zones[rd.zone_idx];
                                        let (sx, sy) = Self::snap_to_grid(z.x, z.y);
                                        let (sex, sey) = Self::snap_to_grid(z.x + z.width, z.y + z.height);
                                        z.x = sx;
                                        z.y = sy;
                                        z.width = (sex - sx).max(GRID_SNAP);
                                        z.height = (sey - sy).max(GRID_SNAP);
                                        self.edit_x = z.x.to_string();
                                        self.edit_y = z.y.to_string();
                                        self.edit_w = z.width.to_string();
                                        self.edit_h = z.height.to_string();
                                    }
                                }
                            }
                            self.resize_drag = None;
                            self.preview_dirty = true;
                        }
                        in_resize = true;
                    }

                    // ── Edge hover / start resize on mouse press (not click/release) ──
                    if !in_resize {
                        if let Some((dx, dy)) = dp {
                            if let Some((zone_idx, child_idx, edge)) = self.resize_edge_at(dx, dy) {
                                let cursor = match edge {
                                    ResizeEdge::N | ResizeEdge::S => egui::CursorIcon::ResizeVertical,
                                    ResizeEdge::E | ResizeEdge::W => egui::CursorIcon::ResizeHorizontal,
                                    ResizeEdge::NE | ResizeEdge::SW => egui::CursorIcon::ResizeNeSw,
                                    ResizeEdge::NW | ResizeEdge::SE => egui::CursorIcon::ResizeNwSe,
                                };
                                ctx.set_cursor_icon(cursor);
                                // Start resize immediately on mouse press (not waiting for click/release)
                                if button_just_pressed {
                                    self.save_editor();
                                    self.sel_zone = zone_idx as i32;
                                    if let Some(ci) = child_idx {
                                        self.load_child_editor(ci);
                                    } else {
                                        self.load_editor(zone_idx as i32);
                                    }
                                    self.drawing = None;
                                    self.push_undo();
                                    let (ox, oy, ow, oh) = if let Some(ci) = child_idx {
                                        let p = &self.zones[zone_idx];
                                        let c = &p.children[ci]; // child_idx guaranteed valid from resize_edge_at
                                        (c.x, c.y, c.width, c.height)
                                    } else {
                                        let z = &self.zones[zone_idx];
                                        (z.x, z.y, z.width, z.height)
                                    };
                                    self.resize_drag = Some(ResizeDrag {
                                        zone_idx, child_idx, edge,
                                        orig_x: ox, orig_y: oy, orig_w: ow, orig_h: oh,
                                        start_dx: dx, start_dy: dy,
                                    });
                                    in_resize = true;
                                }
                            }
                        }
                    }

                    // ── Zone drag (move whole zone) ──
                    if !in_resize {
                        if self.zone_drag.is_some() {
                            if button_down {
                                if let Some((dx, dy)) = dp {
                                    let zd = self.zone_drag.as_ref().unwrap();
                                    let delta_x = dx - zd.start_dx;
                                    let delta_y = dy - zd.start_dy;
                                    let nx = zd.orig_x + delta_x;
                                    let ny = zd.orig_y + delta_y;
                                    if let Some(ci) = zd.child_idx {
                                        let (pw, ph) = {
                                            let p = &self.zones[zd.zone_idx];
                                            (p.width, p.height)
                                        };
                                        if let Some(c) = self.zones[zd.zone_idx].children.get_mut(ci) {
                                            c.x = nx.max(0).min(pw - c.width);
                                            c.y = ny.max(0).min(ph - c.height);
                                            self.edit_x = c.x.to_string();
                                            self.edit_y = c.y.to_string();
                                        }
                                    } else {
                                        let z = &mut self.zones[zd.zone_idx];
                                        z.x = nx.max(0).min(self.mon_w - z.width);
                                        z.y = ny.max(0).min(self.mon_h - z.height);
                                        self.edit_x = z.x.to_string();
                                        self.edit_y = z.y.to_string();
                                    }
                                }
                            } else {
                                // Released → snap and finalize
                                if let Some(zd) = &self.zone_drag {
                                    if let Some(ci) = zd.child_idx {
                                        if let Some(c) = self.zones[zd.zone_idx].children.get_mut(ci) {
                                            let (sx, sy) = Self::snap_to_grid(c.x, c.y);
                                            c.x = sx;
                                            c.y = sy;
                                            self.edit_x = c.x.to_string();
                                            self.edit_y = c.y.to_string();
                                        }
                                    } else {
                                        let z = &mut self.zones[zd.zone_idx];
                                        let (sx, sy) = Self::snap_to_grid(z.x, z.y);
                                        z.x = sx;
                                        z.y = sy;
                                        self.edit_x = z.x.to_string();
                                        self.edit_y = z.y.to_string();
                                    }
                                }
                                self.zone_drag = None;
                                self.preview_dirty = true;
                            }
                            in_resize = true;
                        } else if button_just_pressed {
                            // Check if clicking inside a zone (not on edge) to start drag
                            if let Some((dx, dy)) = dp {
                                let on_edge = self.resize_edge_at(dx, dy).is_some();
                                if !on_edge {
                                    if let Some((zone_idx, child_idx)) = self.zone_at(dx, dy) {
                                        self.save_editor();
                                        self.sel_zone = zone_idx as i32;
                                        if let Some(ci) = child_idx {
                                            self.load_child_editor(ci);
                                        } else {
                                            self.load_editor(zone_idx as i32);
                                        }
                                        self.drawing = None;
                                        self.push_undo();
                                        let (ox, oy) = if let Some(ci) = child_idx {
                                            let p = &self.zones[zone_idx];
                                            let c = &p.children[ci];
                                            (c.x, c.y)
                                        } else {
                                            let z = &self.zones[zone_idx];
                                            (z.x, z.y)
                                        };
                                        self.zone_drag = Some(ZoneDrag {
                                            zone_idx, child_idx,
                                            orig_x: ox, orig_y: oy,
                                            start_dx: dx, start_dy: dy,
                                        });
                                        in_resize = true;
                                    }
                                }
                            }
                        }
                    }

                    // ── Normal click / drawing (only when NOT resizing) ──
                    if !in_resize && canvas_resp.clicked() {
                        let click_dp = self.canvas_to_desktop(pos, cr);
                        let mut hit_zone = -1i32;
                        for i in (0..self.zones.len()).rev() {
                            let z = &self.zones[i];
                            if let Some((dx, dy)) = click_dp {
                                if dx >= z.x && dx <= z.x + z.width
                                    && dy >= z.y && dy <= z.y + z.height
                                {
                                    hit_zone = i as i32;
                                    break;
                                }
                            }
                        }
                        if hit_zone >= 0 {
                            self.save_editor();
                            self.sel_zone = hit_zone;
                            self.load_editor(hit_zone);
                            self.drawing = None;
                        } else if let Some((dx, dy)) = click_dp {
                            self.sel_zone = -1;
                            self.load_editor(-1);
                            let sp = self.desktop_to_canvas(dx, dy, cr);
                            self.drawing = Some(Drawing {
                                tool: self.tool,
                                start: sp, current: sp,
                                vertices: if self.tool == ShapeTool::Polygon { vec![sp] } else { vec![] },
                            });
                        }
                    }

                    // Polygon completion / vertex add
                    if !in_resize {
                    let mut close_poly = false;
                    let mut polygon_verts: Option<Vec<Pos2>> = None;
                    if let Some(ref d) = self.drawing {
                        if d.tool == ShapeTool::Polygon {
                            if canvas_resp.double_clicked() || ctx.input(|i| i.pointer.secondary_clicked()) {
                                close_poly = true;
                                polygon_verts = Some(d.vertices.clone());
                            }
                        }
                    }
                    if close_poly {
                        if let Some(verts) = polygon_verts {
                            let mut min_x = f32::MAX; let mut min_y = f32::MAX;
                            let mut max_x = f32::MIN; let mut max_y = f32::MIN;
                            for v in &verts {
                                min_x = min_x.min(v.x); min_y = min_y.min(v.y);
                                max_x = max_x.max(v.x); max_y = max_y.max(v.y);
                            }
                            if let (Some(min_dp), Some(max_dp)) = (
                                self.canvas_to_desktop(pos2(min_x, min_y), cr),
                                self.canvas_to_desktop(pos2(max_x, max_y), cr),
                            ) {
                                let (sx, sy) = Self::snap_to_grid(min_dp.0, min_dp.1);
                                let (ex, ey) = Self::snap_to_grid(max_dp.0, max_dp.1);
                                let w = (ex - sx).max(GRID_SNAP);
                                let h = (ey - sy).max(GRID_SNAP);
                                let z = Zone {
                                    name: self.trf1("zone_default", self.zones.len() + 1),
                                    x: sx, y: sy,
                                    width: w, height: h,
                                    file_types: vec![], icon_spacing_x: 80, icon_spacing_y: 80, enabled: true,
                                    sort_mode: "none".to_string(), is_other: false, children: vec![],
                                    shape: ShapeMode::Rectangle, shape_text: String::new(),
                                };
                                self.zones.push(z);
                                self.sel_zone = self.zones.len() as i32 - 1;
                                self.load_editor(self.sel_zone);
                            }
                            self.drawing = None;
                        }
                    }
                    } // end !in_resize guard

                    // Drag → update preview
                    if !in_resize && button_down {
                        let cp = self.canvas_to_desktop(pos, cr)
                            .map(|(dx, dy)| self.desktop_to_canvas(dx, dy, cr));
                        if let Some(cp) = cp {
                            if let Some(ref mut d) = self.drawing {
                                if d.tool != ShapeTool::Polygon {
                                    d.current = cp;
                                }
                            }
                        }
                    }

                    // Drag stop → create zone
                    if !in_resize && button_just_released && self.drawing.is_some() {
                        let zone_rect = self.drawing.as_ref().and_then(|d| {
                            if d.tool == ShapeTool::Polygon { return None; }
                            let sx = d.start.x.min(d.current.x);
                            let sy = d.start.y.min(d.current.y);
                            let ex = d.start.x.max(d.current.x);
                            let ey = d.start.y.max(d.current.y);
                            match (self.canvas_to_desktop(pos2(sx, sy), cr),
                                   self.canvas_to_desktop(pos2(ex, ey), cr)) {
                                (Some(min_dp), Some(max_dp)) => Some((min_dp, max_dp)),
                                _ => None,
                            }
                        });
                        if let Some((min_dp, max_dp)) = zone_rect {
                            let (sx, sy) = Self::snap_to_grid(min_dp.0, min_dp.1);
                            let (ex, ey) = Self::snap_to_grid(max_dp.0, max_dp.1);
                            let w = (ex - sx).max(GRID_SNAP);
                            let h = (ey - sy).max(GRID_SNAP);
                            let z = Zone {
                                name: self.trf1("zone_default", self.zones.len() + 1),
                                x: sx, y: sy,
                                width: w,
                                height: h,
                                file_types: vec![], icon_spacing_x: 80, icon_spacing_y: 80, enabled: true,
                                sort_mode: "none".to_string(), is_other: false, children: vec![],
                                shape: ShapeMode::Rectangle, shape_text: String::new(),
                            };
                            self.push_undo();
                            self.zones.push(z);
                            self.sel_zone = self.zones.len() as i32 - 1;
                            self.load_editor(self.sel_zone);
                        }
                        self.drawing = None;
                    }
                }
            }

            // ── Zone list + editor (scrollable) ──
            let sidebar_h = ui.available_height() - 10.0;
            egui::ScrollArea::vertical().max_height(sidebar_h).auto_shrink([false; 2]).show(ui, |ui| {
                ui.horizontal(|ui| {
                let mut remove_idx: Option<usize> = None;

                ui.vertical(|ui| {
                    ui.label(self.tr("zone_list_label"));
                    let mut clicked_zone: i32 = -1;
                    egui::ScrollArea::vertical()
                        .max_height(canvas_h * 0.35)
                        .show(ui, |ui| {
                            for (i, z) in self.zones.iter().enumerate() {
                                let sel = i as i32 == self.sel_zone;
                                let label = if z.enabled {
                                    format!("{}  [{},{} {}x{}]", z.name, z.x, z.y, z.width, z.height)
                                } else {
                                    format!("(off) {}  [{},{} {}x{}]", z.name, z.x, z.y, z.width, z.height)
                                };
                                let resp = ui.selectable_label(sel, label);
                                if resp.clicked() {
                                    clicked_zone = i as i32;
                                }
                                resp.context_menu(|ui| {
                                    if ui.button(self.tr("del_btn")).clicked() {
                                        remove_idx = Some(i);
                                        ui.close_menu();
                                    }
                                });
                            }
                        });
                    if clicked_zone >= 0 {
                        self.save_editor();
                        self.sel_zone = clicked_zone;
                        self.load_editor(clicked_zone);
                    }
                    if let Some(i) = remove_idx {
                        self.push_undo();
                        self.zones.remove(i);
                        if self.sel_zone == i as i32 { self.sel_zone = -1; self.load_editor(-1); }
                        else if self.sel_zone > i as i32 { self.sel_zone -= 1; }
                    }
                });

                ui.separator();

                ui.vertical(|ui| {
                    ui.label(self.tr("props_label"));
                    ui.horizontal(|ui| {
                        ui.label(self.tr("name_label")); ui.add_sized([120.0, 20.0], egui::TextEdit::singleline(&mut self.edit_name));
                    });

                    // ── 类型配置：用预设按钮设定区域匹配类型 ──
                    ui.label(self.tr("type_label"));
                    ui.horizontal_wrapped(|ui| {
                        for (name, types) in FILE_TYPE_PRESETS {
                            // Check if current zone already has this type set
                            let current: Vec<&str> = self.edit_types.split(',')
                                .map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
                            let has_type = types.iter().any(|t| current.contains(t))
                                && current.len() == types.len();
                            let pname = preset_name(self.lang, name);
                            let btn = if has_type {
                                egui::Button::new(format!("✓ {}", pname)).fill(Color32::from_rgb(0, 120, 60))
                            } else {
                                egui::Button::new(pname)
                            };
                            if ui.add_sized([80.0, 20.0], btn).clicked() {
                                if has_type {
                                    // Unset: clear types
                                    self.edit_types.clear();
                                } else {
                                    // Set: replace with this preset
                                    self.edit_types = types.join(", ");
                                }
                                self.preview_dirty = true;
                            }
                        }
                    });
                    // Also keep the raw text field for advanced editing
                    ui.horizontal(|ui| {
                        ui.label(self.tr("custom_label"));
                        let custom_hint = self.tr("custom_hint");
                        if ui.add_sized([200.0, 18.0], egui::TextEdit::singleline(&mut self.edit_types).hint_text(custom_hint)).changed() {
                            self.preview_dirty = true;
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label(self.tr("spx_label")); ui.add_sized([40.0, 20.0], egui::TextEdit::singleline(&mut self.edit_spx));
                        ui.label(self.tr("spy_label")); ui.add_sized([40.0, 20.0], egui::TextEdit::singleline(&mut self.edit_spy));
                    });
                    ui.horizontal(|ui| {
                        ui.label(self.tr("pos_x_label")); ui.add_sized([40.0, 20.0], egui::TextEdit::singleline(&mut self.edit_x));
                        ui.label(self.tr("pos_y_label")); ui.add_sized([40.0, 20.0], egui::TextEdit::singleline(&mut self.edit_y));
                        ui.label(self.tr("size_w_label")); ui.add_sized([40.0, 20.0], egui::TextEdit::singleline(&mut self.edit_w));
                        ui.label(self.tr("size_h_label")); ui.add_sized([40.0, 20.0], egui::TextEdit::singleline(&mut self.edit_h));
                    });
                    let enabled_label = self.tr("enabled_label"); ui.checkbox(&mut self.edit_enabled, enabled_label);

                    ui.horizontal(|ui| {
                        ui.label(self.tr("sort_label"));
                        egui::ComboBox::from_id_salt("sort_mode")
                            .selected_text(self.tr(match self.edit_sort_mode.as_str() {
                                "name" => "sort_name",
                                "type" => "sort_type",
                                _ => "sort_none",
                            }))
                            .show_ui(ui, |ui| {
                                {let s=self.tr("sort_none"); ui.selectable_value(&mut self.edit_sort_mode, "none".to_string(), s);}
                                {let s=self.tr("sort_name"); ui.selectable_value(&mut self.edit_sort_mode, "name".to_string(), s);}
                                {let s=self.tr("sort_type"); ui.selectable_value(&mut self.edit_sort_mode, "type".to_string(), s);}
                            });
                    });

                    let other_label = self.tr("other_label"); ui.checkbox(&mut self.edit_is_other, other_label);

                    // ── 形状排列 ──
                    ui.horizontal(|ui| {
                        ui.label(self.tr("shape_layout_label"));
                        egui::ComboBox::from_id_salt("shape_mode")
                            .selected_text(shape_mode_label(self.lang, &self.edit_shape))
                            .show_ui(ui, |ui| {
                                for mode in ShapeMode::all() {
                                    ui.selectable_value(&mut self.edit_shape, mode.clone(), shape_mode_label(self.lang, mode));
                                }
                            });
                    });
                    if self.edit_shape == ShapeMode::Text {
                        ui.horizontal(|ui| {
                            ui.label(self.tr("text_label"));
                            let text_hint = self.tr("text_hint");
                            if ui.add_sized([120.0, 20.0], egui::TextEdit::singleline(&mut self.edit_shape_text).hint_text(text_hint)).changed() {
                                self.preview_dirty = true;
                            }
                        });
                    }

                    // ── 子区域管理 ──
                    if self.sel_zone >= 0 && self.sel_child.is_none() {
                        ui.separator();
                        ui.horizontal(|ui| {
                            ui.label(self.tr("child_label"));
                            if ui.small_button(self.tr("add_child_btn")).clicked() {
                                self.push_undo();
                                let zi = self.sel_zone as usize;
                                let child_num = self.zones[zi].children.len() + 1;
                                let child = Zone {
                                    name: self.trf1("child_default", child_num),
                                    file_types: vec![],
                                    icon_spacing_x: 30,
                                    icon_spacing_y: 30,
                                    enabled: true,
                                    x: 10, y: 10,
                                    width: 80,
                                    height: 80,
                                    sort_mode: "none".to_string(),
                                    is_other: false,
                                    children: vec![],
                                    shape: ShapeMode::Rectangle,
                                    shape_text: String::new(),
                                };
                                self.zones[zi].children.push(child);
                                self.status = self.trf2("add_child_status", child_num, &self.zones[zi].name);
                                self.preview_dirty = true;
                            }
                        });
                        let zi = self.sel_zone as usize;
                        if !self.zones[zi].children.is_empty() {
                            let mut child_clicked: Option<usize> = None;
                            let mut child_remove: Option<usize> = None;
                            egui::ScrollArea::vertical().max_height(120.0).show(ui, |ui| {
                                for (ci, c) in self.zones[zi].children.iter().enumerate() {
                                    let sel = self.sel_child == Some(ci);
                                    let label = if c.enabled {
                                        format!("└ {}  [{},{} {}x{}]", c.name, c.x, c.y, c.width, c.height)
                                    } else {
                                        format!("└ (off) {}  [{},{} {}x{}]", c.name, c.x, c.y, c.width, c.height)
                                    };
                                    let resp = ui.selectable_label(sel, label);
                                    if resp.clicked() {
                                        child_clicked = Some(ci);
                                    }
                                    resp.context_menu(|ui| {
                                        if ui.button(self.tr("del_child_btn")).clicked() {
                                            child_remove = Some(ci);
                                            ui.close_menu();
                                        }
                                    });
                                }
                            });
                            if let Some(ci) = child_clicked {
                                self.save_editor();
                                self.load_child_editor(ci);
                            }
                            if let Some(ci) = child_remove {
                                self.push_undo();
                                self.zones[zi].children.remove(ci);
                                if self.sel_child == Some(ci) { self.sel_child = None; self.load_editor(self.sel_zone); }
                                self.preview_dirty = true;
                            }
                        }
                        // Button to deselect child → go back to parent
                        if self.sel_child.is_some() {
                            if ui.small_button(self.tr("back_parent_btn")).clicked() {
                                self.save_editor();
                                self.sel_child = None;
                                self.load_editor(self.sel_zone);
                            }
                        }
                    }

                    if self.sel_zone >= 0 {
                        self.save_editor();
                    }

                    // ── 图标预览 ──
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label(self.tr("preview_label"));
                        if ui.small_button("🔄").clicked() || self.preview_dirty {
                            self.refresh_preview();
                        }
                    });
                    if !self.preview_icons.is_empty() && self.sel_zone >= 0 && (self.sel_zone as usize) < self.zones.len() {
                        let zone = &self.zones[self.sel_zone as usize];
                        // Determine which icons match this zone
                        let matched: Vec<&desktop::IconClass>;
                        if zone.is_other {
                            // "Other" zone: collect icons that don't match any non-other zone
                            let non_other_zones: Vec<&Zone> = self.zones.iter()
                                .filter(|z| !z.is_other && z.enabled && !z.file_types.is_empty())
                                .collect();
                            matched = self.preview_icons.iter()
                                .filter(|c| !non_other_zones.iter().any(|z|
                                    z.file_types.iter().any(|t| t.to_lowercase() == c.category.to_lowercase())
                                ))
                                .collect();
                        } else if zone.file_types.is_empty() {
                            matched = self.preview_icons.iter().collect();
                        } else {
                            matched = self.preview_icons.iter()
                                .filter(|c| zone.file_types.iter().any(|t| t.to_lowercase() == c.category.to_lowercase()))
                                .collect();
                        }
                        if zone.is_other {
                            ui.label(self.trf1("other_collect", matched.len()));
                        } else {
                            ui.label(self.trf1("match_count", matched.len()));
                        }
                        let thumb_size = 28.0;
                        let cell_w = 48.0;
                        egui::ScrollArea::vertical()
                            .max_height(200.0)
                            .show(ui, |ui| {
                                // Group by category
                                let mut cats: std::collections::BTreeMap<&str, Vec<&desktop::IconClass>> =
                                    std::collections::BTreeMap::new();
                                for cls in &matched {
                                    cats.entry(cls.category.as_str()).or_default().push(cls);
                                }
                                for (cat, icons) in &cats {
                                    ui.label(RichText::new(format!("▸ {}", preset_name(self.lang, cat)))
                                        .size(10.0).color(Color32::from_rgb(140, 140, 140)));
                                    ui.horizontal_wrapped(|ui| {
                                        for cls in icons {
                                            let resp = ui.allocate_ui(
                                                egui::vec2(cell_w, thumb_size + 16.0),
                                                |ui| {
                                                    ui.set_min_size(egui::vec2(cell_w, thumb_size + 16.0));
                                                    let tex_rect = egui::Rect::from_min_size(
                                                        ui.next_widget_position(),
                                                        egui::vec2(thumb_size, thumb_size),
                                                    );
                                                    let mut found = false;
                                                    if let Some(ref path) = cls.full_path {
                                                        if let Some((w, h, pixels)) = self.thumb_cache.get(path) {
                                                            let color_img = egui::ColorImage::from_rgba_unmultiplied(
                                                                [*w as usize, *h as usize], pixels,
                                                            );
                                                            let tex_name = format!("thumb_{}", path);
                                                            let tex = ui.ctx().load_texture(
                                                                &tex_name, color_img,
                                                                egui::TextureOptions::default(),
                                                            );
                                                            let uv = egui::Rect::from_min_max(
                                                                egui::pos2(0.0, 0.0),
                                                                egui::pos2(1.0, 1.0),
                                                            );
                                                            ui.painter()
                                                                .image(tex.id(), tex_rect, uv, Color32::WHITE);
                                                            found = true;
                                                        }
                                                    }
                                                    if !found {
                                                        ui.painter().rect_filled(
                                                            tex_rect, 3.0, Color32::from_gray(55),
                                                        );
                                                        ui.painter().text(
                                                            tex_rect.center(),
                                                            egui::Align2::CENTER_CENTER,
                                                            "?",
                                                            egui::FontId::proportional(12.0),
                                                            Color32::from_gray(120),
                                                        );
                                                    }
                                                    ui.allocate_rect(tex_rect, egui::Sense::hover());
                                                    // Name below icon
                                                    let name_pos = tex_rect.left_bottom() + egui::vec2(-2.0, 2.0);
                                                    ui.painter().text(
                                                        name_pos,
                                                        egui::Align2::LEFT_TOP,
                                                        &cls.name,
                                                        egui::FontId::proportional(8.0),
                                                        Color32::from_rgb(200, 200, 200),
                                                    );
                                                },
                                            );
                                            if resp.response.hovered() {
                                                let layer = ui.layer_id();
                                                egui::show_tooltip_at_pointer(
                                                    ui.ctx(),
                                                    layer,
                                                    egui::Id::new(format!("tt_{}", cls.name)),
                                                    |ui: &mut egui::Ui| {
                                                        ui.label(RichText::new(&cls.name).size(12.0));
                                                        ui.label(RichText::new(self.trf1("category_type", &cls.category)).size(10.0).color(Color32::DARK_GRAY));
                                                    },
                                                );
                                            }
                                        }
                                    });
                                }
                            });
                    } else if self.preview_icons.is_empty() && !self.preview_dirty {
                        ui.colored_label(Color32::DARK_GRAY, self.tr("no_icons"));
                    } else if self.sel_zone < 0 {
                        ui.colored_label(Color32::DARK_GRAY, self.tr("select_zone_for_preview"));
                    }

                    // ── 快捷键 ──
                    ui.separator();
                    let shortcuts_label = self.tr("shortcuts_label"); ui.checkbox(&mut self.keybinds_enabled, shortcuts_label);
                    if self.keybinds_enabled {
                        ui.horizontal(|ui| {
                            if ui.add_enabled(!self.undo_stack.is_empty(), egui::Button::new(self.tr("undo_btn"))).clicked() {
                                self.undo();
                            }
                            if ui.add_enabled(!self.redo_stack.is_empty(), egui::Button::new(self.tr("redo_btn"))).clicked() {
                                self.redo();
                            }
                        });
                        ui.label(RichText::new(self.trf2("undo_depth", self.undo_stack.len(), self.redo_stack.len())).size(10.0).color(Color32::DARK_GRAY));
                        ui.label(RichText::new(self.tr("key_ctrlz")).size(10.0).color(Color32::DARK_GRAY));
                        ui.label(RichText::new(self.tr("key_del")).size(10.0).color(Color32::DARK_GRAY));
                        ui.label(RichText::new(self.tr("key_arrow")).size(10.0).color(Color32::DARK_GRAY));
                        ui.label(RichText::new(self.tr("key_f5")).size(10.0).color(Color32::DARK_GRAY));
                        ui.label(RichText::new(self.tr("key_f2")).size(10.0).color(Color32::DARK_GRAY));
                    }
                });
            });
            }); // ScrollArea::show | ui.horizontal | ui.vertical
        });

        // ── Keyboard shortcuts ──
        if self.keybinds_enabled {
            let key_events: Vec<(egui::Key, egui::Modifiers)> = ctx.input(|i| {
                i.events.iter().filter_map(|e| {
                    if let egui::Event::Key { key, pressed: true, repeat: false, modifiers, .. } = e {
                        Some((key.clone(), *modifiers))
                    } else { None }
                }).collect::<Vec<_>>()
            });
            for (key, mods) in &key_events {
                let ctrl = mods.ctrl;
                // Ctrl+Z → Undo
                if ctrl && *key == egui::Key::Z {
                    self.undo();
                    continue;
                }
                // Ctrl+Y → Redo
                if ctrl && *key == egui::Key::Y {
                    self.redo();
                    continue;
                }
                // (Ctrl+Shift+Z also redo in some apps, same as Ctrl+Y)
                if ctrl && mods.shift && *key == egui::Key::Z {
                    self.redo();
                    continue;
                }
                match key {
                    egui::Key::Delete => {
                        if let Some(ci) = self.sel_child {
                            if self.sel_zone >= 0 {
                                let zi = self.sel_zone as usize;
                                if zi < self.zones.len() && ci < self.zones[zi].children.len() {
                                    self.push_undo();
                                    self.zones[zi].children.remove(ci);
                                    self.status = self.tr("del_child_status").to_string();
                                    self.sel_child = None;
                                    self.load_editor(self.sel_zone);
                                    self.preview_dirty = true;
                                }
                            }
                        } else if self.sel_zone >= 0 && (self.sel_zone as usize) < self.zones.len() {
                            self.push_undo();
                            self.zones.remove(self.sel_zone as usize);
                            self.status = self.tr("del_zone_status").to_string();
                            self.sel_zone = -1;
                            self.sel_child = None;
                            self.load_editor(-1);
                            self.preview_dirty = true;
                        }
                    }
                    egui::Key::ArrowUp => { self.nudge_zone(0, -GRID_SNAP); }
                    egui::Key::ArrowDown => { self.nudge_zone(0, GRID_SNAP); }
                    egui::Key::ArrowLeft => { self.nudge_zone(-GRID_SNAP, 0); }
                    egui::Key::ArrowRight => { self.nudge_zone(GRID_SNAP, 0); }
                    egui::Key::F5 => {
                        self.refresh_preview();
                        self.status = self.tr("refresh_status").to_string();
                    }
                    egui::Key::F2 => {
                        self.organize_selected();
                    }
                    _ => {}
                }
            }
        }

        // Fast repaint during interactive states, slow when idle
        if self.resize_drag.is_some() || self.zone_drag.is_some() || self.drawing.is_some() {
            ctx.request_repaint();
        } else {
            ctx.request_repaint_after(std::time::Duration::from_secs(1));
        }
    }
}

fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    // Strategy: Load fonts in priority order.
    // egui inserts at position 0 so the LAST inserted has highest priority.
    // Malgun Gothic is the primary font: covers Latin + Korean Hangul (full) +
    //   CJK Hanja (broad) — one font handles Korean + Chinese + ASCII.
    // CJK fallback: simhei fills any CJK gaps in Malgun.
    // Devanagari fallback: Segoe UI for Hindi script.
    //
    // Load order (lowest priority first, highest last):
    //   1. deva (Segoe UI)     – lowest priority, only for Devanagari fallback
    //   2. cjk  (simhei/Deng)  – CJK supplement
    //   3. ko   (malgun)       – PRIMARY font (Korean + CJK + Latin)

    let mut load_font = |key: &str, path: &str| -> bool {
        if let Ok(data) = std::fs::read(path) {
            fonts.font_data.insert(
                key.to_owned(),
                std::sync::Arc::new(egui::FontData::from_owned(data)),
            );
            true
        } else {
            false
        }
    };

    // Devanagari fallback: local Noto Sans Devanagari (bundled)
    if !load_font("deva", "assets/NotoSansDevanagari.ttf") {
        if !load_font("deva", r"C:\Windows\Fonts\segoeui.ttf") {
            let _ = load_font("deva", r"C:\Windows\Fonts\tahoma.ttf");
        }
    }

    // 2. CJK supplement
    if !load_font("cjk", r"C:\Windows\Fonts\simhei.ttf") {
        if !load_font("cjk", r"C:\Windows\Fonts\Deng.ttf") {
            let _ = load_font("cjk", r"C:\Windows\Fonts\STXIHEI.TTF");
        }
    }

    // 3. PRIMARY: Malgun Gothic (Korean + CJK Hanja + Latin)
    load_font("ko", r"C:\Windows\Fonts\malgun.ttf");

    // Insert into families — last inserted = highest priority (position 0)
    if let Some(f) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
        if fonts.font_data.contains_key("deva") {
            f.insert(0, "deva".to_owned());
        }
        if fonts.font_data.contains_key("cjk") {
            f.insert(0, "cjk".to_owned());
        }
        if fonts.font_data.contains_key("ko") {
            f.insert(0, "ko".to_owned());
        }
    }
    if let Some(f) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
        if fonts.font_data.contains_key("deva") {
            f.insert(0, "deva".to_owned());
        }
        if fonts.font_data.contains_key("cjk") {
            f.insert(0, "cjk".to_owned());
        }
        if fonts.font_data.contains_key("ko") {
            f.insert(0, "ko".to_owned());
        }
    }

    ctx.set_fonts(fonts);
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_inner_size([1000.0, 700.0])
            .with_title("Desktop Organizer"),
        ..Default::default()
    };
    eframe::run_native(
        "Desktop Organizer",
        options,
        Box::new(|cc| {
            setup_fonts(&cc.egui_ctx);
            Ok(Box::new(App::new()))
        }),
    )
}
