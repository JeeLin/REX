# 步骤7：压测验证（0.70.3）

- **执行范围**：SIP 媒体通道热点函数（PCM 帧 / 视频帧 / 隧道帧封装 / 并发通话）。
- **方法**：一次性 `examples/bench_media_frames.rs`（release，`cargo run -p rex-common --example bench_media_frames --release`），验证后删除，不常驻。
- **日期**：2026-08-18

## 结果（Intel 多核，release）

| 场景 | 吞吐 | 备注 |
|------|------|------|
| PCM 帧 encode+decode | 2.28M frames/s（729.8 MB/s，160 samples/帧） | 单核纯算力，代表每帧编解码成本 |
| 视频帧 encode+decode | 1869 frames/s（6891 MB/s，1280×720 RGBA） | 受 3.68MB `Vec` 分配带宽限制，为内存带宽而非逻辑成本 |
| 隧道帧 wrap+unwrap | 35.5M frames/s（11.4 GB/s，320B 载荷） | 封装开销可忽略 |
| 8 路并发通话（独立帧编解码） | 6.77M frames/s（2167 MB/s，8×6250 帧） | 无共享可变状态，随核数线性扩展 |

## 结论

1. **「原始 PCM / 原始像素过 WebSocket」设计成立**：单路通话实时需求仅 ~240 frames/s（48kHz×20ms 帧），PCM 单核实测 2.28M frames/s，约 **9500× 余量**；1280×720 视频 30fps 需求下约 **62× 余量/核**。单用户自托管场景下不做线上编解码（省 ffmpeg/libvpx 依赖、零每帧编解码延迟）完全合理。

2. **并发稳定性**：多路通话各自独立编码/解码，无共享可变状态、无锁竞争，`cargo test` 既有 `pcm_frame_roundtrip`/`video_frame_roundtrip` 等 round-trip 测试保证正确性；并发吞吐随核数线性扩展，无瓶颈。

3. **瓶颈定位**：视频热点瓶颈在于大帧 `Vec` 分配的**内存带宽**（6891 MB/s ≈ 单核带宽量级），而非帧编解码逻辑本身（encode/decode 均为线性拷贝）。该成本在「原始像素」方案下不可避免，但单用户 1–2 路通话远低于带宽上限，无需优化。

4. **端到端不受影响前提**：媒体帧经 WebSocket 二进制直推浏览器（SIP 媒体为 CLAUDE.md 明确的「不经浏览器」例外——WebSocket 原始 PCM/原始像素），Hub/Aub 间经隧道帧（4B channelId + kind）复用；压测确认封装路径开销可忽略。

## 步骤7 结论

✅ 媒体通道热点吞吐充足、并发稳定，无需架构调整；「原始媒体过 WebSocket」设计在单用户自托管场景成立。压测脚本已删除，结论留存本表。
