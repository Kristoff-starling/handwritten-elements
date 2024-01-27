use proxy_wasm::traits::{Context, HttpContext, RootContext};
use proxy_wasm::types::{Action, LogLevel};

use prost::Message;
pub mod ping {
    include!(concat!(env!("OUT_DIR"), "/ping.rs"));
}

#[no_mangle]
pub fn _start() {
    proxy_wasm::set_log_level(LogLevel::Trace);
    proxy_wasm::set_http_context(|context_id, _| -> Box<dyn HttpContext> {
        Box::new(Mutation { context_id })
    });
}

struct MutationRoot;

impl Context for MutationRoot {}

impl RootContext for MutationRoot {
    fn on_vm_start(&mut self, _: usize) -> bool {
        true
    }
}

struct Mutation {
    #[allow(unused)]
    context_id: u32,
}

impl Mutation {}

impl Context for Mutation {
    fn on_http_call_response(&mut self, _: u32, _: usize, _body_size: usize, _: usize) {
        self.resume_http_request();
    }
}

impl HttpContext for Mutation {
    fn on_http_request_headers(&mut self, _num_of_headers: usize, _end_of_stream: bool) -> Action {
        // log::warn!("[DEBUG] executing on request headers");
        Action::Continue
    }

    fn on_http_request_body(&mut self, body_size: usize, _end_of_stream: bool) -> Action {
        // log::warn!("[DEBUG] executing on request body");
        if let Some(body) = self.get_http_request_body(0, body_size) {
            match ping::PingEchoRequest::decode(&body[5..]) {
                Ok(mut req) => {
                    let value = req.body.clone().to_string();

                    let mut new_body = Vec::new();
                    req.body = value;
                    req.encode(&mut new_body).expect("Failed to encode");
                    let new_body_length = new_body.len() as u32;

                    let mut grpc_header = Vec::new();
                    grpc_header.push(0); // Compression flag
                    grpc_header.extend_from_slice(&new_body_length.to_be_bytes());
                    grpc_header.append(&mut new_body);
                    self.set_http_request_body(0, grpc_header.len(), &grpc_header);
                }
                Err(e) => log::warn!("decode error {}", e),
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
