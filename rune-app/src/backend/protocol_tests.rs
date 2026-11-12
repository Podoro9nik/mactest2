use super::*;
use std::{
    io::Write,
    net::TcpListener,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
};

#[derive(Clone, Copy)]
enum UploadReply {
    Success,
    TooLarge,
    Malformed,
    Disconnect,
}

struct McpFixture {
    endpoint: String,
    requests: Arc<Mutex<Vec<Value>>>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl McpFixture {
    fn new(upload_reply: UploadReply) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!("http://{}/mcp", listener.local_addr().unwrap());
        listener.set_nonblocking(true).unwrap();
        let requests = Arc::new(Mutex::new(Vec::<Value>::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let captured = requests.clone();
        let shutdown = stop.clone();
        let thread = thread::spawn(move || {
            let mut uploaded = 0;
            while !shutdown.load(Ordering::Relaxed) {
                let (mut socket, _) = match listener.accept() {
                    Ok(socket) => socket,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5));
                        continue;
                    }
                    Err(error) => panic!("fixture accept failed: {error}"),
                };
                // Accepted sockets can inherit O_NONBLOCK on macOS.
                // Each fixture request is handled synchronously.
                socket.set_nonblocking(false).unwrap();
                socket
                    .set_write_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                socket
                    .set_read_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                let mut reader = BufReader::new(socket.try_clone().unwrap());
                let mut length = 0;
                loop {
                    let mut line = String::new();
                    reader.read_line(&mut line).unwrap();
                    if line.trim().is_empty() {
                        break;
                    }
                    if let Some((name, value)) = line.split_once(':') {
                        if name.eq_ignore_ascii_case("content-length") {
                            length = value.trim().parse::<usize>().unwrap();
                        }
                    }
                }
                assert!(length <= MAX_MCP_BODY_BYTES);
                let mut body = vec![0u8; length];
                reader.read_exact(&mut body).unwrap();
                let request: Value = serde_json::from_slice(&body).unwrap();
                captured.lock().unwrap().push(request.clone());
                let id = request["id"].clone();
                let tool = request["params"]["name"].as_str().unwrap_or("");
                let mut status = 200;
                let mut content_type = "application/json";
                let response = if request["method"] == "initialize" {
                    json!({"id":id,"result":{"protocolVersion":"2025-03-26"}}).to_string()
                } else if request["method"] == "notifications/initialized" {
                    status = 202;
                    String::new()
                } else if tool == "elyum_upload" {
                    uploaded += 1;
                    match upload_reply {
                        UploadReply::Disconnect => continue,
                        UploadReply::TooLarge => {
                            status = 413;
                            json!({"id":null,"error":{"code":-32000,"message":"Payload Too Large: secret must not be exposed"}}).to_string()
                        }
                        UploadReply::Malformed => "not JSON".into(),
                        UploadReply::Success => {
                            content_type = "text/event-stream";
                            format!(
                                "data: {}\n\ndata: {}\n\n",
                                json!({"method":"notifications/progress"}),
                                json!({"id":id,"result":{"structuredContent":{"url":format!("/media/upload-{uploaded}")}}})
                            )
                        }
                    }
                } else {
                    let data = match tool {
                        "elyum_account" => json!({"balance":100}),
                        "elyum_estimate" => json!({"credits":6}),
                        "elyum_make_image" => json!({"jobId":"g_fixture"}),
                        _ => panic!("unexpected fixture tool: {tool}"),
                    };
                    json!({"id":id,"result":{"structuredContent":data}}).to_string()
                };
                write!(socket, "HTTP/1.1 {status} Fixture\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response}", response.len()).unwrap();
            }
        });
        Self {
            endpoint,
            requests,
            stop,
            thread: Some(thread),
        }
    }

    fn client(&self) -> RemoteClient {
        RemoteClient {
            http: Client::builder()
                .no_proxy()
                .timeout(Duration::from_secs(3))
                .build()
                .unwrap(),
            headers: HeaderMap::new(),
            counter: 0,
            initialized: false,
            test_endpoint: Some(self.endpoint.clone()),
        }
    }

    fn tools(&self) -> Vec<Value> {
        self.requests
            .lock()
            .unwrap()
            .iter()
            .filter(|request| request["method"] == "tools/call")
            .map(|request| request["params"].clone())
            .collect()
    }
}

impl Drop for McpFixture {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        self.thread.take().unwrap().join().unwrap();
    }
}

