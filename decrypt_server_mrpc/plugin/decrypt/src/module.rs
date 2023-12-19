use anyhow::{bail, Result};
use fnv::FnvHashMap as HashMap;
use nix::unistd::Pid;

use phoenix_common::addon::{PhoenixAddon, Version};
use phoenix_common::engine::datapath::DataPathNode;
use phoenix_common::engine::{Engine, EngineType};
use phoenix_common::storage::ResourceCollection;

use super::engine::DecryptEngine;
use crate::config::DecryptConfig;

pub(crate) struct DecryptEngineBuilder {
    node: DataPathNode,
    config: DecryptConfig,
}

impl DecryptEngineBuilder {
    fn new(node: DataPathNode, config: DecryptConfig) -> Self {
        DecryptEngineBuilder { node, config }
    }

    fn build(self) -> Result<DecryptEngine> {
        Ok(DecryptEngine {
            node: self.node,
            indicator: Default::default(),
            config: self.config,
            target: "Banana".to_string(),
        })
    }
}

pub struct DecryptAddon {
    config: DecryptConfig,
}

impl DecryptAddon {
    pub const DECRYPT_ENGINE: EngineType = EngineType("DecryptEngine");
    pub const ENGINES: &'static [EngineType] = &[DecryptAddon::DECRYPT_ENGINE];
}

impl DecryptAddon {
    pub fn new(config: DecryptConfig) -> Self {
        DecryptAddon { config }
    }
}

impl PhoenixAddon for DecryptAddon {
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
        DecryptAddon::ENGINES
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
        if ty != DecryptAddon::DECRYPT_ENGINE {
            bail!("invalid engine type {:?}", ty)
        }

        let builder = DecryptEngineBuilder::new(node, self.config);
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
        if ty != DecryptAddon::DECRYPT_ENGINE {
            bail!("invalid engine type {:?}", ty)
        }

        let engine = DecryptEngine::restore(local, node, prev_version)?;
        Ok(Box::new(engine))
    }
}
