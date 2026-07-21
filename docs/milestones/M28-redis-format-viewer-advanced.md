# M28: Redis FormatViewer 高级格式解码

## Context

M27 完成了 bug 修复和 UX 完善。当前 Redis FormatViewer 仅支持 4 种基础格式（Text/Hex/JSON/Binary），全部在前端检测。PRODUCT.md 3.9 要求 FormatViewer 支持 Msgpack / PHPSerialize / JavaSerialize / Pickle / 压缩格式的自动探测与解码。本里程碑补全这些高级格式支持。

版本类型：minor（新功能），版本号 0.26.1 → 0.27.0。

## 产品边界

**本阶段做：**
- Rust 后端：新增 `redis-codec` 解码器模块，支持 Msgpack / PHPSerialize / JavaSerialize / Pickle / 压缩格式检测与解码
- 后端 API：`get_value` 返回格式元数据（detected_format + decoded_preview）
- 前端 FormatViewer：扩展支持新格式标签、自动探测、解码结果显示

**本阶段不做：**
- 格式编码（只做解码/探测，写入仍用原始字符串）
- Snappy / LZ4 等实时压缩格式（仅支持 zlib/gzip/zstd 压缩检测）
- 自定义序列化格式扩展机制

## 子任务清单

| # | 内容 | 状态 |
|---|------|------|
| 1 | Rust redis-codec 解码器模块（Msgpack/Pickle/PHP/Java/压缩） | ⬜ |
| 2 | 后端 API 扩展（get_value 返回格式元数据） | ⬜ |
| 3 | 前端 FormatViewer 扩展（新格式标签 + 自动探测） | ⬜ |

## 子任务详细设计

### 1 Rust redis-codec 解码器模块

**功能目标**

在 `rex-common` 中新增 `redis_codec` 模块，提供 Redis 值的格式自动探测和解码能力。支持 Msgpack、PHPSerialize、JavaSerialize、Pickle 五种序列化格式和 zlib/gzip/zstd 三种压缩格式。

**文件结构**

修改：
- `crates/rex-common/Cargo.toml` — 添加解码依赖（rmp-serde, flate2, zstd）
- `crates/rex-common/src/lib.rs` — 添加 `pub mod redis_codec;`

新建：
- `crates/rex-common/src/redis_codec.rs` — 格式检测 + 解码逻辑

**格式检测规则（按优先级）**

| 格式 | 检测特征 | 说明 |
|------|----------|------|
| Msgpack | 首字节在 `0x00-0x7f`（正整数）或 `0xc0-0xdf`（容器）或 `0xa0-0xbf`（str）等范围 | MessagePack 二进制格式，需校验结构完整性 |
| PHPSerialize | 首字节为 `a`（array）、`O`（object）、`s`（string）、`i`（int）、`b`（bool）、`N`（null）、`d`（double） | `a:N:{...}` 或 `O:N:"name":N:{...}` |
| JavaSerialize | 首 2 字节为 `0xAC 0xED`（STREAM_MAGIC） | Java 序列化流 magic bytes |
| Pickle | 首字节为 `(`（PROTO）或 `\x80`（PROTO OP） | Python pickle 协议 |
| 压缩（zlib） | 首字节 `0x78`（CMF）+ 校验 | zlib/gzip/zstd 压缩数据 |
| 压缩（gzip） | 首 2 字节 `0x1f 0x8b` | gzip 压缩数据 |
| 压缩（zstd） | 首 4 字节 `0x28 0xB5 0x2F 0xFD` | Zstandard 压缩数据 |

**检测流程**

```
1. 尝试检测压缩格式（zlib/gzip/zstd）
   → 如果是压缩格式，先解压，再对解压结果递归检测序列化格式
2. 尝试检测序列化格式（Msgpack → Pickle → PHP → Java）
   → 按特征匹配，命中即返回
3. 尝试 JSON.parse
   → 成功返回 JSON
4. 检查是否包含不可打印字符
   → 有则标记为 Binary
5. 默认返回 Text
```

**解码结果**

```rust
pub enum DecodedFormat {
    Text,
    Hex,
    Json { pretty: String },
    Binary,
    Msgpack { pretty: String },
    PhpSerialize { pretty: String },
    JavaSerialize { pretty: String },
    Pickle { pretty: String },
    Compressed { algorithm: String, decoded: Box<DecodedFormat> },
}

pub struct FormatDetection {
    pub format: DecodedFormat,
    pub format_name: String,    // "msgpack", "php_serialize", etc.
    pub raw_size: usize,
}

pub fn detect_and_decode(bytes: &[u8]) -> FormatDetection { ... }
```

**依赖选择**

| 格式 | Rust crate | 说明 |
|------|-----------|------|
| Msgpack | `rmp-serde` | Rust MessagePack，`rmpv` 用于任意值解码 |
| PHPSerialize | 自实现 | PHP 序列化格式简单，手写解析器（~100 行） |
| JavaSerialize | `binread` 或自实现 | Java 序列化格式解析，手写基础解码器 |
| Pickle | 自实现（基础协议） | 支持协议 0-4 的常用 opcode（字符串/整数/列表/字典） |
| zlib/gzip | `flate2` | 成熟的压缩解压库 |
| zstd | `zstd` | Zstandard 解压 |

**测试标准**

- 每种格式至少 2 个测试用例（典型值 + 边界值）
- 压缩格式：压缩→解压→检测序列化格式（嵌套场景）
- 未知格式优雅降级为 Text/Binary
- `cargo test --workspace` 通过

**提交信息**: `feat(redis): add redis-codec decoder for Msgpack/Pickle/PHP/Java/compressed formats`

