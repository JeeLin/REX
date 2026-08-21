# 进程模型

REX Hub 和 REX Agent 使用相同的运行时模型：单个可执行文件启动后，父进程作为 supervisor，启动 1 个 worker 子进程负责业务逻辑。

---

## 入口逻辑

```rust
fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.contains(&"--worker".to_string()) {
        // worker 模式：直接运行业务逻辑
        run_worker();
    } else {
        // supervisor 模式：启动 worker 并监控
        run_supervisor();
    }
}
```

## supervisor 循环

```rust
fn run_supervisor() {
    loop {
        // 1. 启动 worker
        let mut child = Command::new(std::env::current_exe().unwrap())
            .arg("--worker")
            .spawn()
            .expect("failed to start worker");

        // 2. 等待 worker 退出
        let exit_status = child.wait().expect("worker wait failed");
        let code = exit_status.code().unwrap_or(12);

        // 3. 读取 update-state.json
        let state = read_update_state();

        // 4. 根据退出码和状态判断下一步
        match code {
            0 => {
                if state.phase == "requested" {
                    replace_binary(&state.staged_path);
                    clean_rollback(&state.rollback_path);
                }
                thread::sleep(Duration::from_secs(1));
            }
            10 => {
                if state.phase == "requested" {
                    replace_binary(&state.staged_path);
                    write_update_state(UpdateState {
                        phase: "starting_new".to_string(),
                        attempt: 0,
                        ..Default::default()
                    });
                }
            }
            11 | 12 => {
                if state.attempt >= 3 {
                    rollback(&state.rollback_path);
                } else {
                    increment_attempt();
                }
            }
            _ => {
                thread::sleep(Duration::from_secs(1));
            }
        }
    }
}
```

## worker 模式

```rust
fn run_worker() {
    // 检查是否处于更新验证阶段
    if env::var("REX_UPDATE_PENDING").is_ok() {
        // 跳过更新检查，直接进入健康检查
        run_health_check();
        return;
    }

    // 正常业务逻辑
    start_business();

    // 定期检查更新
    loop {
        check_for_update();
        sleep(Duration::from_secs(3600)); // 每 24 小时
    }
}
```

---

## 退出码语义

| 退出码 | 含义 | supervisor 行为 |
|--------|------|----------------|
| `0` | 正常退出 | 检查 update-state.json，决定是否替换二进制 |
| `10` | 请求更新 | 读取 update-state.json，替换二进制，启动新版 |
| `11` | 健康检查失败 | attempt + 1，连续 3 次回滚 |
| `12` | 崩溃/异常退出 | attempt + 1，连续 3 次回滚 |

supervisor 决策逻辑：

| 退出码 | update-state.phase | attempt | 动作 |
|--------|-------------------|---------|------|
| 0 | idle | any | 重启 worker |
| 0 | requested | any | 替换二进制，启动新版 |
| 10 | requested | < 3 | 替换二进制，启动新版 |
| 10 | requested | >= 3 | 回滚 |
| 11 | any | < 3 | attempt + 1，重启 |
| 11 | any | >= 3 | 回滚 |
| 12 | any | < 3 | attempt + 1，重启 |
| 12 | any | >= 3 | 回滚 |

---

## Windows 差异

### 问题

- Windows 不支持 POSIX `fork + exec`
- 运行中的 `.exe` 文件通常被锁定，不能直接替换

### 解决方案

```text
用户启动 rex-hub.exe
  ↓
首次启动时复制自身到 data/rex-supervisor.exe
  ↓
rex-supervisor.exe 常驻运行（supervisor 角色）
  ↓
rex-supervisor.exe 启动 data/rex-worker.exe（worker 角色）
  ↓
更新时：
  rex-supervisor.exe 下载新版 → data/rex-worker.new.exe
  停止旧 worker.exe → 替换为新 worker.exe → 启动新 worker.exe
  supervisor 副本不变，不需要在运行时替换自身
```

### 对用户的入口

仍然是单个命令：`rex-hub.exe` 或 `rex-agent.exe`

首次启动后，用户会看到 data 目录下多了 `rex-supervisor.exe` 和 `rex-worker.exe`。
