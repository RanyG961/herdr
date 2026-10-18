use super::*;
use ratatui::style::Color;

const LEFT: &str = "\u{e0b6}";
const RIGHT: &str = "\u{e0b4}";

fn pill_config(extra: &str) -> ClientShellConfig {
    let config = toml::from_str::<Config>(&format!(
        "[ui]\ntab_style = \"pill\"\nmobile_width_threshold = 0\n{extra}"
    ))
    .expect("pill config");
    ClientShellConfig::from_config(&config)
}

fn tab(number: usize, label: &str) -> ClientShellTab {
    ClientShellTab {
        tab_id: format!("tab_{number}"),
        workspace_id: "ws_1".into(),
        number,
        label: label.into(),
        custom_label: false,
        zoomed: false,
        focused: false,
        agent_status: AgentStatus::Idle,
    }
}

/// focused auto-named "alpha", custom "node", zoomed "zsh", plain "four".
fn four_tabs() -> ClientShellSnapshot {
    let mut projected = snapshot();
    projected.tabs = vec![
        ClientShellTab {
            focused: true,
            ..tab(1, "alpha")
        },
        ClientShellTab {
            custom_label: true,
            ..tab(2, "node")
        },
        ClientShellTab {
            zoomed: true,
            ..tab(3, "zsh")
        },
        tab(4, "four"),
    ];
    projected
}

fn render(
    config: ClientShellConfig,
    projected: ClientShellSnapshot,
    width: u16,
) -> ClientShellState {
    let mut state = ClientShellState::new(config);
    state.set_snapshot(Box::new(projected));
    state.set_pane_surface(surface());
    state.compose(width, 20).expect("tab bar frame");
    state
}

fn tab_bar_buffer(state: &mut ClientShellState, width: u16) -> Buffer {
    state
        .compose(width, 20)
        .expect("tab bar frame")
        .to_ratatui_buffer()
        .expect("tab bar buffer")
}

fn row_text(buffer: &Buffer, rect: Rect) -> String {
    (rect.x..rect.right())
        .map(|x| buffer[(x, rect.y)].symbol())
        .collect()
}

fn tab_rect(state: &ClientShellState, tab_id: &str) -> Rect {
    state
        .hits
        .tabs
        .iter()
        .find(|(_, id)| id == tab_id)
        .unwrap_or_else(|| panic!("{tab_id} hit"))
        .0
}

#[test]
fn absent_tab_style_keeps_block_rendering() {
    let config = ClientShellConfig::from_config(&Config::default());
    assert_eq!(config.tab_style, crate::config::TabStyleConfig::Block);
    let accent = config.palette.accent;
    let mut state = render(config, four_tabs(), 106);
    let buffer = tab_bar_buffer(&mut state, 106);
    let rect = tab_rect(&state, "tab_1");
    assert_eq!(rect.width, MIN_TAB_WIDTH.max(5 + 4));
    assert_eq!(row_text(&buffer, rect), "  alpha  ");
    assert!((rect.x..rect.right()).all(|x| buffer[(x, rect.y)].bg == accent));
    let top = row_text(&buffer, Rect::new(0, 0, 106, 1));
    assert!(!top.contains(LEFT) && !top.contains(RIGHT));
}

