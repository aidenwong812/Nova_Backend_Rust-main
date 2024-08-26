use std::{sync::Arc,thread, time::Duration};
use db::client_db;
use reqwest::Client;
use tx_scan_tools::{get_transaction_hash, operate_db, transcation_datas_routes, Config};
use tokio::{sync::Mutex, task};

#[tokio::main]
async fn main() {

    let client=Client::new();
    let (tx_hash_tx,mut tx_hash_rx)=tokio::sync::mpsc::channel(1000000);
    let (nft_msg_tx,mut nft_msg_rx)=tokio::sync::mpsc::channel(1000000);

    let conn=client_db().await.unwrap();
    let conn2=Arc::new(conn.clone());

    if let Ok(config)=Config::read("./tx_scan_tool_config.yaml".to_string()){


        tokio::spawn(async move {operate_db(nft_msg_rx,conn2).await});

        tokio::spawn(async move {
            transcation_datas_routes(tx_hash_rx,nft_msg_tx,conn).await;
        });

        for i in config.start_block .. config.end_block{
            let mut counter =0;

            println!("Scan Block {}",i);
            let gth_res=get_transaction_hash(i,tx_hash_tx.clone()).await;

            if gth_res.is_err(){
                println!("{:#?}",&gth_res.err());
                thread::sleep(Duration::from_secs(5));
                continue;
            };

            println!("Finished\n");
            
            Config::write("./tx_scan_tool_config.yaml".to_string(), i);
            
            thread::sleep(Duration::from_secs_f64(0.5));

            counter+=1;
            if counter==50{
                thread::sleep(Duration::from_secs(5));
                counter=0;
            }
        }
        
    }else {
        panic!("read config erro")
    }
}
