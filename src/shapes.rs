//! Shape-based icon arrangement: heart, circle, star, spiral, sine, etc.
//! Each shape generates (x, y) positions relative to the zone center,
//! scaled to fit within the zone dimensions.

use std::f32::consts::PI;
use crate::config::ShapeMode;

/// Shape generation parameters: zone bounding box and icon count
pub struct ShapeParams {
    pub icon_count: usize,
    pub zone_w: i32,
    pub zone_h: i32,
    pub spacing_x: i32,
    pub spacing_y: i32,
    pub spacing_scale: f32,
}

impl ShapeParams {
    pub fn half_w(&self) -> f32 { (self.zone_w as f32) / 2.0 }
    pub fn half_h(&self) -> f32 { (self.zone_h as f32) / 2.0 }
}

/// Scale positions from zone center by spacing_scale
fn apply_scale(pts: Vec<(i32, i32)>, cx: f32, cy: f32, scale: f32) -> Vec<(i32, i32)> {
    if scale == 1.0 { return pts; }
    pts.into_iter().map(|(x, y)| {
        ((cx + (x as f32 - cx) * scale).round() as i32,
         (cy + (y as f32 - cy) * scale).round() as i32)
    }).collect()
}

/// Force-directed spread: push apart overlapping positions within zone bounds.
/// Returns a new vector with minimum spacing enforced.
pub fn spread_out(
    positions: &[(i32, i32)],
    zone_w: i32,
    zone_h: i32,
    min_dist: f32,
    max_iter: usize,
) -> Vec<(i32, i32)> {
    if positions.len() <= 1 || min_dist <= 0.0 {
        return positions.to_vec();
    }
    let mut pts: Vec<(f32, f32)> = positions.iter()
        .map(|(x, y)| (*x as f32, *y as f32))
        .collect();
    // Simple force-directed: if two points are too close, push them apart
    for _iter in 0..max_iter {
        let mut any_moved = false;
        for i in 0..pts.len() {
            for j in (i + 1)..pts.len() {
                let dx = pts[j].0 - pts[i].0;
                let dy = pts[j].1 - pts[i].1;
                let dist_sq = dx * dx + dy * dy;
                if dist_sq < min_dist * min_dist && dist_sq > 0.0001 {
                    let dist = dist_sq.sqrt();
                    let repel = (min_dist - dist) / dist * 0.48;
                    let rx = dx * repel;
                    let ry = dy * repel;
                    pts[i].0 = (pts[i].0 - rx).max(0.0).min(zone_w as f32 - 1.0);
                    pts[i].1 = (pts[i].1 - ry).max(0.0).min(zone_h as f32 - 1.0);
                    pts[j].0 = (pts[j].0 + rx).max(0.0).min(zone_w as f32 - 1.0);
                    pts[j].1 = (pts[j].1 + ry).max(0.0).min(zone_h as f32 - 1.0);
                    any_moved = true;
                }
            }
        }
        if !any_moved {
            break;
        }
    }
    pts.into_iter().map(|(x, y)| (x.round() as i32, y.round() as i32)).collect()
}

// ── Shape generators ──

pub fn rectangle(p: &ShapeParams) -> Vec<(i32, i32)> {
    let sx = (p.spacing_x as f32 * p.spacing_scale) as i32;
    let sy = (p.spacing_y as f32 * p.spacing_scale) as i32;
    let cols = if sx > 0 { (p.zone_w / sx).max(1) } else { 1 };
    let offset = sx / 2;
    (0..p.icon_count).map(|j| {
        let col = (j as i32) % cols;
        let row = (j as i32) / cols;
        (offset + col * sx, sy / 2 + row * sy)
    }).collect()
}

