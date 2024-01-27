use chrono::{DateTime, Utc};
use lazy_static::lazy_static;
use proxy_wasm::traits::{Context, HttpContext, RootContext};
use proxy_wasm::types::{Action, LogLevel};
use std::sync::RwLock;

use prost::Message;
pub mod ping {
    include!(concat!(env!("OUT_DIR"), "/ping.rs"));
}

lazy_static! {
    static ref BYTES: RwLock<f64> = RwLock::new(0.0);
    static ref LAST_TS: RwLock<f64> = RwLock::new(0.0);
}

struct BandwidthLimitRoot;

impl Context for BandwidthLimitRoot {}

impl RootContext for BandwidthLimitRoot {
    fn on_vm_start(&mut self, _: usize) -> bool {
        let mut last_ts = LAST_TS.write().unwrap();
        let now: DateTime<Utc> = self.get_current_time().into();
        *last_ts = now.timestamp_micros() as f64;
        true
    }
}

struct BandwidthLimitBody {
    #[allow(unused)]
    context_id: u32,
    limit: f64,
    per_sec: f64,
}

impl BandwidthLimitBody {}

#[no_mangle]
pub fn _start() {
    proxy_wasm::set_log_level(LogLevel::Trace);
    proxy_wasm::set_root_context(|_| -> Box<dyn RootContext> { Box::new(BandwidthLimitRoot) });
    proxy_wasm::set_http_context(|context_id, _| -> Box<dyn HttpContext> {
        Box::new(BandwidthLimitBody {
            context_id,
            limit: 100000.0,
            per_sec: 100000.0,
        })
    });
}

impl Context for BandwidthLimitBody {
    fn on_http_call_response(&mut self, _: u32, _: usize, _body_size: usize, _: usize) {
        self.resume_http_request();
    }
}

impl HttpContext for BandwidthLimitBody {
    fn on_http_request_headers(&mut self, _num_of_headers: usize, _end_of_stream: bool) -> Action {
        Action::Continue
    }

    fn on_http_request_body(&mut self, body_size: usize, _end_of_stream: bool) -> Action {
        if let Some(body) = self.get_http_request_body(0, body_size) {
            match ping::PingEchoRequest::decode(&body[5..]) {
                Ok(req) => {
                    let now: DateTime<Utc> = self.get_current_time().into();
                    let mut last_ts = LAST_TS.write().unwrap();
                    let mut bytes = BYTES.write().unwrap();
                    let bytes_to_store = f64::min(
                        *bytes
                            + (now.timestamp_micros() as f64 - *last_ts) / 1000000.0 * self.per_sec,
                        self.limit,
                    );
                    *bytes = bytes_to_store;
                    *last_ts = now.timestamp_micros() as f64;

                    let size_bw = req.body.bytes().len() as f64;
                    if *bytes < size_bw {
                        self.send_http_response(403, vec![("grpc-status", "1")], None);
                        return Action::Pause;
                    } else {
                        *bytes -= size_bw;
                    }
                }
                Err(e) => log::warn!("decode error: {}", e),
            }
        }

        Action::Continue
    }

    fn on_http_response_headers(&mut self, _num_headers: usize, _end_of_stream: bool) -> Action {
        Action::Continue
    }

    fn on_http_response_body(&mut self, _body_size: usize, _end_of_stream: bool) -> Action {
        Action::Continue
    }
}
