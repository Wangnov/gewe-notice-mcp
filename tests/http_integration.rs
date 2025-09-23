use std::future::Future;
use std::sync::Arc;

use once_cell::sync::Lazy;
use reqwest::StatusCode;
use serde_json::json;
use tokio::sync::Mutex;

use gewe_notice_mcp::config::Config;
use gewe_notice_mcp::errors::GeweNoticeError;
use gewe_notice_mcp::gewe_api::GeweApiClient;

static INIT_TRACING: Lazy<()> = Lazy::new(|| {
    let _ = tracing_subscriber::fmt::try_init();
});

struct MockServer {
    address: String,
    handle: tokio::task::JoinHandle<()>,
}

impl MockServer {
    async fn spawn(routes: axum::Router) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind address");
        let address = format!("http://{}", listener.local_addr().unwrap());
        let handle = tokio::spawn(async move {
            if let Err(err) = axum::serve(listener, routes.into_make_service()).await {
                eprintln!("mock server error: {err}");
            }
        });

        Self { address, handle }
    }
}

impl Drop for MockServer {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

fn base_config(base_url: String, at_list: Option<Vec<String>>) -> Config {
    Config {
        base_url,
        token: "00000000-0000-0000-0000-000000000000".into(),
        app_id: "wx_test_app".into(),
        wxid: "wxid_target@chatroom".into(),
        at_list,
    }
}

async fn with_client<F, Fut>(routes: axum::Router, at_list: Option<Vec<String>>, test: F)
where
    F: FnOnce(GeweApiClient) -> Fut,
    Fut: Future<Output = ()>,
{
    let server = MockServer::spawn(routes).await;
    let client =
        GeweApiClient::new(base_config(server.address.clone(), at_list)).expect("create client");
    test(client).await;
}

#[tokio::test]
async fn check_online_success() {
    Lazy::force(&INIT_TRACING);

    let route = axum::Router::new().route(
        "/gewe/v2/api/login/checkOnline",
        axum::routing::post(|body: axum::Json<serde_json::Value>| async move {
            assert_eq!(body["appId"], "wx_test_app");
            axum::Json(json!({
                "ret": 200,
                "msg": "操作成功",
                "data": true
            }))
        }),
    );

    with_client(route, None, |client| async move {
        assert!(client.check_online().await.expect("online"));
    })
    .await;
}

#[tokio::test]
async fn check_online_handles_false_data() {
    let route = axum::Router::new().route(
        "/gewe/v2/api/login/checkOnline",
        axum::routing::post(|_: axum::Json<serde_json::Value>| async move {
            axum::Json(json!({
                "ret": 200,
                "msg": "操作成功",
                "data": false
            }))
        }),
    );

    with_client(route, None, |client| async move {
        let err = client.check_online().await.expect_err("offline");
        assert!(matches!(err, GeweNoticeError::BotOffline));
    })
    .await;
}

#[tokio::test]
async fn check_online_propagates_api_error() {
    let route = axum::Router::new().route(
        "/gewe/v2/api/login/checkOnline",
        axum::routing::post(|_: axum::Json<serde_json::Value>| async move {
            axum::Json(json!({
                "ret": 500,
                "msg": "服务器内部错误",
                "data": null
            }))
        }),
    );

    with_client(route, None, |client| async move {
        match client.check_online().await.expect_err("api error") {
            GeweNoticeError::ApiError { code, message } => {
                assert_eq!(code, 500);
                assert_eq!(message, "服务器内部错误");
            }
            other => panic!("unexpected error {other:?}"),
        }
    })
    .await;
}

#[tokio::test]
async fn post_text_mentions_specific_members() {
    static POST_INVOCATIONS: Lazy<Arc<Mutex<Vec<serde_json::Value>>>> =
        Lazy::new(|| Arc::new(Mutex::new(Vec::new())));

    let routes = axum::Router::new()
        .route(
            "/gewe/v2/api/group/getChatroomMemberList",
            axum::routing::post(|body: axum::Json<serde_json::Value>| async move {
                assert_eq!(body["chatroomId"], "wxid_target@chatroom");
                axum::Json(json!({
                    "ret": 200,
                    "msg": "操作成功",
                    "data": {
                        "memberList": [
                            {
                                "wxid": "user_a",
                                "nickName": "A",
                                "displayName": "显示A",
                                "inviterUserName": null,
                                "memberFlag": 1,
                                "bigHeadImgUrl": "",
                                "smallHeadImgUrl": ""
                            },
                            {
                                "wxid": "user_b",
                                "nickName": "B",
                                "displayName": null,
                                "inviterUserName": null,
                                "memberFlag": 1,
                                "bigHeadImgUrl": "",
                                "smallHeadImgUrl": ""
                            }
                        ],
                        "chatroomOwner": null,
                        "adminWxid": null
                    }
                }))
            }),
        )
        .route(
            "/gewe/v2/api/message/postText",
            axum::routing::post(|body: axum::Json<serde_json::Value>| async move {
                POST_INVOCATIONS.lock().await.push(body.0.clone());
                (
                    StatusCode::OK,
                    axum::Json(json!({
                        "ret": 200,
                        "msg": "操作成功",
                        "data": {
                            "toWxid": "wxid_target@chatroom",
                            "createTime": 1703841160,
                            "msgId": 0,
                            "newMsgId": 888,
                            "type": 1
                        }
                    })),
                )
            }),
        );

    with_client(routes, Some(vec![" user_a ".into()]), |client| async move {
        POST_INVOCATIONS.lock().await.clear();
        client.post_text("任务完成").await.expect("post success");

        let calls = POST_INVOCATIONS.lock().await.clone();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0]["content"], "@显示A 任务完成");
        assert_eq!(calls[0]["ats"], "user_a");
    })
    .await;
}

