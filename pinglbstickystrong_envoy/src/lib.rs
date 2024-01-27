use lazy_static::lazy_static;
use proxy_wasm::traits::{Context, HttpContext, RootContext};
use proxy_wasm::types::{Action, LogLevel};
use rand::Rng;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::RwLock;
use std::time::Duration;

use prost::Message;
pub mod ping {
    include!(concat!(env!("OUT_DIR"), "/ping.rs"));
}

lazy_static! {
    static ref PINGLBSTICKYSTRONG_RPC_MAP: RwLock<HashMap<u32, usize>> =
        RwLock::new(HashMap::new());
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
    fn on_http_call_response(&mut self, _: u32, _: usize, body_size: usize, _: usize) {
        // log::warn!("[DEBUG] executing on on_http_call_response");
        if let Some(body) = self.get_http_call_response_body(0, body_size) {
            if let Ok(body_str) = std::str::from_utf8(&body) {
                let rpc_hashmap_inner = PINGLBSTICKYSTRONG_RPC_MAP.read().unwrap();
                let rpc_body_size = *rpc_hashmap_inner.get(&self.context_id).unwrap();
                let mut lb_tab_read: Option<String> = None;
                match serde_json::from_str::<Value>(body_str) {
                    Ok(json) => match json.get("GET") {
                        Some(get) => {
                            if !get.is_null() {
                                lb_tab_read = match get {
                                    serde_json::Value::Null => None,
                                    _ => Some(get.as_str().unwrap().to_string()),
                                };
                            }
                        }
                        _ => {
                            return;
                        }
                    },
                    Err(_) => log::warn!("Response body: [Invalid JSON data]"),
                }
                if let Some(body) = self.get_http_request_body(0, rpc_body_size) {
                    match ping::PingEchoRequest::decode(&body[5..]) {
                        Ok(rpc_request) => {
                            match lb_tab_read {
                                Some(_dst) => {}
                                None => {
                                    let lb_value = rand::thread_rng().gen_range(0.0, 1.0);
                                    self.dispatch_http_call(
                                        "webdis-service-pinglbstickystrong", // or your service name
                                        vec![
                                            (":method", "GET"),
                                            (
                                                ":path",
                                                &format!(
                                                    "/SET/{}/{}",
                                                    rpc_request.body.clone().to_string()
                                                        + "_lb_tab",
                                                    lb_value.to_string()
                                                ),
                                            ),
                                            (":authority", "webdis-service-pinglbstickystrong"), // Replace with the appropriate authority if needed
                                        ],
                                        None,
                                        vec![],
                                        Duration::from_secs(5),
                                    )
                                    .unwrap();
                                }
                            };
                        }
                        Err(e) => log::warn!("decode error: {}", e),
                    }
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
            let mut rpc_hashmap_inner = PINGLBSTICKYSTRONG_RPC_MAP.write().unwrap();
            match ping::PingEchoRequest::decode(&body[5..]) {
                Ok(req) => {
                    rpc_hashmap_inner.insert(self.context_id, body_size);
                    self.dispatch_http_call(
                        "webdis-service-pinglbstickystrong",
                        vec![
                            (":method", "GET"),
                            (
                                ":path",
                                &format!("/GET/{}", req.body.clone().to_string() + "_lb_tab"),
                            ),
                            (":authority", "webdis-service-pinglbstickystrong"),
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
