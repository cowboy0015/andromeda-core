#[cfg(feature = "modules")]
use crate::ado_base::modules::Module;
use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

pub struct ADOContract<'a> {
    pub(crate) owner: Item<Addr>,
    pub(crate) original_publisher: Item<Addr>,
    pub(crate) block_height: Item<u64>,
    pub(crate) ado_type: Item<String>,
    pub(crate) app_contract: Item<Addr>,
    pub(crate) kernel_address: Item<Addr>,
    pub(crate) permissioned_actions: Map<String, bool>,
    #[cfg(feature = "modules")]
    pub(crate) module_info: Map<&'a str, Module>,
    #[cfg(feature = "modules")]
    pub(crate) module_idx: Item<u64>,
}

impl<'a> Default for ADOContract<'a> {
    fn default() -> Self {
        ADOContract {
            owner: Item::new("owner"),
            original_publisher: Item::new("original_publisher"),
            block_height: Item::new("block_height"),
            ado_type: Item::new("ado_type"),
            app_contract: Item::new("app_contract"),
            kernel_address: Item::new("kernel_address"),
            permissioned_actions: Map::new("andr_permissioned_actions"),
            #[cfg(feature = "modules")]
            module_info: Map::new("andr_modules"),
            #[cfg(feature = "modules")]
            module_idx: Item::new("andr_module_idx"),
        }
    }
}