fn task() -> RemoteTask {
    RemoteTask {
        job_id: "g_abc".into(),
        gen_id: None,
        status: "queued".into(),
        url: None,
        cost: Some(9.0),
    }
}
#[test]
fn sse_matches_id_and_ignores_progress() {
    let stream = b"event: message\r\ndata: {\"method\":\"notifications/progress\"}\r\n\r\ndata: {\"id\":4,\"result\":{\"ok\":true}}\n\n";
    assert_eq!(read_sse(&stream[..], 4).unwrap()["result"]["ok"], true);
}
#[test]
fn sse_handles_final_frame_without_blank_line() {
    assert_eq!(
        read_sse(&b"data: {\"id\":2,\"result\":{}}"[..], 2).unwrap()["id"],
        2
    );
}
#[test]
fn supports_both_tool_result_formats() {
    assert_eq!(
        tool_data(json!({"structuredContent":{"credits":6}})).unwrap()["credits"],
        6
    );
    assert_eq!(
        tool_data(json!({"content":[{"type":"text","text":"{\"credits\":6}"}]})).unwrap()["credits"],
        6
    );
}
#[test]
fn provider_errors_never_echo_sensitive_diagnostics() {
    let e = tool_data(json!({"isError":true,"structuredContent":{"error":"secret prompt token","code":"scope","status":403}})).unwrap_err();
    assert_eq!(e, "scope");
}
#[test]
fn payload_rejection_is_not_a_network_error() {
    assert_eq!(provider_error(None, Some(413)), "upload_too_large");
    let error = json!({"error":{"status":413,"message":"secret key and prompt"}});
    for result in [
        json!({"isError":true,"structuredContent":error}),
        json!({"isError":true,"content":[{"type":"text","text":error.to_string()}]}),
    ] {
        assert_eq!(tool_data(result).unwrap_err(), "upload_too_large");
    }
    assert_eq!(
        provider_error(Some("new_provider_error"), None),
        "provider_error"
    );
}
#[test]
fn uploads_photo_and_style_with_matching_types_and_order_before_submit() {
    let server = McpFixture::new(UploadReply::Success);
    let task = server
        .client()
        .submit(
            "A test scene",
            &[
                "data:image/jpeg;base64,cGhvdG8=".into(),
                "data:image/png;base64,c3R5bGU=".into(),
            ],
            "1:1",
            "1K",
            false,
            "fixture-client-ref",
            6.0,
        )
        .unwrap();
    assert_eq!(task.job_id, "g_fixture");
    let calls = server.tools();
    assert_eq!(
        calls
            .iter()
            .map(|call| call["name"].as_str().unwrap())
            .collect::<Vec<_>>(),
        [
            "elyum_account",
            "elyum_estimate",
            "elyum_upload",
            "elyum_upload",
            "elyum_make_image"
        ]
    );
    assert_eq!(calls[2]["arguments"]["mimeType"], "image/jpeg");
    assert_eq!(calls[2]["arguments"]["filename"], "reference-1.jpg");
    assert_eq!(calls[2]["arguments"]["base64"], "cGhvdG8=");
    assert_eq!(calls[3]["arguments"]["filename"], "reference-2.png");
    assert_eq!(calls[3]["arguments"]["mimeType"], "image/png");
    assert_eq!(
        calls[4]["arguments"]["imageUrls"],
        json!([
            "https://elyum.ai/media/upload-1",
            "https://elyum.ai/media/upload-2"
        ])
    );
    assert_eq!(calls[4]["arguments"]["clientRef"], "fixture-client-ref");
}
#[test]
fn upload_failure_stops_before_generation_and_never_retries_writes() {
    for (reply, expected) in [
        (UploadReply::TooLarge, "upload_too_large"),
        (UploadReply::Malformed, "upload_response"),
        (UploadReply::Disconnect, "upload_failed"),
    ] {
        let server = McpFixture::new(reply);
        let error = server
            .client()
            .submit(
                "A test scene",
                &["data:image/jpeg;base64,cGhvdG8=".into()],
                "1:1",
                "1K",
                false,
                "fixture-client-ref",
                6.0,
            )
            .unwrap_err();
        assert_eq!(error, expected);
        let calls = server.tools();
        assert_eq!(
            calls
                .iter()
                .map(|call| call["name"].as_str().unwrap())
                .collect::<Vec<_>>(),
            ["elyum_account", "elyum_estimate", "elyum_upload"]
        );
    }
}

