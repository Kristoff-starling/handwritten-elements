use lazy_static::lazy_static;
use proxy_wasm::traits::{Context, HttpContext, RootContext};
use proxy_wasm::types::{Action, LogLevel};
use std::sync::RwLock;

use prost::Message;
pub mod ping {
    include!(concat!(env!("OUT_DIR"), "/ping.rs"));
}

lazy_static! {
    static ref RECORD_REQ: RwLock<Vec<String>> = RwLock::new(Vec::new());
    static ref RECORD_RESP: RwLock<Vec<String>> = RwLock::new(Vec::new());
}

#[no_mangle]
pub fn _start() {
    proxy_wasm::set_log_level(LogLevel::Trace);
    proxy_wasm::set_http_context(|context_id, _| -> Box<dyn HttpContext> {
        Box::new(Logging { context_id })
    });
}

struct LoggingRoot;

impl Context for LoggingRoot {}

impl RootContext for LoggingRoot {
    fn on_vm_start(&mut self, _: usize) -> bool {
        true
    }
}

struct Logging {
    #[allow(unused)]
    context_id: u32,
}

impl Logging {}

impl Context for Logging {
    fn on_http_call_response(&mut self, _: u32, _: usize, _body_size: usize, _: usize) {
        self.resume_http_request();
    }
}

impl HttpContext for Logging {
    fn on_http_request_headers(&mut self, _num_of_headers: usize, _end_of_stream: bool) -> Action {
        Action::Continue
    }

    fn on_http_request_body(&mut self, body_size: usize, _end_of_stream: bool) -> Action {
        if let Some(body) = self.get_http_request_body(0, body_size) {
            match ping::PingEchoRequest::decode(&body[5..]) {
                Ok(req) => {
                    let mut record_req_inner = RECORD_REQ.write().unwrap();
                    record_req_inner.push(req.body.clone().to_string());
                }
                Err(e) => log::warn!("decode error: {}", e),
            }
        }

        Action::Continue
    }

    fn on_http_response_headers(&mut self, _num_headers: usize, _end_of_stream: bool) -> Action {
        Action::Continue
    }

    fn on_http_response_body(&mut self, body_size: usize, _end_of_stream: bool) -> Action {
        if let Some(body) = self.get_http_response_body(0, body_size) {
            match ping::PingEchoResponse::decode(&body[5..]) {
                Ok(req) => {
                    let mut record_resp_inner = RECORD_RESP.write().unwrap();
                    record_resp_inner.push(req.body.clone().to_string());
                }
                Err(e) => log::warn!("decode error: {}", e),
            }
        }

        Action::Continue
    }
}
