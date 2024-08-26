mod rp_structs;

use serde::{Deserialize, Serialize};
use std::fs;
use std::time::Duration;
use std::{fs::File, thread};
use std::io::{BufReader, Write};
use db::{update_nfts_holding, update_nfts_transactions, update_token_transaction};
use sqlx::{PgConnection, Pool, Postgres};
use std::collections::HashMap;
use anyhow::{anyhow, Result};
use reqwest::Client;
use rp_structs::BlockTransactionInfo;
use sei_client::field_data::{data_structions::HashData, field_data_structions::{NFTtransaction, _NftTransaction}, nft_transaction::{nft_transaction_data, NftMessage}, token_swap::swap_datas};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::{Mutex,mpsc::{Sender,Receiver}};

pub async fn get_transaction_hash(block_hash:usize,tx_hash_sender:Sender<String>) -> Result<()> {
    let client=Client::builder().timeout(Duration::from_secs(10)).build()?;
    let url=format!("https://celatone-api-prod.alleslabs.dev/v1/sei/pacific-1/blocks/{}/txs?limit=1000&offset=0&is_wasm=true&is_move=false&is_initia=false",block_hash);
    
    let data_rp:Value=client.get(url).send()
                            .await?
                            .json().await?;
    
    if data_rp.get("items").is_some(){
        if let Ok(mut items)=serde_json::from_value::<Vec<BlockTransactionInfo>>(data_rp.get("items").unwrap().to_owned()){
            if !items.is_empty(){
                items.retain(|item| item.success&&item.messages.iter().any(|message| message._type != "/seiprotocol.seichain.oracle.MsgAggregateExchangeRateVote"));
                
                for item in items{
                    tx_hash_sender.clone().send(item.hash.get(2..).unwrap().to_string().to_uppercase()).await.unwrap();
                };
                Ok(())
            }else {
                return  Err(anyhow!("null"));
            }
        }else {
            return  Err(anyhow!("1"));
        }
    }else if data_rp.get("code").is_some() {
        let err_code=data_rp.get("code").unwrap().as_i64().unwrap();
        if err_code==500{
            return  Err(anyhow!("500"));
        }else {
            return  Err(anyhow!("0"));
        }
    }else {
        return  Err(anyhow!("0"));
    }

}

            // token 的路由 logs 中 的events中的 event的 type 为 mint 或  coinbase  delegate 
            //  token 的路由 logs 中 的events中的 event的 type 为 wasm 里面的key 为swap   || pool swap
            
            // nft 路由   logs 中 的events中的 event的 type 为 wasm 里面的key 为  mint_nft |  bacth_bids  | transfer_nft  | purchase_cart
            //nft 路由   logs 中 的events中的 event的 type 为 wasm-buy_now  wasm-accept_bid  wasm-withdraw_bid  wasm-create_auction  wasm-place_bid

pub async fn transcation_datas_routes(
    mut tx_hash_rx:Receiver<String>,
        nft_msg_tx:Sender<NftMessage>,
        conn:Pool<Postgres>,
)->Result<()>{ //Result<Map<String,Value>,Box<dyn Error>>{
  
    //定义闭包
    // token swap
    let is_token=|hash_data:&HashData|->bool{

        hash_data.tx_response.logs.iter().any(|log|{ 
            if log.events.iter().find(|event| event._type=="wasm").is_some(){
                log.events.iter().any(|event|{
                    event._type=="wasm" && event.attributes.iter().any(|attr| attr.value=="swap")
                })
            }else {
                false
            }
        })
    };
    
    let is_nft=|hash_data:&HashData|->bool{
        hash_data.tx_response.logs.iter().any(|log|{
            if log.events.iter().find(|event| event._type=="wasm").is_some(){
                log.events.iter().any(|event|{
                    event._type=="wasm" && event.attributes.iter().any(|attr|{
                        attr.value=="transfer_nft".to_string() || 
                        attr.value=="batch_bids".to_string()   ||
                        attr.value=="purchase_cart".to_string() ||
                        attr.value=="mint_nft".to_string() 
                    })
                })
            }else {
                log.events.iter().any(|event|{
                    event._type=="wasm-accept_bid".to_string() ||                       //同意bid
                    event._type==" wasm-removed_token_from_offers".to_string() ||       // 交易成功后的处理
                    event._type=="wasm-create_auction".to_string() ||                   //创建list
                    event._type=="wasm-place_bid".to_string() ||                        //bid
                    event._type=="wasm-cancel_auction".to_string() ||                   //取消list 
                    event._type=="wasm-buy_now".to_string() ||                          //sales
                    event._type=="wasm-withdraw_bid".to_string()                       
                })
            }
        })
    };
     
    let conn=Arc::new(conn);
    let nft_msg_tx=Arc::new(nft_msg_tx);
    
    while let Some(hash) = tx_hash_rx.recv().await {
        let conn=Arc::clone(&conn);
        let nft_msg_tx=Arc::clone(&nft_msg_tx);
        tokio::spawn(async move {
            
            if let Ok(hash_data) = get_transaction_by_scan(&hash).await{
                if hash_data.tx_response.code==0{
                    if is_token(&hash_data){
                        if let Ok(mut conn) =conn.acquire().await  {
                            for token_swap_data in swap_datas(hash_data){
                                if update_token_transaction(&token_swap_data.account, &mut conn, vec![token_swap_data.clone()]).await.is_some(){
                                    println!("update token swap success -> {:?}",&token_swap_data.tx);
                                }else {
                                    println!("update token swap erro -> {:?}",&token_swap_data.tx);
                                }
                            };
                            drop(conn)
                        }else {
                            println!("pool erro")
                        }
                       
                    }else if is_nft(&hash_data){
                       nft_transaction_data(hash_data, nft_msg_tx).await;
                    }
                }
            };
        }).await?
    };
    Ok(())
}


