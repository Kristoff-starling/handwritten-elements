use proxy_wasm::traits::{Context, HttpContext, RootContext};
use proxy_wasm::types::{Action, LogLevel};
use rand::Rng;

pub mod ping {
    include!(concat!(env!("OUT_DIR"), "/ping.rs"));
}

#[no_mangle]
pub fn _start() {
    proxy_wasm::set_log_level(LogLevel::Trace);
    proxy_wasm::set_http_context(|context_id, _| -> Box<dyn HttpContext> {
        Box::new(Fault {
            context_id,
            abort_probability: 0.05,
        })
    });
}

struct FaultRoot;

impl Context for FaultRoot {}

impl RootContext for FaultRoot {
    fn on_vm_start(&mut self, _: usize) -> bool {
        true
    }
}

struct Fault {
    #[allow(unused)]
    context_id: u32,
    abort_probability: f32,
}

impl Fault {}

impl Context for Fault {
    fn on_http_call_response(&mut self, _: u32, _: usize, _body_size: usize, _: usize) {
        self.resume_http_request();
    }
}

impl HttpContext for Fault {
    fn on_http_request_headers(&mut self, _num_of_headers: usize, _end_of_stream: bool) -> Action {
        // log::warn!("[DEBUG] executing on request headers");
        Action::Continue
    }

    fn on_http_request_body(&mut self, _body_size: usize, _end_of_stream: bool) -> Action {
        // log::warn!("[DEBUG] executing on request body");
        let mut rng = rand::thread_rng();
        let rand_num = rng.gen_range(0.0, 1.0);
        if rand_num < self.abort_probability {
            self.send_http_response(403, vec![("grpc-status", "7")], None);
            return Action::Pause;
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
