#!/usr/bin/env bash
# Builds the launcher, signs it and prepares everything for a release in ./deploy
#
#   ./deploy.sh
#
# Key path:      $SIGN_KEY (default: ./private.key)
# Key password:  $SIGN_KEY_PASSWORD, otherwise asked interactively (Enter = no password)
set -euo pipefail
cd "$(dirname "$0")"

REPO="Amirust/rfad-launcher"
EXE_NAME="rfad-launcher.exe"
KEY="${SIGN_KEY:-private.key}"
OUT="deploy"

die() { echo "ERROR: $*" >&2; exit 1; }

[ -f "$KEY" ] || die "private key not found: $KEY"

VERSION=$(sed -n 's/^[[:space:]]*"version":[[:space:]]*"\([^"]*\)".*/\1/p' src-tauri/tauri.conf.json | head -1)
CARGO_VERSION=$(sed -n 's/^version = "\([^"]*\)"/\1/p' src-tauri/Cargo.toml | head -1)
[ -n "$VERSION" ] || die "cannot read version from tauri.conf.json"
[ "$VERSION" = "$CARGO_VERSION" ] || die "version mismatch: tauri.conf.json=$VERSION, Cargo.toml=$CARGO_VERSION"

if [ -z "${SIGN_KEY_PASSWORD+x}" ]; then
  read -rsp "Signing key password: " SIGN_KEY_PASSWORD
  echo
fi

sign() {
  # Password goes through env so an empty one also works
  TAURI_PRIVATE_KEY_PASSWORD="$SIGN_KEY_PASSWORD" bunx tauri signer sign -f "$KEY" "$1" >/dev/null \
    || die "signing $1 failed (wrong password?)"
  [ -s "$1.sig" ] || die "signature $1.sig was not created"
}

echo "==> Checking key password"
TMP=$(mktemp)
echo test > "$TMP"
sign "$TMP"
rm -f "$TMP" "$TMP.sig"

echo "==> Running tests"
(cd src-tauri && cargo test --lib --quiet)

echo "==> Building v$VERSION"
bunx tauri build --no-bundle

echo "==> Preparing $OUT/"
rm -rf "$OUT"
mkdir -p "$OUT"
cp "src-tauri/target/release/$EXE_NAME" "$OUT/$EXE_NAME"

echo "==> Signing $EXE_NAME"
sign "$OUT/$EXE_NAME"

SHA256=$(sha256sum "$OUT/$EXE_NAME" | cut -d' ' -f1)
URL="https://github.com/$REPO/releases/download/v$VERSION/$EXE_NAME"

echo "==> Writing and signing lversions.json"
# The signature covers these exact bytes: don't edit the file after signing
printf '{\n  "version": "%s",\n  "downloadUrl": "%s",\n  "sha256": "%s"\n}\n' \
  "$VERSION" "$URL" "$SHA256" > "$OUT/lversions.json"
sign "$OUT/lversions.json"

# The manifest the launchers read lives in the repo root
cp "$OUT/lversions.json" "$OUT/lversions.json.sig" .

cat <<EOF

Done: v$VERSION
  $OUT/$EXE_NAME
  $OUT/$EXE_NAME.sig
  $OUT/lversions.json        (also copied to repo root)
  $OUT/lversions.json.sig    (also copied to repo root)
  sha256: $SHA256

Release order:
  1. GitHub release with tag v$VERSION -> upload $EXE_NAME and $EXE_NAME.sig
  2. Only after that commit and push lversions.json + lversions.json.sig to main
EOF
