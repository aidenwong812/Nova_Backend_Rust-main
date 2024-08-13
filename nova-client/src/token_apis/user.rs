use std::{collections::HashMap, error::Error};
use crate::token_apis::response_structs::TokenRouteData;
use anyhow::{anyhow, Result};
use super::response_structs::{self, TokenTradeInfo, TokenTransaction, UserTokenHolding, UserTopLossTokenInfo, UserTopToken};
use db::{client_db, search_user};
use sei_client::{apis::_apis::{get_ibc_info, get_token_smartcontract_info, get_wallet_balances}, field_data::field_data_structions::TokenSwap};
use reqwest::{header::DATE, Client, Proxy};
use serde_json::Value;
use sqlx::PgConnection;
use chrono::{DateTime, Utc};

pub async fn get_user_tokens_holding(wallet_address:&str,conn:&mut PgConnection) ->Option<Vec<UserTokenHolding>> {
    
    if let Ok(balances)=get_wallet_balances(wallet_address).await{

        let mut re_rp:Vec<UserTokenHolding>=vec![]; 
        if let Some(user_info)=search_user(wallet_address, conn).await{
           
            let user_token_transactions=user_info.token_transactions;
            for token in balances{
                
                if token.denom == "usei"{
                    re_rp.push(UserTokenHolding{
                        name:token.denom.to_owned(),
                        demon:token.denom,
                        decimals:Some(6),
                        logo_url:Some(String::from("https://raw.githubusercontent.com/astroport-fi/astroport-token-lists/main/img/sei.png")),
                        amount:token.amount.to_owned(),
                        worth_usei:Some(token.amount.to_owned()),
                        buy_price:None,
                        token_transactions:user_token_transactions.clone().retain(|transaction| transaction.target_token=="usei" || transaction.source_amount=="usei")
                    })
                }else if token.denom.get(0..7).unwrap()=="factory"{
                    
                    let demon={
                        let mut indexs:Vec<usize>=vec![];
                        let demon_vec:Vec<char>=token.denom.chars().collect();
                        demon_vec.iter().enumerate().for_each(|(index,t)|{
                            if t.to_string()=="/".to_string(){
                                indexs.push(index)
                            }
                        });
    
                        token.denom.get(indexs[1]+1..).unwrap()
                    };
                    let amount=token.amount.parse::<usize>().unwrap();

                    let mut demon_transactions: Vec<TokenSwap>=vec![];
                    user_token_transactions.iter().for_each(|t|{
                        if t.source_token==token.denom || t.target_token==token.denom{
                            demon_transactions.push(t.to_owned())
                        }
                    });

                    let mut sells:Vec<(String,usize,usize)>=vec![];
                    let mut buys:Vec<(String,usize,usize)>=vec![];

                    demon_transactions.iter().for_each(|t|{
                        if t.target_token==token.denom{
                            buys.push((t.ts,t.source_amount.parse::<usize>().unwrap(),t.target_amount.parse::<usize>().unwrap()))
                        }else {
                            sells.push((t.ts,t.target_amount.parse::<usize>().unwrap(),t.source_amount.parse::<usize>().unwrap()))
                        }
                    });

                    sells.iter().for_each(|s|{
                        let seel_time=DateTime::parse_from_rfc3339(s.0.as_str()).unwrap().with_timezone(&Utc).naive_utc();
                        buys.iter_mut().for_each(|b|{
                            
                            
                        })
                    });

                    if let Ok(swap_info)=token_swap_routes(&token.denom, "usei", &amount).await{
                        let swap_info=&swap_info[0];
                        let decimals=swap_info.decimals_in;
                        let amount_out=&swap_info.amount_out; 
                        re_rp.push(UserTokenHolding { 
                            name: demon.to_owned(), 
                            demon:token.denom,
                            decimals:Some(decimals),
                            logo_url:None,
                            amount:token.amount , 
                            worth_usei: Some(amount_out.to_owned()), 
                            token_transactions: demon_transactions })
                    }else {
                        re_rp.push(UserTokenHolding { 
                            name: demon.to_owned(), 
                            demon:token.denom,
                            decimals:None,
                            logo_url:None,
                            amount:token.amount , 
                            worth_usei: None,
                            token_transactions: demon_transactions })
                    }
    


                }else if token.denom.get(0..3).unwrap()=="ibc" {

                    let amount=token.amount.parse::<usize>().unwrap();
                    
                    if let Ok(ibc)=get_ibc_info(&token.denom).await{
                        if let Ok(swap_info)=token_swap_routes(&token.denom,"usei", &amount).await{
                            
                            let swap_info=&swap_info[0];
                            let decimals=swap_info.decimals_in;
                            let amount_out=&swap_info.amount_out; 
                            
                            re_rp.push(UserTokenHolding { 
                                name: ibc.base_denom.clone() ,
                                demon:token.denom,
                                decimals: Some(decimals),
                                logo_url:Some(format!("https://raw.githubusercontent.com/astroport-fi/astroport-token-lists/main/img/{}.svg",ibc.base_denom.get(1..).unwrap())), 
                                amount: token.amount, 
                                worth_usei: Some(amount_out.to_owned()), 
                                token_transactions: vec![] })
                        }else {
                            re_rp.push(UserTokenHolding { 
                                name: ibc.base_denom.clone() ,
                                demon:token.denom,
                                decimals: None,
                                logo_url:Some(format!("https://raw.githubusercontent.com/astroport-fi/astroport-token-lists/main/img/{}.svg",ibc.base_denom.get(1..).unwrap())), 
                                amount: token.amount, 
                                worth_usei: None, 
                                token_transactions: vec![] })
                        }
                    }else {
                        if let Ok(swap_info)=token_swap_routes(&token.denom,"usei", &amount).await{
                            let swap_info=&swap_info[0];
                            let decimals=swap_info.decimals_in;
                            let amount_out=&swap_info.amount_out; 
                            
                            re_rp.push(UserTokenHolding { 
                                name: token.denom.clone() ,
                                demon:token.denom,
                                decimals: Some(decimals),
                                logo_url:None,  
                                amount: token.amount, 
                                worth_usei: Some(amount_out.to_owned()), 
                                token_transactions: vec![] })
                        }else {
                            re_rp.push(UserTokenHolding { 
                                name: token.denom.clone() ,
                                demon:token.denom,
                                decimals: None,
                                logo_url:None,
                                amount: token.amount, 
                                worth_usei: None, 
                                token_transactions: vec![] })
                        }
                    }
                    
                }else {
                    let amount=token.amount.parse::<usize>().unwrap();
                    if let Ok(token_info) =get_token_smartcontract_info(&token.denom).await  {
                        if let Ok(swap_info)=token_swap_routes(&token.denom,"usei", &amount).await{
                            let swap_info=&swap_info[0];
                            let decimals=swap_info.decimals_in;
                            let amount_out=&swap_info.amount_out;
                            re_rp.push(UserTokenHolding { 
                                name: token_info.name, 
                                demon:token.denom,
                                decimals:Some(token_info.decimals), 
                                logo_url:Some(token_info.logo_url),
                                amount: token.amount, 
                                worth_usei: Some(amount_out.to_owned()), 
                                token_transactions: vec![] })
                        }else {
                            re_rp.push(UserTokenHolding { 
                                name: token_info.name, 
                                demon:token.denom,
                                decimals:Some(token_info.decimals), 
                                logo_url:Some(token_info.logo_url),
                                amount: token.amount, 
                                worth_usei: None,
                                token_transactions: vec![] })
                        }
                    }else {
                        if let Ok(swap_info)=token_swap_routes(&token.denom,"usei", &amount).await{
                            let swap_info=&swap_info[0];
                            let decimals=swap_info.decimals_in;
                            let amount_out=&swap_info.amount_out; 
                            re_rp.push(UserTokenHolding { 
                                name: token.denom.clone(), 
                                demon:token.denom,
                                decimals:Some(decimals), 
                                logo_url:None,
                                amount: token.amount, 
                                worth_usei: Some(amount_out.to_owned()), 
                                token_transactions: vec![] })
                        }else {
                            re_rp.push(UserTokenHolding { 
                                name: token.denom.clone(), 
                                demon:token.denom,
                                decimals:None,
                                logo_url:None,
                                amount: token.amount, 
                                worth_usei: None, 
                                token_transactions: vec![] })
                        }
                    }
                }
            };
            Some(re_rp)
        
       
        }else {

            for token in balances{

                if token.denom == "usei"{
                    re_rp.push(UserTokenHolding{
                        name:token.denom.to_owned(),
                        demon:token.denom,
                        decimals:Some(6),
                        logo_url:Some(String::from("https://raw.githubusercontent.com/astroport-fi/astroport-token-lists/main/img/sei.png")),
                        amount:token.amount.to_owned(),
                        worth_usei:Some(token.amount.to_owned()),
                        buy_price:None,
                        token_transactions:vec![]
                    })
                }else if token.denom.get(0..7).unwrap()=="factory"{
                    let demon={
                        let mut indexs:Vec<usize>=vec![];
                        let demon_vec:Vec<char>=token.denom.chars().collect();
                        demon_vec.iter().enumerate().for_each(|(index,t)|{
                            if t.to_string()=="/".to_string(){
                                indexs.push(index)
                            }
                        });
    
                        token.denom.get(indexs[1]+1..).unwrap()
                    };
                    let amount=token.amount.parse::<usize>().unwrap();
                    if let Ok(swap_info)=token_swap_routes(&token.denom, "usei", &amount).await{
                        let swap_info=&swap_info[0];
                        let decimals=swap_info.decimals_in;
                        let amount_out=&swap_info.amount_out; 
                        re_rp.push(UserTokenHolding { 
                            name: demon.to_owned(), 
                            demon:token.denom,
                            decimals:Some(decimals),
                            logo_url:None,
                            buy_price:None,
                            amount:token.amount , 
                            worth_usei: Some(amount_out.to_owned()), 
                            token_transactions: vec![] })
                    }else {
                        re_rp.push(UserTokenHolding { 
                            name: demon.to_owned(), 
                            demon:token.denom,
                            decimals:None,
                            logo_url:None,
                            buy_price:None,
                            amount:token.amount , 
                            worth_usei: None,
                            token_transactions: vec![] })
                    }
    
                }else if token.denom.get(0..3).unwrap()=="ibc" {
    
                    let amount=token.amount.parse::<usize>().unwrap();
                    
                    if let Ok(ibc)=get_ibc_info(&token.denom).await{
                        if let Ok(swap_info)=token_swap_routes(&token.denom,"usei", &amount).await{
                            
                            let swap_info=&swap_info[0];
                            let decimals=swap_info.decimals_in;
                            let amount_out=&swap_info.amount_out; 
                            
                            re_rp.push(UserTokenHolding { 
                                name: ibc.base_denom.clone() ,
                                demon:token.denom,
                                decimals: Some(decimals),
                                logo_url:Some(format!("https://raw.githubusercontent.com/astroport-fi/astroport-token-lists/main/img/{}.svg",ibc.base_denom.get(1..).unwrap())), 
                                amount: token.amount, 
                                buy_price:None,
                                worth_usei: Some(amount_out.to_owned()), 
                                token_transactions: vec![] })
                        }else {
                            re_rp.push(UserTokenHolding { 
                                name: ibc.base_denom.clone() ,
                                demon:token.denom,
                                decimals: None,
                                logo_url:Some(format!("https://raw.githubusercontent.com/astroport-fi/astroport-token-lists/main/img/{}.svg",ibc.base_denom.get(1..).unwrap())), 
                                amount: token.amount, 
                                worth_usei: None, 
                                buy_price:None,
                                token_transactions: vec![] })
                        }
                    }else {
                        if let Ok(swap_info)=token_swap_routes(&token.denom,"usei", &amount).await{
                            let swap_info=&swap_info[0];
                            let decimals=swap_info.decimals_in;
                            let amount_out=&swap_info.amount_out; 
                            
                            re_rp.push(UserTokenHolding { 
                                name: token.denom.clone() ,
                                demon:token.denom,
                                decimals: Some(decimals),
                                logo_url:None,  
                                amount: token.amount, 
                                worth_usei: Some(amount_out.to_owned()), 
                                buy_price:None,
                                token_transactions: vec![] })
                        }else {
                            re_rp.push(UserTokenHolding { 
                                name: token.denom.clone() ,
                                demon:token.denom,
                                decimals: None,
                                logo_url:None,
                                amount: token.amount, 
                                worth_usei: None, 
                                buy_price:None,
                                token_transactions: vec![] })
                        }
                    }
                    
                    
    
    
                }else {
                    let amount=token.amount.parse::<usize>().unwrap();
                    if let Ok(token_info) =get_token_smartcontract_info(&token.denom).await  {
                        if let Ok(swap_info)=token_swap_routes(&token.denom,"usei", &amount).await{
                            let swap_info=&swap_info[0];
                            let decimals=swap_info.decimals_in;
                            let amount_out=&swap_info.amount_out;
                            re_rp.push(UserTokenHolding { 
                                name: token_info.name, 
                                demon:token.denom,
                                decimals:Some(token_info.decimals), 
                                logo_url:Some(token_info.logo_url),
                                amount: token.amount, 
                                worth_usei: Some(amount_out.to_owned()), 
                                buy_price:None,
                                token_transactions: vec![] })
                        }else {
                            re_rp.push(UserTokenHolding { 
                                name: token_info.name, 
                                demon:token.denom,
                                decimals:Some(token_info.decimals), 
                                logo_url:Some(token_info.logo_url),
                                amount: token.amount, 
                                worth_usei: None,
                                buy_price:None,
                                token_transactions: vec![] })
                        }
                    }else {
                        if let Ok(swap_info)=token_swap_routes(&token.denom,"usei", &amount).await{
                            let swap_info=&swap_info[0];
                            let decimals=swap_info.decimals_in;
                            let amount_out=&swap_info.amount_out; 
                            re_rp.push(UserTokenHolding { 
                                name: token.denom.clone(), 
                                demon:token.denom,
                                decimals:Some(decimals), 
                                logo_url:None,
                                amount: token.amount, 
                                worth_usei: Some(amount_out.to_owned()), 
                                buy_price:None,
                                token_transactions: vec![] })
                        }else {
                            re_rp.push(UserTokenHolding { 
                                name: token.denom.clone(), 
                                demon:token.denom,
                                decimals:None,
                                logo_url:None,
                                amount: token.amount, 
                                worth_usei: None, 
                                buy_price:None,
                                token_transactions: vec![] })
                        }
                    }
                }
            };
            Some(re_rp)
        }
        

    }else {
        None
    }
}

