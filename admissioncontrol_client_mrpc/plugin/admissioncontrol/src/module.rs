use anyhow::{bail, Result};
use fnv::FnvHashMap as HashMap;
use nix::unistd::Pid;

use phoenix_common::addon::{PhoenixAddon, Version};
use phoenix_common::engine::datapath::DataPathNode;
use phoenix_common::engine::{Engine, EngineType};
use phoenix_common::storage::ResourceCollection;

use super::engine::AdmissioncontrolEngine;
use crate::config::AdmissioncontrolConfig;

pub(crate) struct AdmissioncontrolEngineBuilder {
    node: DataPathNode,
    config: AdmissioncontrolConfig,
}

impl AdmissioncontrolEngineBuilder {
    fn new(node: DataPathNode, config: AdmissioncontrolConfig) -> Self {
        AdmissioncontrolEngineBuilder { node, config }
    }

    fn build(self) -> Result<AdmissioncontrolEngine> {
        Ok(AdmissioncontrolEngine {
            node: self.node,
            indicator: Default::default(),
            total: 0,
            success: 0,
            multiplier: 0.0,
            last_ts: std::time::Instant::now(),
            config: self.config,
        })
    }
}

pub struct AdmissioncontrolAddon {
    config: AdmissioncontrolConfig,
}

impl AdmissioncontrolAddon {
    pub const ADMISSIONCONTROL_ENGINE: EngineType = EngineType("AdmissioncontrolEngine");
    pub const ENGINES: &'static [EngineType] = &[AdmissioncontrolAddon::ADMISSIONCONTROL_ENGINE];
}

impl AdmissioncontrolAddon {
    pub fn new(config: AdmissioncontrolConfig) -> Self {
        AdmissioncontrolAddon { config }
    }
}

impl PhoenixAddon for AdmissioncontrolAddon {
    fn check_compatibility(&self, _prev: Option<&Version>) -> bool {
        true
    }

    fn decompose(self: Box<Self>) -> ResourceCollection {
        let addon = *self;
        let mut collections = ResourceCollection::new();
        collections.insert("config".to_string(), Box::new(addon.config));
        collections
    }

    #[inline]
    fn migrate(&mut self, _prev_addon: Box<dyn PhoenixAddon>) {}

    fn engines(&self) -> &[EngineType] {
        AdmissioncontrolAddon::ENGINES
    }

    fn update_config(&mut self, config: &str) -> Result<()> {
        self.config = toml::from_str(config)?;
        Ok(())
    }

    fn create_engine(
        &mut self,
        ty: EngineType,
        _pid: Pid,
        node: DataPathNode,
    ) -> Result<Box<dyn Engine>> {
        if ty != AdmissioncontrolAddon::ADMISSIONCONTROL_ENGINE {
            bail!("invalid engine type {:?}", ty)
        }

        let builder = AdmissioncontrolEngineBuilder::new(node, self.config);
        let engine = builder.build()?;
        Ok(Box::new(engine))
    }

    fn restore_engine(
        &mut self,
        ty: EngineType,
        local: ResourceCollection,
        node: DataPathNode,
        prev_version: Version,
    ) -> Result<Box<dyn Engine>> {
        if ty != AdmissioncontrolAddon::ADMISSIONCONTROL_ENGINE {
            bail!("invalid engine type {:?}", ty)
        }

        let engine = AdmissioncontrolEngine::restore(local, node, prev_version)?;
        Ok(Box::new(engine))
    }
}
