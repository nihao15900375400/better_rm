#!/usr/bin/env bash
set -euo pipefail

# ─── Configuration ───────────────────────────────────────────────────────────
CRATE_NAME="better-rm"
BIN_NAME="del"                          # 发布时重命名为 del
VERSION="$(grep '^version' Cargo.toml | head -1 | sed 's/.*= *"\(.*\)"/\1/')"
RELEASE_DIR="release"
PROFILE="release"

TARGETS=(
    "x86_64-unknown-linux-musl:amd64:x86_64"
    "aarch64-unknown-linux-musl:arm64:aarch64"
)

META_FILES=(
    "README.md"
    "README.zh-CN.md"
    "LICENSE"
)

# ─── Colours / helpers ───────────────────────────────────────────────────────
RED='\033[0;31m'; GREEN='\033[0;32m'; CYAN='\033[0;36m'; NC='\033[0m'
info()  { echo -e "${CYAN}[build]${NC} $*"; }
ok()    { echo -e "${GREEN}[ ✓  ]${NC} $*"; }
err()   { echo -e "${RED}[ ✗  ]${NC} $*"; exit 1; }

cleanup() {
    rm -rf "$RELEASE_DIR/_deb_tmp"
}
trap cleanup EXIT

# ─── Sanity checks ───────────────────────────────────────────────────────────
command -v cargo-zigbuild >/dev/null 2>&1 || err "cargo-zigbuild not installed — run: cargo install cargo-zigbuild"
command -v zig >/dev/null 2>&1          || err "zig not found — install from https://ziglang.org/download/"
command -v dpkg-deb >/dev/null 2>&1     || err "dpkg-deb not found (needed for .deb packaging)"

# ─── Build ───────────────────────────────────────────────────────────────────
mkdir -p "$RELEASE_DIR"

for entry in "${TARGETS[@]}"; do
    IFS=":" read -r rust_target deb_arch label <<< "$entry"

    info "Building for ${rust_target} (${label}) …"

    cargo zigbuild --target "$rust_target" --profile "$PROFILE" --quiet

    src_bin="target/${rust_target}/${PROFILE}/${CRATE_NAME}"
    if [ ! -f "$src_bin" ]; then
        info "  (trying without profile directory …)"
        src_bin="target/${rust_target}/${PROFILE}/${CRATE_NAME}"
        [ -f "$src_bin" ] || err "binary not found for ${rust_target}"
    fi

    # ── Binary ───────────────────────────────────────────────────────────
    bin_out="${RELEASE_DIR}/${BIN_NAME}_${label}"
    cp "$src_bin" "$bin_out"
    chmod +x "$bin_out"
    ok "Binary: ${bin_out}"

    # ── tar.xz (bin + README* + LICENSE) ─────────────────────────────────
    tar_name="${RELEASE_DIR}/${BIN_NAME}_${VERSION}_${label}.tar.xz"
    tar_dir="${BIN_NAME}_${VERSION}_${label}"
    tmp_tar="$(mktemp -d)"

    mkdir -p "${tmp_tar}/${tar_dir}"
    cp "$bin_out" "${tmp_tar}/${tar_dir}/${BIN_NAME}"
    for f in "${META_FILES[@]}"; do
        [ -f "$f" ] && cp "$f" "${tmp_tar}/${tar_dir}/"
    done

    tar -cJf "$tar_name" -C "$tmp_tar" "$tar_dir"
    rm -rf "$tmp_tar"
    ok "tar.xz: ${tar_name}"

    # ── .deb ──────────────────────────────────────────────────────────────
    deb_name="${RELEASE_DIR}/${BIN_NAME}_${VERSION}_${deb_arch}.deb"
    deb_root="$(mktemp -d)"

    # binary
    install_dir="${deb_root}/usr/bin"
    mkdir -p "$install_dir"
    cp "$bin_out" "${install_dir}/${BIN_NAME}"

    # control
    ctrl_dir="${deb_root}/DEBIAN"
    mkdir -p "$ctrl_dir"
    cat > "${ctrl_dir}/control" <<DEBCTRL
Package: ${BIN_NAME}
Version: ${VERSION}
Section: utils
Priority: optional
Architecture: ${deb_arch}
Maintainer: ywnh1 <ywnh1@outlook.com>
Description: A safe file deletion and trash management tool
 Instead of permanently removing files like rm, it archives them into
 a designated trash directory, records metadata in SQLite, and supports
 restore, query, and cleanup.
DEBCTRL

    # doc
    doc_dir="${deb_root}/usr/share/doc/${BIN_NAME}"
    mkdir -p "$doc_dir"
    for f in "${META_FILES[@]}"; do
        [ -f "$f" ] && cp "$f" "${doc_dir}/"
    done

    dpkg-deb --build --root-owner-group "$deb_root" "$deb_name" >/dev/null
    rm -rf "$deb_root"
    ok "deb:    ${deb_name}"

    echo
done

# ─── Summary ─────────────────────────────────────────────────────────────────
echo -e "${GREEN}══════════════════════════════════════════════════════════════${NC}"
info "All done!  Artifacts in ${CYAN}${RELEASE_DIR}/${NC}"
echo
ls -lh "${RELEASE_DIR}/" | grep -v '_deb_tmp'
