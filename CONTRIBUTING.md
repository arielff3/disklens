# Contributing to Disklens

Thanks for helping! Bug reports, ideas and pull requests are all welcome.

## Getting started

Requirements: Node.js 22+, Rust (MSVC toolchain), Visual Studio Build Tools with the C++ workload, and WebView2.

```bash
npm install
npm run tauri dev
```

### Working on the interface without the backend

`npm run dev` serves the interface at http://localhost:1420. Outside the Tauri window it uses sample data from `src/mockApi.ts`, so you can work on screens in any browser. A few query parameters help:

- `?demo` opens the sample drive right away
- `&tab=explore|systemData|biggest|changes|projects|cleanup|apps`
- `&theme=sand|ink|lagoon&mode=light|dark`

For example: http://localhost:1420/?demo&tab=projects&theme=lagoon&mode=dark

### Command-line examples

`src-tauri/examples/` has small tools that run the backend against your machine without the interface:

```bash
cd src-tauri
cargo run --release --example scan -- C:\
cargo run --release --example system_data
cargo run --release --example apps
cargo run --release --example projects -- C:\path\to\your\projects
cargo run --release --example delete_speed
```

## Before you open a pull request

```bash
npm test
npm run build
cd src-tauri
cargo test
cargo clippy --all-targets -- -D warnings
```

CI runs the same checks on Windows.

## Conventions

- **Copy lives in `src/strings.ts`**, so a translation is one file.
- **Colors come from the themes** (`src/themes/`). Components use CSS variables; no hard-coded colors.
- **One module, one job.** Pure logic (parsing, matching, layout) gets unit tests; anything that touches the file system gets a test on a temporary folder.
- **Deleting is a big deal.** Anything that removes files must:
  - only remove paths the backend itself offered to the user,
  - pass `cleanup::safety::check`,
  - never follow junctions or symbolic links,
  - prefer the Recycle Bin; permanent deletion needs a confirmation that lists every path.
- **No network and no telemetry.** Disklens works entirely on the user's PC.
- Commit messages follow [Conventional Commits](https://www.conventionalcommits.org) (`feat:`, `fix:`, `perf:`, `docs:`…).

## Antivirus false positives

Some antivirus products (Kaspersky, for one) flag stripped Rust build-script executables as suspicious and delete them mid-build. `src-tauri/Cargo.toml` keeps debug info in build scripts to avoid it. If your build still fails with "Access is denied" or "being used by another process" on a `build_script_build` file, check your antivirus quarantine.

## Reporting security issues

Please don't open a public issue; see [SECURITY.md](SECURITY.md).
