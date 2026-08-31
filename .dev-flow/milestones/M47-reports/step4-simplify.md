# 代码精简：M47 i18n 全面补全

## 检查范围

本次变更涉及：
- `packages/rex-console-web/src/i18n/locales/zh.json`：添加 `settings.langEn` / `settings.langZh`
- `packages/rex-console-web/src/i18n/locales/en.json`：添加 `settings.langEn` / `settings.langZh`
- `packages/rex-console-web/src/pages/SettingsPage.vue`：语言选择器使用 `t()` 替代硬编码

## 精简结论

- **无需精简**：变更仅为添加 2 个 i18n key 和更新 1 处模板绑定，代码已符合项目现有风格
- **功能未改变**：语言选择器行为不变，仅从硬编码字符串改为 i18n 调用
