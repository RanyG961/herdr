use super::*;
use crate::config::{TabPillConfig, TabStyleConfig};
use crate::protocol::ClientShellTabStatusSegment;

const TAB_SCROLL_BUTTON_WIDTH: u16 = 3;
const MIN_TAB_STRIP_WIDTH: u16 =
    MIN_TAB_WIDTH + NEW_TAB_WIDTH + TAB_SCROLL_BUTTON_WIDTH.saturating_mul(2);

pub(crate) fn render_tab_bar(
    buffer: &mut Buffer,
    area: Rect,
    snapshot: &ClientShellSnapshot,
    config: &ClientShellConfig,
    tab_scroll: &mut usize,
    reveal_focused_tab: &mut bool,
    tab_drag_insert_index: Option<usize>,
    hits: &mut ShellHitMap,
) {
    let palette = &config.palette;
    buffer.set_style(area, Style::default().bg(palette.panel_bg));
    let tabs = snapshot
        .tabs
        .iter()
        .filter(|tab| Some(tab.workspace_id.as_str()) == snapshot.focused_workspace_id.as_deref())
        .collect::<Vec<_>>();
    let pill = config.tab_style == TabStyleConfig::Pill;
    let desired_widths = tabs
        .iter()
        .map(|tab| {
            let label = tab_label(tab);
            if pill {
                pill_tab_width(&label, tab.number, &config.tab_pill).max(MIN_TAB_WIDTH)
            } else {
                display_width(&label).saturating_add(4).max(MIN_TAB_WIDTH)
            }
        })
        .collect::<Vec<_>>();
    let content = tab_bar_content_area(snapshot, config, area);
    let mouse_chrome = config.mouse_capture;
    let new_tab_width = if mouse_chrome { NEW_TAB_WIDTH } else { 0 };
    let desired_total = desired_widths
        .iter()
        .copied()
        .fold(0_u16, u16::saturating_add)
        .saturating_add(tabs.len().saturating_sub(1).min(u16::MAX as usize) as u16)
        .saturating_add(new_tab_width);
    let overflow =
        desired_total > content.width && (!mouse_chrome || content.width >= MIN_TAB_STRIP_WIDTH);
    let available = if overflow && mouse_chrome {
        content
            .width
            .saturating_sub(NEW_TAB_WIDTH)
            .saturating_sub(TAB_SCROLL_BUTTON_WIDTH.saturating_mul(2))
    } else {
        content.width.saturating_sub(new_tab_width)
    };
    let max_scroll = max_tab_scroll(&desired_widths, available);
    if !overflow {
        *tab_scroll = 0;
    } else if *reveal_focused_tab {
        if let Some(focused) = tabs.iter().position(|tab| tab.focused) {
            *tab_scroll = centered_tab_scroll(focused, &desired_widths, available).min(max_scroll);
        }
    } else {
        *tab_scroll = (*tab_scroll).min(max_scroll);
    }
    *reveal_focused_tab = false;

    let mut x = content.x;
    let tab_right = if overflow && mouse_chrome {
        hits.tab_scroll_left = Rect::new(
            content.x,
            content.y,
            TAB_SCROLL_BUTTON_WIDTH.min(content.width),
            1,
        );
        put_text(
            buffer,
            hits.tab_scroll_left.x,
            content.y,
            hits.tab_scroll_left.width,
            " < ",
            Style::default()
                .fg(if *tab_scroll > 0 {
                    palette.overlay1
                } else {
                    palette.overlay0
                })
                .bg(palette.surface0),
        );
        x = hits.tab_scroll_left.right();
        content
            .right()
            .saturating_sub(NEW_TAB_WIDTH + TAB_SCROLL_BUTTON_WIDTH)
    } else {
        content.right().saturating_sub(new_tab_width)
    };

    let mut first_visible = None;
    let mut last_visible = None;
    for (index, tab) in tabs.iter().enumerate().skip(*tab_scroll) {
        let name = tab_label(tab);
        let desired = desired_widths[index];
        let remaining = tab_right.saturating_sub(x);
        let width = desired.min(remaining);
        if width == 0 {
            break;
        }
        let rect = Rect::new(x, area.y, width, 1);
        if pill {
            render_pill_tab(buffer, rect, tab, &name, index, config);
        } else {
            let style = if tab.focused {
                let base = Style::default()
                    .fg(panel_contrast_fg(palette))
                    .bg(palette.accent);
                if tab.custom_label {
                    base.add_modifier(Modifier::BOLD)
                } else {
                    base
                }
            } else if tab.custom_label {
                Style::default().fg(palette.overlay1).bg(palette.surface0)
            } else {
                Style::default().fg(palette.overlay0).bg(palette.surface0)
            };
            let padding = width.saturating_sub(display_width(&name));
            let left = padding / 2;
            let text = format!(
                "{empty:left$}{name}{empty:right_padding$}",
                empty = "",
                left = left as usize,
                right_padding = padding.saturating_sub(left) as usize,
            );
            put_text(buffer, rect.x, rect.y, rect.width, &text, style);
        }
        hits.tabs.push((rect, tab.tab_id.clone()));
        first_visible.get_or_insert(index);
        last_visible = Some(index);
        x = x.saturating_add(width + 1);
        if width < desired {
            break;
        }
    }

    if overflow && mouse_chrome {
        hits.tab_scroll_right = Rect::new(tab_right, area.y, TAB_SCROLL_BUTTON_WIDTH, 1);
        let can_scroll_right = *tab_scroll < max_scroll;
        put_text(
            buffer,
            hits.tab_scroll_right.x,
            area.y,
            hits.tab_scroll_right.width,
            " > ",
            Style::default()
                .fg(if can_scroll_right {
                    palette.overlay1
                } else {
                    palette.overlay0
                })
                .bg(palette.surface0),
        );
        hits.new_tab = Rect::new(
            hits.tab_scroll_right.right(),
            area.y,
            content
                .right()
                .saturating_sub(hits.tab_scroll_right.right())
                .min(NEW_TAB_WIDTH),
            1,
        );
    } else if mouse_chrome {
        hits.new_tab = Rect::new(
            x.min(content.right()),
            area.y,
            content.right().saturating_sub(x).min(NEW_TAB_WIDTH),
            1,
        );
    }
    if mouse_chrome {
        put_text(
            buffer,
            hits.new_tab.x,
            area.y,
            hits.new_tab.width,
            " + ",
            Style::default().fg(palette.overlay1).bg(palette.panel_bg),
        );
    }

    if first_visible.is_some_and(|index| index > 0) {
        let ellipsis_x = if hits.tab_scroll_left.width > 0 {
            hits.tab_scroll_left.right()
        } else {
            content.x
        };
        put_text(
            buffer,
            ellipsis_x,
            area.y,
            u16::from(ellipsis_x < content.right()),
            "…",
            Style::default().fg(palette.overlay0),
        );
    }
    if last_visible.is_some_and(|index| index + 1 < tabs.len()) {
        let ellipsis_x = if hits.tab_scroll_right.width > 0 {
            hits.tab_scroll_right.x.saturating_sub(1)
        } else {
            content.right().saturating_sub(1)
        };
        put_text(
            buffer,
            ellipsis_x,
            area.y,
            u16::from(ellipsis_x >= content.x && ellipsis_x < content.right()),
            "…",
            Style::default().fg(palette.overlay0),
        );
    }

    if let Some(insert_index) = tab_drag_insert_index {
        if let Some(indicator_x) = tab_drop_indicator_x(hits, &tabs, insert_index) {
            put_text(
                buffer,
                indicator_x.min(content.right().saturating_sub(1)),
                area.y,
                1,
                "│",
                Style::default().fg(palette.accent),
            );
        }
    }
    render_tab_bar_status(buffer, area, snapshot, config);
}

