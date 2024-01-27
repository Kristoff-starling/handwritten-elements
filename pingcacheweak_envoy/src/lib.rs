use lazy_static::lazy_static;
use proxy_wasm::traits::{Context, HttpContext, RootContext};
use proxy_wasm::types::{Action, LogLevel};
use std::collections::HashMap;
use std::sync::RwLock;

use prost::Message;
pub mod ping {
    include!(concat!(env!("OUT_DIR"), "/ping.rs"));
}

lazy_static! {
    static ref REQUEST_BODIES: RwLock<HashMap<String, String>> = RwLock::new(HashMap::new());
}

struct CacheRoot;

impl Context for CacheRoot {}

impl RootContext for CacheRoot {
    fn on_vm_start(&mut self, _: usize) -> bool {
        true
    }
}

struct Cache {
    #[allow(unused)]
    context_id: u32,
}

#[no_mangle]
pub fn _start() {
    proxy_wasm::set_log_level(LogLevel::Trace);
    proxy_wasm::set_http_context(|context_id, _| -> Box<dyn HttpContext> {
        Box::new(Cache { context_id })
    });
}

impl Context for Cache {
    fn on_http_call_response(&mut self, _: u32, _: usize, _body_size: usize, _: usize) {
        self.resume_http_request();
    }
}

impl HttpContext for Cache {
    fn on_http_request_headers(&mut self, _num_of_headers: usize, _end_of_stream: bool) -> Action {
        // log::warn!("[DEBUG] executing on request headers");
        Action::Continue
    }

    fn on_http_request_body(&mut self, body_size: usize, _end_of_stream: bool) -> Action {
        // log::warn!("[DEBUG] executing on request body");
        if let Some(body) = self.get_http_request_body(0, body_size) {
            match ping::PingEchoRequest::decode(&body[5..]) {
                Ok(req) => {
                    let map = REQUEST_BODIES.read().unwrap();

                    if map.contains_key(&req.body) {
                        self.send_http_response(200, vec![("grpc-status", "1")], None);
                    }
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

    fn on_http_response_body(&mut self, body_size: usize, _end_of_stream: bool) -> Action {
        // log::warn!("[DEBUG] executing on response body");
        if let Some(body) = self.get_http_response_body(0, body_size) {
            match ping::PingEchoResponse::decode(&body[5..]) {
                Ok(req) => {
                    let mut map = REQUEST_BODIES.write().unwrap();
                    map.insert(req.body, "cached".to_string());
                }
                Err(e) => log::warn!("decode error: {}", e),
            }
        }
        Action::Continue
    }
}