#[tokio::test]
async fn post_text_handles_at_all_permission_denied() {
    static RETRY_INVOCATIONS: Lazy<Arc<Mutex<Vec<serde_json::Value>>>> =
        Lazy::new(|| Arc::new(Mutex::new(Vec::new())));

    let routes = axum::Router::new()
        .route(
            "/gewe/v2/api/group/getChatroomMemberList",
            axum::routing::post(|body: axum::Json<serde_json::Value>| async move {
                assert_eq!(body["chatroomId"], "wxid_target@chatroom");
                axum::Json(json!({
                    "ret": 200,
                    "msg": "操作成功",
                    "data": {
                        "memberList": [
                            {
                                "wxid": "user_all",
                                "nickName": "All",
                                "displayName": null,
                                "inviterUserName": null,
                                "memberFlag": 1,
                                "bigHeadImgUrl": "",
                                "smallHeadImgUrl": ""
                            }
                        ],
                        "chatroomOwner": null,
                        "adminWxid": null
                    }
                }))
            }),
        )
        .route(
            "/gewe/v2/api/message/postText",
            axum::routing::post(|body: axum::Json<serde_json::Value>| async move {
                let mut calls = RETRY_INVOCATIONS.lock().await;
                calls.push(body.0.clone());
                if calls.len() == 1 {
                    return (
                        StatusCode::OK,
                        axum::Json(json!({
                            "ret": 500,
                            "msg": "操作失败",
                            "data": {"code": "-2"}
                        })),
                    );
                }
                (
                    StatusCode::OK,
                    axum::Json(json!({
                        "ret": 200,
                        "msg": "操作成功",
                        "data": {
                            "toWxid": "wxid_target@chatroom",
                            "createTime": 1703841160,
                            "msgId": 0,
                            "newMsgId": 999,
                            "type": 1
                        }
                    })),
                )
            }),
        );

    let at_list = Some(vec!["all".into()]);

    with_client(routes.clone(), at_list.clone(), |client| async move {
        RETRY_INVOCATIONS.lock().await.clear();
        client.post_text("测试").await.expect("post success");

        let calls = RETRY_INVOCATIONS.lock().await.clone();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0]["content"], "@所有人 测试");
        assert_eq!(calls[0]["ats"], "notify@all");
        assert_eq!(calls[1]["content"], "测试");
        assert!(calls[1].get("ats").is_none());
    })
    .await;
}