/// Heart shape: x = 16sin³t, y = 13cost - 5cos2t - 2cos3t - cos4t
pub fn heart(p: &ShapeParams) -> Vec<(i32, i32)> {
    let hw = p.half_w() * 0.8;
    let hh = p.half_h() * 0.85;
    let cx = p.zone_w as f32 / 2.0;
    let cy = p.zone_h as f32 / 2.0 + p.zone_h as f32 * 0.05; // slight downward shift
    let scale_x = hw / 16.5; // max |x| ≈ 16
    let scale_y = hh / 19.0; // max y range ≈ 19

    let pts = param_points(p.icon_count, 0.0, 2.0 * PI, |t| {
        let x = 16.0 * t.sin().powi(3);
        let y = 13.0 * t.cos() - 5.0 * (2.0 * t).cos() - 2.0 * (3.0 * t).cos() - t.cos();
        let y = -y; // flip (math coords → screen coords)
        (cx + x as f32 * scale_x, cy + y as f32 * scale_y)
    });
    apply_scale(pts, cx, cy, p.spacing_scale)
}

/// Circle / ellipse
pub fn circle(p: &ShapeParams) -> Vec<(i32, i32)> {
    let rx = p.half_w() * 0.75;
    let ry = p.half_h() * 0.75;
    let cx = p.zone_w as f32 / 2.0;
    let cy = p.zone_h as f32 / 2.0;

    let pts = param_points(p.icon_count, -PI, PI, |t| {
        (cx + t.cos() as f32 * rx, cy + t.sin() as f32 * ry)
    });
    apply_scale(pts, cx, cy, p.spacing_scale)
}

/// Equilateral triangle (base down, apex up)
pub fn triangle(p: &ShapeParams) -> Vec<(i32, i32)> {
    let hw = p.half_w() * 0.85;
    let hh = p.half_h() * 0.85;
    let cx = p.zone_w as f32 / 2.0;
    let cy = p.zone_h as f32 / 2.0;
    let top_y = cy - hh;
    let bot_y = cy + hh;
    let left_x = cx - hw;
    let right_x = cx + hw;

    let pts: Vec<(i32, i32)> = (0..p.icon_count).map(|j| {
        let t = if p.icon_count == 1 { 0.5 } else { j as f32 / (p.icon_count - 1) as f32 };
        // Concentric layers from center out
        let n = p.icon_count;
        let idx = if n == 1 { 0 } else { (t * (n - 1) as f32) as usize };
        // Barycentric fill
        let u = ((idx as f32 * 1.732).fract() + 0.3).min(1.0);
        let v = ((idx as f32 * 2.236).fract() + 0.3).min(1.0 - u);
        let px = left_x * (1.0 - u - v) as f32 + cx * v + right_x * u;
        let py = bot_y * (1.0 - u - v) as f32 + top_y * u + cy * v;
        (px.round() as i32, py.round() as i32)
    }).collect();
    apply_scale(pts, cx, cy, p.spacing_scale)
}

/// Diamond
pub fn diamond(p: &ShapeParams) -> Vec<(i32, i32)> {
    let hw = p.half_w() * 0.8;
    let hh = p.half_h() * 0.8;
    let cx = p.zone_w as f32 / 2.0;
    let cy = p.zone_h as f32 / 2.0;

    let pts: Vec<(i32, i32)> = (0..p.icon_count).map(|j| {
        let t = if p.icon_count == 1 { 0.5 } else { j as f32 / (p.icon_count - 1) as f32 };
        let py = cy + (t * 2.0 - 1.0) * hh;
        let w_at_y = hw * (1.0 - (t * 2.0 - 1.0).abs());
        let col = (j as f32 * 1.618).fract();
        let px = cx + (col * 2.0 - 1.0) * w_at_y;
        (px.round() as i32, py.round() as i32)
    }).collect();
    apply_scale(pts, cx, cy, p.spacing_scale)
}

