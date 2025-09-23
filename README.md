# gewe-notice-mcp

<p align="center">
  <img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="MIT License"/>
  <img src="https://img.shields.io/github/v/release/wangnov/gewe-notice-mcp?label=release" alt="Release"/>
  <img src="https://img.shields.io/npm/v/gewe-notice-mcp.svg" alt="npm Version"/>
  <img src="https://img.shields.io/badge/rust-1.75%2B-orange.svg" alt="Rust"/>
  <img src="https://img.shields.io/badge/platform-macOS%20%7C%20Linux%20%7C%20Windows-success.svg" alt="Platforms"/>
</p>

> 通过 [Gewe API](https://www.geweapi.com/) 提供的微信机器人推送任务通知的 MCP 服务器，使用 Rust 构建。

---

## ✨ 特性亮点

- ⚡ **原生性能**：单个本地二进制进程，无需 Node 运行时。
- 🌍 **跨平台**：官方提供 macOS (Intel/ARM)、Linux (glibc/musl)、Windows 预构建包。
- 🛠️ **易于部署**：可手动下载或通过 npx 继续使用传统Node.js。
- 🛡️ **智能容错**：
  - 自动获取群成员昵称，使 @ 信息更具可读性
  - 无效 wxid 自动跳过并打印警告
  - 群成员列表获取失败时降级为纯文本，确保消息送达
  - @所有人权限不足时自动重试为普通消息

---

## 📸 效果图

<table>
  <tr>
    <td align="center">个人通知效果</td>
    <td align="center">群@通知效果</td>
  </tr>
  <tr>
    <td><img src="https://raw.githubusercontent.com/wangnov/gewe-notice-mcp/main/assets/friend_notice.png" width="350"></td>
    <td><img src="https://raw.githubusercontent.com/wangnov/gewe-notice-mcp/main/assets/group_notice.png" width="350"></td>
  </tr>
</table>

## 🤔 使用场景说明

`gewe-notice-mcp` 的设计初衷是作为**单向的、非交互式**的状态通知工具：

- **单向通知（本工具）**: 核心价值在于**无人值守**的场景。当您的 AI Agent 在执行耗时任务时（如：服务器上的代码生成、数据分析、自动化测试等），它可以主动将关键节点的状态通过微信推送给您。这是一种"即发即忘"的通知机制，不中断 Agent 的执行流程。

- **交互式工具（其他）**: 会暂停 Agent 的执行流程，明确地**等待用户输入**来决定下一步操作。

简单来说：如果您需要 Agent **通知**您发生了什么，请使用本工具。如果您需要 Agent **询问**该怎么做，请使用交互式工具。

建议在 User Rules、Project Rules 或 Agent Rules 中固化这些区分概念，以免在存在多种工具调用时的冲突情况。

---

## 🚀 安装方式

### 方式一：下载预构建二进制（推荐）

选择适合您系统的版本：

<details>
<summary><b>🍎 macOS (Apple Silicon M1/M2/M3)</b></summary>

```bash
# 下载 ARM64 版本
curl -L -o gewe-notice-mcp.tgz \
  https://github.com/wangnov/gewe-notice-mcp/releases/latest/download/gewe-notice-mcp-darwin-arm64.tgz

# 解压文件
tar -xzf gewe-notice-mcp.tgz

# 添加执行权限
chmod +x gewe-notice-mcp

# 移动到 PATH 目录（可选）
sudo mv gewe-notice-mcp /usr/local/bin/

# 验证安装
gewe-notice-mcp --version
```
</details>

<details>
<summary><b>🍎 macOS (Intel)</b></summary>

```bash
# 下载 x64 版本
curl -L -o gewe-notice-mcp.tgz \
  https://github.com/wangnov/gewe-notice-mcp/releases/latest/download/gewe-notice-mcp-darwin-x64.tgz

# 解压文件
tar -xzf gewe-notice-mcp.tgz

# 添加执行权限
chmod +x gewe-notice-mcp

# 移动到 PATH 目录（可选）
sudo mv gewe-notice-mcp /usr/local/bin/

# 验证安装
gewe-notice-mcp --version
```
</details>

<details>
<summary><b>🐧 Linux (x64)</b></summary>

```bash
# 下载 Linux x64 版本
curl -L -o gewe-notice-mcp.tgz \
  https://github.com/wangnov/gewe-notice-mcp/releases/latest/download/gewe-notice-mcp-linux-x64.tgz

# 解压文件
tar -xzf gewe-notice-mcp.tgz

# 添加执行权限
chmod +x gewe-notice-mcp

# 移动到 PATH 目录（可选）
sudo mv gewe-notice-mcp /usr/local/bin/

# 验证安装
gewe-notice-mcp --version
```
</details>

<details>
<summary><b>🪟 Windows (x64)</b></summary>

**PowerShell：**
```powershell
# 下载 Windows x64 版本
Invoke-WebRequest -Uri "https://github.com/wangnov/gewe-notice-mcp/releases/latest/download/gewe-notice-mcp-win32-x64.tgz" -OutFile gewe-notice-mcp.tgz

# 使用 tar 解压（Windows 10+ 内置）
tar -xzf gewe-notice-mcp.tgz

# 移动到合适的目录
New-Item -ItemType Directory -Force -Path "C:\Program Files\gewe-notice-mcp"
Move-Item -Force gewe-notice-mcp.exe "C:\Program Files\gewe-notice-mcp\"

# 添加到 PATH（需要管理员权限）
[Environment]::SetEnvironmentVariable("Path", $env:Path + ";C:\Program Files\gewe-notice-mcp", [EnvironmentVariableTarget]::Machine)

# 验证安装（新开一个终端）
gewe-notice-mcp --version
```

**命令提示符 (CMD)：**
```batch
:: 下载 Windows x64 版本
curl -L -o gewe-notice-mcp.tgz https://github.com/wangnov/gewe-notice-mcp/releases/latest/download/gewe-notice-mcp-win32-x64.tgz

:: 解压文件
tar -xzf gewe-notice-mcp.tgz

:: 验证安装
gewe-notice-mcp.exe --version
```
</details>

### 方式二：兼容 npx

见 [MCP客户端配置指南](#️-mcp-客户端配置指南)

建议逐步迁移到 Rust 版本的二进制，以获得更好的性能与维护体验。

---

## ⚙️ 前置条件

在开始之前，您需要从您的 Gewe API 管理后台的 [微信管理页面](https://manager.geweapi.com/#/account/wechat) 中获取以下信息：

0. **Base_Url**: 默认无需配置，已设为 `http://api.geweapi.com` ，如果 Gewe 通知您 Base_Url 发生变动，则可通过管理后台的 [用户主页](https://manager.geweapi.com/index#/account/index) - 开通信息 查看 `API接口域名` 。如果你使用私有部署的Gewe API服务，则设为对应的服务器域名或IP地址。
1.  **API Token**: 用于认证的 `X-GEWE-TOKEN`。
2.  **App ID**: 您的微信机器人实例的 `appId`。
3.  **接收者 WXID**: 您希望接收通知的个人微信ID (`wxid_...`) 或群聊ID (`..._chatroom`)。
4.  **(可选) @对象的 WXID**: 如果您想在群聊中 `@` 特定的人，需要预先知道他们的 `wxid` 或者直接输入 `all` 以@所有人（需要管理员或群主权限）。

>关于如何获取他人或群聊的wxid，需要您自行测试，例如：启动一个http服务，用于接收 Gewe API 的回调消息。然后在官方微信客户端中发送消息（或从群聊中发送）给您的微信机器人，在回调消息结构体中找到对应的wxid。
>
>又例如：您可以调用 Gewe API 的 `搜索好友` 接口来搜索您想找到的机器人的好友 wxid ，调用 Gewe API 的 `获取通讯录列表` 接口来列出您的全部通讯录信息，从中找到想要获取的好友或群的 wxid 等。

---

## ⚙️ 环境变量

| 变量名                 | 必填 | 说明                                                                 |
| ---------------------- | ---- | -------------------------------------------------------------------- |
| `GEWE_NOTICE_BASE_URL` | 否   | Gewe API 基础地址，默认 `https://www.geweapi.com`                    |
| `GEWE_NOTICE_TOKEN`    | ✅   | Gewe API Token（UUID）                                               |
| `GEWE_NOTICE_APP_ID`   | ✅   | 微信机器人 App ID，须以 `wx_` 开头                                   |
| `GEWE_NOTICE_WXID`     | ✅   | 接收者 WXID；群聊需以 `@chatroom` 结尾                               |
| `GEWE_NOTICE_AT_LIST`  | 否   | 逗号分隔的 WXID 列表或 `all`，用于 @ 指定成员或全体                   |

### 验证命令（二进制方式）

```bash
GEWE_NOTICE_TOKEN=... \
GEWE_NOTICE_APP_ID=wx_xxx \
GEWE_NOTICE_WXID=wxid_xxx \
./gewe-notice-mcp
```

```bash
# 使用 npx 快速验证
GEWE_NOTICE_TOKEN=... \
GEWE_NOTICE_APP_ID=wx_xxx \
GEWE_NOTICE_WXID=wxid_xxx \
npx -y gewe-notice-mcp
```

---

## 🛠️ MCP 工具: `post_text`

本 MCP 服务器提供了一个名为 `post_text` 的工具，用于发送通知。

### 参数

- `content` (string): 要发送的通知文本内容

### 推荐的消息格式

为了保持通知的一致性，建议使用以下格式：

`[状态表情] [模块/主题] - [具体消息]`

**示例：**
- ✅ **成功**: `✅ [Project Init] - 项目初始化成功，所有依赖已安装`
- ❌ **失败**: `❌ [API Call] - 任务失败，请检查API密钥或网络连接`
- ⚠️ **警告**: `⚠️ [Build] - 构建完成但有警告信息`
- 📊 **进度**: `📊 [Data Analysis] - 数据分析完成50%`

您也可以在 Agent Rules 中自定义通知格式。

---

## 🖥️ MCP 客户端配置指南

### 配置示例

<details>
<summary>点击展开查看四种不同场景的配置示例</summary>

#### 1. 发送给个人

```json
"gewe-notice": {
  "command": "/path/to/gewe-notice-mcp",
  "args": [],
  "env": {
    "GEWE_NOTICE_TOKEN": "YOUR_GEWE_TOKEN",
    "GEWE_NOTICE_APP_ID": "YOUR_BOT_APP_ID",
    "GEWE_NOTICE_WXID": "wxid_xxxxxxxxxxxxx"
  }
}
```

#### 2. 发送到群聊（不@任何人）

```json
"gewe-notice": {
  "command": "/path/to/gewe-notice-mcp",
  "args": [],
  "env": {
    "GEWE_NOTICE_TOKEN": "YOUR_GEWE_TOKEN",
    "GEWE_NOTICE_APP_ID": "YOUR_BOT_APP_ID",
    "GEWE_NOTICE_WXID": "xxxxxxxxxx@chatroom"
  }
}
```

#### 3. 发送到群聊并@所有人

```json
"gewe-notice": {
  "command": "/path/to/gewe-notice-mcp",
  "args": [],
  "env": {
    "GEWE_NOTICE_TOKEN": "YOUR_GEWE_TOKEN",
    "GEWE_NOTICE_APP_ID": "YOUR_BOT_APP_ID",
    "GEWE_NOTICE_WXID": "xxxxxxxxxx@chatroom",
    "GEWE_NOTICE_AT_LIST": "all"
  }
}
```

#### 4. 发送到群聊并@特定成员

环境变量 `GEWE_NOTICE_AT_LIST` 接受一个用**逗号**分隔的 `wxid` 字符串。

```json
"gewe-notice": {
  "command": "/path/to/gewe-notice-mcp",
  "args": [],
  "env": {
    "GEWE_NOTICE_TOKEN": "YOUR_GEWE_TOKEN",
    "GEWE_NOTICE_APP_ID": "YOUR_BOT_APP_ID",
    "GEWE_NOTICE_WXID": "xxxxxxxxxx@chatroom",
    "GEWE_NOTICE_AT_LIST": "wxid_aaaaaaaa,wxid_bbbbbbbb"
  }
}
```

>注：对于二进制方式，使用 `/path/to/gewe-notice-mcp` 指向实际的二进制文件路径。对于 npx 方式，将 `command` 改为 `"npx"`，`args` 改为 `["-y", "gewe-notice-mcp"]`。

</details>

配置完成后，您的 AI IDE 或 AI CLI 会在启动时自动运行 `gewe-notice` 服务器。

### 各客户端具体配置

以下示例分别提供"本地二进制"（推荐）与"npx 兼容"两种写法。请将 App ID、Token、WXID 等替换为真实值。

<details>
<summary><b>Cursor</b></summary>

#### 本地二进制

```json
{
  "mcpServers": {
    "gewe-notice": {
      "command": "/path/to/gewe-notice-mcp",
      "args": [],
      "env": {
        "GEWE_NOTICE_BASE_URL": "https://www.geweapi.com",
        "GEWE_NOTICE_TOKEN": "YOUR_TOKEN",
        "GEWE_NOTICE_APP_ID": "wx_xxx",
        "GEWE_NOTICE_WXID": "wxid_xxx"
      }
    }
  }
}
```

#### npx 兼容

```json
{
  "mcpServers": {
    "gewe-notice": {
      "command": "npx",
      "args": ["-y", "gewe-notice-mcp"],
      "env": {
        "GEWE_NOTICE_BASE_URL": "https://www.geweapi.com",
        "GEWE_NOTICE_TOKEN": "YOUR_TOKEN",
        "GEWE_NOTICE_APP_ID": "wx_xxx",
        "GEWE_NOTICE_WXID": "wxid_xxx"
      }
    }
  }
}
```

</details>

<details>
<summary><b>Codex（OpenAI Desktop Client）</b></summary>

#### 本地二进制

```toml
[mcp_servers.gewe_notice]
command = "/path/to/gewe-notice-mcp"
env = {
  GEWE_NOTICE_BASE_URL = "https://www.geweapi.com",
  GEWE_NOTICE_TOKEN = "YOUR_TOKEN",
  GEWE_NOTICE_APP_ID = "wx_xxx",
  GEWE_NOTICE_WXID = "wxid_xxx"
}
```

#### npx 兼容

```toml
[mcp_servers.gewe_notice]
command = "npx"
args = ["-y", "gewe-notice-mcp"]
env = {
  GEWE_NOTICE_BASE_URL = "https://www.geweapi.com",
  GEWE_NOTICE_TOKEN = "YOUR_TOKEN",
  GEWE_NOTICE_APP_ID = "wx_xxx",
  GEWE_NOTICE_WXID = "wxid_xxx"
}
```

#### Windows 注意事项

若在 Windows 上遇到 `request timed out` 或进程无法启动，可显式指定 Node 路径，或通过 `cmd` 间接调用：

```toml
[mcp_servers.gewe_notice]
command = "C:\\Program Files\\nodejs\\node.exe"
args = [
  "C:\\Users\\yourname\\AppData\\Roaming\\npm\\node_modules\\gewe-notice-mcp\\dist\\index.js",
  "--transport",
  "stdio"
]
```

或者：

```toml
[mcp_servers.gewe_notice]
command = "cmd"
args = [
  "/c",
  "npx",
  "-y",
  "gewe-notice-mcp"
]
env = { SystemRoot = "C:\\Windows" }
startup_timeout_ms = 20_000
```

上述配置可提升 Codex CLI 在 Windows 上的稳定性。

</details>

<details>
<summary><b>Claude Code CLI</b></summary>

#### 本地二进制

```bash
claude mcp add gewe-notice -- /path/to/gewe-notice-mcp \
  --env GEWE_NOTICE_BASE_URL=https://www.geweapi.com \
  --env GEWE_NOTICE_TOKEN=YOUR_TOKEN \
  --env GEWE_NOTICE_APP_ID=wx_xxx \
  --env GEWE_NOTICE_WXID=wxid_xxx
```

#### npx 兼容

```bash
claude mcp add gewe-notice -- npx -y gewe-notice-mcp \
  --env GEWE_NOTICE_BASE_URL=https://www.geweapi.com \
  --env GEWE_NOTICE_TOKEN=YOUR_TOKEN \
  --env GEWE_NOTICE_APP_ID=wx_xxx \
  --env GEWE_NOTICE_WXID=wxid_xxx
```

完成后可运行 `claude mcp list` 验证是否注册成功。

</details>

<details>
<summary><b>VS Code / GitHub Copilot Chat</b></summary>

#### 本地二进制

```json
"mcp": {
  "servers": {
    "gewe-notice": {
      "type": "stdio",
      "command": "/path/to/gewe-notice-mcp",
      "env": {
        "GEWE_NOTICE_BASE_URL": "https://www.geweapi.com",
        "GEWE_NOTICE_TOKEN": "YOUR_TOKEN",
        "GEWE_NOTICE_APP_ID": "wx_xxx",
        "GEWE_NOTICE_WXID": "wxid_xxx"
      }
    }
  }
}
```

#### npx 兼容

```json
"mcp": {
  "servers": {
    "gewe-notice": {
      "type": "stdio",
      "command": "npx",
      "args": ["-y", "gewe-notice-mcp"],
      "env": {
        "GEWE_NOTICE_BASE_URL": "https://www.geweapi.com",
        "GEWE_NOTICE_TOKEN": "YOUR_TOKEN",
        "GEWE_NOTICE_APP_ID": "wx_xxx",
        "GEWE_NOTICE_WXID": "wxid_xxx"
      }
    }
  }
}
```

</details>

---

## 🧱 项目结构

```
src/
├── main.rs      # 程序入口，解析配置 & 启动服务
├── config.rs    # clap 配置解析与校验
├── server.rs    # MCP ServerHandler 实现
├── gewe_api.rs  # 调用 Gewe HTTP API
└── errors.rs    # 错误类型定义
```

依赖：`rmcp`、`tokio`、`reqwest`、`clap`、`tracing`、`serde`。

---

## ✅ 开发与测试

```bash
cargo fmt
cargo clippy -- -D warnings
cargo test
cargo build --release
```

建议使用 GitHub Actions 构建多平台产物并自动上传 Release。

---

## ❓ 常见问题

**Q: MCP 服务器亮红灯或工具未出现？**

A: 可能原因及解决方案：
1. **机器人离线**：
   - 确保微信客户端已登录且能正常收发消息
   - 重新加载或重启 IDE 使 MCP 服务器重新初始化

2. **环境变量格式错误**：
   - `GEWE_NOTICE_TOKEN` 必须是有效的 UUID 格式（例：`e90f8g4-12f3-45f7-a151-bg43cc6ff2e6`）
   - `GEWE_NOTICE_APP_ID` 必须以 `wx_` 开头
   - 群聊 `GEWE_NOTICE_WXID` 必须以 `@chatroom` 结尾
   - `GEWE_NOTICE_AT_LIST` 可以是 `all` 或逗号分隔的 wxid 列表

**Q: @ 全体成员失败？**

A: 这通常是权限问题。只有群主或管理员才能 @所有人。程序检测到权限不足（code = -2）时会自动降级为普通消息，确保通知仍能送达。

**Q: 在线检查耗时过长？**

A: Rust 版本已采用异步检查，不会阻塞 MCP 初始化。若频繁超时：
- 检查网络代理设置
- 确认防火墙未阻拦 `geweapi.com` 域名
- 可尝试修改 `GEWE_NOTICE_BASE_URL` 为备用地址

**Q: 群成员 @ 功能不工作？**

A: 请检查：
- 机器人是否在目标群内
- 提供的 wxid 是否为群成员
- 群成员列表 API 是否正常（查看日志中的警告信息）

---

## 📄 许可证

本项目使用 [MIT License](LICENSE)。

---

## 🤝 贡献指南

欢迎 Issue 与 PR！提交前请确保：

1. 通过 `cargo fmt`、`cargo clippy -- -D warnings`；
2. `cargo test` 全部通过；
3. 更新相关文档或示例。

如需新特性或遇到兼容性问题，请在 Issue 中附 MCP 客户端日志，便于定位。

---

💬 使用反馈、功能建议欢迎随时提出，我们会继续改进 Rust 版本的体验。
