use chrono::{DateTime, Utc};
use lazy_static::lazy_static;
use proxy_wasm::traits::{Context, HttpContext, RootContext};
use proxy_wasm::types::{Action, LogLevel};
use std::collections::HashMap;
use std::sync::RwLock;

pub mod ping {
    include!(concat!(env!("OUT_DIR"), "/ping.rs"));
}

lazy_static! {
    static ref TIMESTAMP: RwLock<HashMap<u32, f64>> = RwLock::new(HashMap::new());
    static ref LATENCY: RwLock<Vec<f64>> = RwLock::new(Vec::new());
}

#[no_mangle]
pub fn _start() {
    proxy_wasm::set_log_level(LogLevel::Trace);
    proxy_wasm::set_http_context(|context_id, _| -> Box<dyn HttpContext> {
        Box::new(Metrics { context_id })
    });
}

struct MetricsRoot;

impl Context for MetricsRoot {}

impl RootContext for MetricsRoot {
    fn on_vm_start(&mut self, _: usize) -> bool {
        true
    }
}

struct Metrics {
    #[allow(unused)]
    context_id: u32,
}

impl Metrics {}

impl Context for Metrics {
    fn on_http_call_response(&mut self, _: u32, _: usize, _body_size: usize, _: usize) {
        self.resume_http_request();
    }
}

impl HttpContext for Metrics {
    fn on_http_request_headers(&mut self, _num_of_headers: usize, _end_of_stream: bool) -> Action {
        Action::Continue
    }

    fn on_http_request_body(&mut self, _body_size: usize, _end_of_stream: bool) -> Action {
        let mut timestamp_inner = TIMESTAMP.write().unwrap();
        let now: DateTime<Utc> = self.get_current_time().into();
        timestamp_inner.insert(self.context_id, now.timestamp() as f64);

        Action::Continue
    }

    fn on_http_response_headers(&mut self, _num_headers: usize, _end_of_stream: bool) -> Action {
        Action::Continue
    }

    fn on_http_response_body(&mut self, _body_size: usize, _end_of_stream: bool) -> Action {
        let mut timestamp_inner = TIMESTAMP.write().unwrap();
        let mut latency_inner = LATENCY.write().unwrap();
        match timestamp_inner.get(&self.context_id) {
            Some(last_ts) => {
                let now: DateTime<Utc> = self.get_current_time().into();
                latency_inner.push(now.timestamp() as f64 - last_ts);
                timestamp_inner.remove(&self.context_id);
            }
            None => log::warn!("no matched timestamp for {}", self.context_id),
        }

        Action::Continue
    }
}
