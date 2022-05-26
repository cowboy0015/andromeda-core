use crate::state::ADOContract;
use common::{
    ado_base::{
        operators::{IsOperatorResponse, OperatorsResponse},
        ownership::ContractOwnerResponse,
        AndromedaQuery, QueryMsg,
    },
    encode_binary,
    error::ContractError,
    parse_message, require,
};
use cosmwasm_std::{Binary, Deps, Env, Order};
use serde::de::DeserializeOwned;

type QueryFunction<Q> = fn(Deps, Env, Q) -> Result<Binary, ContractError>;

impl<'a> ADOContract<'a> {
    #[allow(unreachable_patterns)]
    pub fn query<Q: DeserializeOwned>(
        &self,
        deps: Deps,
        env: Env,
        msg: AndromedaQuery,
        query_function: QueryFunction<Q>,
    ) -> Result<Binary, ContractError> {
        match msg {
            AndromedaQuery::Get(data) => {
                require(
                    !self.is_nested::<QueryMsg>(&data),
                    ContractError::NestedAndromedaMsg {},
                )?;
                let received: Q = parse_message(&data)?;
                (query_function)(deps, env, received)
            }
            AndromedaQuery::Owner {} => encode_binary(&self.query_contract_owner(deps)?),
            AndromedaQuery::Operators {} => encode_binary(&self.query_operators(deps)?),
            AndromedaQuery::IsOperator { address } => {
                encode_binary(&self.query_is_operator(deps, &address)?)
            }
            #[cfg(feature = "modules")]
            AndromedaQuery::Module { id } => encode_binary(&self.query_module(deps, id)?),
            #[cfg(feature = "modules")]
            AndromedaQuery::ModuleIds {} => encode_binary(&self.query_module_ids(deps)?),
            _ => Err(ContractError::UnsupportedOperation {}),
        }
    }
}
impl<'a> ADOContract<'a> {
    pub fn query_contract_owner(&self, deps: Deps) -> Result<ContractOwnerResponse, ContractError> {
        let owner = self.owner.load(deps.storage)?;

        Ok(ContractOwnerResponse {
            owner: owner.to_string(),
        })
    }

    pub fn query_is_operator(
        &self,
        deps: Deps,
        addr: &str,
    ) -> Result<IsOperatorResponse, ContractError> {
        Ok(IsOperatorResponse {
            is_operator: self.operators.has(deps.storage, addr),
        })
    }

    pub fn query_operators(&self, deps: Deps) -> Result<OperatorsResponse, ContractError> {
        let operators: Result<Vec<String>, _> = self
            .operators
            .keys(deps.storage, None, None, Order::Ascending)
            .collect();
        Ok(OperatorsResponse {
            operators: operators?,
        })
    }
}
