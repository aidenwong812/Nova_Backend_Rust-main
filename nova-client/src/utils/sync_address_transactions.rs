use std::{collections::HashMap, fmt::format};
use sei_client::field_data::{data_structions::Event, field_data_structions::{NFTtransaction, _NftTransaction}, nft_transaction::{accept_bid_nft, batch_bids_nfts, cancel_auction_nft, create_auction_nft, fixed_sell_nfts, mint_nft_datas, only_create_aucton_nft, only_transfer_nft, purchase_cart_nft}, token_swap::{height_token_swap, normal_token_swap}};
use reqwest::Client;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{postgres::any::driver, PgConnection, Postgres};
use db::{search_user, update_contract_create_auctions, update_nfts_holding, update_nfts_transactions, update_token_transaction};

#[derive(Serialize, Deserialize,Clone,Debug)]
struct Transactions{
        blockHeight:Value,
        code:Value,
        fee:String,
        gas:Value,
    pub hash:String,
        memo:Value,
        messages:Value,
        rawLog:String,
        sender:String,
        time:String,
        timeoutHeight:Value,
        #[serde(rename = "type")]
        _type:Value,
}

#[derive(Serialize, Deserialize,Clone,Debug)]
pub struct _Event{
    pub events:Vec<Event> 
}



pub async fn sync(address:&str,mut conn:sqlx::pool::PoolConnection<Postgres>) -> Result<()> {

    let client=Client::new();
    let mut transaction_datas:HashMap<String,(String,String,Vec<_Event>)>=HashMap::new();
    let transaction_api_url=format!("https://api.seistream.app/accounts/{}/transactions",address);
    
    let hsitroy_rawlogs_data1:Value=client.get(&transaction_api_url)
                    .query(&[("offset",0),("limit",50)])
                    .send().await?
                    .json().await?;
    
    let pagination=hsitroy_rawlogs_data1.get("pagination").unwrap();
    let pages=pagination.get("pages").unwrap().as_u64();
    if pages.is_none(){
        return  Err(anyhow!("don't have transactions data"));
    };

    let items=hsitroy_rawlogs_data1.get("items").unwrap();
    let datas=serde_json::from_value::<Vec<Transactions>>(items.to_owned()).unwrap();
    datas.iter().for_each(|data|{
        transaction_datas.insert(data.hash.to_string(), (data.sender.to_string(),data.time.to_string(),serde_json::from_str::<Vec<_Event>>(data.rawLog.as_str()).unwrap()));
    });

    for page in 1..pages.unwrap()+1{
        let hsitroy_rawlogs_data:Value=client.get(&transaction_api_url)
                                            .query(&[("offset",page*50),("limit",50)])
                                            .send().await?
                                            .json().await?;
      
       let items=hsitroy_rawlogs_data.get("items").unwrap();
       let datas=serde_json::from_value::<Vec<Transactions>>(items.to_owned()).unwrap();
       datas.iter().for_each(|data|{
            transaction_datas.insert(data.hash.to_string(), (data.sender.to_string(),data.time.to_string(),serde_json::from_str::<Vec<_Event>>(data.rawLog.as_str()).unwrap()));
        });
    };


    // 过滤
    //定义闭包
    // token swap
    let is_heiht_token=|events:&_Event|->bool{
        events.events.iter().any(|event|{
            event._type=="wasm" && event.attributes.iter().any(|attr|{
                attr.value=="execute_swap_and_action" || 
                attr.value=="dispatch_user_swap" ||
                attr.value=="dispatch_post_swap_action" ||
                attr.value=="execute_user_swap"
            })
        })
    };

    let  is_noraml_token=|events:&_Event|->bool{
        events.events.iter().any(|event|{
            event._type=="wasm" && event.attributes.iter().any(|attr|{
                attr.value=="swap"})
        })
    };

    
    let is_nft=|events:&_Event|->bool{
        if events.events.iter().any(|event|{
            event._type=="wasm" && event.attributes.iter().any(|attr|{
                attr.value=="transfer_nft".to_string() || 
                attr.value=="batch_bids".to_string()   ||
                attr.value=="purchase_cart".to_string() ||
                attr.value=="mint_nft".to_string() 
            })
        }){
            true
        }else if events.events.iter().any(|event|{
            event._type=="wasm-accept_bid".to_string() ||                       //同意bid
            event._type==" wasm-removed_token_from_offers".to_string() ||       // 交易成功后的处理
            event._type=="wasm-create_auction".to_string() ||                   //创建list
            event._type=="wasm-place_bid".to_string() ||                        //bid
            event._type=="wasm-cancel_auction".to_string() ||                   //取消list 
            event._type=="wasm-buy_now".to_string() ||                          //sales
            event._type=="wasm-withdraw_bid".to_string()     
        }) {
            true
        }else {
            false
        }
    };

    for(tx_hash,(sender,time,transaction_event)) in transaction_datas{
        for event in transaction_event{
            // println!("{:#?}",event);
            if is_nft(&event){
                    // 定义过滤闭包
                    // mint nft
                    let is_mint_nft=|events:&Vec<Event>| ->bool{
                        events.iter().any(|event|{
                            event._type=="wasm" && event.attributes.iter().any(|attribute|{attribute.value=="mint_nft"})
                        })
                    };
                    // batch_bids
                    let is_batch_bids=|events:&Vec<Event>| ->bool{
                        if events.iter().any(|event|{
                            event._type=="wasm" && event.attributes.iter().any(|attribute|{attribute.value=="batch_bids"}) && event.attributes.iter().any(|attribute|{attribute.value=="transfer_nft"}) 
                        }) && events.iter().any(|event|{event._type=="wasm-buy_now"}){
                            return  true;
                        }else {
                            return false;
                        }
                    };
                    let is_fixed_sell=|events:&Vec<Event>| ->bool{
                    if events.iter().any(|event|{
                        event._type=="wasm" &&event.attributes.iter().any(|attr|{attr.value=="fixed_sell"})
                    }){
                        return true;
                    }else {
                        return false;
                    }
                    };
                    let is_only_transfer_nft=|events:&Vec<Event>|->bool{
                        if events.iter().any(|event|{
                            event._type=="wasm" &&event.attributes.iter().any(|attribute|{attribute.value=="transfer_nft"})&&event.attributes[1].value=="transfer_nft"
                            }) && events.last().unwrap()._type=="wasm"{
                                return  true;
                            }else {
                                return  false;
                            }
                    };
                    let is_create_auction_nft=|events:&Vec<Event>| -> bool{
                    if events.iter().any(|event|{event._type=="wasm"}) && events.iter().any(|event|{event._type=="wasm-create_auction"}){
                        return true;
                    }else {
                        return false;
                    }
                    };
                    let is_cancel_auction_nft=|events:&Vec<Event>| -> bool{
                        if events.iter().any(|event|{event._type=="wasm"}) && events.iter().any(|event|{event._type=="wasm-cancel_auction"}){
                            return true;
                        }else {
                            return false;
                        }
                    };
                    let is_purchase_nft=|events:&Vec<Event>| ->bool{
                        if events.iter().any(|event|{
                            event._type=="wasm" && event.attributes.iter().any(|attribute|{attribute.value=="purchase_cart"}) && event.attributes.iter().any(|attribute|{attribute.value=="transfer_nft"}) 
                        }) && events.iter().any(|event|{event._type=="wasm-buy_now"}){
                            return  true;
                        }else {
                            return false;
                        }
                    };
                    // only create auction
                    let is_only_create_aucton_nft=|events:&Vec<Event>|->bool{
                        if events.iter().any(|event|{event._type=="wasm-create_auction"}){
                            return true;
                        }else {
                            return false;
                        }
                    };
                    let is_accept_bid_nft=|events:&Vec<Event>| -> bool{
                        if events.iter().any(|event|{event._type=="wasm"}) && events.iter().any(|event|{event._type=="wasm-accept_bid"}){
                            return true;
                        }else {
                            return false;
                        }
                    };

                    let events=event.events;
                    if is_mint_nft(&events){
                        let mint_event=events.iter().find(|event|{event._type=="wasm"}).unwrap();
                        let data=mint_nft_datas(mint_event.to_owned().attributes,tx_hash.to_owned(),time.to_owned()).await;
                        if data.is_some(){
                            let data=data.unwrap();
                            for msg in data{
                                let transaction=NFTtransaction{
                                    transaction:_NftTransaction::Mint(msg.clone()),
                                    _type:"mint_nft".to_string()
                                };
                                // update_nfts_holding(&msg.recipient, &msg.collection, &msg.token_id, "add", &mut conn).await;
                                update_nfts_transactions(&msg.recipient, &mut conn, vec![transaction.clone()]).await;

                            }
                        }

                    }else if is_batch_bids(&events) {
                        
                        let data=batch_bids_nfts(events,tx_hash.to_owned(),time.to_owned()).await;
                        if data.is_some(){
                            let megs=data.unwrap();
                            for msg in megs{
                                let transaction=NFTtransaction{
                                    transaction:_NftTransaction::BatchBids(msg.clone()),
                                    _type:"batch_bids_nft".to_string()
                                };
                                    // update_nfts_holding(&msg.transfer.sender, &msg.transfer.collection, &msg.transfer.token_id, "del", &mut conn).await;
                                    // update_nfts_holding(&msg.transfer.recipient, &msg.transfer.collection, &msg.transfer.token_id, "add", &mut conn).await;
                                
                                    update_nfts_transactions(&msg.transfer.sender, &mut conn, vec![transaction.clone()]).await;
                                    update_nfts_transactions(&msg.transfer.recipient, &mut conn, vec![transaction.clone()]).await;

                                }
                        }    
                    }else if is_fixed_sell(&events) {

                        let fixed_sell_event=events.iter().find(|event|{event._type=="wasm"}).unwrap();
                        let data=fixed_sell_nfts(fixed_sell_event.to_owned().attributes,tx_hash.to_owned(),time.to_owned()).await;

                        if data.is_some(){
                            let megs=data.unwrap();
                            for msg in megs{
                            
                                let transaction=NFTtransaction{
                                    transaction:_NftTransaction::FixedSell(msg.clone()),
                                    _type:"fixed_price".to_string(),
                                };
                                // update_nfts_holding(&msg.transfer.sender, &msg.transfer.collection, &msg.transfer.token_id, "del", &mut conn).await;
                                // update_nfts_holding(&msg.transfer.recipient, &msg.transfer.collection, &msg.transfer.token_id, "add", &mut conn).await;
                                update_nfts_transactions(&msg.transfer.sender, &mut conn, vec![transaction.clone()]).await;
                                update_nfts_transactions(&msg.transfer.recipient, &mut conn, vec![transaction.clone()]).await;
                            }
                        }
                    }else if is_only_transfer_nft(&events) {

                        let only_transfer_nft_event=events.iter().find(|event|{event._type=="wasm"}).unwrap();
                        let data=only_transfer_nft(only_transfer_nft_event.to_owned().attributes,tx_hash.to_owned(),time.to_owned()).await;
                        if data.is_some(){
                            let megs=data.unwrap();
                            for msg in megs{

                                let transaction=NFTtransaction{
                                    transaction:_NftTransaction::OnlyTransfer(msg.clone()),
                                    _type:"only_transfer_nft".to_string()
                                };
                                // update_nfts_holding(&msg.sender, &msg.collection, &msg.token_id, "del", &mut conn).await;
                                // update_nfts_holding(&msg.recipient, &msg.collection, &msg.token_id, "add", &mut conn).await;
                                
                                update_nfts_transactions(&msg.sender, &mut conn, vec![transaction.clone()]).await;
                                update_nfts_transactions(&msg.recipient, &mut conn, vec![transaction.clone()]).await;
                            }
                        }

                    }else if is_create_auction_nft(&events) {
                        
                        let data=create_auction_nft(events,tx_hash.to_owned(),time.to_owned()).await;
                        if data.is_some(){
                            let megs=data.unwrap();
                            for msg in megs{
                        
                                let transaction=NFTtransaction{
                                    transaction:_NftTransaction::CretaeAuction(msg.clone()),
                                    _type:"create_auction_nft".to_string()
                                };                                
                                // update_nfts_holding(&msg.transfer.sender, &msg.transfer.collection, &msg.transfer.token_id, "del", &mut conn).await;
                                // update_nfts_holding(&msg.transfer.recipient, &msg.transfer.collection, &msg.transfer.token_id, "add", &mut conn).await;
                                
                                update_nfts_transactions(&msg.transfer.sender, &mut conn, vec![transaction.clone()]).await;
                                update_nfts_transactions(&msg.transfer.recipient, &mut conn, vec![transaction.clone()]).await;
                            }
                        }

                    }else if is_cancel_auction_nft(&events) {
                        
                        let data=cancel_auction_nft(events,tx_hash.to_owned(),time.to_owned()).await;
                        if data.is_some(){
                            let megs=data.unwrap();
                            for msg in megs{
                        
                                let transaction=NFTtransaction{
                                    transaction:_NftTransaction::CancelAuction(msg.clone()),
                                    _type:"cancel_auction_nft".to_string()
                                };
                                // update_nfts_holding(&msg.transfer.sender, &msg.transfer.collection, &msg.transfer.token_id, "del", &mut conn).await;
                                // update_nfts_holding(&msg.transfer.recipient, &msg.transfer.collection, &msg.transfer.token_id, "add", &mut conn).await;
                                update_nfts_transactions(&msg.transfer.sender, &mut conn, vec![transaction.clone()]).await;
                                update_nfts_transactions(&msg.transfer.recipient, &mut conn, vec![transaction.clone()]).await;
                            }
                        }

                    }else if is_purchase_nft(&events) {
                        
                        let data=purchase_cart_nft(events,tx_hash.to_owned(),time.to_owned()).await;
                        if data.is_some(){
                            let megs=data.unwrap();
                            for msg in megs{

                                let transaction=NFTtransaction{
                                    transaction:_NftTransaction::PurchaseCart(msg.clone()),
                                    _type:"purchase_cart_nft".to_string()
                                };
                                    // update_nfts_holding(&msg.transfer.sender, &msg.transfer.collection, &msg.transfer.token_id, "del", &mut conn).await;
                                    // update_nfts_holding(&msg.transfer.recipient, &msg.transfer.collection, &msg.transfer.token_id, "add", &mut conn).await;
                                
                                    update_nfts_transactions(&msg.transfer.sender, &mut conn, vec![transaction.clone()]).await;
                                    update_nfts_transactions(&msg.transfer.recipient, &mut conn, vec![transaction.clone()]).await;
                            }
                        }

                    }else if is_only_create_aucton_nft(&events) {
                        let only_create_aucton_event=events.iter().find(|event|{event._type=="wasm-create_auction"}).unwrap();
                        let data=only_create_aucton_nft(only_create_aucton_event.to_owned().attributes,time.to_owned()).await;
                        if data.is_some(){
                            let megs=data.unwrap();
                            for msg in megs{
                                update_contract_create_auctions(&msg.collection_address, vec![msg.clone()], &mut conn).await;
                            }
                        }

                    }else if is_accept_bid_nft(&events) {
                        let data=accept_bid_nft(events,tx_hash.to_owned(),time.to_owned()).await;
                        if data.is_some(){
                            let megs=data.unwrap();
                            for msg in megs{
                        
                                let transaction=NFTtransaction{
                                    transaction:_NftTransaction::AcceptBid(msg.clone()),
                                    _type:"accpet_bid_nft".to_string(),
                                };
                                    // update_nfts_holding(&msg.transfer.sender, &msg.transfer.collection, &msg.transfer.token_id, "del", &mut conn).await;
                                    // update_nfts_holding(&msg.transfer.recipient, &msg.transfer.collection, &msg.transfer.token_id, "add", &mut conn).await;
                                
                                    update_nfts_transactions(&msg.transfer.sender, &mut conn, vec![transaction.clone()]).await;
                                    update_nfts_transactions(&msg.transfer.recipient, &mut conn, vec![transaction.clone()]).await;

                            }
                        }
                    }


            }else if is_heiht_token(&event) {
                
                let events=event.events;
                
                let wasm_event=events.iter().find(|event|{event._type=="wasm"}).unwrap();
                let data=normal_token_swap(wasm_event.to_owned().attributes,tx_hash.to_owned(),time.to_owned(),sender.to_owned());
                if data.is_some(){
                    let data=data.unwrap();
                    update_token_transaction(&data.account, &mut conn, vec![data.to_owned()]).await;
                }
            }else if is_noraml_token(&event) {
                
                let events=event.events;

                let wasm_event=events.iter().find(|event|{event._type=="wasm"}).unwrap();
                let data=height_token_swap(wasm_event.to_owned().attributes,tx_hash.to_owned(),time.to_owned(),sender.to_owned());
                update_token_transaction(&data.account, &mut conn, vec![data.to_owned()]).await;
            }
        }
    
    }
    // 写入数据库

    Ok(())
    
}

