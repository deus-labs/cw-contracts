use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, DepsMut, StdResult, Uint128, Uint64};
use cw_storage_plus::{Item, Map};

#[cw_serde]
pub struct Config {
    pub owner: Addr,
    pub cw20_addr: Addr,
}

pub const CONFIG: Item<Config> = Item::new("config");

#[cw_serde]
pub struct Pot {
    /// target_addr is the address that will receive the pot
    pub target_addr: Addr,
    /// threshold_amount is the token threshold amount
    pub threshold: Uint128,
    /// collected keeps information on how much is collected for this pot.
    pub collected: Uint128,
}
/// POT_SEQ holds the last pot ID
pub const POT_SEQ: Item<u64> = Item::new("pot_seq");
pub const POTS: Map<u64, Pot> = Map::new("pot");

pub fn save_pot(deps: DepsMut, pot: &Pot) -> StdResult<()> {
    // increment id if exists, or return 1
    let id = POT_SEQ.load(deps.storage)?;
    let id = Uint64::new(id).checked_add(Uint64::new(1))?.u64();
    POT_SEQ.save(deps.storage, &id)?;

    // save pot with id
    POTS.save(deps.storage, id, pot)
}
