use anyhow::{bail, Result};
use nix::unistd::Pid;

use phoenix_common::addon::{PhoenixAddon, Version};
use phoenix_common::engine::datapath::DataPathNode;
use phoenix_common::engine::{Engine, EngineType};
use phoenix_common::storage::ResourceCollection;

use super::engine::AclEngine;
use crate::config::{create_log_file, AclConfig};
use crate::engine::struct_acl;

use chrono::prelude::*;
use itertools::iproduct;

pub(crate) struct AclEngineBuilder {
    node: DataPathNode,
    config: AclConfig,
}

impl AclEngineBuilder {
    fn new(node: DataPathNode, config: AclConfig) -> Self {
        AclEngineBuilder { node, config }
    }
    // TODO! LogFile
    fn build(self) -> Result<AclEngine> {
        let mut table_acl = Vec::new();
        table_acl.push(struct_acl {
            name: "apple".to_string(),
            permission: "Y".to_string(),
        });
        table_acl.push(struct_acl {
            name: "orang".to_string(),
            permission: "Y".to_string(),
        });
        table_acl.push(struct_acl {
            name: "appleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleappleapple".to_string(),
            permission: "Y".to_string(),
        });
        table_acl.push(struct_acl {
            name: "orangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeorangeoran".to_string(),
            permission: "Y".to_string(),
        });

        Ok(AclEngine {
            node: self.node,
            indicator: Default::default(),
            config: self.config,
            table_acl,
        })
    }
}

pub struct AclAddon {
    config: AclConfig,
}

impl AclAddon {
    pub const ACL_ENGINE: EngineType = EngineType("AclEngine");
    pub const ENGINES: &'static [EngineType] = &[AclAddon::ACL_ENGINE];
}

impl AclAddon {
    pub fn new(config: AclConfig) -> Self {
        AclAddon { config }
    }
}

impl PhoenixAddon for AclAddon {
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
        AclAddon::ENGINES
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
        if ty != AclAddon::ACL_ENGINE {
            bail!("invalid engine type {:?}", ty)
        }

        let builder = AclEngineBuilder::new(node, self.config);
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
        if ty != AclAddon::ACL_ENGINE {
            bail!("invalid engine type {:?}", ty)
        }

        let engine = AclEngine::restore(local, node, prev_version)?;
        Ok(Box::new(engine))
    }
}
