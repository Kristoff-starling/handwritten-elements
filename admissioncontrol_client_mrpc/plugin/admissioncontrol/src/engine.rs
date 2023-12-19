//! This engine can only be placed at the sender side for now.
use anyhow::{anyhow, Result};
use fnv::FnvHashMap as HashMap;
use futures::future::BoxFuture;
use rand::Rng;
use std::num::NonZeroU32;
use std::os::unix::ucred::UCred;
use std::pin::Pin;
use std::ptr::Unique;
use std::time::Instant;

use phoenix_api::rpc::{RpcId, StatusCode, TransportStatus};
use phoenix_api_policy_admissioncontrol::control_plane;

use phoenix_common::engine::datapath::message::{EngineRxMessage, EngineTxMessage, RpcMessageTx};
use phoenix_common::engine::datapath::node::DataPathNode;
use phoenix_common::engine::{future, Decompose, Engine, EngineResult, Indicator, Vertex};
use phoenix_common::envelop::ResourceDowncast;
use phoenix_common::impl_vertex_for_engine;
use phoenix_common::log;
use phoenix_common::module::Version;
use phoenix_common::storage::{ResourceCollection, SharedStorage};

use super::DatapathError;
use crate::config::AdmissioncontrolConfig;

pub mod hello {
    // The string specified here must match the proto package name
    include!("rpc_hello.rs");
}

pub(crate) struct AdmissioncontrolEngine {
    pub(crate) node: DataPathNode,

    pub(crate) indicator: Indicator,

    pub(crate) total: u32,
    pub(crate) success: u32,
    pub(crate) multiplier: f32,
    pub(crate) last_ts: Instant,
    pub(crate) config: AdmissioncontrolConfig,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Status {
    Progress(usize),
    Disconnected,
}

use Status::Progress;

impl Engine for AdmissioncontrolEngine {
    fn activate<'a>(self: Pin<&'a mut Self>) -> BoxFuture<'a, EngineResult> {
        Box::pin(async move { self.get_mut().mainloop().await })
    }

    fn description(self: Pin<&Self>) -> String {
        "AdmissioncontrolEngine".to_owned()
    }

    #[inline]
    fn tracker(self: Pin<&mut Self>) -> &mut Indicator {
        &mut self.get_mut().indicator
    }

    fn handle_request(&mut self, request: Vec<u8>, _cred: UCred) -> Result<()> {
        let request: control_plane::Request = bincode::deserialize(&request[..])?;

        match request {
            control_plane::Request::NewConfig() => {
                // Update config
                self.config = AdmissioncontrolConfig {};
            }
        }
        Ok(())
    }
}

impl_vertex_for_engine!(AdmissioncontrolEngine, node);

impl Decompose for AdmissioncontrolEngine {
    fn flush(&mut self) -> Result<usize> {
        let mut work = 0;
        while !self.tx_inputs()[0].is_empty() || !self.rx_inputs()[0].is_empty() {
            if let Progress(n) = self.check_input_queue()? {
                work += n;
            }
        }
        Ok(work)
    }

    fn decompose(
        self: Box<Self>,
        _shared: &mut SharedStorage,
        _global: &mut ResourceCollection,
    ) -> (ResourceCollection, DataPathNode) {
        let engine = *self;

        let mut collections = ResourceCollection::with_capacity(2);
        collections.insert("config".to_string(), Box::new(engine.config));
        collections.insert("success".to_string(), Box::new(engine.success as u32));
        collections.insert("total".to_string(), Box::new(engine.total as u32));
        collections.insert("multiplier".to_string(), Box::new(engine.multiplier as f32));
        collections.insert("last_ts".to_string(), Box::new(engine.last_ts));
        (collections, engine.node)
    }
}

