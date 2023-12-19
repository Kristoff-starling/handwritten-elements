use anyhow::{bail, Result};
use fnv::FnvHashMap as HashMap;
use nix::unistd::Pid;

use phoenix_common::addon::{PhoenixAddon, Version};
use phoenix_common::engine::datapath::DataPathNode;
use phoenix_common::engine::{Engine, EngineType};
use phoenix_common::storage::ResourceCollection;

use super::engine::EncryptEngine;
use crate::config::EncryptConfig;

pub(crate) struct EncryptEngineBuilder {
    node: DataPathNode,
    config: EncryptConfig,
}

impl EncryptEngineBuilder {
    fn new(node: DataPathNode, config: EncryptConfig) -> Self {
        EncryptEngineBuilder { node, config }
    }

    fn build(self) -> Result<EncryptEngine> {
        Ok(EncryptEngine {
            node: self.node,
            indicator: Default::default(),
            config: self.config,
            target: "Banana".to_string(),
        })
    }
}

pub struct EncryptAddon {
    config: EncryptConfig,
}

impl EncryptAddon {
    pub const ENCRYPT_ENGINE: EngineType = EngineType("EncryptEngine");
    pub const ENGINES: &'static [EngineType] = &[EncryptAddon::ENCRYPT_ENGINE];
}

impl EncryptAddon {
    pub fn new(config: EncryptConfig) -> Self {
        EncryptAddon { config }
    }
}

impl PhoenixAddon for EncryptAddon {
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
        EncryptAddon::ENGINES
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
        if ty != EncryptAddon::ENCRYPT_ENGINE {
            bail!("invalid engine type {:?}", ty)
        }

        let builder = EncryptEngineBuilder::new(node, self.config);
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
        if ty != EncryptAddon::ENCRYPT_ENGINE {
            bail!("invalid engine type {:?}", ty)
        }

        let engine = EncryptEngine::restore(local, node, prev_version)?;
        Ok(Box::new(engine))
    }
}
