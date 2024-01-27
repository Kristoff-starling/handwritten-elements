use lazy_static::lazy_static;
use proxy_wasm::traits::{Context, HttpContext, RootContext};
use proxy_wasm::types::{Action, LogLevel};
use std::sync::RwLock;
use std::collections::HashSet;

pub mod ping {
    include!(concat!(env!("OUT_DIR"), "/ping.rs"));
}

lazy_static! {
    static ref PENDING_REQ: RwLock<usize> = RwLock::new(0);
    static ref REACHED_REQ: RwLock<HashSet<u32>> = RwLock::new(HashSet::new());
}

struct CircuitbreakerRoot;

impl Context for CircuitbreakerRoot {}

impl RootContext for CircuitbreakerRoot {
    fn on_vm_start(&mut self, _: usize) -> bool {
        let mut pending_req = PENDING_REQ.write().unwrap();
        *pending_req = 0;
        true
    }
}

struct Circuitbreaker {
    #[allow(unused)]
    context_id: u32,
    max_concurrent_req: usize,
}

#[no_mangle]
pub fn _start() {
    proxy_wasm::set_log_level(LogLevel::Trace);
    proxy_wasm::set_http_context(|context_id, _| -> Box<dyn HttpContext> {
        Box::new(Circuitbreaker {
            context_id,
            max_concurrent_req: 1,
        })
    });
}

impl Context for Circuitbreaker {
    fn on_http_call_response(&mut self, _: u32, _: usize, _body_size: usize, _: usize) {
        self.resume_http_request();
    }
}

impl HttpContext for Circuitbreaker {
    fn on_http_request_headers(&mut self, _num_of_headers: usize, _end_of_stream: bool) -> Action {
        // log::warn!("[DEBUG] executing on request headers");
        Action::Continue
    }

    fn on_http_request_body(&mut self, _body_size: usize, _end_of_stream: bool) -> Action {
        // log::warn!("[DEBUG] executing on request body");
        let mut pending_req = PENDING_REQ.write().unwrap();
        if *pending_req > self.max_concurrent_req {
            self.send_http_response(403, vec![("grpc-status", "1")], None);
            return Action::Pause;
        }
        else {
            *pending_req += 1;
            let mut reached_req = REACHED_REQ.write().unwrap();
            reached_req.insert(self.context_id);
        }

        Action::Continue
    }

    fn on_http_response_headers(&mut self, _num_headers: usize, _end_of_stream: bool) -> Action {
        // log::warn!("[DEBUG] executing on response headers");
        let mut reached_req = REACHED_REQ.write().unwrap();
        if reached_req.contains(&self.context_id) {
            let mut pending_req = PENDING_REQ.write().unwrap();
            *pending_req -= 1;
            reached_req.remove(&self.context_id);
        }

        Action::Continue
    }

    fn on_http_response_body(&mut self, _body_size: usize, _end_of_stream: bool) -> Action {
        // log::warn!("[DEBUG] executing on response body");
        Action::Continue
    }
}