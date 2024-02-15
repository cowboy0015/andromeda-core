#![cfg(feature = "secret")]
use andromeda_std::os::adodb::{ADOVersion, ActionFee};
use secret_cosmwasm_std::{StdResult, Storage};
use secret_toolkit::storage::{Item, Keymap};

pub static VOTERS: Keymap<Addr, u8> = Keymap::new(VOTE_PREFIX.as_bytes());
/// Stores a mapping between ADO type and its latest code ID
pub const CODE_ID: Keymap<&str, u64> = Keymap::new("code_id".as_bytes());
/// Stores a mapping between a code ID and its type
pub const ADO_TYPE: Keymap<&str, String> = Keymap::new("ado_type".as_bytes());
/// Stores a mapping between a code ID and its publisher
pub const PUBLISHER: Keymap<String, String> = Keymap::new("publisher".as_bytes());
/// Stores a mapping between an ADO type/version and its code ID
pub const VERSION_CODE_ID: Keymap<(String, String), u64> =
    Keymap::new("version_code_id".as_bytes());
/// Stores a mapping between an ADO type and its action fees
pub const ACTION_FEES: Keymap<(String, String), ActionFee> = Keymap::new("action_fees".as_bytes());
/// Stores the latest version for a given ADO type
pub const LATEST_VERSION: Keymap<String, String> = Keymap::new("latest_version".as_bytes());

pub fn store_code_id(
    storage: &mut dyn Storage,
    ado_version: &ADOVersion,
    code_id: u64,
) -> StdResult<()> {
    CODE_ID.insert(storage, &ado_version.get_type(), &code_id)?;
    ADO_TYPE.insert(storage, &code_id.to_string(), &ado_version.get_type())?;
    LATEST_VERSION.insert(storage, &ado_version.get_type(), &ado_version.get_version())?;
    VERSION_CODE_ID.insert(
        storage,
        &(ado_version.get_type(), ado_version.get_version()),
        &code_id,
    )
}

pub fn read_code_id(storage: &dyn Storage, code_id_key: &str) -> StdResult<u64> {
    CODE_ID.load(storage, code_id_key)
}
