# 缺陷池

| 提出版本 | 优先级 | 标题 | 来源 | 描述 |
|----------|--------|------|------|------|
| v0.71.3 | 🟢 | Rollup 编译警告 @vueuse/core #__PURE__ 注释 | 用户反馈 | bun run build 时 Rollup 报告 @vueuse/core/dist/index.js 中 /* #__PURE__ */ 注释位置无法被 Rollup 解析，会被自动移除。构建正常完成但有警告输出（第三方依赖，非项目代码问题） |