/// 5-pointed star
pub fn star(p: &ShapeParams) -> Vec<(i32, i32)> {
    let hw = p.half_w() * 0.75;
    let hh = p.half_h() * 0.75;
    let cx = p.zone_w as f32 / 2.0;
    let cy = p.zone_h as f32 / 2.0;

    // Generate star outline then spiral-fill inward
    let outer_r = hw.min(hh);
    let inner_r = outer_r * 0.38;

    let pts: Vec<(i32, i32)> = (0..p.icon_count).map(|j| {
        let t = if p.icon_count == 1 { 0.0 } else { j as f32 / (p.icon_count - 1) as f32 };
        let r_frac = (t * p.icon_count as f32).sqrt() / (p.icon_count as f32).sqrt();
        let angle = t * 10.0 * PI;
        let px = cx + angle.cos() as f32 * outer_r * r_frac;
        let py = cy + angle.sin() as f32 * outer_r * r_frac;
        // Clamp to star shape by projecting radially
        let dist = ((px - cx).powi(2) + (py - cy).powi(2)).sqrt();
        let ang = (py - cy).atan2(px - cx);
        let star_r = star_radius_at(ang, outer_r, inner_r);
        let scale = if dist > 0.0 { (star_r / dist).min(1.0) } else { 1.0 };
        (cx + (px - cx) * scale, cy + (py - cy) * scale)
    }).map(|(x, y)| (x.round() as i32, y.round() as i32)).collect();
    apply_scale(pts, cx, cy, p.spacing_scale)
}

fn star_radius_at(angle: f32, outer: f32, inner: f32) -> f32 {
    let a = (angle + PI / 2.0 + PI / 10.0).rem_euclid(2.0 * PI);
    let seg = (a / (PI / 5.0)) as i32 % 10;
    if seg % 2 == 0 { outer } else { inner }
}

/// Archimedean spiral
pub fn spiral(p: &ShapeParams) -> Vec<(i32, i32)> {
    let max_r = p.half_w().min(p.half_h()) * 0.85;
    let cx = p.zone_w as f32 / 2.0;
    let cy = p.zone_h as f32 / 2.0;

    let pts: Vec<(i32, i32)> = (0..p.icon_count).map(|j| {
        let t = if p.icon_count == 1 { 0.5 } else { j as f32 / (p.icon_count - 1) as f32 };
        let r = max_r * t.sqrt();
        let a = t * p.icon_count as f32 * 0.6;
        (cx + r * a.cos(), cy + r * a.sin())
    }).map(|(x, y)| (x.round() as i32, y.round() as i32)).collect();
    apply_scale(pts, cx, cy, p.spacing_scale)
}

/// Sine wave: icons placed along a sine curve
pub fn sine(p: &ShapeParams) -> Vec<(i32, i32)> {
    let amp = p.half_h() * 0.7;
    let cx = p.zone_w as f32 / 2.0;
    let cy = p.zone_h as f32 / 2.0;

    let pts: Vec<(i32, i32)> = (0..p.icon_count).map(|j| {
        let t = if p.icon_count == 1 { 0.0 } else {
            (j as f32 / (p.icon_count - 1) as f32) * 2.0 - 1.0
        };
        let px = cx + t * p.half_w() * 0.9;
        let py = cy + (t * 2.5 * PI).sin() * amp;
        (px.round() as i32, py.round() as i32)
    }).collect();
    apply_scale(pts, cx, cy, p.spacing_scale)
}

/// V-shape / arrow (icons placed along a V outline)
pub fn vshape(p: &ShapeParams) -> Vec<(i32, i32)> {
    let hw = p.half_w() * 0.85;
    let hh = p.half_h() * 0.85;
    let cx = p.zone_w as f32 / 2.0;
    let top_y = p.half_h() - hh;
    let bot_y = p.half_h() + hh;

    let half = p.icon_count / 2;
    // left arm then right arm
    let mut pts = Vec::with_capacity(p.icon_count);
    // Left arm: bottom-left → top-center
    for j in 0..=half.min(p.icon_count.saturating_sub(1)) {
        let t = if half == 0 { 0.0 } else { j as f32 / half as f32 };
        pts.push((cx - hw * (1.0 - t), top_y + (bot_y - top_y) * (1.0 - t)));
    }
    // Right arm: top-center → bottom-right
    let remaining = p.icon_count.saturating_sub(pts.len());
    for j in 0..remaining {
        let t = if remaining <= 1 { 1.0 } else { j as f32 / (remaining - 1) as f32 };
        pts.push((cx + hw * t, top_y + (bot_y - top_y) * t));
    }
    let pts: Vec<(i32, i32)> = pts.into_iter().map(|(x, y)| (x.round() as i32, y.round() as i32)).collect();
    let cy = p.zone_h as f32 / 2.0;
    apply_scale(pts, cx, cy, p.spacing_scale)
}

