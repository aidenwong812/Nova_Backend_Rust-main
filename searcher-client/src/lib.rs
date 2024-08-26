use std::collections::HashMap;
use std::error::Error;
use std::sync::{mpsc::{Sender,Receiver},Mutex as MX};
use std::sync::Arc;
use std::thread;
use sei_client::field_data::nft_transaction::{nft_transaction_data, NftMessage};
use sei_client::field_data::{
    data_structions::{HashData,Log},
    token_swap::swap_datas,
};
use sei_client::field_data::field_data_structions::{NFTtransaction, NftToken, OnlyCreateAuction, TokenSwap, _NftTransaction};
use anyhow::Result;
use sqlx::{PgConnection, Pool, Postgres};
use tokio::sync::{Mutex, MutexGuard, Semaphore};
use tokio::{sync::{self, mpsc::channel as tokio_channel}, time::{sleep, Duration}};

use websocket::{WebSocketError, OwnedMessage};
use websocket::client::ClientBuilder;
use sei_client::apis::_apis::{get_nft_info, get_transaction_txs_by_tx};
use serde_json::{Map, Value};
use db::{client_db, update_contract_create_auctions, update_nfts_holding, update_nfts_transactions, update_token_transaction};


            // token 的路由 logs 中 的events中的 event的 type 为 mint 或  coinbase  delegate 
            //  token 的路由 logs 中 的events中的 event的 type 为 wasm 里面的key 为swap   || pool swap
            
            // nft 路由   logs 中 的events中的 event的 type 为 wasm 里面的key 为  mint_nft |  bacth_bids  | transfer_nft  | purchase_cart
            //nft 路由   logs 中 的events中的 event的 type 为 wasm-buy_now  wasm-accept_bid  wasm-withdraw_bid  wasm-create_auction  wasm-place_bid

pub async fn transcation_datas_routes(
    hash_rx:Receiver<String>,
    nft_msg_tx:tokio::sync::mpsc::Sender<NftMessage>,
    conn:Pool<Postgres>,
    semaphore:Arc<Semaphore>) {
  
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

    while let Ok(hash) = hash_rx.recv() {

        let semaphore = Arc::clone(&semaphore);
        let conn=Arc::clone(&conn);
        let nft_msg_tx=Arc::clone(&nft_msg_tx);

        tokio::spawn(async move {
            let permit = semaphore.acquire().await.unwrap();
            if let Ok(hash_vale) = get_transaction_txs_by_tx(&hash).await{
                let hash_data =serde_json::from_value::<HashData>(hash_vale).unwrap();  //反序列化数据
                if hash_data.tx_response.code==0{
                    if is_token(&hash_data){
                        if let Ok(mut conn) = conn.acquire().await  {
                            for token_swap_data in swap_datas(hash_data){
                                if update_token_transaction(&token_swap_data.account, &mut conn, vec![token_swap_data.clone()]).await.is_some(){
                                    println!("Update token swap data success")
                                }else {
                                    println!("add token swap to db erro");
                                    println!("{:#?}",&token_swap_data.tx);
                                }
                            };
                            drop(conn);
                        }else {
                            println!("pool erro");
                        }
                    }else if is_nft(&hash_data){
                        nft_transaction_data(hash_data,nft_msg_tx);
                    }
                }
            }
             //释放限制
             drop(permit);
        });
    }
}

// run wss  || retrun hash
pub fn websocket_run(url:&str,query:&str,hash_sender:Sender<String>) {

    let sub_msg=serde_json::json!({
        "jsonrpc": "2.0",
        "id": 420,
        "method": "subscribe",
        "params": {
             "query":query
        }
    });

    // let unsub_msg=serde_json::json!({
    //     "jsonrpc": "2.0",
    //     "id": 420,
    //     "method": "unsubscribe",
       
    // });

    let tx=Arc::new(hash_sender);

    loop {
        
        let client_result=ClientBuilder::new(url).unwrap().connect_insecure();

        if client_result.is_err(){
            sleep(Duration::from_secs(5));
            continue;
        };
        let client = Arc::new(MX::new(client_result.unwrap()));

        let message = OwnedMessage::Text(sub_msg.to_string());
        client.lock().unwrap().send_message(&message).unwrap();

        let tx=Arc::clone(&tx);
        let receive = thread::spawn(move || {
            loop {
                match client.lock().unwrap().recv_message() {
                    Ok(message) => {
                        match message {
                            OwnedMessage::Text(text) => {
                                let _data:Value=serde_json::from_str(&text).unwrap();
                               //定义 过滤 json 闭包  || 排除 投票
                                let is_not_aggreate_vote=|json:&Value| ->bool{
                                    json.get("result")
                                        .and_then(|result| result.get("events"))
                                        .map_or(true, |event| !event.get("aggregate_vote.exchange_rates").is_some())
                                };
                                // 获取 tx hash 闭包
                                let get_hash=|json:&Value,keys:&Vec<&str>| ->Option<String>{
                                    keys.iter().fold(Some(json),|acc,&key|{

                                        acc.and_then(|inner| inner.get(key))
                                    
                                    }).and_then(|val| val.as_array().map(|hash_arr| {
                                        hash_arr.iter().filter_map(|v| v.as_str().map(|hash| hash.to_string())).collect()}))};

                                    // 解析 tx hash 路径
                                let keys_paths:&Vec<&str>=&vec!["result","events","tx.hash"]; 
                                
                                if let Some(hash) = get_hash(&_data,keys_paths) {
                                    if is_not_aggreate_vote(&_data){
                                        if tx.send(hash).is_err(){
                                            break;
                                        }
                                    }

                                }else {
                                    // println!("{:#?}",text);
                                    if let None =_data.get("result")  {
                                        continue;
                                    }
                                };
                            },
                            _=>{},
                        }
                    },
                    Err(_)=>break,
                }
            };
        });
        receive.join().unwrap();
    }

}



