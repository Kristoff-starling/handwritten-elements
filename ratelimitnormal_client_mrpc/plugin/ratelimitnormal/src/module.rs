use std::collections::VecDeque;

use anyhow::{bail, Result};
use minstant::Instant;
use nix::unistd::Pid;

use phoenix_common::addon::{PhoenixAddon, Version};
use phoenix_common::engine::datapath::DataPathNode;
use phoenix_common::engine::{Engine, EngineType};
use phoenix_common::storage::ResourceCollection;

use super::engine::RatelimitnormalEngine;
use crate::config::RatelimitnormalConfig;

pub(crate) struct RatelimitnormalEngineBuilder {
    node: DataPathNode,
    config: RatelimitnormalConfig,
}

impl RatelimitnormalEngineBuilder {
    fn new(node: DataPathNode, config: RatelimitnormalConfig) -> Self {
        RatelimitnormalEngineBuilder { node, config }
    }

    fn build(self) -> Result<RatelimitnormalEngine> {
        Ok(RatelimitnormalEngine {
            node: self.node,
            indicator: Default::default(),
            config: self.config,
            last_ts: Instant::now(),
            num_tokens: self.config.bucket_size as _,
        })
    }
}

pub struct RatelimitnormalAddon {
    config: RatelimitnormalConfig,
}

impl RatelimitnormalAddon {
    pub const RATELIMITNORMAL_ENGINE: EngineType = EngineType("RatelimitnormalEngine");
    pub const ENGINES: &'static [EngineType] = &[RatelimitnormalAddon::RATELIMITNORMAL_ENGINE];
}

impl RatelimitnormalAddon {
    pub fn new(config: RatelimitnormalConfig) -> Self {
        RatelimitnormalAddon { config }
    }
}

impl PhoenixAddon for RatelimitnormalAddon {
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
        RatelimitnormalAddon::ENGINES
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
        if ty != RatelimitnormalAddon::RATELIMITNORMAL_ENGINE {
            bail!("invalid engine type {:?}", ty)
        }

        let builder = RatelimitnormalEngineBuilder::new(node, self.config);
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
        if ty != RatelimitnormalAddon::RATELIMITNORMAL_ENGINE {
            bail!("invalid engine type {:?}", ty)
        }

        let engine = RatelimitnormalEngine::restore(local, node, prev_version)?;
        Ok(Box::new(engine))
    }
}
