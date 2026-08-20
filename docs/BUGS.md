# 缺陷池

| 提出版本 | 优先级 | 标题 | 来源 | 描述 |
|----------|--------|------|------|------|
| 0.70.4 | 🟢 | 移除 load_sip_conn 顶层 host 回退逃生舱 | 步骤5代码审查 | 模型已声明 server 完全下沉账户，但 load_sip_conn 仍保留 info.host 回退；仅 legacy/异常 payload 触发，移除属行为变更，待后续 |
| 0.70.4 | 🟢 | 抽出共享 SipProfile TS 类型与 active/first 解析规则 | 步骤5代码审查 | SipProfile 形状与 "active 或 first" 在 load_sip_conn / SipPage.parseSipProfile / WizardModal.buildConfig 三处镜像，存在类型/逻辑漂移风险 |
| 0.70.4 | 🟢 | selectAccount 减少多余 GET 往返 | 步骤5代码审查 | 切换账户先 resourcesApi.get 再 update，后端 update_resource 要求全字段非根因；引入专用 set_active_account 端点后消除 ||