#[test]
fn pill_tabs_draw_caps_number_and_label_blocks() {
    let config = pill_config("");
    let palette = config.palette.clone();
    let mut state = render(config, four_tabs(), 106);
    let buffer = tab_bar_buffer(&mut state, 106);

    let focused = tab_rect(&state, "tab_1");
    assert_eq!(
        row_text(&buffer, focused),
        format!("{LEFT} 1   alpha {RIGHT}")
    );
    let cap = &buffer[(focused.x, focused.y)];
    assert_eq!((cap.fg, cap.bg), (palette.green, palette.panel_bg));
    let number = &buffer[(focused.x + 2, focused.y)];
    assert_eq!(number.symbol(), "1");
    assert_eq!(number.bg, palette.green);
    assert!(number.modifier.contains(Modifier::BOLD));
    let label = &buffer[(focused.x + 6, focused.y)];
    assert_eq!(label.symbol(), "a");
    assert_eq!((label.fg, label.bg), (palette.accent, palette.surface1));
    assert!(!label.modifier.contains(Modifier::BOLD));
    let right_cap = &buffer[(focused.right() - 1, focused.y)];
    assert_eq!(
        (right_cap.fg, right_cap.bg),
        (palette.surface1, palette.panel_bg)
    );

    let custom = tab_rect(&state, "tab_2");
    let label = &buffer[(custom.x + 6, custom.y)];
    assert_eq!((label.fg, label.bg), (palette.overlay1, palette.surface0));
    assert!(label.modifier.contains(Modifier::BOLD));
    assert!(!buffer[(custom.x + 2, custom.y)]
        .modifier
        .contains(Modifier::BOLD));

    let zoomed = tab_rect(&state, "tab_3");
    assert_eq!(
        row_text(&buffer, zoomed),
        format!("{LEFT} 3   zsh Z {RIGHT}")
    );
    let plain = &buffer[(tab_rect(&state, "tab_4").x + 6, zoomed.y)];
    assert_eq!((plain.fg, plain.bg), (palette.overlay0, palette.surface0));
    assert!(!plain.modifier.contains(Modifier::BOLD));
}

#[test]
fn pill_number_colors_cycle_and_accept_hex() {
    let config = pill_config("[ui.tab_pill]\nindex_colors = [\"red\", \"#00ff00\"]\n");
    let palette = config.palette.clone();
    let mut projected = four_tabs();
    projected.tabs.push(tab(5, "five"));
    let mut state = render(config, projected, 120);
    let buffer = tab_bar_buffer(&mut state, 120);
    let expected = [
        palette.red,
        Color::Rgb(0, 255, 0),
        palette.red,
        Color::Rgb(0, 255, 0),
        palette.red,
    ];
    for (index, color) in expected.into_iter().enumerate() {
        let rect = state.hits.tabs[index].0;
        // The number block follows the left cap.
        assert_eq!(buffer[(rect.x, rect.y)].fg, color, "tab {index}");
        assert_eq!(buffer[(rect.x + 1, rect.y)].bg, color, "tab {index}");
    }
}

#[test]
fn pill_hit_rects_match_drawn_cells() {
    let config = pill_config("");
    let panel_bg = config.palette.panel_bg;
    let mut state = render(config, four_tabs(), 106);
    let buffer = tab_bar_buffer(&mut state, 106);
    let widths = ["alpha", "node", "zsh Z", "four"]
        .map(|label| 2 + super::render::display_width(label) + 1 + 5)
        .map(|width| width.max(MIN_TAB_WIDTH));
    let rects = state
        .hits
        .tabs
        .iter()
        .map(|(rect, _)| *rect)
        .collect::<Vec<_>>();
    assert_eq!(
        rects.iter().map(|rect| rect.width).collect::<Vec<_>>(),
        widths
    );
    for pair in rects.windows(2) {
        // One untouched bar-background column separates neighbouring pills.
        assert_eq!(pair[1].x, pair[0].right() + 1);
        let gap = &buffer[(pair[0].right(), pair[0].y)];
        assert_eq!((gap.symbol(), gap.bg), (" ", panel_bg));
    }
    for rect in &rects {
        assert_eq!(buffer[(rect.x, rect.y)].symbol(), LEFT);
        assert_eq!(buffer[(rect.right() - 1, rect.y)].symbol(), RIGHT);
    }
}

