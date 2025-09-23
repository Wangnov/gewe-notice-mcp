# src/

## 文件结构
```
main.rs      92行  入口
server.rs   394行  MCP实现
gewe_api.rs 522行  API客户端（增强并发/重试）
config.rs   289行  配置（新类型模式）
errors.rs   180行  错误类型（分层架构）
lib.rs        4行  模块导出
```

## main.rs
```
L13-92   main()                    异步入口,初始化tracing,解析配置,启动MCP服务器
L15-21   tracing初始化             stderr输出,默认info级别
L25-31   Config::parse()           环境变量->配置
L33-39   GeweApiClient::new()      HTTP客户端,10秒超时
L65-69   serve_server()            stdio绑定MCP服务
```

## server.rs
```
L24-30   GeweNoticeServer          api_client:Arc<GeweApiClient>, peer:Arc<RwLock<Option<Peer>>>, min_log_level:Arc<AtomicU8>
L43-74   spawn_online_check()      异步检查机器人状态
L90-101  level_value()             LoggingLevel->u8映射
L108-110 store_min_level()         原子更新日志级别
L128-165 emit_log_message()        本地tracing+MCP转发
L176-207 handle_post_text()        处理post_text工具调用
L211-232 initialize()              MCP初始化,设置peer,触发在线检查
L234-250 set_level()               动态日志级别
L252-272 get_info()                服务器能力:tools,logging,version="1.0.1"
L274-318 list_tools()              工具:"post_text"
L320-333 call_tool()               分发到handle_post_text
```

## gewe_api.rs
```
L17-19   CheckOnlineRequest        app_id:AppId
L21-26   CheckOnlineResponse       ret,msg,data:Option<bool>
L30-33   GetChatroomMemberListRequest  app_id:AppId,chatroom_id:WxId
L36-41   ChatroomMember            wxid,nick_name,display_name
L51-55   GetChatroomMemberListResponse ret,msg,data
L59-65   PostTextRequest           app_id:AppId,to_wxid:WxId,content,ats:Option
L68-95   PostTextResponse          ret:ApiRet,msg,data,增加is_success()、failure_code()
L97-100  PostTextData              code:Option<String>

L102-130 FailureCode枚举           NotInGroup(-219)、GroupNotExist(-104)、PermissionDenied(-2)等
L132-148 ApiRet枚举               Success(200)、Error(500)、Unknown

L151-161 GeweApiClient             client:Client,config:Config,semaphore:Arc<Semaphore>
L164-185 new()                     10秒超时,Semaphore(3)并发限制,连接池优化
L187-243 check_online()            增加超时保护,重试机制
L245-322 get_chatroom_member_names() 增加超时保护,错误恢复
L324-522 post_text()
  L336-385  post_text_with_timeout() 10秒超时包装
  L387-432  post_text_with_retry()   指数退避重试(最多3次)
  L434-522  execute_post_text()      核心执行逻辑
    L450-455  @all处理              ats="notify@all"
    L457-487  @特定成员             获取昵称,构建@文本
    L505-513  降级重试              PermissionDenied时移除ats重发
```

## config.rs
```
L20-26   ValidatedToken            新类型，UUID格式验证
L28-36   AppId                     新类型，wx_前缀验证
L38-55   WxId                      新类型，统一处理'all'特殊值
L57-75   RawConfig                 原始环境变量配置
L77-85   Config                    包含验证后的强类型字段

L88-117  ValidatedToken::new()     UUID格式验证
L119-133 AppId::new()              wx_前缀验证
L135-156 WxId::new()               统一处理'all'->notify@all转换
L158-162 WxId::is_all()            判断是否为@全员

L164-204 RawConfig::parse()        环境变量读取
L206-226 RawConfig::validate()     转换为Config，触发所有验证
L228-238 Config::is_chatroom()     判断是否群聊
L240-256 Config::redact()          敏感信息脱敏
L258-289 Config::normalized_at_list() 清理at列表，处理'all'大小写
```

## errors.rs
```
L8-20    ConfigValidationError     InvalidToken、InvalidAppId、InvalidWxId等配置验证错误
L23-38   NetworkError              Timeout、Dns、TlsFailed、ConnectionRefused等网络错误
L41-56   ApiBusinessError          NotInGroup、GroupNotExist、PermissionDenied等业务错误
L59-74   ApiErrorCode枚举          映射API错误码到具体错误类型

L77-97   GeweNoticeError           分层错误：Config、Network、Api、Json、BotOffline
L99-116  NetworkError::classify()  从reqwest错误分类网络错误类型
L118-130 ApiBusinessError::from_code() 错误码转换为业务错误

L132-145 GeweNoticeError实现
  L134-136 is_retryable()          判断是否可重试（网络错误）
  L138-142 is_fatal()              判断是否致命错误（配置/业务）

L147-180 Display和Error trait实现  错误信息格式化
```

## lib.rs
```
L1-5     模块导出                  config,errors,gewe_api,server
```

## 环境变量
```
GEWE_NOTICE_TOKEN     UUID格式
GEWE_NOTICE_APP_ID    wx_开头
GEWE_NOTICE_WXID      接收者ID
GEWE_NOTICE_BASE_URL  默认https://www.geweapi.com
GEWE_NOTICE_AT_LIST   逗号分隔wxid或"all"
```

## API端点
```
/gewe/v2/api/login/checkOnline
/gewe/v2/api/group/getChatroomMemberList
/gewe/v2/api/message/postText
```

## MCP工具
```
post_text(content:string)  发送通知
```