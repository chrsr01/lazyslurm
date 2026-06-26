#!/usr/bin/env python3
"""
Update the Homebrew tap formula for a released version.

Waits for the GitHub release assets (built by CI) to appear, reads their
sha256 checksums, regenerates Formula/lazyslurm.rb in the tap repo, and pushes.

Usage:
  python3 scripts/update_tap.py <version>   # e.g. 0.3.0
"""

from __future__ import annotations

import subprocess
import sys
import tempfile
import time
from pathlib import Path

REPO = "hill/lazyslurm"
TAP_REMOTE = "git@github.com:hill/homebrew-tap.git"
FORMULA_PATH = "Formula/lazyslurm.rb"

GIT_NAME = "Tom Hill"
GIT_EMAIL = "tomhill98@me.com"

# (homebrew branch, release asset target triple) in formula order
TARGETS = [
    ("arm", "aarch64-apple-darwin"),
    ("intel", "x86_64-apple-darwin"),
    ("linux", "x86_64-unknown-linux-gnu"),
]

POLL_INTERVAL = 15
POLL_TIMEOUT = 20 * 60


def run(cmd: list[str]) -> str:
    return subprocess.run(cmd, check=True, text=True, capture_output=True).stdout


def published_assets(version: str) -> set[str]:
    try:
        out = run(["gh", "release", "view", f"v{version}", "-R", REPO,
                   "--json", "assets", "--jq", ".assets[].name"])
    except subprocess.CalledProcessError:
        return set()
    return set(out.split())


def wait_for_assets(version: str) -> None:
    needed = {f"lazyslurm-{t}.sha256" for _, t in TARGETS}
    start = time.time()
    while True:
        missing = needed - published_assets(version)
        if not missing:
            return
        if time.time() - start > POLL_TIMEOUT:
            sys.exit(f"Timed out waiting for release assets: {sorted(missing)}")
        print(f"Waiting for CI to publish binaries... still missing {sorted(missing)}")
        time.sleep(POLL_INTERVAL)


def fetch_sha256(version: str, target: str, dest: Path) -> str:
    name = f"lazyslurm-{target}.sha256"
    out = dest / name
    run(["gh", "release", "download", f"v{version}", "-R", REPO,
         "-p", name, "-O", str(out), "--clobber"])
    # checksum file is "<hash>  <filename>"
    return out.read_text().split()[0]


def render_formula(version: str, sha: dict[str, str]) -> str:
    base = f"https://github.com/{REPO}/releases/download/v{version}"
    return f'''class Lazyslurm < Formula
  desc "A terminal UI for monitoring and managing SLURM jobs"
  homepage "https://github.com/hill/lazyslurm"
  version "{version}"

  if OS.mac? && Hardware::CPU.arm?
    url "{base}/lazyslurm-aarch64-apple-darwin.tar.gz"
    sha256 "{sha['arm']}"
  elsif OS.mac? && Hardware::CPU.intel?
    url "{base}/lazyslurm-x86_64-apple-darwin.tar.gz"
    sha256 "{sha['intel']}"
  elsif OS.linux?
    url "{base}/lazyslurm-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "{sha['linux']}"
  end

  license "MIT"

  def install
    bin.install "lazyslurm"
  end

  test do
    system "#{{bin}}/lazyslurm", "--version"
  end
end
'''


def main() -> None:
    if len(sys.argv) != 2:
        sys.exit("Usage: update_tap.py <version>")
    version = sys.argv[1].lstrip("v")

    print(f"Updating Homebrew tap for v{version}")
    wait_for_assets(version)

    with tempfile.TemporaryDirectory() as tmp_str:
        tmp = Path(tmp_str)

        sha = {}
        for branch, target in TARGETS:
            sha[branch] = fetch_sha256(version, target, tmp)
            print(f"  {target}: {sha[branch]}")

        clone = tmp / "tap"
        run(["git", "clone", "--depth", "1", TAP_REMOTE, str(clone)])

        formula = clone / FORMULA_PATH
        formula.parent.mkdir(parents=True, exist_ok=True)
        formula.write_text(render_formula(version, sha))

        if not run(["git", "-C", str(clone), "status", "--porcelain"]).strip():
            print("Tap formula already up to date.")
            return

        run(["git", "-C", str(clone), "add", FORMULA_PATH])
        run(["git", "-C", str(clone),
             "-c", f"user.name={GIT_NAME}", "-c", f"user.email={GIT_EMAIL}",
             "commit", "-m", f"lazyslurm {version}"])
        run(["git", "-C", str(clone), "push", "origin", "HEAD"])
        print(f"Pushed lazyslurm {version} to homebrew-tap.")


if __name__ == "__main__":
    main()
