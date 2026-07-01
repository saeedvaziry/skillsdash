# Releasing

Releases are cut manually from the **Actions** tab: run the **Release** workflow
and enter a version like `1.2.0` (no leading `v`). The workflow then:

1. Validates the version, bumps `Cargo.toml`, commits, and tags `v<version>`.
2. Builds binaries for all five targets.
3. Creates the GitHub Release with archives + `SHA256SUMS`.
4. Rewrites `Formula/skillsdash.rb` and commits it to `main`.
5. Fills `packaging/aur/PKGBUILD` and pushes `skillsdash-bin` to the AUR.

## Build targets

| target | platform |
| --- | --- |
| `aarch64-apple-darwin` | macOS Apple Silicon |
| `x86_64-apple-darwin` | macOS Intel |
| `x86_64-unknown-linux-gnu` | Linux x86_64 (glibc) |
| `aarch64-unknown-linux-gnu` | Linux arm64 (glibc) |
| `x86_64-unknown-linux-musl` | Linux x86_64 (static) |

## Required GitHub secrets

Set these in **Settings → Secrets and variables → Actions**:

| secret | purpose |
| --- | --- |
| `RELEASE_PAT` | GitHub token to commit the version bump + formula and push tags. Classic token with `repo` scope, or fine-grained with **Contents: read/write** on this repo. |
| `AUR_SSH_PRIVATE_KEY` | Private SSH key whose public half is registered on your AUR account. Used to push the AUR package. |
| `AUR_USERNAME` | Git commit author name for the version-bump, formula, and AUR commits. |
| `AUR_EMAIL` | Git commit author email (use the email tied to your AUR account). |

The built-in `GITHUB_TOKEN` creates the Release itself — no extra secret needed.

## One-time AUR setup (before the first release)

1. Create an account at <https://aur.archlinux.org>.
2. Add your **SSH public key** under *My Account → SSH Public Key*. The matching
   **private** key goes into the `AUR_SSH_PRIVATE_KEY` secret.
3. Nothing else is required — the AUR auto-creates the `skillsdash-bin` package
   repository on the first valid push.

## First-run checklist

- [ ] All four secrets set.
- [ ] AUR SSH public key registered on your AUR account.
- [ ] Default branch is `main`.
- [ ] Run the **Release** workflow with a version, e.g. `0.1.0`.

After the first successful run, verify:

- The GitHub Release has 5 `.tar.gz` archives + `.sha256` files + `SHA256SUMS`.
- `Formula/skillsdash.rb` on `main` has the real version and four sha256 values.
- `https://aur.archlinux.org/packages/skillsdash-bin` exists.
- `curl -fsSL .../install.sh | sh` installs the new version.

## Notes

- The Homebrew tap lives in this repo, so `brew tap saeedvaziry/skillsdash <url>`
  works without a separate `homebrew-*` repo.
- The AUR package is prebuilt (`skillsdash-bin`) — it downloads the release
  binary and verifies its checksum; users don't need a Rust toolchain.
- Re-running the workflow with an existing version fails fast (the tag check).
