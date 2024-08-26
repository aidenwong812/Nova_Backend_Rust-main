use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize,Clone,Debug)]
pub struct BlockTransactionInfo{
    pub created:String,
    pub hash:String,
    pub height :Value,
        is_clear_admin:bool,
        is_execute:bool,
        is_ibc:bool,
        is_instantiate:bool,
        is_migrate:bool,
        is_send:bool,
        is_store_code:bool,
        is_update_admin:bool,
    pub messages:Vec<BlockMessage>,
    pub sender:String,
    pub success:bool,
}

#[derive(Serialize, Deserialize,Clone,Debug)]
pub struct BlockMessage{
    pub detail:Value,
    #[serde(rename = "type")]
    pub _type:String,
}

// {
//     "code": 500,
//     "description": "[{'message': 'value \"62000000000000\" is out of range for type integer', 'extensions': {'path': '$', 'code': 'data-exception'}}]"
//   }