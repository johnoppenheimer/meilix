# meilix

A terminal UI to manage [Meilisearch](https://www.meilisearch.com) indexes: list
indexes with their document counts, create indexes, and browse or search their
documents — all from the keyboard.

## Install

With Cargo ([crates.io/crates/meilix](https://crates.io/crates/meilix)):

```
cargo install meilix
```

With Homebrew:

```
brew install johnoppenheimer/tap/meilix
```

## Usage

```
meilix                                    # connects to http://localhost:7700
meilix --url https://my.meili.host        # custom host
MEILI_MASTER_KEY=yourkey meilix           # with an API key
```

Connection settings are read from env vars, overridable by flags:

| Flag    | Env var            | Default                 |
|---------|--------------------|-------------------------|
| `--url` | `MEILI_URL`        | `http://localhost:7700` |
| `--key` | `MEILI_MASTER_KEY` | *(none)*                |

Run `meilix --help` for the full list.

## Keys

Indexes are on the left, documents on the right. Focus follows what you open.

- **Index list:** `↑↓`/`jk` move · `c` create · `Enter` open · `r` refresh · `q` quit
- **Documents:** `↑↓`/`jk` move · `/` search · `n`/`p` next/prev page · `Esc` back · `q` quit

## Development

Tooling is managed with [mise](https://mise.jdx.dev), which pins the Rust version.

```
mise trust     # first time only
mise install   # install the pinned toolchain
```

Tasks — run with `mise run <task>`:

| Task       | What it does                                    |
|------------|-------------------------------------------------|
| `build`    | Compile the binary.                             |
| `run`      | Build and launch the TUI (args after `--`).     |
| `test`     | Run unit tests.                                 |
| `fmt`      | Format the code.                                |
| `check`    | CI gate: format check, clippy, test.            |
| `publish`  | Run `check`, then publish to crates.io.         |
| `brew-sha` | Print the sha256 of the published crate tarball.|

```
mise run run -- --url http://localhost:7700
```
