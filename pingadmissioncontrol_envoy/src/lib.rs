use lazy_static::lazy_static;
use proxy_wasm::traits::{Context, HttpContext, RootContext};
use proxy_wasm::types::{Action, LogLevel};
use rand::Rng;
use std::sync::RwLock;

pub mod ping {
    include!(concat!(env!("OUT_DIR"), "/ping.rs"));
}

lazy_static! {
    static ref REQUESTS: RwLock<usize> = RwLock::new(0);
    static ref ACCEPTS: RwLock<usize> = RwLock::new(0);
}

struct AdmissioncontrolRoot;

impl Context for AdmissioncontrolRoot {}

impl RootContext for AdmissioncontrolRoot {
    fn on_vm_start(&mut self, _: usize) -> bool {
        true
    }
}

struct Admissioncontrol {
    #[allow(unused)]
    context_id: u32,
    multiplier: f64,
}

#[no_mangle]
pub fn _start() {
    proxy_wasm::set_log_level(LogLevel::Trace);
    proxy_wasm::set_http_context(|context_id, _| -> Box<dyn HttpContext> {
        Box::new(Admissioncontrol {
            context_id,
            multiplier: 0.95,
        })
    });
}

impl Context for Admissioncontrol {
    fn on_http_call_response(&mut self, _: u32, _: usize, _body_size: usize, _: usize) {
        self.resume_http_request();
    }
}

impl HttpContext for Admissioncontrol {
    fn on_http_request_headers(&mut self, _num_of_headers: usize, _end_of_stream: bool) -> Action {
        Action::Continue
    }

    fn on_http_request_body(&mut self, _body_size: usize, _end_of_stream: bool) -> Action {
        let mut requests = REQUESTS.write().unwrap();
        let accepts = ACCEPTS.read().unwrap();

        let rej_prob = f64::max(
            0.0,
            (*requests as f64 - self.multiplier * *accepts as f64) / (*requests as f64 + 1.0),
        );
        *requests += 1;

        let rand_value = rand::thread_rng().gen_range(0.0, 1.0);
        if rand_value < rej_prob {
            self.send_http_response(403, vec![("grpc-status", "1")], None);
        }

        Action::Continue
    }

    fn on_http_response_headers(&mut self, _num_headers: usize, _end_of_stream: bool) -> Action {
        if let Some(status_code) = self.get_http_response_header(":status") {
            if status_code == "200" {
                let mut accepts = ACCEPTS.write().unwrap();
                *accepts += 1;
            }
        }

        Action::Continue
    }

    fn on_http_response_body(&mut self, _body_size: usize, _end_of_stream: bool) -> Action {
        Action::Continue
    }
}
