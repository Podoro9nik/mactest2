use crate::{
    imaging,
    types::{valid_remote_id, AppResult, RemoteTask},
};
use reqwest::{
    blocking::Client,
    header::{HeaderMap, HeaderValue},
};
use serde_json::{json, Value};
use std::{
    io::{BufRead, BufReader, Read},
    time::Duration,
};

pub const ENDPOINT: &str = "https://elyum.ai/mcp";
const MODEL: &str = "nano-banana-2.1";
// The MCP gateway limits the entire HTTP body, including base64 and JSON.
const MAX_MCP_BODY_BYTES: usize = 4 * 1024 * 1024;

pub fn configured_key() -> String {
    std::env::var("ELYUM_MCP_KEY")
        .ok()
        .filter(|k| !k.trim().is_empty())
        .unwrap_or_else(|| {
            option_env!("CHIIKAWA_EMBEDDED_MCP_KEY")
                .unwrap_or("")
                .to_string()
        })
        .trim()
        .to_string()
}

pub struct Elyum {
    http: Client,
    headers: HeaderMap,
    counter: u64,
    initialized: bool,
    #[cfg(test)]
    test_endpoint: Option<String>,
}

impl Elyum {
    pub fn new() -> AppResult<Self> {
        let key = configured_key();
        if key.is_empty() {
            return Err("missing_key".into());
        }
        let mut headers = HeaderMap::new();
        let mut authorization =
            HeaderValue::from_str(&format!("Bearer {key}")).map_err(|_| "auth")?;
        authorization.set_sensitive(true);
        headers.insert("authorization", authorization);
        headers.insert(
            "accept",
            HeaderValue::from_static("application/json, text/event-stream"),
        );
        let http = Client::builder()
            .connect_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(65))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| "network")?;
        Ok(Self {
            http,
            headers,
            counter: 0,
            initialized: false,
            #[cfg(test)]
            test_endpoint: None,
        })
    }

    fn endpoint(&self) -> &str {
        #[cfg(test)]
        if let Some(endpoint) = &self.test_endpoint {
            return endpoint;
        }
        ENDPOINT
    }

    fn rpc(
        &mut self,
        method: &str,
        params: Value,
        notification: bool,
        retry: bool,
    ) -> AppResult<Value> {
        self.counter += 1;
        let id = self.counter;
        let mut payload = json!({"jsonrpc":"2.0","method":method,"params":params});
        if !notification {
            payload["id"] = json!(id);
        }
        let body = serde_json::to_vec(&payload).map_err(|_| "invalid_settings")?;
        if method == "tools/call"
            && payload["params"]["name"] == "elyum_upload"
            && body.len() > MAX_MCP_BODY_BYTES
        {
            return Err("upload_too_large".into());
        }
        let attempts = if retry { 3 } else { 1 };
        for attempt in 0..attempts {
            let sent = self
                .http
                .post(self.endpoint())
                .headers(self.headers.clone())
                .header("content-type", "application/json")
                .body(body.clone())
                .send();
            let response = match sent {
                Ok(response) => response,
                Err(_) if attempt + 1 < attempts => {
                    std::thread::sleep(Duration::from_secs(attempt + 1));
                    continue;
                }
                Err(_) => return Err("network".into()),
            };
            let status = response.status().as_u16();
            if matches!(status, 429 | 500 | 502 | 503 | 504) && attempt + 1 < attempts {
                std::thread::sleep(Duration::from_secs(attempt + 1));
                continue;
            }
            if !response.status().is_success() {
                return Err(provider_error(None, Some(status)));
            }
            if let Some(session) = response.headers().get("mcp-session-id") {
                self.headers.insert("mcp-session-id", session.clone());
            }
            if notification {
                return Ok(json!({}));
            }
            let sse = response
                .headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .is_some_and(|v| v.contains("text/event-stream"));
            let message = if sse {
                read_sse(BufReader::new(response.take(8 * 1024 * 1024)), id)?
            } else {
                let value: Value = serde_json::from_reader(response.take(8 * 1024 * 1024))
                    .map_err(|_| "provider_response")?;
                if value["id"].as_u64() != Some(id) {
                    return Err("provider_response".into());
                }
                value
            };
            if let Some(error) = message.get("error") {
                return Err(if error["code"].as_i64() == Some(-32602) {
                    "invalid_settings".into()
                } else {
                    provider_error(
                        error["code"].as_str(),
                        error["data"]["status"].as_u64().map(|s| s as u16),
                    )
                });
            }
            return message
                .get("result")
                .filter(|r| r.is_object())
                .cloned()
                .ok_or_else(|| "provider_response".into());
        }
        Err("network".into())
    }

    fn initialize(&mut self) -> AppResult<()> {
        if self.initialized {
            return Ok(());
        }
        let result = self.rpc("initialize", json!({"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"Chiikawizator","version":env!("CARGO_PKG_VERSION")}}), false, true)?;
        let protocol = result["protocolVersion"]
            .as_str()
            .ok_or("provider_response")?;
        self.headers.insert(
            "mcp-protocol-version",
            HeaderValue::from_str(protocol).map_err(|_| "provider_response")?,
        );
        self.rpc("notifications/initialized", json!({}), true, false)?;
        self.initialized = true;
        Ok(())
    }

    pub fn tool(&mut self, name: &str, args: Value) -> AppResult<Value> {
        let read = matches!(
            name,
            "elyum_account" | "elyum_models" | "elyum_estimate" | "elyum_wait" | "elyum_job_status"
        );
        if !read && !matches!(name, "elyum_upload" | "elyum_make_image" | "elyum_keep") {
            return Err("invalid_settings".into());
        }
        self.initialize()?;
        let result = self
            .rpc(
                "tools/call",
                json!({"name":name,"arguments":args}),
                false,
                read,
            )
            .map_err(|e| {
                if e == "network" && name == "elyum_make_image" {
                    "submit_uncertain".into()
                } else if e == "network" && name == "elyum_keep" {
                    "keep_uncertain".into()
                } else {
                    e
                }
            })?;
        tool_data(result)
    }

    pub fn account(&mut self) -> AppResult<Value> {
        self.tool("elyum_account", json!({}))
    }

    pub fn submit(
        &mut self,
        prompt: &str,
        refs: &[String],
        ratio: &str,
        resolution: &str,
        credit_mode: bool,
        client_ref: &str,
    ) -> AppResult<RemoteTask> {
        if refs.len() > 10 {
            return Err("too_many_photos".into());
        }
        self.account()?;
        let estimate = self.tool("elyum_estimate", json!({"kind":"image","model":MODEL,"images":refs.len(),"resolution":resolution,"prompt":prompt}))?;
        if estimate["browserOnly"].as_bool() == Some(true) && !credit_mode {
            return Err("browser_only".into());
        }
        let mut urls = Vec::new();
        for (i, reference) in refs.iter().enumerate() {
            urls.push(self.upload_reference(reference, i + 1)?);
        }
        let mut params = json!({"prompt":prompt,"model":MODEL,"aspectRatio":ratio,"resolution":resolution,"imageUrls":urls,"clientRef":client_ref});
        if credit_mode {
            params["creditMode"] = json!(true);
        }
        let result = self.tool("elyum_make_image", params)?;
        let id = result["jobId"]
            .as_str()
            .filter(|id| valid_remote_id(id))
            .ok_or("provider_response")?;
        Ok(RemoteTask {
            job_id: id.into(),
            gen_id: None,
            status: "queued".into(),
            url: None,
            cost: finite_cost(&estimate["credits"]),
        })
    }

    fn upload_reference(&mut self, reference: &str, index: usize) -> AppResult<String> {
        let (header, encoded) = reference.split_once(',').ok_or("image_invalid")?;
        let (mime, extension) = match header {
            "data:image/png;base64" => ("image/png", "png"),
            "data:image/jpeg;base64" => ("image/jpeg", "jpg"),
            "data:image/webp;base64" => ("image/webp", "webp"),
            _ => return Err("image_invalid".into()),
        };
        let result = self.tool("elyum_upload", json!({"base64":encoded,"mimeType":mime,"filename":format!("reference-{index}.{extension}")}))
            .and_then(|upload| media_url(upload["url"].as_str().ok_or("provider_response")?));
        result.map_err(|error| match error.as_str() {
            "network" => "upload_failed".into(),
            "provider_response" => "upload_response".into(),
            _ => error,
        })
    }

    pub fn status(&mut self, old: &RemoteTask) -> AppResult<RemoteTask> {
        if !valid_remote_id(&old.job_id) {
            return Err("invalid_job_id".into());
        }
        let result = self.tool(
            "elyum_wait",
            json!({"jobId":old.job_id,"timeoutSeconds":10,"thumbnails":false}),
        )?;
        parse_task(&result, old)
    }

    /// Called only after a fresh status check and explicit price confirmation by the user.
    /// Write requests are deliberately never retried; the next attempt re-reads the job.
    pub fn keep(&mut self, task: &RemoteTask, confirmed_cost: f64) -> AppResult<RemoteTask> {
        if !crate::types::check_keep(task, confirmed_cost)? {
            return Ok(task.clone());
        }
        let result = self.tool("elyum_keep", json!({"genId":task.gen_id,"index":0}))?;
        if let Some(url) = result["url"].as_str() {
            return Ok(RemoteTask {
                status: "completed".into(),
                url: Some(media_url(url)?),
                ..task.clone()
            });
        }
        self.status(task)
    }
}

pub fn read_sse<R: BufRead>(mut reader: R, id: u64) -> AppResult<Value> {
    let mut data = String::new();
    let mut line = String::new();
    loop {
        line.clear();
        let count = reader.read_line(&mut line).map_err(|_| "network")?;
        if count == 0 || line.trim().is_empty() {
            if !data.is_empty() {
                let parsed: Value = serde_json::from_str(&data).map_err(|_| "provider_response")?;
                if parsed["id"].as_u64() == Some(id) {
                    return Ok(parsed);
                }
                data.clear();
            }
            if count == 0 {
                break;
            }
        } else if let Some(value) = line.strip_prefix("data:") {
            if !data.is_empty() {
                data.push('\n');
            }
            data.push_str(value.trim());
        }
    }
    Err("provider_response".into())
}

fn tool_data(result: Value) -> AppResult<Value> {
    let data = result
        .get("structuredContent")
        .filter(|v| v.is_object())
        .cloned()
        .or_else(|| {
            result["content"]
                .as_array()?
                .iter()
                .filter(|item| item["type"] == "text")
                .find_map(|item| {
                    serde_json::from_str::<Value>(item["text"].as_str()?)
                        .ok()
                        .filter(|v| v.is_object())
                })
        })
        .ok_or("provider_response")?;
    if result["isError"].as_bool() == Some(true)
        || data
            .get("error")
            .is_some_and(|v| !v.is_null() && *v != Value::Bool(false))
    {
        let error = data.get("error").filter(|v| v.is_object()).unwrap_or(&data);
        return Err(provider_error(
            error["code"].as_str().or(data["code"].as_str()),
            error["status"]
                .as_u64()
                .or(data["status"].as_u64())
                .map(|s| s as u16),
        ));
    }
    Ok(data)
}

fn provider_error(code: Option<&str>, status: Option<u16>) -> String {
    match code {
        Some(
            c @ ("auth"
            | "scope"
            | "key_cap"
            | "maintenance"
            | "renewal_failed"
            | "insufficient_credits"
            | "content_refused"
            | "browser_only"),
        ) => c.into(),
        Some("prompt_too_long") => "description_too_long".into(),
        Some("locked") => "preview_not_ready".into(),
        Some("payload_too_large" | "request_too_large" | "file_too_large") => {
            "upload_too_large".into()
        }
        _ => match status {
            Some(401) => "auth",
            Some(402) => "insufficient_credits",
            Some(403) => "scope",
            Some(404) => "job_not_found",
            Some(413) => "upload_too_large",
            Some(429) => "rate_limit",
            Some(400) => "invalid_settings",
            _ => "provider_error",
        }
        .into(),
    }
}

fn finite_cost(value: &Value) -> Option<f64> {
    value.as_f64().filter(|c| c.is_finite() && *c >= 0.0)
}

pub fn parse_task(data: &Value, old: &RemoteTask) -> AppResult<RemoteTask> {
    match data["status"].as_str().unwrap_or("") {
        "queued" | "pending" => {
            return Ok(RemoteTask {
                status: "queued".into(),
                ..old.clone()
            })
        }
        "running" | "processing" => {
            return Ok(RemoteTask {
                status: "running".into(),
                ..old.clone()
            })
        }
        "failed" | "error" | "canceled" | "cancelled" => {
            return Ok(RemoteTask {
                status: "failed".into(),
                ..old.clone()
            })
        }
        "done" | "completed" | "succeeded" => (),
        _ => return Err("provider_response".into()),
    }
    let result = data.get("result").filter(|r| r.is_object()).unwrap_or(data);
    let model = result["model"].as_str().or(data["model"].as_str());
    if model.is_some_and(|m| ![MODEL, "nano-banana-2-1", "nano-banana-2-1-edit"].contains(&m)) {
        return Err("wrong_model".into());
    }
    let output = result
        .get("outputs")
        .or_else(|| result.get("images"))
        .and_then(|a| a.as_array())
        .and_then(|a| a.first())
        .ok_or("provider_response")?;
    let locked = output["locked"]
        .as_bool()
        .or(result["locked"].as_bool())
        .or(data["locked"].as_bool())
        .unwrap_or(true);
    let url = if locked {
        output["previewUrl"]
            .as_str()
            .or(output["url"].as_str())
            .or(output.as_str())
    } else {
        output["url"].as_str().or(output.as_str())
    }
    .ok_or("provider_response")?;
    let cost = finite_cost(&output["credits"])
        .or_else(|| finite_cost(&result["unlockCredits"]))
        .or_else(|| finite_cost(&result["credits"]))
        .or_else(|| {
            if ["preview", "completed"].contains(&old.status.as_str()) {
                old.cost
            } else {
                None
            }
        });
    Ok(RemoteTask {
        job_id: old.job_id.clone(),
        gen_id: result["genId"]
            .as_str()
            .or(data["genId"].as_str())
            .map(String::from)
            .or(old.gen_id.clone()),
        status: if locked { "preview" } else { "completed" }.into(),
        url: Some(media_url(url)?),
        cost,
    })
}

pub fn media_url(raw: &str) -> AppResult<String> {
    let value = if raw.starts_with("/media/") {
        format!("https://elyum.ai{raw}")
    } else {
        raw.to_string()
    };
    let url = url::Url::parse(&value).map_err(|_| "provider_response")?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err("provider_response".into());
    }
    Ok(value)
}

pub fn download(url: &str) -> AppResult<Vec<u8>> {
    let url = media_url(url)?;
    // A completely separate client: the MCP bearer token never reaches image hosts.
    let http = Client::builder()
        .timeout(Duration::from_secs(60))
        .connect_timeout(Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::custom(|a| {
            if a.previous().len() >= 5 || a.url().scheme() != "https" {
                a.stop()
            } else {
                a.follow()
            }
        }))
        .build()
        .map_err(|_| "network")?;
    for attempt in 0..3 {
        let response = http.get(&url).send();
        let mut response = match response {
            Ok(r) if r.status().is_success() => r,
            Ok(r) if !matches!(r.status().as_u16(), 429 | 500 | 502 | 503 | 504) => {
                return Err("download_failed".into())
            }
            _ if attempt < 2 => {
                std::thread::sleep(Duration::from_secs(attempt + 1));
                continue;
            }
            _ => return Err("download_failed".into()),
        };
        if response
            .content_length()
            .is_some_and(|v| v > imaging::MAX_BYTES as u64)
        {
            return Err("image_too_large".into());
        }
        let mut bytes = Vec::new();
        response
            .by_ref()
            .take((imaging::MAX_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|_| "download_failed")?;
        let image = imaging::decode(&bytes).map_err(|_| "download_failed")?;
        return imaging::png_bytes(&image);
    }
    Err("download_failed".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::Write,
        net::TcpListener,
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc, Mutex,
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
                            "elyum_account" => json!({"credits":100}),
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

        fn client(&self) -> Elyum {
            Elyum {
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
            tool_data(json!({"content":[{"type":"text","text":"{\"credits\":6}"}]})).unwrap()
                ["credits"],
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
        let result = json!({"status":"done","genId":"g_abc","outputs":[{"url":"/media/test","locked":true}]});
        let preview = parse_task(&result, &task()).unwrap();
        assert_eq!(preview.cost, None);
        assert_eq!(
            crate::types::check_keep(&preview, 9.0),
            Err("unknown_price".into())
        );
    }
    #[test]
    #[ignore = "Requires an Elyum key; account read only, no generation or spending"]
    fn live_account_read_only() {
        let account = Elyum::new().unwrap().account().unwrap();
        assert!(account.is_object());
    }
    #[test]
    #[ignore = "Requires an Elyum key; uploads one synthetic image, no generation or spending"]
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
        let prepared = imaging::prepare_elyum_bytes(&png).unwrap();
        assert!(prepared.starts_with("data:image/jpeg;base64,"));
        let url = Elyum::new()
            .unwrap()
            .upload_reference(&prepared, 1)
            .unwrap();
        assert!(url.starts_with("https://"));
    }
}
