use andromeda_std::os::adodb::ADOVersion;
use cosmwasm_std::{attr, Attribute};

pub struct Attributes;
impl Attributes {
    const ACTION: &'static str = "action";
    pub fn update_code_id(ado_version: &str, code_id: u64) -> Vec<Attribute> {
        vec![
            attr(Self::ACTION, "update_code_id"),
            attr("code_id_key", ado_version),
            attr("code_id", code_id.to_string()),
        ]
    }

    pub fn publish() -> Vec<Attribute> {
        vec![attr(Self::ACTION, "publish_ado")]
    }

    pub fn update_action_fees(ado_version: &str) -> Vec<Attribute> {
        vec![
            attr(Self::ACTION, "update_action_fees"),
            attr("ado_type", ado_version),
        ]
    }
    pub fn update_action_fee(action: &str) -> Attribute {
        attr("update_action", action)
    }

    pub fn remove_action_fees(ado_version: &str) -> Vec<Attribute> {
        vec![
            attr(Self::ACTION, "remove_action_fees"),
            attr("ado_type", ado_version),
        ]
    }
    pub fn remove_action_fee(action: &str) -> Attribute {
        attr("remove_action", action)
    }

    pub fn store_code_id(ado_version: &ADOVersion, code_id: u64) -> Vec<Attribute> {
        vec![
            attr(Self::ACTION, "store_code_id"),
            attr("ado_type", ado_version.get_type()),
            attr("version", ado_version.get_version()),
            attr("key", ado_version.as_str()),
            attr("code_id", code_id.to_string()),
        ]
    }
    pub fn update_latest_version(ado_version: &ADOVersion) -> Vec<Attribute> {
        vec![
            attr(Self::ACTION, "update_latest_version"),
            attr("version", ado_version.as_str()),
        ]
    }

    pub fn update_metadata(ado_version: &ADOVersion) -> Vec<Attribute> {
        vec![
            attr(Self::ACTION, "update_metadata"),
            attr("update_metadata", ado_version.as_str()),
        ]
    }

    pub fn update_publisher(publisher: &str) -> Vec<Attribute> {
        vec![
            attr(Self::ACTION, "update_publisher"),
            attr("update_publisher", publisher),
        ]
    }
}
