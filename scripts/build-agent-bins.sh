#!/bin/bash
# scripts/build-agent-bins.sh
# 交叉编译所有平台的 Agent 二进制，输出到 target/agent-bins/{os}/{arch}/
#
# 用法：
#   ./scripts/build-agent-bins.sh           # 编译所有平台
#   ./scripts/build-agent-bins.sh linux     # 只编译 Linux
#   ./scripts/build-agent-bins.sh linux/amd64  # 只编译指定平台

set -euo pipefail

OUT_DIR="target/agent-bins"

# {rust_target} → {os}/{arch}
declare -A PLATFORMS=(
    ["x86_64-unknown-linux-musl"]="linux/amd64"
    ["aarch64-unknown-linux-musl"]="linux/arm64"
    ["x86_64-pc-windows-msvc"]="windows/amd64"
)

# 过滤平台
filter="${1:-}"

for rust_target in "${!PLATFORMS[@]}"; do
    os_arch="${PLATFORMS[$rust_target]}"

    # 应用过滤
    if [ -n "$filter" ]; then
        case "$filter" in
            */*)
                [ "$os_arch" != "$filter" ] && continue
                ;;
            *)
                os_part="${os_arch%%/*}"
                [ "$os_part" != "$filter" ] && continue
                ;;
        esac
    fi

    echo "═══ Building for $rust_target → $os_arch ═══"

    cargo build --release --bin rex-agent --target "$rust_target"

    # 复制到统一输出目录
    mkdir -p "$OUT_DIR/$os_arch"

    src="target/$rust_target/release/rex-agent"
    dst="$OUT_DIR/$os_arch/rex-agent"

    # Windows 二进制加 .exe 后缀
    if [[ "$rust_target" == *"windows"* ]]; then
        src="${src}.exe"
        dst="${dst}.exe"
    fi

    cp "$src" "$dst"
    chmod +x "$dst"

    size=$(du -h "$dst" | cut -f1)
    echo "  → $dst ($size)"
done

echo ""
echo "═══ All agent binaries built ═══"
echo "Output: $OUT_DIR/"
find "$OUT_DIR" -type f -name "rex-agent*" | sort