pub(crate) fn tab_bar_status_width(
    snapshot: &ClientShellSnapshot,
    config: &ClientShellConfig,
) -> u16 {
    let content = snapshot.tab_bar_right.iter().fold(0u16, |width, segment| {
        width.saturating_add(status_segment_width(segment, &config.tab_pill))
    });
    let separators = snapshot.tab_bar_right.len().saturating_sub(1);
    content.saturating_add(
        display_width(&snapshot.tab_bar_right_separator)
            .saturating_mul(separators.min(u16::MAX as usize) as u16),
    )
}

fn tab_bar_status_area(
    snapshot: &ClientShellSnapshot,
    config: &ClientShellConfig,
    area: Rect,
) -> Option<Rect> {
    let width = tab_bar_status_width(snapshot, config);
    if width == 0 {
        return None;
    }
    let reserved = width.saturating_add(1);
    (area.width.saturating_sub(reserved) >= MIN_TAB_STRIP_WIDTH)
        .then(|| Rect::new(area.right().saturating_sub(width), area.y, width, 1))
}

fn tab_bar_content_area(
    snapshot: &ClientShellSnapshot,
    config: &ClientShellConfig,
    area: Rect,
) -> Rect {
    let reserved = tab_bar_status_area(snapshot, config, area)
        .map(|status| status.width.saturating_add(1))
        .unwrap_or(0);
    Rect {
        width: area.width.saturating_sub(reserved),
        ..area
    }
}