pub async fn get_user_top_tokens(wallet_address:&str,conn:&mut PgConnection) ->Option<UserTopLossTokenInfo> {
    
    let is_buy_transaction=|transaction:&TokenSwap,target_token:&str|->bool{
        if transaction.target_token==target_token && transaction.source_token=="usei"{
            true
        }else {
            false
        }
    };
    
    if let (Some(tokens_hold),Some(user_info)) =(get_user_tokens_holding(wallet_address,conn).await,search_user(wallet_address,conn).await)  {
        
        let mut TopGainersToken:Vec<UserTopToken>=vec![];
        let mut TopLooser:Vec<UserTopToken>=vec![];
        let mut UnkonwToken:Vec<UserTopToken>=vec![];

        let tokens_transactions=user_info.token_transactions;

        for token_hold in tokens_hold{

            let mut buy_prices:Vec<usize>=vec![];
            let mut buy_amounts:Vec<usize>=vec![];
            
            tokens_transactions.iter().for_each(|transaction|{
                if is_buy_transaction(transaction,&token_hold.demon){
                    buy_prices.push(transaction.source_amount.parse::<usize>().unwrap());
                    buy_amounts.push(transaction.target_amount.parse::<usize>().unwrap())
                }
            });

            if !buy_prices.is_empty() && !buy_amounts.is_empty(){
                
                let all_buy_price:usize=buy_prices.iter().sum();
                let all_buy_amount:usize=buy_amounts.iter().sum();

                let buy_average_price=all_buy_price/all_buy_amount;

                if let Some(now_price)=token_hold.worth_usei{
                    let now_price_usize=now_price.parse::<usize>().unwrap();
                    let unrealized_gains=now_price_usize-buy_average_price;
                    let price_fluctuation=unrealized_gains as f64 /buy_average_price as f64; 
                    
                    if unrealized_gains<0{
                        TopLooser.push(UserTopToken { 
                            name: token_hold.name, 
                            demon: token_hold.demon, 
                            decimals: token_hold.decimals, 
                            logo_url: token_hold.logo_url, 
                            amount: token_hold.amount, 
                            worth_usei:Some(now_price), 
                            price_fluctuation: Some(price_fluctuation), 
                            unrealized_gains: Some(unrealized_gains.to_string()) })
                    }else {
                        TopGainersToken.push(UserTopToken { 
                            name: token_hold.name, 
                            demon: token_hold.demon, 
                            decimals: token_hold.decimals, 
                            logo_url: token_hold.logo_url, 
                            amount: token_hold.amount, 
                            worth_usei:Some(now_price), 
                            price_fluctuation: Some(price_fluctuation), 
                            unrealized_gains: Some(unrealized_gains.to_string()) })
                    }
                  

                }
            }else {
                if let Some(now_price)=token_hold.worth_usei{
   
                    TopGainersToken.push(UserTopToken { 
                        name: token_hold.name, 
                        demon: token_hold.demon, 
                        decimals: token_hold.decimals, 
                        logo_url: token_hold.logo_url, 
                        amount: token_hold.amount, 
                        worth_usei:Some(now_price.clone()), 
                        price_fluctuation: None, 
                        unrealized_gains: Some(now_price) })
                }else {
                    UnkonwToken.push(UserTopToken { 
                        name: token_hold.name, 
                        demon: token_hold.demon, 
                        decimals: token_hold.decimals, 
                        logo_url: token_hold.logo_url, 
                        amount: token_hold.amount, 
                        worth_usei:None, 
                        price_fluctuation: None, 
                        unrealized_gains: None })
                }
            }
            
        }
        
        TopGainersToken.sort_by_key(|token| parse_unrealized_gains(&token.unrealized_gains).unwrap_or(0));
        TopLooser.sort_by_key(|token| parse_unrealized_gains(&token.unrealized_gains).unwrap_or(0));

        Some(UserTopLossTokenInfo{
            top_gainers:TopGainersToken,
            top_looser:TopLooser,
            unkonw:UnkonwToken
        })
    }else {
        None
    }
}


