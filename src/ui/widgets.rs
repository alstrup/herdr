use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::app::state::Palette;

pub(super) fn render_panel_shell(
    frame: &mut Frame,
    area: Rect,
    border_color: Color,
    bg: Color,
) -> Option<Rect> {
    if area.width < 2 || area.height < 2 {
        return None;
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .border_set(ratatui::symbols::border::PLAIN)
        .style(Style::default().bg(bg));
    let inner = block.inner(area);
    frame.render_widget(Clear, area);
    frame.render_widget(block, area);
    Some(inner)
}

pub(super) fn panel_contrast_fg(p: &Palette) -> Color {
    match p.panel_bg {
        Color::Reset => p.surface_dim,
        color => color,
    }
}

/// Resolve a tab's color: explicit tab color, else explicit workspace
/// color, else (if `auto_assign` is on) a deterministic per-tab color from
/// the Halton/OKLCH algorithm.
pub(crate) fn tab_color(
    app: &crate::app::AppState,
    ws: &crate::workspace::Workspace,
    tab: &crate::workspace::Tab,
) -> Option<Color> {
    if let Some(raw) = tab.color.as_deref() {
        return crate::config::try_parse_color(raw);
    }
    if let Some(raw) = ws.color.as_deref() {
        // Workspace was given an explicit color; tabs without their own
        // color inherit it — matches the manual-color precedence.
        return crate::config::try_parse_color(raw);
    }
    if app.entity_color.auto_assign {
        return Some(auto_color_for(&tab_identity(ws, tab)));
    }
    None
}

/// Resolve a workspace's color: explicit color, else (if `auto_assign` is
/// on) a deterministic auto color.
pub(crate) fn workspace_color(
    app: &crate::app::AppState,
    ws: &crate::workspace::Workspace,
) -> Option<Color> {
    if let Some(raw) = ws.color.as_deref() {
        return crate::config::try_parse_color(raw);
    }
    if app.entity_color.auto_assign {
        return Some(auto_color_for(&workspace_identity(ws)));
    }
    None
}

fn workspace_identity(ws: &crate::workspace::Workspace) -> String {
    // Prefer the user-given name; fall back to a stable label derived from
    // the workspace's identity cwd basename (so /a/foo and /b/foo collide
    // intentionally — same project name → same color).
    if let Some(name) = &ws.custom_name {
        return name.clone();
    }
    crate::workspace::derive_label_from_cwd(&ws.identity_cwd)
}

fn tab_identity(ws: &crate::workspace::Workspace, tab: &crate::workspace::Tab) -> String {
    // The first tab inherits the workspace identity so a fresh workspace
    // looks cohesive — its tab bar and pane bg share one color. Additional
    // tabs compose with their label/number so they're visually distinct.
    if tab.number == 1 {
        return workspace_identity(ws);
    }
    let tab_part = tab
        .custom_name
        .clone()
        .unwrap_or_else(|| tab.number.to_string());
    format!("{}\u{1}{}", workspace_identity(ws), tab_part)
}

/// Deterministic color from an arbitrary string, using FNV-1a → 3D Halton
/// → OKLCH → sRGB. Stable across restarts and machines.
pub(crate) fn auto_color_for(identity: &str) -> Color {
    let index = fnv1a_64(identity.as_bytes());
    color_from_halton_index(index)
}

fn fnv1a_64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &byte in bytes {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100_0000_01b3);
    }
    hash
}

/// Halton sequence value at `index` for the given prime `base`. Produces a
/// quasi-random point in [0, 1) with low discrepancy.
fn halton(mut index: u64, base: u32) -> f32 {
    let base_f = base as f32;
    let mut f = 1.0_f32;
    let mut result = 0.0_f32;
    let base = base as u64;
    while index > 0 {
        f /= base_f;
        result += f * (index % base) as f32;
        index /= base;
    }
    result
}

