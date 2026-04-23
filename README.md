# Wowland

A Wayland compositor built with Smithay.

## Features

- Tiling window manager with MasterStack and Grid layouts
- XDG config file support with fallback to defaults
- Configurable keybindings, gaps, workspaces, and decoration colors
- Spawn applications via keybindings

## Build

```bash
cargo build --release
```

## Run

Wowland must be run as a nested compositor (inside an existing Wayland session):

```bash
# Set the Wayland display to point to wowland
export WAYLAND_DISPLAY=wowland

# Run wowland
cargo run --release
```

Or use the compiled binary:

```bash
./target/release/wowland
```

## Configuration

Config is loaded from (in order of priority):

1. `--config <path>` CLI argument
2. `$XDG_CONFIG_HOME/wowland/keybindings.toml`
3. `~/.config/wowland/keybindings.toml`
4. Embedded default config

Run with `--print-default-config` to see the default configuration:

```bash
cargo run -- --print-default-config
```

### Example Config

```toml
super_modifier = "logo"

[gaps]
inner = 5
outer = 10

[workspace]
count = 4

[[keybindings]]
action = "quit"
key = "Q"
modifiers = ["super"]

[[keybindings]]
action = "next-layout"
key = "space"
modifiers = ["super"]

[[keybindings]]
action = { spawn = "wofi --show drun" }
key = "Return"
modifiers = ["super"]
```

### Configuration Options

- `super_modifier`: Set to "logo" (default) or "alt" for Super key behavior
- `gaps.inner`: Inner gap between windows (default: 5)
- `gaps.outer`: Outer gap from screen edges (default: 10)
- `workspace.count`: Number of workspaces (default: 4)
- `decoration_focused`: Hex color for focused window border (e.g., "#476178")
- `decoration_unfocused`: Hex color for unfocused window border (e.g., "#2E333D")

## Keybindings

Default keybindings (with super_modifier = "logo"):

| Action | Key | Modifiers |
|--------|-----|-----------|
| Quit | Q | Super |
| Next Layout | Space | Super |
| Previous Layout | Space | Super+Shift |
| Focus Next | J | Super |
| Focus Prev | K | Super |
| Toggle Float | F | Super |
| Toggle Maximize | M | Super+Shift |
| Toggle Minimize | M | Super |
| Close Window | W | Super |
| Cycle Opacity | O | Super |
| Workspace Prev | Left | Super |
| Workspace Next | Right | Super |
| Move to Prev Workspace | Left | Super+Shift |
| Move to Next Workspace | Right | Super+Shift |
| Spawn | Enter | Super |

## Troubleshooting

- **Black screen**: Ensure you're running inside a Wayland compositor (not X11)
- **No windows**: Applications must request xdg-shell surfaces
- **Keybindings not working**: Check `super_modifier` setting (logo vs alt)

## License

MIT