fn render_tab_bar_status(
    buffer: &mut Buffer,
    area: Rect,
    snapshot: &ClientShellSnapshot,
    config: &ClientShellConfig,
) {
    let palette = &config.palette;
    let Some(status) = tab_bar_status_area(snapshot, config, area) else {
        return;
    };
    let separator_width = display_width(&snapshot.tab_bar_right_separator);
    let mut x = status.x;
    for (index, segment) in snapshot.tab_bar_right.iter().enumerate() {
        if index > 0 && separator_width > 0 {
            put_text(
                buffer,
                x,
                area.y,
                separator_width,
                &snapshot.tab_bar_right_separator,
                Style::default().fg(palette.overlay0).bg(palette.panel_bg),
            );
            x = x.saturating_add(separator_width);
        }
        if let Some(icon) = segment.icon.as_deref() {
            x = render_pill_status_segment(buffer, x, area.y, segment, icon, config);
            continue;
        }
        let width = display_width(&segment.text);
        let style = if segment.accent {
            Style::default()
                .fg(panel_contrast_fg(palette))
                .bg(palette.accent)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(palette.overlay1).bg(palette.panel_bg)
        };
        put_text(buffer, x, area.y, width, &segment.text, style);
        x = x.saturating_add(width);
    }
}

fn tab_drop_indicator_x(
    hits: &ShellHitMap,
    tabs: &[&ClientShellTab],
    insert_index: usize,
) -> Option<u16> {
    let visible = hits
        .tabs
        .iter()
        .filter_map(|(rect, tab_id)| {
            tabs.iter()
                .position(|tab| tab.tab_id == *tab_id)
                .map(|index| (index, *rect))
        })
        .collect::<Vec<_>>();
    let (first_index, first_rect) = *visible.first()?;
    let (last_index, last_rect) = *visible.last()?;
    if insert_index == 0 {
        return Some(if first_index == 0 {
            first_rect.x
        } else {
            hits.tab_scroll_left.right()
        });
    }
    if let Some((_, rect)) = visible.iter().find(|(index, _)| *index == insert_index) {
        return Some(rect.x.saturating_sub(1));
    }
    if insert_index >= tabs.len() {
        return Some(if last_index + 1 >= tabs.len() {
            last_rect.right()
        } else {
            hits.tab_scroll_right.x.saturating_sub(1)
        });
    }
    None
}

fn centered_tab_scroll(focused: usize, widths: &[u16], available: u16) -> usize {
    let mut best = focused;
    let mut best_distance = u16::MAX;
    for start in 0..=focused {
        let before = widths
            .iter()
            .copied()
            .enumerate()
            .skip(start)
            .take(focused.saturating_sub(start))
            .fold(0u16, |width, (_, tab)| width.saturating_add(tab + 1));
        if before >= available {
            continue;
        }
        let focused_width = widths[focused].min(available.saturating_sub(before));
        let center = before.saturating_mul(2).saturating_add(focused_width);
        let distance = center.abs_diff(available);
        if distance <= best_distance {
            best_distance = distance;
            best = start;
        }
    }
    best
}