/// Arc / rainbow
pub fn arc(p: &ShapeParams) -> Vec<(i32, i32)> {
    let r = p.half_w().min(p.half_h()) * 0.85;
    let cx = p.zone_w as f32 / 2.0;
    let cy = p.zone_h as f32 / 2.0 + r * 0.3;

    let pts: Vec<(i32, i32)> = (0..p.icon_count).map(|j| {
        let t = if p.icon_count == 1 { 0.5 } else { j as f32 / (p.icon_count - 1) as f32 };
        let a = PI + t * PI; // from π to 2π (left to right via bottom)
        (cx + r * a.cos(), cy + r * a.sin())
    }).map(|(x, y)| (x.round() as i32, y.round() as i32)).collect();
    apply_scale(pts, cx, cy, p.spacing_scale)
}

/// Cross / plus sign
pub fn cross(p: &ShapeParams) -> Vec<(i32, i32)> {
    let hw = p.half_w() * 0.85;
    let hh = p.half_h() * 0.85;
    let cx = p.zone_w as f32 / 2.0;
    let cy = p.zone_h as f32 / 2.0;
    let arm_w = 16.0; // arm half-width in pixels

    let pts: Vec<(i32, i32)> = (0..p.icon_count).map(|j| {
        let t = if p.icon_count == 1 { 0.5 } else { j as f32 / (p.icon_count - 1) as f32 };
        if t < 0.33 {
            // top arm
            let ty = cy - hh * (1.0 - t * 3.0);
            (cx + ((j as f32 * 7.173).fract() * 2.0 - 1.0) * arm_w, ty)
        } else if t < 0.66 {
            // horizontal arm  
            let tx = cx + hw * ((t - 0.33) * 6.0 - 1.0);
            (tx, cy + ((j as f32 * 5.731).fract() * 2.0 - 1.0) * arm_w)
        } else {
            // bottom arm
            let ty = cy + hh * ((t - 0.66) * 3.0);
            (cx + ((j as f32 * 11.371).fract() * 2.0 - 1.0) * arm_w, ty)
        }
    }).map(|(x, y)| (x.round() as i32, y.round() as i32)).collect();
    apply_scale(pts, cx, cy, p.spacing_scale)
}

/// Custom text: render each character as a pixel grid, place icons at pixel positions.
/// Returns None if the text is empty.
pub fn text_shape(text: &str, p: &ShapeParams) -> Option<Vec<(i32, i32)>> {
    if text.is_empty() || p.icon_count == 0 { return None; }
    let glyphs = rasterize_text(text);
    if glyphs.is_empty() { return None; }

    // Scale glyph positions to fit zone
    let (gw, gh) = glyph_bounds(&glyphs);
    let scale_x = p.zone_w as f32 / gw.max(1) as f32;
    let scale_y = p.zone_h as f32 / gh.max(1) as f32;
    let scale = scale_x.min(scale_y) * 0.85;

    let cx = p.zone_w as f32 / 2.0;
    let cy = p.zone_h as f32 / 2.0;
    let ox = cx - gw as f32 * scale / 2.0;
    let oy = cy - gh as f32 * scale / 2.0;

    let pts: Vec<(i32, i32)> = glyphs.iter()
        .map(|&(gx, gy)| {
            ((ox + gx as f32 * scale).round() as i32,
             (oy + gy as f32 * scale).round() as i32)
        })
        .collect();
    let pts = apply_scale(pts, cx, cy, p.spacing_scale);

    // If more icons than glyph pixels, repeat the pattern
    if pts.len() >= p.icon_count {
        Some(pts.into_iter().take(p.icon_count).collect())
    } else {
        let mut out = Vec::with_capacity(p.icon_count);
        let rep = (p.icon_count + pts.len() - 1) / pts.len();
        for _ in 0..rep {
            out.extend(&pts);
        }
        out.truncate(p.icon_count);
        Some(out)
    }
}

