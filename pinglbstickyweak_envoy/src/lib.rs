use lazy_static::lazy_static;
use proxy_wasm::traits::{Context, HttpContext, RootContext};
use proxy_wasm::types::{Action, LogLevel};
use rand::Rng;
use std::collections::HashMap;
use std::sync::RwLock;

use prost::Message;
pub mod ping {
    include!(concat!(env!("OUT_DIR"), "/ping.rs"));
}

lazy_static! {
    static ref LB_TABLE: RwLock<HashMap<String, f64>> = RwLock::new(HashMap::new());
}

struct LoadBalanceRoot;

impl Context for LoadBalanceRoot {}

impl RootContext for LoadBalanceRoot {
    fn on_vm_start(&mut self, _: usize) -> bool {
        true
    }
}

struct LoadBalanceBody {
    #[allow(unused)]
    context_id: u32,
}

impl Context for LoadBalanceBody {
    fn on_http_call_response(&mut self, _: u32, _: usize, _body_size: usize, _: usize) {
        self.resume_http_request();
    }
}

#[no_mangle]
pub fn _start() {
    proxy_wasm::set_log_level(LogLevel::Trace);
    proxy_wasm::set_root_context(|_| -> Box<dyn RootContext> { Box::new(LoadBalanceRoot) });
    proxy_wasm::set_http_context(|context_id, _| -> Box<dyn HttpContext> {
        Box::new(LoadBalanceBody { context_id })
    });
}

impl HttpContext for LoadBalanceBody {
    fn on_http_request_headers(&mut self, _num_of_headers: usize, _end_of_stream: bool) -> Action {
        // log::warn!("[DEBUG] executing on request headers");
        Action::Continue
    }

    fn on_http_request_body(&mut self, body_size: usize, _end_of_stream: bool) -> Action {
        // log::warn!("[DEBUG] executing on request body");
        if let Some(body) = self.get_http_request_body(0, body_size) {
            match ping::PingEchoRequest::decode(&body[5..]) {
                Ok(req) => {
                    let mut map = LB_TABLE.write().unwrap();

                    if !map.contains_key(&req.body) {
                        let lb_value = rand::thread_rng().gen_range(0.0, 1.0);
                        map.insert(req.body, lb_value);
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

    fn on_http_response_body(&mut self, _body_size: usize, _end_of_stream: bool) -> Action {
        // log::warn!("[DEBUG] executing on response body");
        Action::Continue
    }
}
