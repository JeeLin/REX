# 步骤5：代码审查报告

## 审查范围

`git diff HEAD~2..HEAD`，5 个代码文件，289 行新增。

## 审查维度

### 1. 正确性

| 发现 | 严重度 | 文件 | 说明 |
|------|--------|------|------|
| `keyData` 字段未使用 | 🟡 | ResourceNew.vue | sshConfig.keyData 在 handleKeyFile 中赋值，但 submitResource 和 buildConfigJson 从未读取。实际提交时 key 内容通过 FileReader 存入 keyFile 字段，keyData 是冗余的 |
| 上传端点未对接前端 | 🟡 | resource.rs + ResourceNew.vue | POST /ssh-key 端点已实现，但前端创建资源时未调用。当前设计：密钥内容直接存入 config_json（由现有 config encryption 加密），上传端点成为孤立代码 |
| 密钥格式检测 | 🟢 | resource.rs | `detect_key_format` 通过文本内容嗅探格式，对二进制 PPK 文件可能误判。但 SSH key 文件通常有文本头，影响有限 |

### 2. 安全性

| 发现 | 严重度 | 文件 | 说明 |
|------|--------|------|------|
| 文件大小限制 | ✅ | resource.rs | 64KB 限制合理 |
| 文件权限 | ✅ | resource.rs | Unix 下设置 0600，正确 |
| 路径安全 | ✅ | resource.rs | 使用 resource_id 作为文件名，无路径遍历风险 |
| 格式验证 | ✅ | resource.rs | 上传前检测格式，拒绝未知格式 |

### 3. 架构一致性

| 发现 | 严重度 | 文件 | 说明 |
|------|--------|------|------|
| 密钥存储路径一致 | 🟢 | resource.rs | `{data_dir}/keys/{resource_id}.key` 路径结构合理，但与 config_json 中 private_key_path 机制并行存在两套密钥存储方式 |

### 4. 错误处理

| 发现 | 严重度 | 文件 | 说明 |
|------|--------|------|------|
| 错误消息完整 | ✅ | resource.rs | 各错误路径返回明确的中文错误信息 |
| 前端静默处理 | 🟢 | ResourceNew.vue | handleKeyFile 无格式校验提示，用户可上传任意文件 |

### 5. 代码质量

| 发现 | 严重度 | 文件 | 说明 |
|------|--------|------|------|
| css 按功能域组织 | ✅ | ResourceNew.vue | 新增 CSS 类结构清晰，与现有样式一致 |
| 函数拆分合理 | ✅ | ResourceNew.vue | triggerKeyUpload/handleKeyFile/removeKeyFile/formatSize 各司其职 |

## 严重度汇总

| 🔴 必须修复 | 0 |
|-------------|---|
| 🟡 应该修复 | 2 |
| 🟢 可选改进 | 3 |

## 🟡 应该修复项详情

### Y1: keyData 冗余字段
**文件**: ResourceNew.vue
**问题**: sshConfig.keyData 存储 File 对象但从未在提交流程中使用
**建议**: 移除 keyData 字段，仅保留 keyFileName/keySize 用于 UI 展示

### Y2: 上传端点未对接
**文件**: resource.rs + ResourceNew.vue
**问题**: POST /ssh-key 端点已实现但前端未调用。当前创建资源时密钥内容直接存入 config_json
**影响**: 上传端点成为孤立代码。但 config_json + config encryption 方案对单用户场景足够，上传端点可留作后续密钥管理功能使用
**建议**: 在 ResourceNew 提交时调用上传端点，或标注此端点为预留 API

## 结论

✅ **无 🔴 必须修复项**。代码功能正确、安全。建议清理 Y1 冗余字段。Y2 可保留上传端点作为预留 API，不影响当前功能。通过，可进入步骤6。