pub async fn operate_db(mut nft_msg_rx:Receiver<NftMessage>,conn:Arc<Pool<Postgres>>)->Result<()>  {
    // println!("操作数据库");

    while let Some(transaction) =nft_msg_rx.recv().await {
        
            match transaction {
                NftMessage::AcceptBidNft(msgs)=>{
                    for msg in msgs{
                        
                        let transaction=NFTtransaction{
                            transaction:_NftTransaction::AcceptBid(msg.clone()),
                            _type:"accpet_bid_nft".to_string(),
                        };
                        let mut  conn=conn.acquire().await?;
                     
                        
                        let update_sender_nft_transactions=update_nfts_transactions(&msg.transfer.sender, &mut conn, vec![transaction.clone()]).await;
                        let update_recipient_nft_transactons=update_nfts_transactions(&msg.transfer.recipient, &mut conn, vec![transaction.clone()]).await;

                        if  update_sender_nft_transactions.is_some() && update_recipient_nft_transactons.is_some(){
                            println!("update db sucess || accpet bid  {}",&msg.transfer.tx)
                        }else {
                            println!("update db err || accpet bid  ");
                            println!("{:?}\n",transaction)
                        };
                        drop(conn);
                    }
                },
                NftMessage::CretaeAuctionNft(msgs)=>{
                    for msg in msgs{
                        
                        let transaction=NFTtransaction{
                            transaction:_NftTransaction::CretaeAuction(msg.clone()),
                            _type:"create_auction_nft".to_string()
                        };
                        let mut  conn=conn.acquire().await?;
                        
                        let update_sender_nft_transactions=update_nfts_transactions(&msg.transfer.sender, &mut conn, vec![transaction.clone()]).await;
                        let update_recipient_nft_transactons=update_nfts_transactions(&msg.transfer.recipient, &mut conn, vec![transaction.clone()]).await;


                        if update_sender_nft_transactions.is_some() && update_recipient_nft_transactons.is_some(){
                            println!("update db sucess || CretaeAuctionNft -> {}",&msg.transfer.tx);
                        }else {
                            println!("update db eroo || CretaeAuctionNft");
                            println!("{:?}\n",transaction)
                        }
                        drop(conn);
                    }
                },
                NftMessage::CancelAuctionNft(msgs)=>{
                    for msg in msgs{
                        
                        let transaction=NFTtransaction{
                            transaction:_NftTransaction::CancelAuction(msg.clone()),
                            _type:"cancel_auction_nft".to_string()
                        };
                        let mut  conn=conn.acquire().await?;                       
                        let update_sender_nft_transactions=update_nfts_transactions(&msg.transfer.sender, &mut conn, vec![transaction.clone()]).await;
                        let update_recipient_nft_transactons=update_nfts_transactions(&msg.transfer.recipient, &mut conn, vec![transaction.clone()]).await;


                        if  update_sender_nft_transactions.is_some() && update_recipient_nft_transactons.is_some(){
                            println!("update db sucess || CancelAuctionNft  -> {}",&msg.transfer.tx)
                        }else {
                            println!("update db eroo || CancelAuctionNft");
                            println!("{:?}\n",transaction)
                        }
                        drop(conn);
                    }
                },
                NftMessage::OnlyTransferNft(msgs)=>{
                    for msg in msgs{

                        let transaction=NFTtransaction{
                            transaction:_NftTransaction::OnlyTransfer(msg.clone()),
                            _type:"only_transfer_nft".to_string()
                        };
                        let mut  conn=conn.acquire().await?;                      
                        let update_sender_nft_transactions=update_nfts_transactions(&msg.sender, &mut conn, vec![transaction.clone()]).await;
                        let update_recipient_nft_transactons=update_nfts_transactions(&msg.recipient, &mut conn, vec![transaction.clone()]).await;


                        if  update_sender_nft_transactions.is_some() && update_recipient_nft_transactons.is_some(){
                            println!("update db sucess || OnlyTransferNft  -> {}",&msg.tx)
                        }else {
                            println!("update db eroo || OnlyTransferNft");
                            println!("{:?}\n",transaction)
                        }

                        drop(conn);
                    }
                },
                NftMessage::BatchBids(msgs)=>{
                    for msg in msgs{

                        let transaction=NFTtransaction{
                            transaction:_NftTransaction::BatchBids(msg.clone()),
                            _type:"batch_bids_nft".to_string()
                        };
                        let mut  conn=conn.acquire().await?;
                        let update_sender_nft_transactions=update_nfts_transactions(&msg.transfer.sender, &mut conn, vec![transaction.clone()]).await;
                        let update_recipient_nft_transactons=update_nfts_transactions(&msg.transfer.recipient, &mut conn, vec![transaction.clone()]).await;


                        if update_sender_nft_transactions.is_some() && update_recipient_nft_transactons.is_some(){
                            println!("update db sucess || BatchBids  ->  {}",&msg.transfer.tx )
                        }else {
                            println!("update db eroo || BatchBids");
                            println!("{:?}\n",transaction)
                        }

                        drop(conn);
                    }
                },
                NftMessage::PurchaseCartNft(msgs)=>{
                    for msg in msgs{

                        let transaction=NFTtransaction{
                            transaction:_NftTransaction::PurchaseCart(msg.clone()),
                            _type:"purchase_cart_nft".to_string()
                        };
                        let mut  conn=conn.acquire().await?;
                        let update_sender_nft_transactions=update_nfts_transactions(&msg.transfer.sender, &mut conn, vec![transaction.clone()]).await;
                        let update_recipient_nft_transactons=update_nfts_transactions(&msg.transfer.recipient, &mut conn, vec![transaction.clone()]).await;


                        if update_sender_nft_transactions.is_some() && update_recipient_nft_transactons.is_some(){
                            println!("update db sucess || PurchaseCartNft -> {}" ,&msg.transfer.tx)
                        }else {
                            println!("update db eroo || PurchaseCartNft");
                            println!("{:?}\n",transaction)
                        }

                        drop(conn);
                    }
                },
                NftMessage::Mint(msgs)=>{
                    for msg in msgs{
                        
                        let transaction=NFTtransaction{
                            transaction:_NftTransaction::Mint(msg.clone()),
                            _type:"mint_nft".to_string()
                        };

                        let mut  conn=conn.acquire().await?;                        
                        let update_recipient_nft_transactons=update_nfts_transactions(&msg.recipient, &mut conn, vec![transaction.clone()]).await;


                        if  update_recipient_nft_transactons.is_some(){
                            println!("update db sucess || Mint -> {}",&msg.tx )
                        }else {
                            println!("update db eroo || Mint");
                            println!("{:?}\n",transaction)
                        }
                        drop(conn);
                    }
                },
                NftMessage::FixedSellNft(msgs)=>{
                    for msg in msgs{
                        
                        let transaction=NFTtransaction{
                            transaction:_NftTransaction::FixedSell(msg.clone()),
                            _type:"fixed_price".to_string(),
                        };
                        let mut  conn=conn.acquire().await?;                        
                        let update_sender_nft_transactions=update_nfts_transactions(&msg.transfer.sender, &mut conn, vec![transaction.clone()]).await;
                        let update_recipient_nft_transactons=update_nfts_transactions(&msg.transfer.recipient, &mut conn, vec![transaction.clone()]).await;

                        if update_sender_nft_transactions.is_some() && update_recipient_nft_transactons.is_some(){
                            println!("update db sucess || PurchaseCartNft -> {}",&msg.transfer.tx)
                        }else {
                            println!("update db eroo || PurchaseCartNft");
                            println!("{:?}\n",transaction)
                        }
                        drop(conn);
                    }
                },
                _=>{},
            };  
             
    }
    
    thread::sleep(Duration::from_secs(3));
    Ok(())
}


async fn get_transaction_by_scan(tx_hash:&str) -> Result<HashData> {
    let client=Client::builder().timeout(Duration::from_secs(10)).build()?;
    let url=format!("https://celatone-api-prod.alleslabs.dev/v1/sei/pacific-1/txs/{}",tx_hash);
    println!("{:?}",url);
    let rp_data:Value=client.get(url)
                         .send().await?
                         .json().await?;
    let data=serde_json::from_value::<HashData>(rp_data)?;
    
    Ok(data)
}

#[derive(Debug, Deserialize,Clone,Serialize)]
pub struct Config{
    pub start_block:usize,
    pub end_block:usize,
    pub sync_block:usize
}impl Config {
    pub fn read(config_path:String)->Result<Self> {
        let  config_life = File::open(config_path)?;
        let reader=BufReader::new(config_life);
        let config:Config=serde_yaml::from_reader(reader)?;
        Ok(config)
    }

    pub fn write(config_path:String,now_block:usize)->Result<()>{
        let mut config=Config::read(config_path.clone())?;
        config.sync_block=now_block;
        let config=serde_yaml::to_string(&config)?;
        fs::write(config_path, config);
        Ok(())
    }
}