pub async fn get_user_trade_info_tokens(wallet_address:&str,conn:&mut PgConnection) ->Option<TokenTradeInfo> {
    if let Some(user_info) =search_user(wallet_address, conn).await {
        let user_transactions=user_info.token_transactions;
        let mut buy_transactions:Vec<TokenSwap>=vec![];
        let mut sell_transactions:Vec<TokenSwap>=vec![];
        let mut unkonw_transactions:Vec<TokenSwap>=vec![];

        let mut AllTokenTransaction=TokenTransaction::new();
        let mut SellTokenTransaction=TokenTransaction::new();
        let mut BuyTokenTransaction=TokenTransaction::new();

        user_transactions.iter().for_each(|token_transaction|{
            if token_transaction.source_token=="usei"{
                buy_transactions.push(token_transaction.to_owned())
            }else if token_transaction.target_token=="usei" {
                sell_transactions.push(token_transaction.to_owned())
            }else {
                unkonw_transactions.push(token_transaction.to_owned())
            }
        });

        TokenTransaction::add_data(&mut AllTokenTransaction, user_transactions.clone(), "all");
        TokenTransaction::add_data(&mut BuyTokenTransaction,buy_transactions , "buy");
        TokenTransaction::add_data(&mut SellTokenTransaction, sell_transactions, "sell");

        Some(TokenTradeInfo{
            all:AllTokenTransaction,
            sell:SellTokenTransaction,
            buy:BuyTokenTransaction
        })

    }else {
        None
    }
}

async fn token_swap_routes(source_token:&str,target_source:&str,amount:&usize)->Result<Vec<TokenRouteData>> {
    
    let url=format!("https://sei.astroport.fi/api/routes?start={}&end={}&amount={}&chainId=pacific-1&limit=1",source_token,target_source,amount);
    let client=Client::new();
    let data_rp:Value=client.get(url)
        .send().await?
        .json().await?;


    let data=serde_json::from_value::<Vec<TokenRouteData>>(data_rp);
    Ok(data?)
}
fn parse_unrealized_gains(unrealized_gains: &Option<String>) -> Option<i64> {
    unrealized_gains.as_ref().and_then(|s| s.parse::<i64>().ok())
}