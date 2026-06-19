#!/bin/bash
# REX Hub 开发续接脚本
# 用法: bun run continue
set -e

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"

echo "=== REX Hub 开发续接 ==="
echo ""

# 1. Milestone 状态
echo "📋 当前里程碑状态:"
for f in "$ROOT"/docs/milestones/M*.md; do
  [ -f "$f" ] || continue
  name=$(basename "$f" .md)
  total=$(grep -cE '^\| [0-9]' "$f" 2>/dev/null || echo 0)
  done=$(grep -cE '^\| .* ✅' "$f" 2>/dev/null || echo 0)
  echo "  $name: $done/$total 子任务完成"
done
echo ""

# 2. Git 状态
echo "📊 Git 状态:"
cd "$ROOT"
git status --short
echo ""

# 3. 前端检查
echo "🔍 前端检查:"
cd "$ROOT/packages/rex-console-web"
bun run type-check 2>&1 | tail -3
echo ""

echo "=== 继续开发 ==="
echo "告诉 Claude: '按照开发规划继续开发'"