### 2 后端 API 扩展（get_value 返回格式元数据）

**功能目标**

修改 Redis `get_value` 响应，在返回值的同时包含格式检测信息，让前端无需重复实现解码逻辑。

**文件结构**

修改：
- `crates/rex-common/src/redis.rs` — `RedisValue::String` 扩展为包含格式元数据
- `crates/rex-redis/src/lib.rs` — `get_value` 调用 `redis_codec::detect_and_decode`

**接口设计**

当前 `get_value` 返回 `RedisValue::String(String)`。修改为：

```rust
pub enum RedisValue {
    String {
        value: String,
        format: Option<FormatInfo>,  // 格式检测结果
    },
    List(Vec<String>),
    Set(Vec<String>),
    ZSet(Vec<(String, f64)>),
    Hash(Vec<(String, String)>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatInfo {
    pub detected: String,      // "text" | "json" | "hex" | "binary" | "msgpack" | "php_serialize" | "java_serialize" | "pickle" | "compressed"
    pub decoded: Option<String>, // 解码后的可读文本（如果有）
    pub compression: Option<String>, // 如果是压缩格式，显示压缩算法
}
```

**后端流程**

1. `get_value` 获取到 String 值后，取原始字节（`val.as_bytes()`）
2. 调用 `redis_codec::detect_and_decode(bytes)` 获取 `FormatDetection`
3. 构造 `FormatInfo` 填入 `RedisValue::String { value, format }`

**测试标准**

- String 类型值返回包含 `format` 字段
- Msgpack 编码的值返回 `detected: "msgpack"` + `decoded` 预览
- 普通文本值返回 `detected: "text"`，`decoded: null`
- `cargo test --workspace` 通过

**提交信息**: `feat(redis): add format metadata to get_value response`

### 3 前端 FormatViewer 扩展

**功能目标**

FormatViewer 支持新格式的标签显示、自动探测和解码结果展示。

**文件结构**

修改：
- `packages/rex-console-web/src/features/redis/FormatViewer.vue` — 扩展格式标签和检测逻辑

**交互设计**

当前 FormatViewer 有 4 个标签：Text / Hex / JSON / Binary。

修改后：

1. **接收格式元数据**：props 新增 `formatInfo?: { detected: string, decoded?: string, compression?: string }`
2. **标签列表动态化**：根据 `formatInfo.detected` 添加对应标签
   - text → Text（默认激活）
   - json → JSON
   - hex → Hex
   - binary → Binary
   - msgpack → Msgpack（显示解码后的 pretty JSON）
   - php_serialize → PHP
   - java_serialize → Java
   - pickle → Pickle（显示解码后的 pretty JSON）
   - compressed → 先显示压缩信息（如 "zlib → msgpack"），再展开解码结果
3. **自动激活检测到的格式**：如果检测到非 text 格式，默认激活该标签
4. **解码结果显示**：高级格式标签的内容区显示 `formatInfo.decoded`（已格式化的 JSON 或文本）
5. **格式标签样式**：新格式标签使用不同颜色区分（如 Msgpack=蓝色，PHP=紫色，Java=橙色，Pickle=绿色）

**组件 Props 变更**

```typescript
const props = defineProps<{
  value: string
  formatInfo?: {
    detected: string      // 格式名
    decoded?: string      // 解码后的文本
    compression?: string  // 压缩算法名
  }
}>()
```

**标签配置扩展**

```typescript
const FORMAT_META: Record<string, { label: string; color: string }> = {
  text: { label: 'Text', color: '' },
  hex: { label: 'Hex', color: '' },
  json: { label: 'JSON', color: '' },
  binary: { label: 'Binary', color: '' },
  msgpack: { label: 'Msgpack', color: '#58A6FF' },
  php_serialize: { label: 'PHP', color: '#8B5CF6' },
  java_serialize: { label: 'Java', color: '#D29922' },
  pickle: { label: 'Pickle', color: '#3FB950' },
  compressed: { label: 'Compressed', color: '#F85149' },
}
```

**显示逻辑**

- Text/Hex/JSON/Binary：使用现有 `displayValue` computed（前端本地转换）
- Msgpack/PHP/Java/Pickle：使用 `formatInfo.decoded`（后端已解码）
- Compressed：显示压缩算法标签 + 解码结果

**测试标准**

- 普通文本值：显示 Text 标签（默认），行为不变
- JSON 值：显示 JSON 标签（默认），行为不变
- Msgpack 编码值：显示 Msgpack 标签，内容为解码后的 JSON
- PHP 序列化值：显示 PHP 标签，内容为解码后的结构
- 无 formatInfo 时：降级为现有行为（纯前端检测）
- type-check + build 通过

**提交信息**: `feat(redis): extend FormatViewer with Msgpack/PHP/Java/Pickle format tabs`

## 设计核对点

- ✅ 符合产品定位（单用户、自托管）
- ✅ 不引入新概念（纯功能增强）
- ✅ 不跳阶段实现
- ✅ 实现细节不污染产品文档
- ✅ 与 ARDM 对标（FormatViewer 自动探测 + 多格式支持）

## Flow Status

- [ ] 步骤1：编写里程碑文档
- [ ] 步骤2：设计核对
- [ ] 步骤3：开发
- [ ] 步骤4：代码精简
- [ ] 步骤5：代码审查
- [ ] 步骤6：测试验证
- [ ] 步骤7：设计再确认
- [ ] 步骤8：提交

## 打回记录

（打回时追加一条，创建里程碑时留空）

| 时间 | 步骤 | 原因 |
|------|------|------|
| | | |
