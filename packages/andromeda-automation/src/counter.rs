use common::{
    ado_base::{AndromedaMsg, AndromedaQuery},
    app::AndrAddress,
};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;

#[cw_serde]
pub struct InstantiateMsg {
    pub whitelist: Vec<AndrAddress>,
}

#[cw_serde]
pub enum ExecuteMsg {
    AndrReceive(AndromedaMsg),
    /// Increments the count by one
    IncrementOne {},
    /// Increments the count by two
    IncrementTwo {},
    /// Resets the count to zero
    Reset {},
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(AndromedaQuery)]
    AndrQuery(AndromedaQuery),

    #[returns(CounterResponse)]
    Count {},

    #[returns(Uint128)]
    CurrentCount {},

    #[returns(bool)]
    IsZero {},

    #[returns(Vec<AndrAddress>)]
    Whitelist {},
}

#[cw_serde]
pub struct CounterResponse {
    pub count: Uint128,
    pub previous_count: Uint128,
}
