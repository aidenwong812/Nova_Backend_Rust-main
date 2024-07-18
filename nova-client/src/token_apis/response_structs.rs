use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize,Clone,Debug)]
pub struct TokenRouteData{
        id:String,
    pub swaps:Vec<SwapData>,
    pub denom_in:String,
    pub decimals_in:u64,
    pub price_in:f64,
    pub value_in:String,
    pub amount_in:String,
    pub denom_out:String,
    pub decimals_out:u64,
    pub price_out:u64,
    pub value_out:String,
    pub amount_out:String,
    pub price_difference:Option<f64>,
    pub price_impact :f64,
}

#[derive(Serialize, Deserialize,Clone,Debug)]
pub struct SwapData{
    contract_addr:String,
    from:String,
    to:String,
    #[serde(rename = "type")]
    _type:String,
    illiquid:bool,
}

