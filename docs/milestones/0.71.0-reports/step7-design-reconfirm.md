# Step 7: 设计再确认报告

## 实现 vs 里程碑文档对比

### 子任务 1：i18n 基础设施

| 设计项 | 实现 | 一致 |
|--------|------|------|
| 创建 src/i18n/index.ts | ✅ 已有，添加 t() 函数 | ✅ |
| 导出 i18n 实例 | ✅ export const i18n | ✅ |
| 导出 t() 函数 | ✅ export function t() | ✅ |
| t() 读取当前语言 | ✅ 通过 i18n.global.t() | ✅ |

### 子任务 2：api/client.ts

| 设计项 | 实现 | 一致 |
|--------|------|------|
| 导入 t() | ✅ import { t } from '@/i18n' | ✅ |
| 429 错误使用 t() | ✅ t('api.error.rateLimit') | ✅ |
| 5xx 错误使用 t() | ✅ t('api.error.serverError', { status }) | ✅ |
| 超时错误使用 t() | ✅ t('api.error.timeout') | ✅ |
| 网络错误使用 t() | ✅ t('api.error.network') | ✅ |

### 子任务 3：GlobalQueryModal.vue

| 设计项 | 实现 | 一致 |
|--------|------|------|
| 添加 useI18n | ✅ const { t } = useI18n() | ✅ |
| 替换所有硬编码中文 | ✅ 12+ 处已替换 | ✅ |

### 子任务 4：WorkspaceSql.vue

| 设计项 | 实现 | 一致 |
|--------|------|------|
| 使用 t() 替换硬编码 | ✅ 8 处已替换 | ✅ |
| 新增 sql.shortcutHint 键 | ✅ 已添加 | ✅ |

### 子任务 5：ProfileSection.vue 和 TabBar.vue

| 设计项 | 实现 | 一致 |
|--------|------|------|
| ProfileSection: confirmPassword | ✅ 使用 t('settings.profile.confirmPassword') | ✅ |
| TabBar: title 属性 | ✅ 使用 t('ws.tab.newConnection') | ✅ |

## 设计核对点验证

| 检查项 | 结果 |
|--------|------|
| 不引入新的外部依赖 | ✅ 使用已有 vue-i18n |
| i18n 实例可独立导入 | ✅ client.ts 成功使用 |
| 语言切换时 API 错误消息跟随切换 | ✅ 通过 i18n.global.t() |
| 不修改现有功能行为 | ✅ 仅替换显示文本 |

## 结论

✅ 设计再确认通过。所有实现与里程碑文档一致。
