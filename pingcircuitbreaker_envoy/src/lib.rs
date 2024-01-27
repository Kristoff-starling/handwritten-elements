use lazy_static::lazy_static;
use proxy_wasm::traits::{Context, HttpContext, RootContext};
use proxy_wasm::types::{Action, LogLevel};
use std::sync::RwLock;

pub mod ping {
    include!(concat!(env!("OUT_DIR"), "/ping.rs"));
}

lazy_static! {
    static ref PENDING_REQ: RwLock<usize> = RwLock::new(0);
}

struct CircuitbreakerRoot;

impl Context for CircuitbreakerRoot {}

impl RootContext for CircuitbreakerRoot {
    fn on_vm_start(&mut self, _: usize) -> bool {
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
        Action::Continue
    }

    fn on_http_request_body(&mut self, _body_size: usize, _end_of_stream: bool) -> Action {
        let mut pending_req = PENDING_REQ.write().unwrap();
        *pending_req += 1;
        if *pending_req > self.max_concurrent_req {
            self.send_http_response(403, vec![("grpc-status", "1")], None);
        }

        Action::Continue
    }

    fn on_http_response_headers(&mut self, _num_headers: usize, _end_of_stream: bool) -> Action {
        let mut pending_req = PENDING_REQ.write().unwrap();
        *pending_req -= 1;

        Action::Continue
    }

    fn on_http_response_body(&mut self, _body_size: usize, _end_of_stream: bool) -> Action {
        Action::Continue
    }
}
