# Pill tabs (personal fork)

This branch (`pill-tabs`) adds two commits, `feat: pill tab style` and
`feat: pane user-var pill`, on top of an upstream herdr release tag. Never open PRs, issues or comments on
`herdrdev/herdr`, and never push there.

## Rebase onto a new upstream release

```sh
git fetch upstream --tags
# OLD is the tag the branch currently sits on (v0.9.5 when this was written).
git rebase --onto vX.Y.Z OLD pill-tabs
just ci
git push --force-with-lease origin pill-tabs
```

Conflicts usually come from `src/client/shell/tabs.rs`, `src/config/tab_bar.rs`
and the `tab_bar_right` plumbing (`src/app/tab_bar_status.rs`,
`src/server/client_shell.rs`).

## Build

Building needs Zig 0.16.0, so use the repo's dev shell:

```sh
nix develop --command cargo build --release --locked
./target/release/herdr
```

On macOS, nix's `xcrun` has no `nmedit`, and the vendored libghostty-vt build
needs it. Put a shim earlier on `PATH` that runs
`/Library/Developer/CommandLineTools/usr/bin/nmedit` for `xcrun nmedit` and
hands every other call to the original `xcrun`.

## Config

```toml
[ui]
tab_style = "pill"            # default "block" keeps upstream rendering
tab_bar_right = [
  # icon/icon_bg render an entry as a pill. Inline tables must stay on one line.
  { type = "command", command = "basename \"$PWD\"", interval_seconds = 5, icon = "\uf07b", icon_bg = "mauve" },
  # Focused pane's OSC 1337 SetUserVar value; colors pick the icon block per value.
  { type = "pane_var", name = "vi_mode", icon = "\ue7c5", icon_bg = "overlay1", colors = { insert = "green", normal = "blue", visual = "mauve", visual_line = "mauve", replace = "red" } },
]

[ui.tab_pill]
# Theme tokens follow the active herdr theme: accent text subtext0 overlay0
# overlay1 surface0 surface1 mauve green yellow red blue teal peach.
# "#RGB"/"#RRGGBB" values are fixed overrides.
index_colors = ["green", "blue", "peach", "mauve"]
left_cap = "\ue0b6"           # "" for terminals without Nerd Fonts
right_cap = "\ue0b4"

# Kolada (https://github.com/KaliaHayes/Kolada-VS-Code-Theme) as herdr theme
# overrides. The pills pick these up through the tokens above.
[theme.custom]
accent = "#ffb85c"
panel_bg = "#000000"
sidebar_bg = "#000000"
active_row_bg = "#101010"
selection_bg = "#141414"
surface0 = "#141414"
surface1 = "#1e1e1e"
surface_dim = "#101010"
overlay0 = "#888888"
overlay1 = "#a6a6a6"
text = "#ffffff"
subtext0 = "#dddddd"
mauve = "#af8bff"
green = "#32feaf"
yellow = "#ffe779"
red = "#ff8888"
blue = "#5cd4ff"
teal = "#aaefff"
peach = "#ffb85c"
```

Each tab is drawn as `(n name)`: a colored number block, then the name on a
dim block. The focused tab's name uses the theme accent on `surface1`, and
renamed tabs are bold. Zoomed tabs keep their ` Z` marker. Changing
`theme.name` or `[theme.custom]` recolors the pills.