impl AdmissioncontrolEngine {
    pub(crate) fn restore(
        mut local: ResourceCollection,
        node: DataPathNode,
        _prev_version: Version,
    ) -> Result<Self> {
        let total = *local
            .remove("total")
            .unwrap()
            .downcast::<u32>()
            .map_err(|x| anyhow!("fail to downcast, type_name={:?}", x.type_name()))?;
        let success = *local
            .remove("success")
            .unwrap()
            .downcast::<u32>()
            .map_err(|x| anyhow!("fail to downcast, type_name={:?}", x.type_name()))?;
        let multiplier = *local
            .remove("multiplier")
            .unwrap()
            .downcast::<f32>()
            .map_err(|x| anyhow!("fail to downcast, type_name={:?}", x.type_name()))?;
        let last_ts = *local
            .remove("last_ts")
            .unwrap()
            .downcast::<Instant>()
            .map_err(|x| anyhow!("fail to downcast, type_name={:?}", x.type_name()))?;
        let config = *local
            .remove("config")
            .unwrap()
            .downcast::<AdmissioncontrolConfig>()
            .map_err(|x| anyhow!("fail to downcast, type_name={:?}", x.type_name()))?;

        let engine = AdmissioncontrolEngine {
            node,
            indicator: Default::default(),
            total,
            success,
            multiplier,
            last_ts,
            config,
        };
        Ok(engine)
    }
}

impl AdmissioncontrolEngine {
    async fn mainloop(&mut self) -> EngineResult {
        loop {
            let mut work = 0;
            // check input queue, ~100ns
            loop {
                match self.check_input_queue()? {
                    Progress(0) => break,
                    Progress(n) => work += n,
                    Status::Disconnected => return Ok(()),
                }
            }

            // If there's pending receives, there will always be future work to do.
            self.indicator.set_nwork(work);

            future::yield_now().await;
        }
    }
}

impl AdmissioncontrolEngine {
    fn check_input_queue(&mut self) -> Result<Status, DatapathError> {
        use phoenix_common::engine::datapath::TryRecvError;

        match self.tx_inputs()[0].try_recv() {
            Ok(msg) => {
                self.total += 1;
                match msg {
                    EngineTxMessage::RpcMessage(msg) => {
                        let conn_id = unsafe { &*msg.meta_buf_ptr.as_meta_ptr() }.conn_id;
                        let call_id = unsafe { &*msg.meta_buf_ptr.as_meta_ptr() }.call_id;
                        let rpc_id = RpcId {
                            0: conn_id,
                            1: call_id,
                        };
                        let prob = rand::random::<f32>();
                        let threshold = (self.total as f32 - self.multiplier * (self.success as f32)) / (self.total as f32 + 1.0);
                        if prob > threshold
                        {
                            let error = EngineRxMessage::Ack(
                                rpc_id,
                                TransportStatus::Error(unsafe { NonZeroU32::new_unchecked(403) }),
                            );
                            self.rx_outputs()[0].send(error).unwrap_or_else(|e| {
                                log::warn!("error when bubbling up the error, send failed e: {}", e)
                            });
                        } else {
                            self.tx_outputs()[0].send(EngineTxMessage::RpcMessage(msg))?;
                        }
                    }
                    // XXX TODO(cjr): it is best not to reorder the message
                    m => self.tx_outputs()[0].send(m)?,
                }
                return Ok(Progress(1));
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => return Ok(Status::Disconnected),
        }

        // forward all rx msgs
        match self.rx_inputs()[0].try_recv() {
            Ok(m) => {
                match m {
                    EngineRxMessage::RpcMessage(msg) => {
                        let meta = unsafe { &*msg.meta.as_ptr() };
                        if meta.status_code == StatusCode::Success {
                            self.success += 1;
                        }
                        self.rx_outputs()[0].send(EngineRxMessage::RpcMessage(msg))?;
                    }
                    m => {
                        self.rx_outputs()[0].send(m)?;
                    }
                };
                return Ok(Progress(1));
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => return Ok(Status::Disconnected),
        }

        Ok(Progress(0))
    }
}
