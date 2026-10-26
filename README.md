# odx for Zed

[odx](https://odx.readthedocs.io/) in Zed's assistant: slash commands that
put what odx knows about the open Odoo project into the conversation.

| Command | Runs |
| --- | --- |
| `/odx-dashboard [section…]` | `odx dashboard --format markdown`, optionally only some sections (`pinned`, `droppable`, `new-upstream`, `forward-port`, `backport`, `auto-update`, `glue`, `drift`, `suppressed`) |
| `/odx-status` | `odx status`: the aggregated and pinned pull requests and their state upstream |
| `/odx-changelog [rev]` | `odx changelog --since rev` (default `HEAD`): release notes of `odx.lock` |
| `/odx-glue` | `odx glue`: glue addons Odoo would install with what the project has |
| `/odx-sniff [rev]` | `odx sniff --since rev` (default `HEAD`): migration scripts merges bring in |

Lint diagnostics for Odoo modules come from the odoo-lint extension.

## The odx executable

The extension runs `odx` from the project's `PATH` (for example an activated
virtual environment). Without one, it downloads the latest `odx-cli` release
from PyPI into its own directory. Install it yourself with
`uv tool install odx-cli`.

## Tasks

Zed extensions cannot add tasks; put the odx commands you run often in the
project's `.zed/tasks.json`:

```json
[
  { "label": "odx aggregate", "command": "odx", "args": ["aggregate"] },
  { "label": "odx lock", "command": "odx", "args": ["lock"] },
  { "label": "odx vendor", "command": "odx", "args": ["vendor"] },
  { "label": "odx verify", "command": "odx", "args": ["verify"] },
  { "label": "odx dashboard", "command": "odx", "args": ["dashboard"] }
]
```

`odx dashboard --format tui` opens the interactive view in an odx built with
the `tui` feature (`cargo install odx-cli --features tui`); the PyPI wheel
leaves it out.

## Development

```console
$ rustup target add wasm32-wasip2
$ cargo test
$ cargo build --release --target wasm32-wasip2
```

In Zed, *zed: install dev extension* and pick this directory.

## License

MIT
