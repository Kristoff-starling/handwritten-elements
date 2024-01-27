use proxy_wasm::traits::{Context, HttpContext, RootContext};
use proxy_wasm::types::{Action, LogLevel};
use serde_json::Value;
use std::time::Duration;

use prost::Message;
pub mod ping {
    include!(concat!(env!("OUT_DIR"), "/ping.rs"));
}

struct AccessControlRoot;

impl Context for AccessControlRoot {}

impl RootContext for AccessControlRoot {
    fn on_vm_start(&mut self, _: usize) -> bool {
        self.dispatch_http_call(
            "webdis-service-pingaclstrong", // or your service name
            vec![
                (":method", "GET"),
                (
                    ":path",
                    &format!("/SET/{}/{}", "test".to_string() + "_acl", "No".to_string()),
                ),
                (":authority", "webdis-service-pingaclstrong"), // Replace with the appropriate authority if needed
            ],
            None,
            vec![],
            Duration::from_secs(5),
        )
        .unwrap();
        true
    }
}

struct AccessControlBody {
    #[allow(unused)]
    context_id: u32,
}

impl Context for AccessControlBody {
    fn on_http_call_response(&mut self, _: u32, _: usize, body_size: usize, _: usize) {
        // log::warn!("[DEBUG] executing on on_http_call_response");
        if let Some(body) = self.get_http_call_response_body(0, body_size) {
            if let Ok(body_str) = std::str::from_utf8(&body) {
                match serde_json::from_str::<Value>(body_str) {
                    Ok(json) => match json.get("GET") {
                        Some(get) if !get.is_null() => match get.as_str() {
                            Some("No") => {
                                self.send_http_response(403, vec![("grpc-status", "1")], None);
                            }
                            Some(_) => log::warn!("The request is not cached."),
                            None => log::warn!("GET value is not a string"),
                        },
                        _ => {}
                    },
                    Err(_) => log::warn!("Response body: [Invalid JSON data]"),
                }
            } else {
                log::warn!("Response body: [Non-UTF8 data]");
            }
            self.resume_http_request();
        }
    }
}

#[no_mangle]
pub fn _start() {
    proxy_wasm::set_log_level(LogLevel::Trace);
    proxy_wasm::set_root_context(|_| -> Box<dyn RootContext> { Box::new(AccessControlRoot) });
    proxy_wasm::set_http_context(|context_id, _| -> Box<dyn HttpContext> {
        Box::new(AccessControlBody { context_id })
    });
}

impl HttpContext for AccessControlBody {
    fn on_http_request_headers(&mut self, _num_of_headers: usize, _end_of_stream: bool) -> Action {
        // log::warn!("[DEBUG] executing on request headers");
        Action::Continue
    }

    fn on_http_request_body(&mut self, body_size: usize, _end_of_stream: bool) -> Action {
        // log::warn!("[DEBUG] executing on request body");
        if let Some(body) = self.get_http_request_body(0, body_size) {
            match ping::PingEchoRequest::decode(&body[5..]) {
                Ok(req) => {
                    self.dispatch_http_call(
                        "webdis-service-pingaclstrong",
                        vec![
                            (":method", "GET"),
                            (
                                ":path",
                                &format!("/GET/{}", req.body.clone().to_string() + "_acl"),
                            ),
                            (":authority", "webdis-service-pingaclstrong"),
                        ],
                        None,
                        vec![],
                        Duration::from_secs(5),
                    )
                    .unwrap();
                    return Action::Pause;
                }
                Err(e) => log::warn!("decode error: {}", e),
            }
        }

        Action::Continue
    }

    fn on_http_response_headers(&mut self, _num_headers: usize, _end_of_stream: bool) -> Action {
        // log::warn!("[DEBUG] executing on response headers");
        Action::Continue
    }

    fn on_http_response_body(&mut self, _body_size: usize, _end_of_stream: bool) -> Action {
        // log::warn!("[DEBUG] executing on response body");
        Action::Continue
    }
}
