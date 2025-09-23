# tests/

## http_integration.rs (409行，+37行：新类型适配)

### 测试基础
```
L14-16   INIT_TRACING              全局tracing初始化
L18-43   MockServer                axum模拟服务器,随机端口,Drop自动清理
L45-58   base_config()             使用新类型：ValidatedToken、AppId、WxId构造
L60-69   with_client()             测试包装器:启动mock->创建客户端->执行测试
```

### 测试用例
```
L65-85   check_online_success()                   ret:200,data:true -> Ok(true)
L87-105  check_online_handles_false_data()        ret:200,data:false -> BotOffline
L107-130 check_online_propagates_api_error()      ret:500 -> ApiError{code,message}

L132-203 post_text_mentions_specific_members()
         - 获取群成员 -> 构建@昵称 -> 发送
         - 验证content包含@昵称,ats参数正确

L205-282 post_text_handles_at_all_permission_denied()
         - 第1次:code="-2" -> 失败
         - 第2次:移除ats -> 成功
         - 验证降级重试机制

L284-312 post_text_propagates_api_failure()
         - code="-104" -> "该群聊不存在"

L314-371 post_text_member_lookup_failure_skips_mentions()
         - 群成员API失败 -> 错误传播
         - postText未被调用
```

### 运行
```bash
cargo test --test http_integration
RUST_LOG=debug cargo test -- --nocapture
```