// ── Text rasterizer (simple 5×7 bitmap font) ──

/// Crude 5x7 bitmap for ASCII 32-127
fn glyph_bitmap(ch: u8) -> &'static [u8; 7] {
    let c = if (32..=127).contains(&ch) { ch } else { b'?' };
    let idx = (c - 32) as usize;
    &FONT_5X7[idx]
}

fn rasterize_text(text: &str) -> Vec<(u8, u8)> {
    let mut glyphs = Vec::new();
    let mut x_off = 0u8;
    for &b in text.as_bytes() {
        if b == b'\n' { continue; }
        let bm = glyph_bitmap(b);
        for row in 0..7 {
            let bits = bm[row];
            for col in 0..5 {
                if (bits >> (4 - col)) & 1 != 0 {
                    glyphs.push((x_off + col, row as u8));
                }
            }
        }
        x_off += 6; // 5px glyph + 1px gap
    }
    glyphs
}

fn glyph_bounds(glyphs: &[(u8, u8)]) -> (u8, u8) {
    let max_x = glyphs.iter().map(|p| p.0).max().unwrap_or(0);
    let max_y = glyphs.iter().map(|p| p.1).max().unwrap_or(0);
    (max_x + 1, max_y + 1)
}

// ── Utilities ──

/// Generate `n` points evenly distributed along a parametric curve f(t)
fn param_points<F>(n: usize, t0: f32, t1: f32, f: F) -> Vec<(i32, i32)>
where F: Fn(f32) -> (f32, f32)
{
    (0..n).map(|j| {
        let t = if n == 1 { (t0 + t1) / 2.0 } else { t0 + (j as f32 / (n - 1) as f32) * (t1 - t0) };
        let (x, y) = f(t);
        (x.round() as i32, y.round() as i32)
    }).collect()
}

