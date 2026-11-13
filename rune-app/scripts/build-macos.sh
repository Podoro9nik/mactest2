#!/usr/bin/env bash
set -euo pipefail
APP_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION=4.1.0
SKIP_BUILD=false
SKIP_TESTS=false
SKIP_SMOKE=false
for argument in "$@"; do
  case "$argument" in
    --skip-build) SKIP_BUILD=true ;;
    --skip-tests) SKIP_TESTS=true ;;
    --skip-smoke) SKIP_SMOKE=true ;;
    *) printf 'Unknown argument: %s\n' "$argument" >&2; exit 2 ;;
  esac
done
[[ "$(uname -s)" == Darwin ]] || { echo 'Run this packaging script on macOS.' >&2; exit 1; }
case "$(uname -m)" in
  arm64) ARCH=arm64 ;;
  x86_64) ARCH=x64 ;;
  *) echo 'Unsupported macOS architecture.' >&2; exit 1 ;;
esac
export CARGO_TARGET_DIR="$APP_ROOT/target"
export MACOSX_DEPLOYMENT_TARGET=13.0
cd "$APP_ROOT"
if [[ "$SKIP_BUILD" == false ]]; then
  if [[ -z "${CHIIKAWA_EMBEDDED_MCP_KEY:-}" && -f "$APP_ROOT/../.env" ]]; then
    while IFS= read -r line || [[ -n "$line" ]]; do
      if [[ "$line" =~ ^[[:space:]]*(CHIIKAWA_EMBEDDED_MCP_KEY|ELYUM_MCP_KEY)[[:space:]]*=[[:space:]]*(.*)$ ]]; then
        build_key="${BASH_REMATCH[2]%$'\r'}"
        build_key="${build_key%"${build_key##*[![:space:]]}"}"
        build_key="${build_key#\"}"; build_key="${build_key%\"}"
        build_key="${build_key#\'}"; build_key="${build_key%\'}"
        export CHIIKAWA_EMBEDDED_MCP_KEY="$build_key"
        unset build_key
        break
      fi
    done < "$APP_ROOT/../.env"
  fi
  [[ -n "${CHIIKAWA_EMBEDDED_MCP_KEY:-}" ]] || { echo 'Set CHIIKAWA_EMBEDDED_MCP_KEY before creating the personal installer.' >&2; exit 1; }
  [[ -f vendor/rune/crates/rune/Cargo.toml ]] || { echo 'Pinned Rune checkout missing at vendor/rune.' >&2; exit 1; }
  SDKROOT="$(xcrun --sdk macosx --show-sdk-path)"
  export SDKROOT
  [[ -d "$SDKROOT" ]] || { echo 'The macOS SDK could not be located.' >&2; exit 1; }
  # linker-flavor selects Darwin LLD, but does not locate Rust's bundled executable.
  RUST_SYSROOT="$(rustc +1.99.1 --print sysroot)"
  RUST_HOST="$(rustc +1.99.1 -vV | sed -n 's/^host: //p')"
  RUST_LLD="$RUST_SYSROOT/lib/rustlib/$RUST_HOST/bin/rust-lld"
  [[ -x "$RUST_LLD" ]] || { printf 'Bundled Rust linker is missing: %s\n' "$RUST_LLD" >&2; exit 1; }
  case "$ARCH" in
    arm64) export CARGO_TARGET_AARCH64_APPLE_DARWIN_LINKER="$RUST_LLD" ;;
    x64) export CARGO_TARGET_X86_64_APPLE_DARWIN_LINKER="$RUST_LLD" ;;
  esac
  "$RUST_LLD" -flavor darwin --version
  if [[ "$SKIP_TESTS" == false ]]; then cargo +1.99.1 test --release --locked -- --test-threads=1; fi
  cargo +1.99.1 build --release --locked
fi
BIN="$APP_ROOT/target/release/chiikawizator-rune"
[[ -f "$BIN" ]] || { echo 'The release executable is missing.' >&2; exit 1; }
STAGE="$(mktemp -d "$APP_ROOT/target/macos-bundle.XXXXXX")"
trap 'rm -rf -- "$STAGE"' EXIT
APP="$STAGE/Chiikawizator.app"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources" "$APP_ROOT/dist"
cp "$BIN" "$APP/Contents/MacOS/chiikawizator"
chmod 755 "$APP/Contents/MacOS/chiikawizator"
cp installer/Info.plist "$APP/Contents/Info.plist"
cp assets/icons/icon.icns "$APP/Contents/Resources/icon.icns"
cp vendor/rune/LICENSE "$APP/Contents/Resources/RUNE-LICENSE.txt"
cp assets/fonts/OFL-Yomogi.txt "$APP/Contents/Resources/OFL-Yomogi.txt"
cp assets/fonts/NOTICE.md "$APP/Contents/Resources/FONT-NOTICE.md"
plutil -lint "$APP/Contents/Info.plist"
codesign --force --deep --sign - "$APP"
codesign --verify --deep --strict "$APP"
if [[ "$SKIP_SMOKE" == false ]]; then
  # Long CI builds can leave the display asleep; native surfaces need an awake display.
  /usr/bin/caffeinate -d -i -u node scripts/verify-package.mjs "$APP/Contents/MacOS/chiikawizator" "$APP_ROOT/verification/macos-$ARCH"
fi
ln -s /Applications "$STAGE/Applications"
DMG="$APP_ROOT/dist/Chiikawizator_${VERSION}_macos-${ARCH}.dmg"
hdiutil create -ov -volname Chiikawizator -srcfolder "$STAGE" -format UDZO "$DMG"
hdiutil verify "$DMG"
shasum -a 256 "$DMG" > "$DMG.sha256"
printf 'Installer ready: %s\n' "$DMG"
