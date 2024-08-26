use db::{client_db, update_nfts_holding};
use sync_nft_holding::get_collection_list;
use sei_client::apis::_apis::{get_contract_info, get_nft_info, get_nft_owner};
use tokio::sync::Semaphore;
use std::sync::Arc;


#[tokio::main]
async fn main() {

    let mut client=client_db().await.unwrap().acquire().await.unwrap();
    let collection_list=get_collection_list(&mut client).await.unwrap();

    let info_seamaphore=Arc::new(Semaphore::new(6));

    let mut handles=vec![];


    for collection in collection_list{
        
        let info_seamaphore=Arc::clone(&info_seamaphore);
        
        let _handle=tokio::spawn(async move {

            let info_permit=info_seamaphore.acquire().await.unwrap();

            if let Some(collection_info)=get_contract_info(collection.clone()).await{
                
                let nft_nums=collection_info.count.parse::<usize>().unwrap();
                let conn=Arc::new(client_db().await.unwrap());
                let collection=Arc::new(collection.clone());
                let add_seamaphore=Arc::new(Semaphore::new(5));


                let mut add_handles: Vec<tokio::task::JoinHandle<()>>=vec![];

                for  token_id in 1..=nft_nums{

                    let conn=Arc::clone(&conn);
                    let collection=Arc::clone(&collection);
                    let add_seamaphore=Arc::clone(&add_seamaphore);

                    let add_handle=tokio::spawn(async move {
                        let permit=add_seamaphore.acquire().await.unwrap();
                        let nft_owner=get_nft_owner(collection.as_str(), token_id.to_string().as_str()).await;
                        if let Some(nft_owner) =nft_owner  {
                            let mut conn=conn.acquire().await.unwrap();
                            if update_nfts_holding(&nft_owner, collection.as_str(), token_id.to_string().as_str(), "add", &mut conn).await.is_some(){
                                println!("update success");
                                
                            }else {
                                println!("update erro");
                            }
                        };
                        drop(permit)
                    });

                    add_handles.push(add_handle);
                  
                };

                for handle in add_handles{
                    handle.await.unwrap();
                }


            }else {
                println!("miss {}",collection);
            }
            drop(info_permit)
        });
        
        handles.push(_handle);
    };

    for i in handles{
        i.await.unwrap();
    }
}