// ── 5×7 bitmap font (ASCII 32-127) ──
#[rustfmt::skip]
static FONT_5X7: [[u8; 7]; 96] = [
    [0x00,0x00,0x00,0x00,0x00,0x00,0x00], // U+0020 (space)
    [0x04,0x04,0x04,0x04,0x00,0x00,0x04], // U+0021 (!)
    [0x0A,0x0A,0x0A,0x00,0x00,0x00,0x00], // U+0022 (")
    [0x0A,0x0A,0x1F,0x0A,0x1F,0x0A,0x0A], // U+0023 (#)
    [0x04,0x0F,0x14,0x0E,0x05,0x1E,0x04], // U+0024 ($)
    [0x18,0x19,0x02,0x04,0x08,0x13,0x03], // U+0025 (%)
    [0x0C,0x12,0x14,0x08,0x15,0x12,0x0D], // U+0026 (&)
    [0x04,0x04,0x04,0x00,0x00,0x00,0x00], // U+0027 (')
    [0x02,0x04,0x08,0x08,0x08,0x04,0x02], // U+0028 (()
    [0x08,0x04,0x02,0x02,0x02,0x04,0x08], // U+0029 ())
    [0x00,0x04,0x15,0x0E,0x15,0x04,0x00], // U+002A (*)
    [0x00,0x04,0x04,0x1F,0x04,0x04,0x00], // U+002B (+)
    [0x00,0x00,0x00,0x00,0x04,0x04,0x08], // U+002C (,)
    [0x00,0x00,0x00,0x1F,0x00,0x00,0x00], // U+002D (-)
    [0x00,0x00,0x00,0x00,0x00,0x0C,0x0C], // U+002E (.)
    [0x00,0x01,0x02,0x04,0x08,0x10,0x00], // U+002F (/)
    [0x0E,0x11,0x13,0x15,0x19,0x11,0x0E], // U+0030 (0)
    [0x04,0x0C,0x04,0x04,0x04,0x04,0x0E], // U+0031 (1)
    [0x0E,0x11,0x01,0x02,0x04,0x08,0x1F], // U+0032 (2)
    [0x0E,0x11,0x01,0x06,0x01,0x11,0x0E], // U+0033 (3)
    [0x02,0x06,0x0A,0x12,0x1F,0x02,0x02], // U+0034 (4)
    [0x1F,0x10,0x1E,0x01,0x01,0x11,0x0E], // U+0035 (5)
    [0x06,0x08,0x10,0x1E,0x11,0x11,0x0E], // U+0036 (6)
    [0x1F,0x01,0x02,0x04,0x08,0x08,0x08], // U+0037 (7)
    [0x0E,0x11,0x11,0x0E,0x11,0x11,0x0E], // U+0038 (8)
    [0x0E,0x11,0x11,0x0F,0x01,0x02,0x0C], // U+0039 (9)
    [0x00,0x00,0x04,0x00,0x00,0x04,0x00], // U+003A (:)
    [0x00,0x00,0x04,0x00,0x00,0x04,0x08], // U+003B (;)
    [0x02,0x04,0x08,0x10,0x08,0x04,0x02], // U+003C (<)
    [0x00,0x00,0x1F,0x00,0x1F,0x00,0x00], // U+003D (=)
    [0x08,0x04,0x02,0x01,0x02,0x04,0x08], // U+003E (>)
    [0x0E,0x11,0x01,0x02,0x04,0x00,0x04], // U+003F (?)
    [0x0E,0x11,0x17,0x15,0x17,0x10,0x0F], // U+0040 (@)
    [0x04,0x0A,0x11,0x11,0x1F,0x11,0x11], // U+0041 (A)
    [0x1E,0x11,0x11,0x1E,0x11,0x11,0x1E], // U+0042 (B)
    [0x0E,0x11,0x10,0x10,0x10,0x11,0x0E], // U+0043 (C)
    [0x1C,0x12,0x11,0x11,0x11,0x12,0x1C], // U+0044 (D)
    [0x1F,0x10,0x10,0x1E,0x10,0x10,0x1F], // U+0045 (E)
    [0x1F,0x10,0x10,0x1E,0x10,0x10,0x10], // U+0046 (F)
    [0x0E,0x11,0x10,0x17,0x11,0x11,0x0F], // U+0047 (G)
    [0x11,0x11,0x11,0x1F,0x11,0x11,0x11], // U+0048 (H)
    [0x0E,0x04,0x04,0x04,0x04,0x04,0x0E], // U+0049 (I)
    [0x07,0x02,0x02,0x02,0x02,0x12,0x0C], // U+004A (J)
    [0x11,0x12,0x14,0x18,0x14,0x12,0x11], // U+004B (K)
    [0x10,0x10,0x10,0x10,0x10,0x10,0x1F], // U+004C (L)
    [0x11,0x1B,0x15,0x15,0x11,0x11,0x11], // U+004D (M)
    [0x11,0x11,0x19,0x15,0x13,0x11,0x11], // U+004E (N)
    [0x0E,0x11,0x11,0x11,0x11,0x11,0x0E], // U+004F (O)
    [0x1E,0x11,0x11,0x1E,0x10,0x10,0x10], // U+0050 (P)
    [0x0E,0x11,0x11,0x11,0x15,0x12,0x0D], // U+0051 (Q)
    [0x1E,0x11,0x11,0x1E,0x14,0x12,0x11], // U+0052 (R)
    [0x0E,0x11,0x10,0x0E,0x01,0x11,0x0E], // U+0053 (S)
    [0x1F,0x04,0x04,0x04,0x04,0x04,0x04], // U+0054 (T)
    [0x11,0x11,0x11,0x11,0x11,0x11,0x0E], // U+0055 (U)
    [0x11,0x11,0x11,0x11,0x0A,0x0A,0x04], // U+0056 (V)
    [0x11,0x11,0x11,0x15,0x15,0x1B,0x11], // U+0057 (W)
    [0x11,0x11,0x0A,0x04,0x0A,0x11,0x11], // U+0058 (X)
    [0x11,0x11,0x0A,0x04,0x04,0x04,0x04], // U+0059 (Y)
    [0x1F,0x01,0x02,0x04,0x08,0x10,0x1F], // U+005A (Z)
    [0x0E,0x08,0x08,0x08,0x08,0x08,0x0E], // U+005B ([)
    [0x00,0x10,0x08,0x04,0x02,0x01,0x00], // U+005C (\)
    [0x0E,0x02,0x02,0x02,0x02,0x02,0x0E], // U+005D (])
    [0x04,0x0A,0x11,0x00,0x00,0x00,0x00], // U+005E (^)
    [0x00,0x00,0x00,0x00,0x00,0x00,0x1F], // U+005F (_)
    [0x08,0x04,0x02,0x00,0x00,0x00,0x00], // U+0060 (`)
    [0x00,0x00,0x0E,0x01,0x0F,0x11,0x0F], // U+0061 (a)
    [0x10,0x10,0x16,0x19,0x11,0x11,0x1E], // U+0062 (b)
    [0x00,0x00,0x0E,0x10,0x10,0x11,0x0E], // U+0063 (c)
    [0x01,0x01,0x0D,0x13,0x11,0x11,0x0F], // U+0064 (d)
    [0x00,0x00,0x0E,0x11,0x1F,0x10,0x0E], // U+0065 (e)
    [0x06,0x09,0x08,0x1C,0x08,0x08,0x08], // U+0066 (f)
    [0x00,0x0F,0x11,0x11,0x0F,0x01,0x0E], // U+0067 (g)
    [0x10,0x10,0x16,0x19,0x11,0x11,0x11], // U+0068 (h)
    [0x04,0x00,0x0C,0x04,0x04,0x04,0x0E], // U+0069 (i)
    [0x02,0x00,0x06,0x02,0x02,0x12,0x0C], // U+006A (j)
    [0x10,0x10,0x12,0x14,0x18,0x14,0x12], // U+006B (k)
    [0x0C,0x04,0x04,0x04,0x04,0x04,0x0E], // U+006C (l)
    [0x00,0x00,0x1A,0x15,0x15,0x15,0x15], // U+006D (m)
    [0x00,0x00,0x16,0x19,0x11,0x11,0x11], // U+006E (n)
    [0x00,0x00,0x0E,0x11,0x11,0x11,0x0E], // U+006F (o)
    [0x00,0x00,0x1E,0x11,0x1E,0x10,0x10], // U+0070 (p)
    [0x00,0x00,0x0D,0x13,0x0F,0x01,0x01], // U+0071 (q)
    [0x00,0x00,0x16,0x19,0x10,0x10,0x10], // U+0072 (r)
    [0x00,0x00,0x0E,0x10,0x0E,0x01,0x1E], // U+0073 (s)
    [0x08,0x08,0x1C,0x08,0x08,0x09,0x06], // U+0074 (t)
    [0x00,0x00,0x11,0x11,0x11,0x13,0x0D], // U+0075 (u)
    [0x00,0x00,0x11,0x11,0x0A,0x0A,0x04], // U+0076 (v)
    [0x00,0x00,0x11,0x11,0x15,0x15,0x0A], // U+0077 (w)
    [0x00,0x00,0x11,0x0A,0x04,0x0A,0x11], // U+0078 (x)
    [0x00,0x00,0x11,0x11,0x0F,0x01,0x0E], // U+0079 (y)
    [0x00,0x00,0x1F,0x02,0x04,0x08,0x1F], // U+007A (z)
    [0x02,0x04,0x04,0x08,0x04,0x04,0x02], // U+007B ({)
    [0x04,0x04,0x04,0x04,0x04,0x04,0x04], // U+007C (|)
    [0x08,0x04,0x04,0x02,0x04,0x04,0x08], // U+007D (})
    [0x00,0x04,0x02,0x1F,0x02,0x04,0x00], // U+007E (~)
    [0x00,0x04,0x08,0x1F,0x08,0x04,0x00], // U+007F (DEL)
];