#[test]
fn pill_overflow_scrolls_without_dangling_caps() {
    let config = pill_config("");
    let mut projected = snapshot();
    projected.tabs = (1..=12)
        .map(|number| ClientShellTab {
            focused: number == 1,
            ..tab(number, &format!("tab-{number}"))
        })
        .collect();
    let mut state = render(config, projected, 70);
    let buffer = tab_bar_buffer(&mut state, 70);
    assert!(!state.hits.tab_scroll_right.is_empty());
    assert_eq!(state.hits.tabs[0].1, "tab_1");
    let strip_right = state.hits.tab_scroll_right.x;
    let (last_rect, _) = state.hits.tabs.last().expect("visible tabs");
    for (rect, tab_id) in &state.hits.tabs {
        assert!(
            rect.right() <= strip_right,
            "{tab_id} overlaps the scroll button"
        );
        let last = buffer[(rect.right() - 1, rect.y)].symbol();
        // The overflow marker may replace the cap of the tab cut by the strip edge.
        let allowed: &[&str] = if rect == last_rect {
            &[RIGHT, "…"]
        } else {
            &[RIGHT]
        };
        assert!(allowed.contains(&last), "{tab_id} ends with {last:?}");
    }

    state.tab_scroll = 5;
    state.reveal_focused_tab = false;
    let buffer = tab_bar_buffer(&mut state, 70);
    assert_eq!(state.tab_scroll, 5);
    assert_eq!(state.hits.tabs[0].1, "tab_6");
    let first = state.hits.tabs[0].0;
    assert_eq!(buffer[(first.x, first.y)].symbol(), "…");
    assert_eq!(buffer[(first.x + 1, first.y)].symbol(), " ");
    assert_eq!(buffer[(first.right() - 1, first.y)].symbol(), RIGHT);
}

#[test]
fn truncated_pill_keeps_number_and_closing_cap() {
    let config = pill_config("");
    let surface0 = config.palette.surface0;
    let mut projected = snapshot();
    projected.tabs = vec![ClientShellTab {
        focused: true,
        ..tab(1, "x")
    }];
    projected.tabs.push(tab(2, &"long-label-".repeat(12)));
    let mut state = render(config, projected, 80);
    let buffer = tab_bar_buffer(&mut state, 80);
    let rect = tab_rect(&state, "tab_2");
    let text = row_text(&buffer, rect);
    assert!(text.starts_with(&format!("{LEFT} 2 ")), "{text:?}");
    assert!(text.ends_with(&format!("… {RIGHT}")), "{text:?}");
    assert_eq!(buffer[(rect.right() - 1, rect.y)].fg, surface0);
}

#[test]
fn empty_caps_render_square_pills() {
    let config = pill_config("[ui.tab_pill]\nleft_cap = \"\"\nright_cap = \"\"\n");
    let palette = config.palette.clone();
    let mut state = render(config, four_tabs(), 106);
    let buffer = tab_bar_buffer(&mut state, 106);
    let rect = tab_rect(&state, "tab_2");
    assert_eq!(rect.width, 4 + 1 + 5);
    assert_eq!(row_text(&buffer, rect), " 2   node ");
    assert_eq!(buffer[(rect.x, rect.y)].bg, palette.blue);
    assert_eq!(buffer[(rect.right() - 1, rect.y)].bg, palette.surface0);
}

