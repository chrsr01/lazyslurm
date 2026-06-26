# Release & Distribution

This directory contains files related to distributing LazySlurm.

## Directory Structure

- `packaging/` - Future: RPM/DEB package specs
- `scripts/` - Future: Installation scripts

The Homebrew formula lives in its own tap repo,
[hill/homebrew-tap](https://github.com/hill/homebrew-tap), not here. It is
regenerated automatically on every release (see below).

## Release Process

Run a single command:

```bash
just release 0.3.0     # or: patch | minor | major
```

This will:

1. Bump the version in `Cargo.toml` and refresh `Cargo.lock`
2. Commit, tag `vX.Y.Z`, and push
3. GitHub Actions builds cross-platform binaries, creates the GitHub release,
   uploads assets, and publishes to crates.io
4. Wait for the binaries to land, then regenerate the Homebrew formula in the
   tap with the new version, URLs, and SHA256 hashes, and push it

Update `CHANGELOG.md` before releasing, since `just release` does not touch it.

If the tap step is interrupted (it polls CI for a couple of minutes), re-run it
on its own once the build finishes:

```bash
just update-tap 0.3.0
```

## Distribution Checklist

- [ ] `CHANGELOG.md` updated
- [ ] GitHub release with binaries
- [ ] Published to crates.io
- [ ] Homebrew tap formula updated (automatic)
- [ ] README installation instructions current

## Installing from the tap

```bash
brew install hill/tap/lazyslurm
```
