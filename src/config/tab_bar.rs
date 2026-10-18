use serde::{Deserialize, Serialize};

pub(crate) const DEFAULT_TAB_BAR_COMMAND_INTERVAL_SECONDS: u64 = 5;
pub(crate) const DEFAULT_TAB_BAR_COMMAND_TIMEOUT_SECONDS: u64 = 2;
pub(crate) const MAX_TAB_BAR_COMMAND_INTERVAL_SECONDS: u64 = 31_536_000;
pub(crate) const MAX_TAB_BAR_COMMAND_TIMEOUT_SECONDS: u64 = 3_600;
pub(crate) const MAX_TAB_BAR_RIGHT_ENTRIES: usize = 16;

fn default_datetime_format() -> String {
    "%H:%M".to_string()
}

fn default_command_interval_seconds() -> u64 {
    DEFAULT_TAB_BAR_COMMAND_INTERVAL_SECONDS
}

fn default_command_timeout_seconds() -> u64 {
    DEFAULT_TAB_BAR_COMMAND_TIMEOUT_SECONDS
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum TabBarRightEntryConfig {
    Zoom {
        icon: Option<String>,
        icon_bg: Option<TabPillColor>,
    },
    Hostname {
        icon: Option<String>,
        icon_bg: Option<TabPillColor>,
    },
    Datetime {
        #[serde(default = "default_datetime_format")]
        format: String,
        icon: Option<String>,
        icon_bg: Option<TabPillColor>,
    },
    Text {
        text: String,
        icon: Option<String>,
        icon_bg: Option<TabPillColor>,
    },
    Command {
        command: String,
        #[serde(default = "default_command_interval_seconds")]
        interval_seconds: u64,
        #[serde(default = "default_command_timeout_seconds")]
        timeout_seconds: u64,
        icon: Option<String>,
        icon_bg: Option<TabPillColor>,
    },
}

impl TabBarRightEntryConfig {
    /// Optional icon and icon background that render the entry as a pill.
    pub(crate) fn pill_icon(&self) -> (Option<&str>, Option<&TabPillColor>) {
        let (Self::Zoom { icon, icon_bg }
        | Self::Hostname { icon, icon_bg }
        | Self::Datetime { icon, icon_bg, .. }
        | Self::Text { icon, icon_bg, .. }
        | Self::Command { icon, icon_bg, .. }) = self;
        (icon.as_deref(), icon_bg.as_ref())
    }
}

/// Desktop tab row drawing style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TabStyleConfig {
    #[default]
    Block,
    Pill,
}

/// Theme tokens accepted by [`TabPillColor`].
pub(crate) const TAB_PILL_PALETTE_TOKENS: &[&str] = &[
    "accent", "text", "subtext0", "overlay0", "overlay1", "surface0", "surface1", "mauve", "green",
    "yellow", "red", "blue", "teal", "peach",
];

/// A pill colour: a theme palette token name or `#RGB`/`#RRGGBB`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(try_from = "String", into = "String")]
pub struct TabPillColor(String);

impl TabPillColor {
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for TabPillColor {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if TAB_PILL_PALETTE_TOKENS.contains(&value.as_str())
            || super::SidebarTokenColor::parse(&value).is_some()
        {
            Ok(Self(value))
        } else {
            Err(format!(
                "unknown pill color {value:?}; use #RGB, #RRGGBB, or one of: {}",
                TAB_PILL_PALETTE_TOKENS.join(", ")
            ))
        }
    }
}

