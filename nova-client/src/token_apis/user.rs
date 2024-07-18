use std::error::Error;
use crate::token_apis::response_structs::TokenRouteData;

use super::response_structs;
use sei_client::apis::_apis::get_wallet_balances;
use reqwest::{Client, Proxy};
use serde_json::Value;

pub async fn get_user_tokens_holding(wallet_address:&str)  {
     
}

pub async fn token_swap_routes(source_token:&str,target_source:&str,amount:&usize)->Result<Vec<TokenRouteData>,Box<dyn Error>> {
    
    let url=format!("https://sei.astroport.fi/api/routes?start={}&end={}&amount={}&chainId=pacific-1&limit=1",source_token,target_source,amount);
    let client=Client::new();
    let data_rp:Value=client.get(url)
        .send().await?
        .json().await?;

    let data=serde_json::from_value::<Vec<TokenRouteData>>(data_rp)?;
    Ok(data)
}