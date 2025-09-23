# gewe-notice-mcp

MCP服务器，通过GeWe API发送微信通知。Rust实现，单二进制。

## 代码风格与开发哲学

### Linus原则
- "Good taste": 知道什么该删
- "Do one thing well": 单一职责
- "Talk is cheap, show me the code": 代码自解释
- 无注释：代码即文档
- 无废话：直接、精确、无情

### 代码要求
- 错误处理：Result<T,E>，不panic
- 异步：tokio，不阻塞
- 依赖：最小化，必要时才引入
- 测试：集成测试覆盖主路径
- 性能：先正确，后优化

## 项目结构

```
src/           1481行 Rust核心（+473行：类型安全重构）
tests/          409行 集成测试（+38行：新类型适配）
scripts/        243行 Node.js工具
.github/        213行 CI/CD
.githooks/       30行 pre-commit
```

## 核心模块

### src/
- main.rs:13-92         入口，MCP服务器启动，适配新类型系统
- server.rs:211-333     MCP协议实现，工具："post_text"
- gewe_api.rs:74-522    增强API客户端：并发控制(Semaphore)、指数退避重试、超时保护
- config.rs:5-289       新类型模式：ValidatedToken、AppId、WxId，编译时验证
- errors.rs:3-180       分层错误系统：ConfigValidationError、NetworkError、ApiBusinessError

### 环境变量
```
GEWE_NOTICE_TOKEN     UUID
GEWE_NOTICE_APP_ID    wx_*
GEWE_NOTICE_WXID      接收者
GEWE_NOTICE_BASE_URL  默认https://www.geweapi.com
GEWE_NOTICE_AT_LIST   逗号分隔或"all"
```

### API端点
```
/gewe/v2/api/login/checkOnline           在线检查
/gewe/v2/api/group/getChatroomMemberList 群成员
/gewe/v2/api/message/postText            发送消息
```

### 错误码
```
-219  不在群内
-104  群不存在
-2    无权限(触发降级)
```

## 构建与发布

### 本地
```bash
cargo build --release
cargo test
```

### CI/CD
```
触发: git push --tags v*
流程: checks -> build(4平台) -> publish(npm) -> release(GitHub)
```

### 平台
- darwin-x64/arm64
- linux-x64 (musl静态)
- win32-x64

## 快速定位

| 功能 | 位置 |
|-----|------|
| MCP工具定义 | server.rs:274-318 |
| 消息发送逻辑 | gewe_api.rs:211-351 |
| @全体降级 | gewe_api.rs:280-310 |
| 配置验证 | config.rs:39-65 |
| 版本更新 | scripts/bump-version.js:47 |
| 测试用例 | tests/http_integration.rs:65-371 |

## 记忆文件索引

```
src/CLAUDE.md       98行 (原184行，-47%)
tests/CLAUDE.md     40行 (原210行，-81%)
scripts/CLAUDE.md   30行 (原302行，-90%)
.github/CLAUDE.md   59行 (原384行，-85%)
.githooks/CLAUDE.md 19行 (原239行，-92%)
```

总计：246行 (原1319行，-81%削减)

## 版本
- 当前: 0.0.2
- 服务器: 1.0.1 (server.rs:265)
- 协议: MCP

---
记忆系统v2.0 - Linus风格重构