fn color_from_halton_index(index: u64) -> Color {
    // Coprime bases give the best 3D spread. Small index offsets
    // decorrelate the dimensions so neighboring indices don't share L or C.
    let h_raw = halton(index, 2);
    let c_raw = halton(index.wrapping_add(1), 3);
    let l_raw = halton(index.wrapping_add(3), 5);

    // OKLCH: hue full wheel with a 120° offset so index 0 lands near green;
    // lightness in [0.55, 0.80] for readable mid-range colors; chroma in
    // [0.10, 0.22] so colors stay vivid without going neon.
    let hue = (h_raw * 360.0 + 120.0).rem_euclid(360.0);
    let chroma = 0.10 + c_raw * (0.22 - 0.10);
    let lightness = 0.55 + l_raw * (0.80 - 0.55);

    oklch_to_rgb(lightness, chroma, hue)
}

fn oklch_to_rgb(l: f32, c: f32, h_deg: f32) -> Color {
    let h_rad = h_deg.to_radians();
    let a = c * h_rad.cos();
    let b = c * h_rad.sin();
    oklab_to_rgb(l, a, b)
}

fn oklab_to_rgb(l: f32, a: f32, b: f32) -> Color {
    // Inverse OKLab → linear sRGB matrices from Björn Ottosson.
    let l_ = l + 0.396_337_78 * a + 0.215_803_76 * b;
    let m_ = l - 0.105_561_346 * a - 0.063_854_17 * b;
    let s_ = l - 0.089_484_18 * a - 1.291_485_5 * b;

    let l = l_ * l_ * l_;
    let m = m_ * m_ * m_;
    let s = s_ * s_ * s_;

    let lr = 4.076_741_7 * l - 3.307_711_6 * m + 0.230_969_94 * s;
    let lg = -1.268_438 * l + 2.609_757_4 * m - 0.341_319_4 * s;
    let lb = -0.004_196_086 * l - 0.703_418_57 * m + 1.707_614_7 * s;

    let to_u8 = |x: f32| -> u8 {
        let encoded = linear_to_srgb(x.clamp(0.0, 1.0));
        (encoded.clamp(0.0, 1.0) * 255.0).round() as u8
    };

    Color::Rgb(to_u8(lr), to_u8(lg), to_u8(lb))
}

/// Pick a readable foreground color for text drawn on top of `bg`.
///
/// Uses OKLab lightness (L*) of the background, with proper sRGB gamma
/// decoding, and thresholds at 0.5 — the perceptual midpoint between black
/// and white. This is more reliable than naive BT.601 luma at the
/// boundaries and matches WCAG's intent of contrast judged by human
/// perception rather than raw RGB values.
pub(crate) fn contrast_fg_for(bg: Color, p: &Palette) -> Color {
    let Some((r, g, b)) = srgb_of(bg) else {
        return panel_contrast_fg(p);
    };
    if oklab_lightness(r, g, b) > 0.5 {
        Color::Black
    } else {
        Color::White
    }
}

fn srgb_of(color: Color) -> Option<(u8, u8, u8)> {
    // ANSI named colors render to terminal-dependent RGB; these are the
    // common VGA-style values used for the picker decision. They're
    // approximations, but sufficient for choosing black-or-white text.
    match color {
        Color::Rgb(r, g, b) => Some((r, g, b)),
        Color::Black => Some((0, 0, 0)),
        Color::Red => Some((170, 0, 0)),
        Color::Green => Some((0, 170, 0)),
        Color::Yellow => Some((170, 170, 0)),
        Color::Blue => Some((0, 0, 170)),
        Color::Magenta => Some((170, 0, 170)),
        Color::Cyan => Some((0, 170, 170)),
        Color::Gray => Some((170, 170, 170)),
        Color::DarkGray => Some((85, 85, 85)),
        Color::LightRed => Some((255, 85, 85)),
        Color::LightGreen => Some((85, 255, 85)),
        Color::LightYellow => Some((255, 255, 85)),
        Color::LightBlue => Some((85, 85, 255)),
        Color::LightMagenta => Some((255, 85, 255)),
        Color::LightCyan => Some((85, 255, 255)),
        Color::White => Some((255, 255, 255)),
        Color::Reset | Color::Indexed(_) => None,
    }
}