#[test]
fn pill_labels_center_with_odd_spare_on_the_left() {
    let label_block = |state: &ClientShellState, buffer: &Buffer, tab_id: &str| {
        let rect = tab_rect(state, tab_id);
        let text = row_text(buffer, Rect::new(rect.x + 4, rect.y, rect.width - 5, 1));
        let style = &buffer[(rect.x + 4, rect.y)];
        (text, style.bg)
    };

    // Odd spare (3): two columns before the label, one before the right cap.
    let config = pill_config("");
    let palette = config.palette.clone();
    let mut state = render(config, four_tabs(), 106);
    let buffer = tab_bar_buffer(&mut state, 106);
    assert_eq!(
        label_block(&state, &buffer, "tab_1"),
        ("  alpha ".into(), palette.surface1)
    );
    assert_eq!(
        label_block(&state, &buffer, "tab_2"),
        ("  node ".into(), palette.surface0)
    );

    // A one-character label still gets two columns before and one after.
    let mut projected = snapshot();
    projected.tabs = vec![ClientShellTab {
        focused: true,
        ..tab(1, "1")
    }];
    let mut state = render(pill_config(""), projected, 106);
    let buffer = tab_bar_buffer(&mut state, 106);
    assert_eq!(
        row_text(&buffer, tab_rect(&state, "tab_1")),
        format!("{LEFT} 1   1 {RIGHT}")
    );

    // Even spare (4): a short label widened to MIN_TAB_WIDTH splits evenly.
    let config = pill_config("[ui.tab_pill]\nleft_cap = \"\"\nright_cap = \"\"\n");
    let mut projected = snapshot();
    projected.tabs = vec![ClientShellTab {
        focused: true,
        custom_label: true,
        ..tab(1, "x")
    }];
    let mut state = render(config, projected, 106);
    let buffer = tab_bar_buffer(&mut state, 106);
    let rect = tab_rect(&state, "tab_1");
    assert_eq!(rect.width, MIN_TAB_WIDTH);
    assert_eq!(row_text(&buffer, rect), " 1   x  ");
}

#[test]
fn pill_caps_keep_a_transparent_bar_background() {
    let mut config = pill_config("[theme]\nname = \"terminal\"\n");
    config.palette.panel_bg = Color::Reset;
    let mut state = render(config, four_tabs(), 106);
    let buffer = tab_bar_buffer(&mut state, 106);
    let rect = tab_rect(&state, "tab_1");
    assert_eq!(buffer[(rect.x, rect.y)].bg, Color::Reset);
    assert_eq!(buffer[(rect.right() - 1, rect.y)].bg, Color::Reset);
}

#[test]
fn bottom_pill_tab_bar_keeps_hits_on_the_drawn_row() {
    let mut config = pill_config("");
    config.tab_bar_position = crate::config::TabBarPositionConfig::Bottom;
    let mut state = render(config, four_tabs(), 106);
    let buffer = tab_bar_buffer(&mut state, 106);
    let rect = tab_rect(&state, "tab_1");
    assert_eq!(rect.y, 19);
    assert_eq!(row_text(&buffer, rect), format!("{LEFT} 1   alpha {RIGHT}"));
}

#[test]
fn status_entry_with_icon_renders_as_a_pill() {
    let config = pill_config("");
    let palette = config.palette.clone();
    let mut projected = four_tabs();
    projected.tab_bar_right = vec![
        crate::protocol::ClientShellTabStatusSegment {
            text: "host".into(),
            accent: false,
            icon: None,
            icon_bg: None,
        },
        crate::protocol::ClientShellTabStatusSegment {
            text: "herdr".into(),
            accent: false,
            icon: Some("\u{f07b}".into()),
            icon_bg: Some("mauve".into()),
        },
    ];
    let mut state = render(config, projected, 106);
    let buffer = tab_bar_buffer(&mut state, 106);
    // The icon gets its extra pad after it (away from the left cap), the text before it.
    let expected = format!("host {LEFT} \u{f07b}    herdr {RIGHT}");
    let width = super::render::display_width(&expected);
    let status = Rect::new(106 - width, 0, width, 1);
    assert_eq!(row_text(&buffer, status), expected);
    let icon_cap = &buffer[(status.x + 5, 0)];
    assert_eq!(
        (icon_cap.fg, icon_cap.bg),
        (palette.mauve, palette.panel_bg)
    );
    assert_eq!(buffer[(status.x + 7, 0)].bg, palette.mauve);
    assert_eq!(buffer[(status.x + 9, 0)].bg, palette.mauve);
    assert_eq!(buffer[(status.x + 10, 0)].bg, palette.surface0);
    let text = &buffer[(status.x + 12, 0)];
    assert_eq!((text.symbol(), text.bg), ("h", palette.surface0));
    assert_eq!(buffer[(status.right() - 1, 0)].fg, palette.surface0);
    assert!(state
        .hits
        .tabs
        .iter()
        .all(|(rect, _)| rect.right() < status.x));
}

