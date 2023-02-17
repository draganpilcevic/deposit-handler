use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

use crate::typing::{BondStatus, Config};

/// Config storage
pub const CONFIG: Item<Config> = Item::new("config");

/// Track which address made which bonding operation
pub const ID_TO_ADDRESS_TRACKER: Map<String, Addr> = Map::new("id_to_address_tracker");

/// map id to bond_status
pub const BOND_STATUS_TRACKER: Map<String, BondStatus> = Map::new("bond_status_tracker");