#[test]
fn higher_fresh_estimate_is_rejected_before_upload_or_generation() {
    let server = McpFixture::new(UploadReply::Success);
    let error = server
        .client()
        .submit("A scene", &[], "1:1", "1K", true, "fixture-cap", 3.0)
        .unwrap_err();
    assert_eq!(error, "price_changed");
    let tools: Vec<_> = server
        .tools()
        .into_iter()
        .map(|c| c["name"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(tools, ["elyum_account", "elyum_estimate"]);
}
#[test]
fn oversized_json_is_blocked_before_any_upload_is_sent() {
    let server = McpFixture::new(UploadReply::Success);
    let mut client = server.client();
    client.initialized = true;
    assert_eq!(
        client
            .upload_reference(
                &format!("data:image/png;base64,{}", "A".repeat(MAX_MCP_BODY_BYTES)),
                1
            )
            .unwrap_err(),
        "upload_too_large"
    );
    assert!(server.requests.lock().unwrap().is_empty());
}
#[test]
fn locked_is_default_and_never_implies_payment() {
    let result = json!({"status":"done","genId":"g_abc","model":"nano-banana-2-1-edit","outputs":[{"url":"/media/test","credits":6}]});
    let preview = parse_task(&result, &task()).unwrap();
    assert_eq!(preview.status, "preview");
    assert_eq!(preview.cost, Some(6.0));
    let mut result = result;
    result["outputs"][0]["locked"] = json!(false);
    assert_eq!(parse_task(&result, &preview).unwrap().status, "completed");
}
#[test]
fn rejects_unsupported_model_and_non_https_media() {
    assert!(media_url("http://example.com/picture.png").is_err());
    assert!(parse_task(&json!({"status":"done","model":"other","outputs":[{"url":"https://elyum.ai/media/a"}]}),&task()).is_err());
}
#[test]
fn an_estimate_cannot_substitute_for_a_missing_final_price() {
    let result =
        json!({"status":"done","genId":"g_abc","outputs":[{"url":"/media/test","locked":true}]});
    let preview = parse_task(&result, &task()).unwrap();
    assert_eq!(preview.cost, None);
    assert_eq!(
        super::super::types::check_keep(&preview, 9.0),
        Err("unknown_price".into())
    );
}
#[test]
#[ignore = "Requires an RemoteClient key; account read only, no generation or spending"]
fn live_account_read_only() {
    let account = RemoteClient::new().unwrap().account().unwrap();
    assert!(account.is_object());
    assert!(parse_balance(&account).unwrap() >= 0.0);
}

#[test]
fn account_balance_uses_available_balance_not_credit_limits_or_estimates() {
    assert_eq!(
        parse_balance(&json!({"balance":120.25,"credits":5000,"held":12})),
        Ok(120.25)
    );
    assert_eq!(parse_balance(&json!({"balance":0})), Ok(0.0));
}

#[test]
fn balance_check_only_calls_the_read_only_account_tool() {
    let server = McpFixture::new(UploadReply::Success);
    assert_eq!(server.client().balance(), Ok(100.0));
    let tools = server.tools();
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0]["name"], "elyum_account");
    assert_eq!(tools[0]["arguments"], json!({}));
}

#[test]
fn invalid_or_missing_balance_never_silently_becomes_zero() {
    for value in [
        json!({}),
        json!({"credits":120}),
        json!({"balance":null}),
        json!({"balance":"120"}),
        json!({"balance":-1}),
        json!({"balance":f64::NAN}),
        json!({"balance":f64::INFINITY}),
        json!({"balance":{"remaining":120}}),
    ] {
        assert_eq!(parse_balance(&value), Err("balance_unavailable".into()));
    }
}
#[test]
#[ignore = "Requires an RemoteClient key; uploads one synthetic image, no generation or spending"]
fn live_upload_prepared_photo() {
    let mut seed = 81u32;
    let image = image::RgbImage::from_fn(1600, 1200, |_, _| {
        image::Rgb(std::array::from_fn(|_| {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            seed as u8
        }))
    });
    let png = imaging::png_bytes(&image::DynamicImage::ImageRgb8(image)).unwrap();
    let prepared = imaging::prepare_bytes(&png).unwrap();
    assert!(prepared.starts_with("data:image/jpeg;base64,"));
    let url = RemoteClient::new()
        .unwrap()
        .upload_reference(&prepared, 1)
        .unwrap();
    assert!(url.starts_with("https://"));
}
