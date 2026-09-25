# verz

`verz` is a semver management tool similar to `npm version`, implemented in Rust with zero runtime dependencies. It supports updating both `package.json` and `Cargo.toml` simultaneously and automates git committing and tagging.

## Usage

```bash
A semver management tool similar to npm version

Usage: verz [OPTIONS] [newversion] [COMMAND]

Commands:
  major       Increment major version
  minor       Increment minor version
  patch       Increment patch version
  premajor    Increment premajor version
  preminor    Increment preminor version
  prepatch    Increment prepatch version
  prerelease  Increment prerelease version
  help        Print this message or the help of the given subcommand(s)

Arguments:
  [newversion]  New version to set

Options:
  -n, --no-git-tag-version  Do not create a git commit and tag
  -m, --message <MESSAGE>   Commit message
      --sh                  Append the short commit hash (git rev-parse --short HEAD) to the version
      --suffix <VALUE>      Append a custom suffix to the version (semver prerelease-safe). Combined with --sh in the order the flags appear on the command line
  -h, --help                Print help
  -V, --version             Print version
```

## Version suffixes (`--sh` / `--suffix`)

`--sh` appends the short commit hash and `--suffix <value>` appends a custom label. Both are attached to the semver prerelease field, joined with `-`, **in the order the flags appear on the command line**. They work on a bump or, on their own, to stamp the current version.

```bash
# on 0.63.0:
verz minor --sh --suffix runtime-debug   # -> 0.64.0-d60e0ca1-runtime-debug
verz minor --suffix runtime-debug --sh   # -> 0.64.0-runtime-debug-d60e0ca1  (order follows the command line)

# no subcommand: stamp the current version
verz --sh --suffix runtime-debug         # -> 0.63.0-d60e0ca1-runtime-debug

# an existing prerelease is kept and appended after
verz preminor --sh                       # 0.63.0 -> 0.64.0-0-d60e0ca1
```

Both flags are global, so they may appear before or after the subcommand. `--sh` requires a git repository with at least one commit.

## Installation

### via Homebrew (macOS/Linux)

```bash
brew install rotty3000/tap/verz
```

### via Shell Script (Linux)

You can use the following one-liner to download and install the latest `verz` binary. This script supports multiple architectures (amd64, arm64) and will check if an update is available if `verz` is already installed.

```bash
curl -sSL https://raw.githubusercontent.com/rotty3000/verz/main/scripts/install.sh | bash
```

### from Source

```bash
cargo install --path .
```

### from Releases

See [releases](https://github.com/rotty3000/verz/releases) for pre-built binaries you can download manually.

## Building

Install `cross` to assist with cross compilation:

```bash
cargo install cross --git https://github.com/cross-rs/cross
```

To build static binaries for all supported platforms (Linux on musl amd64/arm64 and Windows on amd64):

```bash
make build
```

To compress the produced binaries using UPX:

```bash
make compress
```

**Note:**
- Cross-compilation uses `cross`, which requires Docker/Podman.
- **Clean builds:** It is highly recommended to run `make clean` before switching between native `cargo` builds and `cross` builds to avoid artifact conflicts.

To run the test suite:

```bash
make test
```

The binaries will be located in the `dist/` directory.
