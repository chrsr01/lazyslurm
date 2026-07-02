<p align="center">
  <img src="media/logo.svg" alt="lazyslurm" width="620">
</p>

<p align="center">
  A terminal UI for <a href="https://slurm.schedmd.com/overview.html">Slurm</a>. Like <a href="https://github.com/jesseduffield/lazygit">lazygit</a>, but for HPC clusters.
</p>

<p align="center">
  <a href="https://github.com/chrsr01/lazyslurm/actions"><img src="https://github.com/chrsr01/lazyslurm/workflows/CI/badge.svg" alt="CI"></a>
  <a href="https://opensource.org/licenses/MIT"><img src="https://img.shields.io/badge/License-MIT-yellow.svg" alt="License: MIT"></a>
</p>

<p align="center">
  <a href="media/demo.mp4"><img src="media/demo.gif" alt="lazyslurm demo" width="860"></a>
</p>

## Why This Exists

Slurm's CLI is powerful but clunky for monitoring.
This gives you the lazygit experience for your cluster.
Built in Rust with [ratatui](https://ratatui.rs/) and released as a single binary.

This is a fork of [hill/lazyslurm](https://github.com/hill/lazyslurm) that removes the
`sinfo` partitions view and adapts node listing to `sinfo_t_idle` output, because some
HPC clusters (e.g. HoreKa) restrict the raw `sinfo` command for regular users. It is not
published to crates.io or Homebrew — install from a [GitHub Release](https://github.com/chrsr01/lazyslurm/releases)
below.

## Features

- Monitor your jobs with live log tailing, job details and ability to cancel jobs
- See per-node state, CPU load, free memory and GPU (gres) allocation across the cluster
- See finished jobs from `sacct` with details
- Automatic refresh every 2 minutes; press `r` any time to refresh immediately

## Installation

### Binary Releases

Download the latest binary for your platform from [GitHub Releases](https://github.com/chrsr01/lazyslurm/releases):

```bash
# Linux x64 (musl, statically linked — recommended for HPC clusters with
# mismatched glibc versions across login/compute nodes, e.g. HoreKa)
curl -L https://github.com/chrsr01/lazyslurm/releases/latest/download/lazyslurm-x86_64-unknown-linux-musl.tar.gz | tar xz
mv lazyslurm ~/.local/bin/   # or: sudo mv lazyslurm /usr/local/bin/

# Linux x64 (glibc)
curl -L https://github.com/chrsr01/lazyslurm/releases/latest/download/lazyslurm-x86_64-unknown-linux-gnu.tar.gz | tar xz
sudo mv lazyslurm /usr/local/bin/

# macOS (Apple Silicon)
curl -L https://github.com/chrsr01/lazyslurm/releases/latest/download/lazyslurm-aarch64-apple-darwin.tar.gz | tar xz
sudo mv lazyslurm /usr/local/bin/

# macOS (Intel)
curl -L https://github.com/chrsr01/lazyslurm/releases/latest/download/lazyslurm-x86_64-apple-darwin.tar.gz | tar xz
sudo mv lazyslurm /usr/local/bin/

# Windows: download lazyslurm-x86_64-pc-windows-msvc.zip from the releases page
```

Each asset has a matching `.sha256` checksum file alongside it.

### Cargo (build from source)

If you have [Rust installed](https://rustup.rs/), install straight from this fork
(the crates.io `lazyslurm` package is upstream's, without the `sinfo_t_idle` patch):

```bash
cargo install --git https://github.com/chrsr01/lazyslurm --tag v0.3.2
```

## Usage

```bash
# Monitor all jobs for the current user
lazyslurm

# Filter to a specific user
lazyslurm --user username

# Filter to a specific partition
lazyslurm --partition gpu
```

The `Jobs` tab works without any extra setup. The `Nodes` tab uses `sinfo_t_idle`, and `History` uses `sacct` (which needs Slurm accounting enabled on the cluster).

### Keyboard Controls

**Global**

| Key | Action |
|-----|--------|
| `q` / `Ctrl+C` | Quit |
| `Tab` / `Shift+Tab` | Switch tabs |
| `1`–`3` | Jump to a tab |
| `↑/↓` or `j/k` | Navigate the current list |
| `r` | Refresh immediately (also refreshes automatically every 2 minutes) |
| `u` | Filter by user |

**Jobs tab**

| Key | Action |
|-----|--------|
| `←/→` or `h/l` | Move focus between panels |
| `Enter` | Fullscreen the focused pane |
| `/` | Filter the list by name or id |
| `P` | Pin / unpin the selected job |
| `c` | Cancel the selected job |
| `y` | Raw log view (with the Logs pane focused) |

**History tab**

| Key | Action |
|-----|--------|
| `Enter` | Open the job detail view |
| `y` | Raw log view (in the detail view) |

**Log views**

| Key | Action |
|-----|--------|
| `G` / `g` | Follow the tail |
| `y` | Open the raw view, then drag-select to copy |
| `Esc` | Back |

## Development

Requires Docker and [just](https://github.com/casey/just). The dev container runs a full Slurm install with accounting (slurmdbd + MariaDB), so every tab works locally.

```bash
# Build and start the Slurm container
just slurm_up

# Get a shell inside it
just slurm_shell

# Inside the container (your code is mounted at /workspace)
cargo run

# Submit some test jobs
just slurm_populate

# Inspect the cluster
just slurm_status    # squeue + sinfo_t_idle
just slurm_prio      # sprio priority breakdown
just slurm_share     # sshare fairshare usage
just slurm_clear_jobs
```

Your source is mounted into the container, so changes are picked up immediately.

## License

MIT
