# meilix

A terminal UI (ratatui) to manage Meilisearch indexes: list indexes with their
document counts, create indexes, browse documents, and search them.

## Setup

Tooling is managed with [mise](https://mise.jdx.dev). It pins the Rust version.

```
mise install   # install the pinned Rust toolchain
mise trust      # trust this project's mise.toml (first time only)
```

## Configuration

Connection is read from env vars, overridable by flags:

| Flag        | Env var             | Default                 |
|-------------|---------------------|-------------------------|
| `--url`     | `MEILI_URL`         | `http://localhost:7700` |
| `--key`     | `MEILI_MASTER_KEY`  | *(none)*                |

## Tasks

Run with `mise run <task>`.

| Task    | Command                                              | What it does                          |
|---------|------------------------------------------------------|---------------------------------------|
| `build` | `cargo build`                                        | Compile the binary.                   |
| `run`   | `cargo run --`                                       | Build and launch the TUI.             |
| `test`  | `cargo test`                                          | Run unit tests.                       |
| `fmt`   | `cargo fmt`                                           | Format the code.                      |
| `check` | `cargo fmt --check` + `cargo clippy -D warnings` + `cargo test` | CI gate: format, lint, test. |

Pass args after the task, e.g.:

```
mise run run -- --url http://localhost:7700
MEILI_MASTER_KEY=yourkey mise run run
```

## Keys

- Index list: `↑↓`/`jk` move, `c` create, `Enter` open, `r` refresh, `q` quit
- Documents: `↑↓`/`jk` move, `/` search, `n`/`p` next/prev page, `Esc` back, `q` quit