fn srgb_to_linear(channel: f32) -> f32 {
    if channel <= 0.04045 {
        channel / 12.92
    } else {
        ((channel + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(channel: f32) -> f32 {
    if channel <= 0.003_130_8 {
        channel * 12.92
    } else {
        1.055 * channel.powf(1.0 / 2.4) - 0.055
    }
}

/// Alpha used when tinting a surface with a workspace/tab color while
/// leaving room for text contrast. The active tab background and the pane
/// background share this value so they look like the same surface.
pub(crate) const ENTITY_TINT_ALPHA: f32 = 0.18;

/// Color used as the "base" when tinting a terminal-area surface — the
/// host terminal's reported default background, or a dark fallback if it
/// was never returned.
pub(crate) fn host_terminal_bg(app: &crate::app::AppState) -> Color {
    app.host_terminal_theme
        .background
        .map(|c| Color::Rgb(c.r, c.g, c.b))
        .unwrap_or(Color::Rgb(0, 0, 0))
}

/// Blend `target` toward `base` in linear sRGB so the mix is perceptually
/// uniform. `alpha` is the weight of `target` (0.0 = pure base, 1.0 = pure
/// target). Falls back to `target` if either color is non-RGB.
pub(crate) fn blend_toward(base: Color, target: Color, alpha: f32) -> Color {
    let Some((br, bg, bb)) = srgb_of(base) else {
        return target;
    };
    let Some((tr, tg, tb)) = srgb_of(target) else {
        return target;
    };
    let mix = |b: u8, t: u8| -> u8 {
        let bl = srgb_to_linear(b as f32 / 255.0);
        let tl = srgb_to_linear(t as f32 / 255.0);
        let m = bl * (1.0 - alpha) + tl * alpha;
        (linear_to_srgb(m).clamp(0.0, 1.0) * 255.0).round() as u8
    };
    Color::Rgb(mix(br, tr), mix(bg, tg), mix(bb, tb))
}

/// OKLab L* for an sRGB color. Returns a value in roughly [0.0, 1.0],
/// where 0 is black and 1 is white, perceptually uniform.
fn oklab_lightness(r: u8, g: u8, b: u8) -> f32 {
    let r = srgb_to_linear(r as f32 / 255.0);
    let g = srgb_to_linear(g as f32 / 255.0);
    let b = srgb_to_linear(b as f32 / 255.0);

    let l = 0.412_221_47 * r + 0.536_332_55 * g + 0.051_445_995 * b;
    let m = 0.211_903_5 * r + 0.680_699_5 * g + 0.107_396_96 * b;
    let s = 0.088_302_46 * r + 0.281_718_85 * g + 0.629_978_7 * b;

    let l_ = l.cbrt();
    let m_ = m.cbrt();
    let s_ = s.cbrt();

    0.210_454_26 * l_ + 0.793_617_8 * m_ - 0.004_072_047 * s_
}

#[cfg(test)]
mod color_tests {
    use super::*;

    #[test]
    fn oklab_lightness_anchors() {
        assert!((oklab_lightness(0, 0, 0)).abs() < 1e-3);
        assert!((oklab_lightness(255, 255, 255) - 1.0).abs() < 1e-3);
    }

    #[test]
    fn contrast_picks_black_on_light_backgrounds() {
        let p = Palette::catppuccin();
        assert_eq!(contrast_fg_for(Color::Rgb(255, 255, 255), &p), Color::Black);
        assert_eq!(contrast_fg_for(Color::Rgb(255, 165, 0), &p), Color::Black);
        assert_eq!(contrast_fg_for(Color::Rgb(255, 182, 193), &p), Color::Black);
    }

    #[test]
    fn contrast_picks_white_on_dark_backgrounds() {
        let p = Palette::catppuccin();
        assert_eq!(contrast_fg_for(Color::Rgb(0, 0, 0), &p), Color::White);
        assert_eq!(contrast_fg_for(Color::Rgb(30, 30, 50), &p), Color::White);
        assert_eq!(contrast_fg_for(Color::Rgb(70, 0, 100), &p), Color::White);
    }

    #[test]
    fn contrast_handles_blue_accent_used_by_catppuccin() {
        // Catppuccin's accent (#89b4fa) is light enough to want dark text.
        let p = Palette::catppuccin();
        assert_eq!(contrast_fg_for(Color::Rgb(0x89, 0xb4, 0xfa), &p), Color::Black);
    }

    #[test]
    fn auto_color_is_deterministic() {
        assert_eq!(auto_color_for("api"), auto_color_for("api"));
        assert_eq!(auto_color_for("/some/path"), auto_color_for("/some/path"));
    }

    #[test]
    fn auto_color_differs_per_identity() {
        // Different identities should land on different points in OKLCH —
        // we don't insist on a minimum perceptual distance here, only that
        // the algorithm doesn't collapse to a single output.
        let a = auto_color_for("api");
        let b = auto_color_for("web");
        let c = auto_color_for("infra");
        assert_ne!(a, b);
        assert_ne!(b, c);
        assert_ne!(a, c);
    }

    #[test]
    fn auto_color_produces_in_gamut_srgb() {
        // Each channel must be a valid u8 (i.e. clamp survived the OKLCH
        // → linear sRGB → encode pipeline).
        for name in ["a", "longer-name", "/abs/path/repo", "with spaces", ""] {
            let Color::Rgb(_, _, _) = auto_color_for(name) else {
                panic!("expected RGB color for {name:?}");
            };
        }
    }

    #[test]
    fn halton_base_2_matches_classical_sequence() {
        // Halton(n, 2): 1/2, 1/4, 3/4, 1/8, 5/8, 3/8, 7/8, ...
        let expected = [0.5, 0.25, 0.75, 0.125, 0.625, 0.375, 0.875];
        for (i, &want) in expected.iter().enumerate() {
            let got = halton((i + 1) as u64, 2);
            assert!((got - want).abs() < 1e-6, "halton({}, 2) = {got}, want {want}", i + 1);
        }
    }
}

pub(crate) fn centered_popup_rect(area: Rect, popup_w: u16, popup_h: u16) -> Option<Rect> {
    let popup_w = popup_w.min(area.width.saturating_sub(4));
    let popup_h = popup_h.min(area.height.saturating_sub(2));
    if popup_w < 4 || popup_h < 4 {
        return None;
    }

    let popup_x = area.x + (area.width.saturating_sub(popup_w)) / 2;
    let popup_y = area.y + (area.height.saturating_sub(popup_h)) / 2;
    Some(Rect::new(popup_x, popup_y, popup_w, popup_h))
}

pub(super) fn render_modal_shell(
    frame: &mut Frame,
    area: Rect,
    popup_w: u16,
    popup_h: u16,
    p: &Palette,
) -> Option<Rect> {
    let popup = centered_popup_rect(area, popup_w, popup_h)?;
    render_panel_shell(frame, popup, p.accent, p.panel_bg)
}

pub(super) fn render_modal_header(frame: &mut Frame, area: Rect, title: &str, p: &Palette) {
    let line = Line::from(vec![Span::styled(
        title,
        Style::default().fg(p.text).add_modifier(Modifier::BOLD),
    )]);
    frame.render_widget(Paragraph::new(line), area);
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ModalStackAreas {
    pub header: Rect,
    pub content: Rect,
    pub footer: Option<Rect>,
    pub actions: Option<Rect>,
}

pub(crate) fn modal_stack_areas(
    inner: Rect,
    header_height: u16,
    footer_height: u16,
    actions_height: u16,
    gap: u16,
) -> ModalStackAreas {
    #[derive(Clone, Copy)]
    enum Slot {
        Header,
        Content,
        Footer,
        Actions,
    }

    let mut constraints = Vec::new();
    let mut slots = Vec::new();
    let mut push = |slot: Slot, constraint: Constraint| {
        if !slots.is_empty() {
            constraints.push(Constraint::Length(gap));
        }
        constraints.push(constraint);
        slots.push(slot);
    };

    push(Slot::Header, Constraint::Length(header_height));
    push(Slot::Content, Constraint::Min(0));
    if footer_height > 0 {
        push(Slot::Footer, Constraint::Length(footer_height));
    }
    if actions_height > 0 {
        push(Slot::Actions, Constraint::Length(actions_height));
    }

    let areas = Layout::vertical(constraints).split(inner);
    let mut header = Rect::default();
    let mut content = Rect::default();
    let mut footer = None;
    let mut actions = None;

    for (slot, area) in slots.into_iter().zip(areas.iter().step_by(2).copied()) {
        match slot {
            Slot::Header => header = area,
            Slot::Content => content = area,
            Slot::Footer => footer = Some(area),
            Slot::Actions => actions = Some(area),
        }
    }

    ModalStackAreas {
        header,
        content,
        footer,
        actions,
    }
}

pub(crate) fn action_button_text(hint: Option<&str>, label: &str) -> String {
    match hint {
        Some(hint) => format!(" {hint} {label} "),
        None => format!(" {label} "),
    }
}

pub(crate) fn action_button_width(hint: Option<&str>, label: &str) -> u16 {
    action_button_text(hint, label).chars().count() as u16
}

pub(crate) struct ActionButtonSpec<'a> {
    pub hint: Option<&'a str>,
    pub label: &'a str,
}

pub(crate) fn action_button_row_rects(
    area: Rect,
    buttons: &[ActionButtonSpec<'_>],
    gap: u16,
    row_offset: u16,
) -> Vec<Rect> {
    let widths: Vec<u16> = buttons
        .iter()
        .map(|button| action_button_width(button.hint, button.label))
        .collect();
    centered_button_row(area, &widths, gap, row_offset)
}

pub(super) fn render_action_button(
    frame: &mut Frame,
    rect: Rect,
    hint: Option<&str>,
    label: &str,
    style: Style,
) {
    frame.render_widget(
        Paragraph::new(action_button_text(hint, label))
            .style(style)
            .alignment(Alignment::Center),
        rect,
    );
}

pub(crate) fn render_modal_description(frame: &mut Frame, area: Rect, text: &str, style: Style) {
    frame.render_widget(
        Paragraph::new(format!(" {text}"))
            .style(style)
            .wrap(Wrap { trim: false }),
        area,
    );
}

pub(crate) fn modal_choice_rows(area: Rect, count: usize, row_height: u16) -> Vec<Rect> {
    let mut rows = Vec::with_capacity(count);
    let mut y = area.y;
    for _ in 0..count {
        if y >= area.y + area.height {
            break;
        }
        let remaining = area.y + area.height - y;
        let height = row_height.min(remaining);
        rows.push(Rect::new(area.x, y, area.width, height));
        y = y.saturating_add(row_height);
    }
    rows
}

pub(crate) fn render_modal_choice_list<T>(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    description: &str,
    options: &[(&str, T)],
    current_value: T,
    selected_idx: usize,
    p: &Palette,
    row_height: u16,
) where
    T: Copy + PartialEq,
{
    let [desc_area, _, list_area] = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(1),
        Constraint::Min(2),
    ])
    .areas::<3>(area);

    render_modal_description(
        frame,
        desc_area,
        description,
        Style::default().fg(p.overlay1),
    );

    let rows = modal_choice_rows(list_area, options.len(), row_height);
    for (idx, ((label, value), row)) in options.iter().zip(rows.iter()).enumerate() {
        let is_active = *value == current_value;
        let is_selected = idx == selected_idx;
        let marker = if is_active { " ✓" } else { "" };
        let style = if is_selected {
            Style::default()
                .bg(p.surface0)
                .fg(p.text)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(p.subtext0)
        };
        frame.render_widget(
            Paragraph::new(format!(" {title}: {label}{marker}"))
                .style(style)
                .wrap(Wrap { trim: false }),
            *row,
        );
    }
}

pub(super) fn centered_button_row(
    inner: Rect,
    widths: &[u16],
    gap: u16,
    row_offset: u16,
) -> Vec<Rect> {
    let total_w = widths
        .iter()
        .copied()
        .sum::<u16>()
        .saturating_add(gap.saturating_mul(widths.len().saturating_sub(1) as u16));
    let mut x = inner.x + inner.width.saturating_sub(total_w) / 2;
    let y = inner.y + row_offset.min(inner.height.saturating_sub(1));
    widths
        .iter()
        .map(|w| {
            let rect = Rect::new(
                x,
                y,
                (*w).min(inner.width.saturating_sub(x.saturating_sub(inner.x))),
                1,
            );
            x = x.saturating_add(*w).saturating_add(gap);
            rect
        })
        .collect()
}