pub async fn operate_db(mut nft_msg_rx:tokio::sync::mpsc::Receiver<NftMessage>,conn:Arc<Pool<Postgres>>)->Result<()> {
    // println!("操作数据库");

    while let Some(transaction) =nft_msg_rx.recv().await  {
        
            match transaction {
                NftMessage::AcceptBidNft(msgs)=>{
                    for msg in msgs{
                        
                        let transaction=NFTtransaction{
                            transaction:_NftTransaction::AcceptBid(msg.clone()),
                            _type:"accpet_bid_nft".to_string(),
                        };
                        let mut  conn=conn.acquire().await?;
                        let update_sender_nft_holding=update_nfts_holding(&msg.transfer.sender, &msg.transfer.collection, &msg.transfer.token_id, "del", &mut conn).await;
                        let update_recipient_nft_holding=update_nfts_holding(&msg.transfer.recipient, &msg.transfer.collection, &msg.transfer.token_id, "add", &mut conn).await;
                        
                        let update_sender_nft_transactions=update_nfts_transactions(&msg.transfer.sender, &mut conn, vec![transaction.clone()]).await;
                        let update_recipient_nft_transactons=update_nfts_transactions(&msg.transfer.recipient, &mut conn, vec![transaction.clone()]).await;

                        if update_sender_nft_holding.is_some() && update_recipient_nft_holding.is_some() && update_sender_nft_transactions.is_some() && update_recipient_nft_transactons.is_some(){
                            // println!("update db sucess || accpet bid ")
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
                        // let mut  conn=conn.lock().await;
                        let mut conn=conn.acquire().await?;
                        let update_sender_nft_holding=update_nfts_holding(&msg.transfer.sender, &msg.transfer.collection, &msg.transfer.token_id, "del", &mut conn).await;
                        let update_recipient_nft_holding=update_nfts_holding(&msg.transfer.recipient, &msg.transfer.collection, &msg.transfer.token_id, "add", &mut conn).await;
                        
                        let update_sender_nft_transactions=update_nfts_transactions(&msg.transfer.sender, &mut conn, vec![transaction.clone()]).await;
                        let update_recipient_nft_transactons=update_nfts_transactions(&msg.transfer.recipient, &mut conn, vec![transaction.clone()]).await;
                        let update_contranct_create_auctions=update_contract_create_auctions(
                            &msg.transfer.collection,
                             vec![OnlyCreateAuction{
                                collection_address:msg.transfer.collection.clone(),
                                token_id:msg.transfer.token_id.clone(),
                                auction_price:msg.auction_price.clone(),
                                ts:msg.transfer.ts.clone()
                             }],
                              &mut conn).await;

                        if update_contranct_create_auctions.is_some() && update_sender_nft_holding.is_some() && update_recipient_nft_holding.is_some() && update_sender_nft_transactions.is_some() && update_recipient_nft_transactons.is_some(){
                            // println!("update db sucess || CretaeAuctionNft")
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
                        let mut conn=conn.acquire().await?;
                        let update_sender_nft_holding=update_nfts_holding(&msg.transfer.sender, &msg.transfer.collection, &msg.transfer.token_id, "del", &mut conn).await;
                        let update_recipient_nft_holding=update_nfts_holding(&msg.transfer.recipient, &msg.transfer.collection, &msg.transfer.token_id, "add", &mut conn).await;
                        
                        let update_sender_nft_transactions=update_nfts_transactions(&msg.transfer.sender, &mut conn, vec![transaction.clone()]).await;
                        let update_recipient_nft_transactons=update_nfts_transactions(&msg.transfer.recipient, &mut conn, vec![transaction.clone()]).await;


                        if update_sender_nft_holding.is_some() && update_recipient_nft_holding.is_some() && update_sender_nft_transactions.is_some() && update_recipient_nft_transactons.is_some(){
                            // println!("update db sucess || CancelAuctionNft")
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
                        let mut conn=conn.acquire().await?;
                        let update_sender_nft_holding=update_nfts_holding(&msg.sender, &msg.collection, &msg.token_id, "del", &mut conn).await;
                        let update_recipient_nft_holding=update_nfts_holding(&msg.recipient, &msg.collection, &msg.token_id, "add", &mut conn).await;
                        
                        let update_sender_nft_transactions=update_nfts_transactions(&msg.sender, &mut conn, vec![transaction.clone()]).await;
                        let update_recipient_nft_transactons=update_nfts_transactions(&msg.recipient, &mut conn, vec![transaction.clone()]).await;


                        if update_sender_nft_holding.is_some() && update_recipient_nft_holding.is_some() && update_sender_nft_transactions.is_some() && update_recipient_nft_transactons.is_some(){
                            // println!("update db sucess || OnlyTransferNft")
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
                        let mut conn=conn.acquire().await?;
                        let update_sender_nft_holding=update_nfts_holding(&msg.transfer.sender, &msg.transfer.collection, &msg.transfer.token_id, "del", &mut conn).await;
                        let update_recipient_nft_holding=update_nfts_holding(&msg.transfer.recipient, &msg.transfer.collection, &msg.transfer.token_id, "add", &mut conn).await;
                        
                        let update_sender_nft_transactions=update_nfts_transactions(&msg.transfer.sender, &mut conn, vec![transaction.clone()]).await;
                        let update_recipient_nft_transactons=update_nfts_transactions(&msg.transfer.recipient, &mut conn, vec![transaction.clone()]).await;


                        if update_sender_nft_holding.is_some() && update_recipient_nft_holding.is_some() && update_sender_nft_transactions.is_some() && update_recipient_nft_transactons.is_some(){
                            // println!("update db sucess || BatchBids")
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
                        let mut conn=conn.acquire().await?;
                        let update_sender_nft_holding=update_nfts_holding(&msg.transfer.sender, &msg.transfer.collection, &msg.transfer.token_id, "del", &mut conn).await;
                        let update_recipient_nft_holding=update_nfts_holding(&msg.transfer.recipient, &msg.transfer.collection, &msg.transfer.token_id, "add", &mut conn).await;
                        
                        let update_sender_nft_transactions=update_nfts_transactions(&msg.transfer.sender, &mut conn, vec![transaction.clone()]).await;
                        let update_recipient_nft_transactons=update_nfts_transactions(&msg.transfer.recipient, &mut conn, vec![transaction.clone()]).await;


                        if update_sender_nft_holding.is_some() && update_recipient_nft_holding.is_some() && update_sender_nft_transactions.is_some() && update_recipient_nft_transactons.is_some(){
                            // println!("update db sucess || PurchaseCartNft")
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

                        let mut conn=conn.acquire().await?;
                        let update_recipient_nft_holding=update_nfts_holding(&msg.recipient, &msg.collection, &msg.token_id, "add", &mut conn).await;
                        
                        let update_recipient_nft_transactons=update_nfts_transactions(&msg.recipient, &mut conn, vec![transaction.clone()]).await;


                        if  update_recipient_nft_holding.is_some() && update_recipient_nft_transactons.is_some(){
                            // println!("update db sucess || Mint")
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
                        let mut conn=conn.acquire().await?;
                        let update_sender_nft_holding=update_nfts_holding(&msg.transfer.sender, &msg.transfer.collection, &msg.transfer.token_id, "del", &mut conn).await;
                        let update_recipient_nft_holding=update_nfts_holding(&msg.transfer.recipient, &msg.transfer.collection, &msg.transfer.token_id, "add", &mut conn).await;
                        
                        let update_sender_nft_transactions=update_nfts_transactions(&msg.transfer.sender, &mut conn, vec![transaction.clone()]).await;
                        let update_recipient_nft_transactons=update_nfts_transactions(&msg.transfer.recipient, &mut conn, vec![transaction.clone()]).await;

                        if update_sender_nft_holding.is_some() && update_recipient_nft_holding.is_some() && update_sender_nft_transactions.is_some() && update_recipient_nft_transactons.is_some(){
                            // println!("update db sucess || PurchaseCartNft")
                        }else {
                            println!("update db eroo || PurchaseCartNft");
                            println!("{:?}\n",transaction)
                        }
                        drop(conn);
                    }
                },
                NftMessage::OnlyCreateAuction(msgs)=>{
                    for msg in msgs{
                        let mut conn=conn.acquire().await?;
                        let update_contranct_create_auctions=update_contract_create_auctions(&msg.collection_address, vec![msg.clone()], &mut conn).await;
                        if update_contranct_create_auctions.is_some(){
                            // println!("update db sucess || OnlyCreateAuction")
                        }else {
                            println!("update db erro || OnlyCreateAuction");
                            println!("{:?}\n",msg)
                        }
                        drop(conn);
                    }
                },
                NftMessage::Unkonw(hash)=>{
                    println!("unkonw : {:?}",hash);
                }
            };  
             
    }
    
    thread::sleep(Duration::from_secs(3));
    Ok(())
}


