use futures_util::{stream::StreamExt, SinkExt};
use serde_json::Value;
use tokio::sync::Mutex;
use std::{error::Error, thread};
use std::sync::mpsc::channel;
use tokio::{sync::{self,Semaphore, mpsc::channel as tokio_channel}, time::{sleep, Duration}};
use std::collections::HashMap;
use std::sync::Arc;
use  searcher_client::{   operate_db, transcation_datas_routes, websocket_run};

use db::client_db;

#[tokio::main]
async fn main()  {


    // 89.163.148.219
    // 127.0.0.1    173.0.55.178
    let query="tm.event='Tx'";
    let ws_url = "ws://173.0.55.178:26657/websocket";

    let (hash_tx,hash_rx)=channel();

    let (nft_msg_tx,nft_msg_rx)=tokio::sync::mpsc::channel(1000000);

    
    let conn=client_db().await.unwrap();
    let conn2=Arc::new(conn.clone());

    
    //start routes   token || nft
    let routes_semaphore = Arc::new(Semaphore::new(2000));
    tokio::spawn(async move{
        transcation_datas_routes(hash_rx, nft_msg_tx,conn,routes_semaphore).await;
    });


    tokio::spawn(async move {operate_db(nft_msg_rx,conn2).await});

        // start websockets
    tokio::spawn(async move {
             websocket_run(ws_url, query, hash_tx);
    }).await;
}