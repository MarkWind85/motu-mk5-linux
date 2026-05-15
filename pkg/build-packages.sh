#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
OUT_DIR="$PROJECT_DIR/target/packages"
VERSION=$(grep '^version' "$PROJECT_DIR/Cargo.toml" | head -1 | sed 's/.*= "//;s/"//')

mkdir -p "$OUT_DIR"

echo "============================================"
echo "  Building motu-mk5 v${VERSION} packages"
echo "============================================"
echo ""

build_deb() {
    echo "[1/3] Building .deb (Debian/Ubuntu)..."
    docker build -t motu-mk5-deb -f "$SCRIPT_DIR/docker/Dockerfile.debian" "$PROJECT_DIR"
    docker run --rm -v "$OUT_DIR:/host" motu-mk5-deb sh -c "cp /out/*.deb /host/"
    echo "  -> $(ls "$OUT_DIR"/*.deb 2>/dev/null | xargs -n1 basename)"
    echo ""
}

build_rpm() {
    echo "[2/3] Building .rpm (Fedora)..."
    docker build -t motu-mk5-rpm -f "$SCRIPT_DIR/docker/Dockerfile.fedora" "$PROJECT_DIR"
    docker run --rm -v "$OUT_DIR:/host" motu-mk5-rpm sh -c "cp /out/*.rpm /host/"
    echo "  -> $(ls "$OUT_DIR"/*.rpm 2>/dev/null | xargs -n1 basename)"
    echo ""
}

build_arch() {
    echo "[3/3] Building .pkg.tar.zst (Arch)..."
    docker build -t motu-mk5-arch -f "$SCRIPT_DIR/docker/Dockerfile.arch" "$PROJECT_DIR"
    docker run --rm -v "$OUT_DIR:/host" motu-mk5-arch sh -c "cp /out/*.pkg.tar.zst /host/"
    echo "  -> $(ls "$OUT_DIR"/*.pkg.tar.zst 2>/dev/null | xargs -n1 basename)"
    echo ""
}

case "${1:-all}" in
    deb)  build_deb ;;
    rpm)  build_rpm ;;
    arch) build_arch ;;
    all)
        build_deb
        build_rpm
        build_arch
        ;;
    *)
        echo "Usage: $0 [deb|rpm|arch|all]"
        exit 1
        ;;
esac

echo "============================================"
echo "  Packages in: $OUT_DIR"
echo "============================================"
ls -lh "$OUT_DIR"/*"$VERSION"* 2>/dev/null || echo "  (no packages found)"