fn max_tab_scroll(widths: &[u16], available: u16) -> usize {
    let Some((&last, preceding)) = widths.split_last() else {
        return 0;
    };
    let mut start = preceding.len();
    let mut used = u32::from(last);
    // Keep the longest fully visible suffix, not merely a sliver of the last tab.
    // An oversized last tab must still be reachable at the start of the strip.
    for width in preceding.iter().rev() {
        let required = used + 1 + u32::from(*width);
        if required > u32::from(available) {
            break;
        }
        used = required;
        start -= 1;
    }
    start
}

fn pill_caps_width(pill: &TabPillConfig) -> u16 {
    display_width(&pill.left_cap).saturating_add(display_width(&pill.right_cap))
}

/// Drawn width of a pill tab: caps, padded number block, and padded label block.
/// The label gets one extra pad column so it reads centered against the right cap.
fn pill_tab_width(label: &str, number: usize, pill: &TabPillConfig) -> u16 {
    pill_caps_width(pill)
        .saturating_add(display_width(label))
        .saturating_add(number.checked_ilog10().unwrap_or(0) as u16 + 1)
        .saturating_add(5)
}

fn status_segment_width(segment: &ClientShellTabStatusSegment, pill: &TabPillConfig) -> u16 {
    match segment.icon.as_deref() {
        Some(icon) => pill_caps_width(pill)
            .saturating_add(display_width(icon))
            .saturating_add(display_width(&segment.text))
            .saturating_add(6),
        None => display_width(&segment.text),
    }
}

/// Resolve a pill color: a theme token name or `#RGB`/`#RRGGBB`.
fn tab_pill_color(palette: &Palette, value: &str) -> Option<ratatui::style::Color> {
    Some(match value {
        "accent" => palette.accent,
        "text" => palette.text,
        "subtext0" => palette.subtext0,
        "overlay0" => palette.overlay0,
        "overlay1" => palette.overlay1,
        "surface0" => palette.surface0,
        "surface1" => palette.surface1,
        "mauve" => palette.mauve,
        "green" => palette.green,
        "yellow" => palette.yellow,
        "red" => palette.red,
        "blue" => palette.blue,
        "teal" => palette.teal,
        "peach" => palette.peach,
        _ => return crate::config::SidebarTokenColor::parse(value).map(|color| color.ratatui()),
    })
}

/// Draw `(n label)` filling exactly `rect`. When `rect` is narrower than the pill,
/// the label is truncated, then dropped, and the caps go last.
fn render_pill_tab(
    buffer: &mut Buffer,
    rect: Rect,
    tab: &ClientShellTab,
    label: &str,
    position: usize,
    config: &ClientShellConfig,
) {
    let palette = &config.palette;
    let pill = &config.tab_pill;
    let mut label_style = if tab.focused {
        Style::default().fg(palette.accent).bg(palette.surface1)
    } else if tab.custom_label {
        Style::default().fg(palette.overlay1).bg(palette.surface0)
    } else {
        Style::default().fg(palette.overlay0).bg(palette.surface0)
    };
    if tab.custom_label {
        label_style = label_style.add_modifier(Modifier::BOLD);
    }
    let number_bg = match pill.index_colors.len() {
        0 => palette.accent,
        len => tab_pill_color(palette, pill.index_colors[position % len].as_str())
            .unwrap_or(palette.accent),
    };
    let mut number_style = Style::default()
        .fg(panel_contrast_fg(palette))
        .bg(number_bg);
    if tab.focused {
        number_style = number_style.add_modifier(Modifier::BOLD);
    }
    let number = format!(" {} ", tab.number);

    let (mut left_cap, mut right_cap) = (pill.left_cap.as_str(), pill.right_cap.as_str());
    if rect.width <= pill_caps_width(pill) {
        (left_cap, right_cap) = ("", "");
    }
    let inner = rect
        .width
        .saturating_sub(display_width(left_cap) + display_width(right_cap));
    let number_width = display_width(&number).min(inner);
    let label_width = inner - number_width;
    let label = if display_width(label) + 2 <= label_width {
        let padding = label_width - display_width(label);
        // The right cap visually extends the block, so odd spare space goes left.
        let left = padding - padding / 2;
        format!(
            "{empty:left$}{label}{empty:right$}",
            empty = "",
            left = left as usize,
            right = (padding - left) as usize,
        )
    } else if label_width >= 3 {
        format!(
            " {} ",
            crate::ui::truncate_end(label, usize::from(label_width - 2))
        )
    } else {
        crate::ui::truncate_end(label, usize::from(label_width))
    };

    let cap_style = |block: Style| {
        Style::default()
            .fg(block.bg.unwrap_or(palette.panel_bg))
            .bg(palette.panel_bg)
    };
    let mut x = put_segment(
        buffer,
        rect.x,
        rect.y,
        rect.right(),
        left_cap,
        cap_style(number_style),
    );
    put_text(buffer, x, rect.y, number_width, &number, number_style);
    x = x.saturating_add(number_width);
    let last_block = if label_width > 0 {
        buffer.set_style(Rect::new(x, rect.y, label_width, 1), label_style);
        put_text(buffer, x, rect.y, label_width, &label, label_style);
        x = x.saturating_add(label_width);
        label_style
    } else {
        number_style
    };
    put_segment(
        buffer,
        x,
        rect.y,
        rect.right(),
        right_cap,
        cap_style(last_block),
    );
}