#[test]
fn status_pill_icon_block_uses_the_projected_value_color() {
    let config = pill_config("");
    let palette = config.palette.clone();
    for (bg, expected) in [("green", palette.green), ("red", palette.red)] {
        let mut projected = four_tabs();
        projected.tab_bar_right = vec![crate::protocol::ClientShellTabStatusSegment {
            text: "insert".into(),
            accent: false,
            icon: Some("V".into()),
            icon_bg: Some(bg.into()),
        }];
        let mut state = render(pill_config(""), projected, 106);
        let buffer = tab_bar_buffer(&mut state, 106);
        let expected_text = format!("{LEFT} V    insert {RIGHT}");
        let width = super::render::display_width(&expected_text);
        let status = Rect::new(106 - width, 0, width, 1);
        assert_eq!(row_text(&buffer, status), expected_text);
        assert_eq!(buffer[(status.x + 2, 0)].bg, expected, "{bg}");
    }
}

#[test]
fn pill_colors_follow_the_active_theme() {
    let number_bgs = |theme: &str| {
        let config = pill_config(&format!("[theme]\nname = \"{theme}\"\n"));
        let palette = config.palette.clone();
        let mut state = render(config, four_tabs(), 106);
        let buffer = tab_bar_buffer(&mut state, 106);
        let bgs = state
            .hits
            .tabs
            .iter()
            .map(|(rect, _)| buffer[(rect.x + 1, rect.y)].bg)
            .collect::<Vec<_>>();
        assert_eq!(
            bgs,
            [palette.green, palette.blue, palette.peach, palette.mauve],
            "{theme}"
        );
        bgs
    };
    assert_ne!(number_bgs("catppuccin"), number_bgs("gruvbox"));
}

#[test]
fn pill_tabs_click_and_drag_by_drawn_rects() {
    let mut state = render(pill_config(""), four_tabs(), 106);
    let mouse = |kind, rect: Rect, column: u16| {
        RawInputEvent::Mouse(crossterm::event::MouseEvent {
            kind,
            column,
            row: rect.y,
            modifiers: KeyModifiers::empty(),
        })
    };
    let first = tab_rect(&state, "tab_1");
    let fourth = tab_rect(&state, "tab_4");
    // Grab the left cap and drop on the right cap: both are part of the pill.
    state.handle_raw_events(vec![mouse(
        MouseEventKind::Down(MouseButton::Left),
        first,
        first.x,
    )]);
    state.handle_raw_events(vec![mouse(
        MouseEventKind::Drag(MouseButton::Left),
        fourth,
        fourth.right() - 1,
    )]);
    let release = state.handle_raw_events(vec![mouse(
        MouseEventKind::Up(MouseButton::Left),
        fourth,
        fourth.right() - 1,
    )]);
    assert!(matches!(
        &release.actions[..],
        [ClientShellAction::Endpoint { request, .. }]
            if matches!(&request.method, crate::api::schema::Method::TabMove(params)
                if params.tab_id == "tab_1" && params.insert_index == 4)
    ));

    state.compose(106, 20).expect("tabs after drag");
    let node = tab_rect(&state, "tab_2");
    let number_cell = node.x + 2;
    state.handle_raw_events(vec![mouse(
        MouseEventKind::Down(MouseButton::Left),
        node,
        number_cell,
    )]);
    let click = state.handle_raw_events(vec![mouse(
        MouseEventKind::Up(MouseButton::Left),
        node,
        number_cell,
    )]);
    assert!(matches!(
        &click.actions[..],
        [ClientShellAction::Endpoint { request, .. }]
            if matches!(&request.method, crate::api::schema::Method::TabFocus(target)
                if target.tab_id == "tab_2")
    ));
}