impl From<TabPillColor> for String {
    fn from(color: TabPillColor) -> Self {
        color.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct TabPillConfig {
    /// Tab number block backgrounds, cycled by tab position. Theme tokens follow the active theme.
    /// Default: ["green", "blue", "peach", "mauve"].
    pub index_colors: Vec<TabPillColor>,
    /// Glyph drawn at the left end of each pill. Empty draws square ends. Default: "\u{e0b6}".
    pub left_cap: String,
    /// Glyph drawn at the right end of each pill. Empty draws square ends. Default: "\u{e0b4}".
    pub right_cap: String,
}

impl Default for TabPillConfig {
    fn default() -> Self {
        Self {
            index_colors: ["green", "blue", "peach", "mauve"]
                .into_iter()
                .map(|token| TabPillColor(token.into()))
                .collect(),
            left_cap: "\u{e0b6}".into(),
            right_cap: "\u{e0b4}".into(),
        }
    }
}

pub(crate) fn parse_tab_bar_datetime_format(
    value: &str,
) -> Result<time::format_description::OwnedFormatItem, String> {
    if value.is_empty() {
        return Err("datetime format is empty".into());
    }
    let format = time::format_description::parse_strftime_owned(value)
        .map_err(|err| format!("invalid datetime format: {err}"))?;
    time::PrimitiveDateTime::MIN
        .format(&format)
        .map_err(|err| format!("unsupported datetime format: {err}"))?;
    Ok(format)
}

pub(crate) fn tab_bar_right_diagnostics(entries: &[TabBarRightEntryConfig]) -> Vec<String> {
    let mut diagnostics = Vec::new();
    if entries.len() > MAX_TAB_BAR_RIGHT_ENTRIES {
        diagnostics.push(format!(
            "ui.tab_bar_right may contain at most {MAX_TAB_BAR_RIGHT_ENTRIES} entries; ignoring extras"
        ));
    }

    for (index, entry) in entries.iter().enumerate().take(MAX_TAB_BAR_RIGHT_ENTRIES) {
        match entry {
            TabBarRightEntryConfig::Datetime { format, .. } => {
                if format.is_empty() {
                    diagnostics.push(format!(
                        "ui.tab_bar_right[{index}] datetime format is empty; hiding entry"
                    ));
                } else if let Err(err) = parse_tab_bar_datetime_format(format) {
                    diagnostics.push(format!("ui.tab_bar_right[{index}] has {err}; hiding entry"));
                }
            }
            TabBarRightEntryConfig::Command {
                command,
                interval_seconds,
                timeout_seconds,
                ..
            } => {
                if command.trim().is_empty() {
                    diagnostics.push(format!(
                        "ui.tab_bar_right[{index}] command is empty; hiding entry"
                    ));
                }
                if *interval_seconds == 0 {
                    diagnostics.push(format!(
                        "ui.tab_bar_right[{index}] interval_seconds must be at least 1; hiding entry"
                    ));
                }
                if *interval_seconds > MAX_TAB_BAR_COMMAND_INTERVAL_SECONDS {
                    diagnostics.push(format!(
                        "ui.tab_bar_right[{index}] interval_seconds may be at most {MAX_TAB_BAR_COMMAND_INTERVAL_SECONDS}; hiding entry"
                    ));
                }
                if *timeout_seconds == 0 {
                    diagnostics.push(format!(
                        "ui.tab_bar_right[{index}] timeout_seconds must be at least 1; hiding entry"
                    ));
                }
                if *timeout_seconds > MAX_TAB_BAR_COMMAND_TIMEOUT_SECONDS {
                    diagnostics.push(format!(
                        "ui.tab_bar_right[{index}] timeout_seconds may be at most {MAX_TAB_BAR_COMMAND_TIMEOUT_SECONDS}; hiding entry"
                    ));
                }
            }
            TabBarRightEntryConfig::Zoom { .. }
            | TabBarRightEntryConfig::Hostname { .. }
            | TabBarRightEntryConfig::Text { .. } => {}
        }
    }

    diagnostics
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tab_bar_entries_parse_with_command_defaults() {
        #[derive(Deserialize)]
        struct Wrapper {
            entries: Vec<TabBarRightEntryConfig>,
        }

        let parsed: Wrapper = toml::from_str(
            r#"
entries = [
  { type = "zoom" },
  { type = "hostname" },
  { type = "datetime", format = "%H:%M" },
  { type = "text", text = "prod" },
  { type = "command", command = "status.sh" },
]
"#,
        )
        .expect("parse tab bar entries");

        assert_eq!(parsed.entries.len(), 5);
        assert!(matches!(
            &parsed.entries[4],
            TabBarRightEntryConfig::Command {
                interval_seconds: DEFAULT_TAB_BAR_COMMAND_INTERVAL_SECONDS,
                timeout_seconds: DEFAULT_TAB_BAR_COMMAND_TIMEOUT_SECONDS,
                ..
            }
        ));
    }

    #[test]
    fn diagnostics_reject_invalid_datetime_and_command_schedules() {
        let entries = vec![
            TabBarRightEntryConfig::Datetime {
                format: "%Q".into(),
                icon: None,
                icon_bg: None,
            },
            TabBarRightEntryConfig::Datetime {
                format: "%z".into(),
                icon: None,
                icon_bg: None,
            },
            TabBarRightEntryConfig::Command {
                command: String::new(),
                interval_seconds: 0,
                timeout_seconds: 0,
                icon: None,
                icon_bg: None,
            },
            TabBarRightEntryConfig::Command {
                command: "status.sh".into(),
                interval_seconds: MAX_TAB_BAR_COMMAND_INTERVAL_SECONDS + 1,
                timeout_seconds: MAX_TAB_BAR_COMMAND_TIMEOUT_SECONDS + 1,
                icon: None,
                icon_bg: None,
            },
        ];

        let diagnostics = tab_bar_right_diagnostics(&entries).join("\n");
        assert!(diagnostics.contains("invalid datetime format"));
        assert!(diagnostics.contains("unsupported datetime format"));
        assert!(diagnostics.contains("command is empty"));
        assert!(diagnostics.contains("interval_seconds must be at least 1"));
        assert!(diagnostics.contains("interval_seconds may be at most"));
        assert!(diagnostics.contains("timeout_seconds must be at least 1"));
        assert!(diagnostics.contains("timeout_seconds may be at most"));
        assert!(parse_tab_bar_datetime_format("").is_err());
    }

    #[test]
    fn pill_style_is_opt_in_with_nerd_font_defaults() {
        let config: crate::config::Config = toml::from_str("[ui]\n").expect("empty ui");
        assert_eq!(config.ui.tab_style, TabStyleConfig::Block);
        assert_eq!(config.ui.tab_pill, TabPillConfig::default());
        assert_eq!(config.ui.tab_pill.left_cap, "\u{e0b6}");
        assert_eq!(config.ui.tab_pill.right_cap, "\u{e0b4}");
        assert_eq!(
            config
                .ui
                .tab_pill
                .index_colors
                .iter()
                .map(TabPillColor::as_str)
                .collect::<Vec<_>>(),
            ["green", "blue", "peach", "mauve"]
        );

        let config: crate::config::Config = toml::from_str(
            r##"
[ui]
tab_style = "pill"
tab_bar_right = [
  { type = "command", command = "basename \"$PWD\"", icon = "\uf07b", icon_bg = "mauve" },
  { type = "zoom", icon = "Z" },
  { type = "hostname" },
]

[ui.tab_pill]
index_colors = ["teal", "#abc", "#a1b2c3"]
left_cap = ""
"##,
        )
        .expect("pill config");
        assert_eq!(config.ui.tab_style, TabStyleConfig::Pill);
        assert_eq!(config.ui.tab_pill.index_colors.len(), 3);
        assert_eq!(config.ui.tab_pill.left_cap, "");
        assert_eq!(config.ui.tab_pill.right_cap, "\u{e0b4}");
        let icons = config
            .ui
            .tab_bar_right
            .iter()
            .map(|entry| {
                let (icon, bg) = entry.pill_icon();
                (icon, bg.map(TabPillColor::as_str))
            })
            .collect::<Vec<_>>();
        assert_eq!(
            icons,
            [
                (Some("\u{f07b}"), Some("mauve")),
                (Some("Z"), None),
                (None, None)
            ]
        );
    }

    #[test]
    fn pill_config_rejects_unknown_colors_and_wrong_types() {
        for (toml, expected) in [
            (
                "[ui.tab_pill]\nindex_colors = [\"pink\"]\n",
                "unknown pill color",
            ),
            (
                "[ui.tab_pill]\nindex_colors = [\"#12\"]\n",
                "unknown pill color",
            ),
            ("[ui.tab_pill]\nindex_colors = \"green\"\n", "invalid type"),
            ("[ui.tab_pill]\nleft_cap = 1\n", "invalid type"),
            ("[ui]\ntab_style = \"round\"\n", "unknown variant"),
            (
                "[ui]\ntab_bar_right = [{ type = \"hostname\", icon_bg = \"pink\" }]\n",
                "unknown pill color",
            ),
            (
                "[ui]\ntab_bar_right = [{ type = \"hostname\", icon = 1 }]\n",
                "invalid type",
            ),
        ] {
            let err = toml::from_str::<crate::config::Config>(toml)
                .expect_err(toml)
                .to_string();
            assert!(err.contains(expected), "{toml}: {err}");
        }
    }
}