#[tokio::test]
async fn post_text_propagates_api_failure() {
    Lazy::force(&INIT_TRACING);
    let routes = axum::Router::new().route(
        "/gewe/v2/api/message/postText",
        axum::routing::post(|body: axum::Json<serde_json::Value>| async move {
            assert_eq!(body["content"], "任务失败");
            (
                StatusCode::OK,
                axum::Json(json!({
                    "ret": 500,
                    "msg": "操作失败",
                    "data": {"code": "-104"}
                })),
            )
        }),
    );

    with_client(routes, None, |client| async move {
        match client.post_text("任务失败").await.expect_err("api error") {
            GeweNoticeError::ApiError { code, message } => {
                assert_eq!(code, 500);
                assert_eq!(message, "该群聊不存在");
            }
            other => panic!("unexpected error {other:?}"),
        }
    })
    .await;
}

#[tokio::test]
async fn post_text_non_json_response_surfaces_parser_error() {
    Lazy::force(&INIT_TRACING);

    let routes = axum::Router::new().route(
        "/gewe/v2/api/message/postText",
        axum::routing::post(|| async move { (StatusCode::OK, "<!DOCTYPE html>oops") }),
    );

    with_client(routes, None, |client| async move {
        match client
            .post_text("非 JSON 响应")
            .await
            .expect_err("non-json should fail")
        {
            GeweNoticeError::JsonError(err) => {
                assert!(err.is_syntax(), "解析错误应被标记为语法错误: {err}");
            }
            other => panic!("unexpected error {other:?}"),
        }
    })
    .await;
}

#[tokio::test]
async fn post_text_member_lookup_failure_skips_mentions() {
    static INVOCATIONS: Lazy<Arc<Mutex<Vec<serde_json::Value>>>> =
        Lazy::new(|| Arc::new(Mutex::new(Vec::new())));

    Lazy::force(&INIT_TRACING);
    let routes = axum::Router::new()
        .route(
            "/gewe/v2/api/group/getChatroomMemberList",
            axum::routing::post(|_: axum::Json<serde_json::Value>| async move {
                axum::Json(json!({
                    "ret": 500,
                    "msg": "获取群成员列表异常:null",
                    "data": null
                }))
            }),
        )
        .route(
            "/gewe/v2/api/message/postText",
            axum::routing::post(|body: axum::Json<serde_json::Value>| async move {
                INVOCATIONS.lock().await.push(body.0.clone());
                (
                    StatusCode::OK,
                    axum::Json(json!({
                        "ret": 200,
                        "msg": "操作成功",
                        "data": {
                            "toWxid": "wxid_target@chatroom",
                            "createTime": 1703841160,
                            "msgId": 0,
                            "newMsgId": 555,
                            "type": 1
                        }
                    })),
                )
            }),
        );

    with_client(
        routes,
        Some(vec!["user_missing".into()]),
        |client| async move {
            INVOCATIONS.lock().await.clear();
            match client.post_text("尝试@缺失成员").await {
                Err(GeweNoticeError::ApiError { code, message }) => {
                    assert_eq!(code, 500);
                    assert_eq!(message, "获取群成员列表异常:null");
                }
                Ok(_) => panic!("expected member lookup failure to surface"),
                Err(other) => panic!("unexpected error {other:?}"),
            }

            let calls = INVOCATIONS.lock().await.clone();
            assert!(calls.is_empty(), "postText 应该在成员查询失败时不被调用");
        },
    )
    .await;
}