/// Draw a right-side entry as `(icon text)` and return the next column. Each block
/// gets an extra pad column on the side away from its cap so it reads centered.
fn render_pill_status_segment(
    buffer: &mut Buffer,
    x: u16,
    y: u16,
    segment: &ClientShellTabStatusSegment,
    icon: &str,
    config: &ClientShellConfig,
) -> u16 {
    let palette = &config.palette;
    let pill = &config.tab_pill;
    let icon_bg = segment
        .icon_bg
        .as_deref()
        .and_then(|value| tab_pill_color(palette, value))
        .unwrap_or(palette.accent);
    let icon_style = Style::default().fg(panel_contrast_fg(palette)).bg(icon_bg);
    let text_style = if segment.accent {
        Style::default()
            .fg(panel_contrast_fg(palette))
            .bg(palette.accent)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(palette.overlay1).bg(palette.surface0)
    };
    let cap_style = |block: Style| {
        Style::default()
            .fg(block.bg.unwrap_or(palette.panel_bg))
            .bg(palette.panel_bg)
    };
    let right = buffer.area.right();
    let x = put_segment(buffer, x, y, right, &pill.left_cap, cap_style(icon_style));
    let x = put_segment(buffer, x, y, right, &format!(" {icon}  "), icon_style);
    let x = put_segment(
        buffer,
        x,
        y,
        right,
        &format!("  {} ", segment.text),
        text_style,
    );
    put_segment(buffer, x, y, right, &pill.right_cap, cap_style(text_style))
}

fn tab_label(tab: &ClientShellTab) -> String {
    if tab.zoomed {
        format!("{} Z", tab.label)
    } else {
        tab.label.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::{max_tab_scroll, tab_pill_color};

    #[test]
    fn every_accepted_pill_token_resolves_against_the_palette() {
        let palette = crate::app::state::Palette::catppuccin();
        for token in crate::config::TAB_PILL_PALETTE_TOKENS {
            assert!(tab_pill_color(&palette, token).is_some(), "{token}");
        }
        assert_eq!(tab_pill_color(&palette, "mauve"), Some(palette.mauve));
        assert_eq!(
            tab_pill_color(&palette, "#f0a"),
            Some(ratatui::style::Color::Rgb(0xff, 0x00, 0xaa))
        );
        assert_eq!(tab_pill_color(&palette, "pink"), None);
    }

    #[test]
    fn trailing_scroll_limit_accounts_for_full_widths_and_separators() {
        for (widths, available, expected) in [
            (&[][..], 0, 0),
            (&[8, 13][..], 0, 1),
            (&[8, 13][..], 1, 1),
            (&[8, 13][..], 12, 1),
            (&[8, 13][..], 21, 1),
            (&[8, 13][..], 22, 0),
            (&[8, 13][..], 30, 0),
            (&[8, u16::MAX][..], u16::MAX, 1),
        ] {
            assert_eq!(
                max_tab_scroll(widths, available),
                expected,
                "widths={widths:?}, available={available}"
            );
        }
    }
}
