# REX Hub 升级说明

## Docker 部署升级

### Hub

```bash
# 1. 备份数据目录
cp -r /path/to/data /path/to/data.bak

# 2. 拉取新版本镜像
docker pull ghcr.io/anthropics/rex-hub:v{version}

# 3. 重启容器（使用 docker-compose 或 docker run）
docker compose up -d hub

# 4. 验证
curl http://localhost:3000/healthz
```

### Agent

```bash
# 拉取新版本镜像
docker pull ghcr.io/anthropics/rex-agent:v{version}

# 重启容器
docker compose up -d agent
```

## 二进制部署升级

### Hub

```bash
# 1. 备份当前二进制
cp $(which rex-hub) /path/to/rex-hub.bak

# 2. 下载新版本
curl -L https://github.com/anthropics/rex/releases/download/v{version}/rex-hub-linux-amd64.zip -o rex-hub.zip
unzip rex-hub.zip

# 3. 替换二进制
mv rex-hub /usr/local/bin/rex-hub

# 4. 重启服务
sudo systemctl restart rex-hub

# 5. 验证
curl http://localhost:3000/healthz
```

### Agent

```bash
# 下载新版本
curl -L https://github.com/anthropics/rex/releases/download/v{version}/rex-agent-linux-amd64.zip -o rex-agent.zip
unzip rex-agent.zip

# 替换二进制
mv rex-agent /usr/local/bin/rex-agent

# 重启服务
sudo systemctl restart rex-agent
```

## 回滚

如果新版本有问题：

### Docker

```bash
# 回滚到旧版本镜像
docker pull ghcr.io/anthropics/rex-hub:v{old-version}
docker compose up -d hub
```

### 二进制

```bash
# 使用备份恢复
cp /path/to/rex-hub.bak /usr/local/bin/rex-hub
sudo systemctl restart rex-hub
```

## 备份建议

升级前建议备份：

1. **数据目录** — 包含数据库、配置文件、Agent 注册信息
2. **SQLite 数据库** — `hub.db`（如果使用 SQLite）
3. **配置文件** — `config.yaml` 或环境变量

```bash
# 完整备份示例
tar -czf rex-backup-$(date +%Y%m%d).tar.gz /path/to/data/
```