// ── Preview helper: generate shape outline for canvas rendering ──
pub fn shape_outline_points(
    shape: &str, text: &str, zone_w: i32, zone_h: i32,
) -> Vec<(i32, i32)> {
    let p = ShapeParams { icon_count: 80, zone_w, zone_h, spacing_x: 40, spacing_y: 40, spacing_scale: 1.0 };
    let pts = match shape {
        "heart" => heart(&p),
        "circle" => circle(&p),
        "triangle" => triangle(&p),
        "diamond" => diamond(&p),
        "star" => star(&p),
        "spiral" => spiral(&p),
        "sine" => sine(&p),
        "vshape" => vshape(&p),
        "arc" => arc(&p),
        "cross" => cross(&p),
        "text" => text_shape(text, &p).unwrap_or_default(),
        _ => rectangle(&p),
    };
    pts
}

/// Unified dispatcher: given ShapeMode and ShapeParams, return icon positions
pub fn generate_positions(mode: &ShapeMode, shape_text: &str, p: &ShapeParams) -> Vec<(i32, i32)> {
    let pts = match mode {
        ShapeMode::Rectangle => rectangle(p),
        ShapeMode::Heart => heart(p),
        ShapeMode::Circle => circle(p),
        ShapeMode::Triangle => triangle(p),
        ShapeMode::Diamond => diamond(p),
        ShapeMode::Star => star(p),
        ShapeMode::Spiral => spiral(p),
        ShapeMode::Sine => sine(p),
        ShapeMode::VShape => vshape(p),
        ShapeMode::Arc => arc(p),
        ShapeMode::Cross => cross(p),
        ShapeMode::Text => text_shape(shape_text, p).unwrap_or_else(|| rectangle(p)),
    };
    // Force-directed spread to prevent icon overlap, especially for curve-based shapes
    let min_dist = (p.spacing_x.min(p.spacing_y) as f32 * 0.55).max(20.0).min(120.0);
    let spread = spread_out(&pts, p.zone_w, p.zone_h, min_dist, 5);
    // Clamp all positions to zone bounds as final safety net
    spread.into_iter().map(|(x, y)| (x.max(0).min(p.zone_w - 1), y.max(0).min(p.zone_h - 1))).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_shapes_produce_points() {
        let p = ShapeParams { icon_count: 10, zone_w: 200, zone_h: 200, spacing_x: 40, spacing_y: 40, spacing_scale: 1.0 };
        assert_eq!(rectangle(&p).len(), 10);
        assert_eq!(heart(&p).len(), 10);
        assert_eq!(circle(&p).len(), 10);
        assert_eq!(triangle(&p).len(), 10);
        assert_eq!(diamond(&p).len(), 10);
        assert_eq!(star(&p).len(), 10);
        assert_eq!(spiral(&p).len(), 10);
        assert_eq!(sine(&p).len(), 10);
        assert_eq!(vshape(&p).len(), 10);
        assert_eq!(arc(&p).len(), 10);
        assert_eq!(cross(&p).len(), 10);
    }

    #[test]
    fn text_shape_works() {
        let p = ShapeParams { icon_count: 20, zone_w: 300, zone_h: 100, spacing_x: 30, spacing_y: 30, spacing_scale: 1.0 };
        let pts = text_shape("HI", &p);
        assert!(pts.is_some());
        assert_eq!(pts.unwrap().len(), 20);
    }

    #[test]
    fn edge_cases() {
        let p = ShapeParams { icon_count: 1, zone_w: 50, zone_h: 50, spacing_x: 10, spacing_y: 10, spacing_scale: 1.0 };
        assert_eq!(heart(&p).len(), 1);
        assert_eq!(star(&p).len(), 1);
        assert_eq!(spiral(&p).len(), 1);
    }
}
