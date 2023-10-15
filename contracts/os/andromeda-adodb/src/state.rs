use andromeda_std::os::adodb::{ADOMetadata, ActionFee};
use cosmwasm_std::Addr;
use cw_storage_plus::Map;

// ado_type@version -> code_id
pub const CODE_ID: Map<&str, u64> = Map::new("code_id");
// ado_type@version -> address
pub const PUBLISHER: Map<&str, Addr> = Map::new("publisher");
// code_id -> ado_type@version
pub const ADO_TYPE: Map<u64, String> = Map::new("ado_type");
// ado_type -> latest_version like 1.0.1
pub const LATEST_VERSION: Map<&str, String> = Map::new("latest_version");
// ado_type@version -> ado_metatada
pub const METADATA: Map<&str, ADOMetadata> = Map::new("metadata");
/// Stores a mapping from an (ADO,Action) to its action fees
pub const ACTION_FEES: Map<&(String, String), ActionFee> = Map::new("action_fees");
