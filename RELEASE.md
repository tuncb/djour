# Release Process

This document describes how to create a new release of djour.

## Prerequisites

- Push access to the repository
- All tests passing on main branch
- CHANGELOG updated (if you have one)

## Creating a Release

1. **Ensure main branch is ready**
   ```bash
   git checkout main
   git pull origin main
   cargo test --all
   cargo clippy --all-targets --all-features -- -D warnings
   ```

2. **Update version in Cargo.toml** (if needed)
   ```toml
   [package]
   name = "djour"
   version = "0.1.0"  # Update this version
   ```

3. **Commit version bump** (if you changed it)
   ```bash
   git add Cargo.toml Cargo.lock
   git commit -m "Bump version to v0.1.0"
   git push origin main
   ```

4. **Create and push a version tag**
   ```bash
   # Create a lightweight tag
   git tag v0.1.0

   # Or create an annotated tag with a message (recommended)
   git tag -a v0.1.0 -m "Release v0.1.0"

   # Push the tag to trigger the release workflow
   git push origin v0.1.0
   ```

5. **Monitor the release build**
   - Go to: https://github.com/YOUR_USERNAME/djour/actions
   - Watch the "Release" workflow run
   - It will build for 4 platforms:
     - Linux (x86_64)
     - macOS (Intel x86_64)
     - macOS (Apple Silicon aarch64)
     - Windows (x86_64)

6. **Verify the release**
   - Go to: https://github.com/YOUR_USERNAME/djour/releases
   - Check that the release was created with:
     - All 4 platform archives (.tar.gz for Unix, .zip for Windows)
     - SHA256 checksums for each archive
     - Auto-generated release notes
     - Installation instructions

## What Gets Released

Each release includes archives for each platform containing:
- `djour` or `djour.exe` - The compiled executable
- `README.md` - User documentation
- `LICENSE` - MIT license

Archive naming format: `djour-v0.1.0-<target>.tar.gz` or `.zip`

Examples:
- `djour-v0.1.0-x86_64-unknown-linux-gnu.tar.gz`
- `djour-v0.1.0-x86_64-apple-darwin.tar.gz`
- `djour-v0.1.0-aarch64-apple-darwin.tar.gz`
- `djour-v0.1.0-x86_64-pc-windows-msvc.zip`

## Version Numbering

Follow [Semantic Versioning](https://semver.org/):
- **MAJOR.MINOR.PATCH** (e.g., 1.2.3)
- **MAJOR**: Breaking changes
- **MINOR**: New features, backwards compatible
- **PATCH**: Bug fixes, backwards compatible

Examples:
- `v0.1.0` - Initial release
- `v0.1.1` - Bug fix
- `v0.2.0` - New feature
- `v1.0.0` - First stable release

## Troubleshooting

### Release workflow didn't trigger
- Verify the tag format starts with `v` (e.g., `v0.1.0`)
- Check you pushed the tag: `git push origin v0.1.0`
- Verify the workflow file exists: `.github/workflows/release.yml`

### Build failed on a platform
- Check the Actions logs for that specific platform
- Common issues:
  - Test failures (fix tests first)
  - Missing dependencies for cross-compilation
  - Target not installed (the workflow installs it automatically)

### Delete and recreate a release
```bash
# Delete local tag
git tag -d v0.1.0

# Delete remote tag
git push origin :refs/tags/v0.1.0

# Delete the GitHub release manually in the web UI

# Create new tag
git tag -a v0.1.0 -m "Release v0.1.0"
git push origin v0.1.0
```

## Testing a Release Locally

Before creating a real release, you can test the packaging locally:

```bash
# Build for your current platform
cargo build --release

# Create a test staging directory
mkdir -p djour-test/djour-v0.1.0-test
cp target/release/djour djour-test/djour-v0.1.0-test/  # or djour.exe on Windows
cp README.md LICENSE djour-test/djour-v0.1.0-test/

# Create archive (Linux/macOS)
cd djour-test
tar czf djour-v0.1.0-test.tar.gz djour-v0.1.0-test/

# Create archive (Windows PowerShell)
Compress-Archive -Path djour-v0.1.0-test -DestinationPath djour-v0.1.0-test.zip

# Generate checksum (Linux/macOS)
shasum -a 256 djour-v0.1.0-test.tar.gz > djour-v0.1.0-test.tar.gz.sha256

# Generate checksum (Windows PowerShell)
(Get-FileHash djour-v0.1.0-test.zip).Hash.ToLower() + "  djour-v0.1.0-test.zip" | Out-File -FilePath djour-v0.1.0-test.zip.sha256 -Encoding ASCII

# Test extraction
tar xzf djour-v0.1.0-test.tar.gz
./djour-v0.1.0-test/djour --version
```
