pub mod tables;
use chrono::{DateTime, Utc, NaiveDate};
use serde_json::Value;
use sqlx::{
    postgres::{PgPoolOptions, PgQueryResult}, types::Json, Connection, PgConnection, Pool, Postgres
    };
use std::{env, error::Error};
use dotenv::dotenv;

use tables::*;

use sei_client::{ apis::_apis::{get_contract_info, get_nft_info}, field_data::field_data_structions::{Collection, _NftTransaction,NFTtransaction, NftToken, TokenSwap, User,FixedSellNft,OnlyCreateAuction,ContractCreateAuctions}};





pub async fn client_db() -> Result<Pool<Postgres>,Box<dyn Error>> {
    
    dotenv().ok();
    let database_url = env::var("DATABASE_URL").unwrap();
    // 连接到数据库
    let connect = PgConnection::connect(&database_url).await?;
    let connect_pool=PgPoolOptions::new()
                    .max_connections(20000000)
                    .idle_timeout(std::time::Duration::from_secs(30))
                    .connect(&database_url).await?;
    Ok(connect_pool)
}


pub async fn search_user(wallet_address:&str,conn:&mut PgConnection) ->Option<User>{
        println!("{}",wallet_address);
        let res=sqlx::query_as::<_,_User>(
            r#"SELECT wallet_address, nfts_holding, nfts_transactions, token_transactions FROM "User" WHERE wallet_address = $1"#
        )
        .bind(wallet_address)
        .fetch_one(conn).await;

        if let Ok(data) =res  {

            let nfts_holding=serde_json::from_value::<Vec<Collection>>(data.nfts_holding).unwrap();
            let nfts_transactions=serde_json::from_value::<Vec<NFTtransaction>>(data.nfts_transactions).unwrap();
            let token_transactions=serde_json::from_value::<Vec<TokenSwap>>(data.token_transactions).unwrap();

            Some(User { 
                wallet_address: data.wallet_address, 
                nfts_holding: nfts_holding, 
                nfts_transactions: nfts_transactions, 
                token_transactions: token_transactions }
            )
        }else {
            None
        }
}

pub async fn insert_user(user:User,conn:&mut PgConnection) -> Option<PgQueryResult> {
        
        let insert=sqlx::query::<sqlx::Postgres>(
            r#"INSERT INTO "User" (wallet_address, nfts_holding, nfts_transactions, token_transactions) VALUES ($1, $2, $3, $4)"#
        )
        .bind(user.wallet_address)
        .bind(serde_json::to_value(user.nfts_holding).unwrap())
        .bind(serde_json::to_value(user.nfts_transactions).unwrap())
        .bind(serde_json::to_value(user.token_transactions).unwrap())
        .execute(conn).await;

        
        if let Ok(result) = insert {
            Some(result)
        }else {
            // Err("inster user erro".into())
            None
        }

}

pub async fn update_nfts_holding(wallet_address:&str,collection_account:&str,nft_id:&str,operate:&str,conn:&mut PgConnection) -> Option<PgQueryResult> {
        
        let nft_res=get_nft_info(collection_account.to_string(), nft_id.to_string()).await;
        if nft_res.is_none(){
            return None;
        }

        let nft=nft_res.unwrap();

        if let Some(mut user) =search_user(wallet_address, conn).await  {
            
          
            // 寻找用户的collection ，如果 有进行 追加或 删除
            if let Some(mut nfts_holding)  = user.nfts_holding.iter_mut().find(|collection|{collection.collection ==collection_account}) {
                
                if operate=="add"{
                    
                    if  nfts_holding.nfts.iter().find(|_nft|{_nft.key==nft.key}).is_none(){
                        nfts_holding.nfts.push(nft);
                    }

                }else if operate=="del" {
                   

                    if let Some(remove_nft)=nfts_holding.nfts.iter().position(|_nft|_nft.key==nft.key){
                        nfts_holding.nfts.remove(remove_nft);
                        
                    }
                }

                if nfts_holding.nfts.len() == 0{
                   if let Some(remove_collection)=user.nfts_holding.iter().position(|collection|collection.collection==collection_account){
                    user.nfts_holding.remove(remove_collection);
                   }
                } 
            
            }else {
                
                if operate=="add"{
                    
                    if let Some(res)=get_contract_info(collection_account.to_string()).await{
                            user.nfts_holding.push(Collection{
                                collection:collection_account.to_string(),
                                name:res.name,
                                symbol:res.symbol,
                                creator:res.creator,
                                count:res.count,
                                nfts:vec![nft],
                            })
                        
                    }
                }
            }

            let update=sqlx::query(
                 r#"UPDATE "User" SET nfts_holding =$2 WHERE wallet_address = $1 "#
            )
            .bind(wallet_address)
            .bind(serde_json::to_value(user.nfts_holding).unwrap())
            .execute(conn)
            .await;

            
            if let Ok(res)=update{
                Some(res)
            }else {
                // Err("update holding nft to user erro".into())
                None
            }


        }else {

            let mut nfts_holding:Vec<Collection>=vec![];
            let mut nfts_transactions:Vec<NFTtransaction>=vec![];
            let mut token_transactions:Vec<TokenSwap>=vec![];
            
            if operate =="add"{
                if let Some(res)=get_contract_info(collection_account.to_string()).await{
                  
                        nfts_holding.push(Collection{
                            collection:collection_account.to_string(),
                            name:res.name,
                            symbol:res.symbol,
                            creator:res.creator,
                            count:res.count,
                            nfts:vec![nft],
                        })
                    
                }
            }
            
            
            let user=User{
                wallet_address:wallet_address.to_string(),
                nfts_holding:nfts_holding,
                nfts_transactions:nfts_transactions,
                token_transactions:token_transactions,
            };

            if let Some(res) =insert_user(user, conn).await {
                Some(res)
            }else {
                // Err("update nfts holding err  || inster user err".into())
                None
            }

        }
}


pub async fn update_nfts_transactions(wallet_address:&str,conn:&mut PgConnection,nfts_transactions:Vec<NFTtransaction>) -> Option<PgQueryResult> {
            
        if let Some(UserData) =search_user(wallet_address, conn).await  {
            
            let mut db_nfts_transactions=UserData.nfts_transactions;
            for t in &nfts_transactions{
                if !db_nfts_transactions.iter().any(|x|{
                    match (&t.transaction,&x.transaction) {
                        (_NftTransaction::Mint(tt),_NftTransaction::Mint(xx))=>{
                            tt.tx==xx.tx
                        },
                        (_NftTransaction::AcceptBid(tt),_NftTransaction::AcceptBid(xx))=>{
                            tt.transfer.tx==xx.transfer.tx
                        },
                        (_NftTransaction::BatchBids(tt),_NftTransaction::BatchBids(xx))=>{
                            tt.transfer.tx==xx.transfer.tx                        },
                        (_NftTransaction::CancelAuction(tt),_NftTransaction::CancelAuction(xx))=>{
                            tt.transfer.tx==xx.transfer.tx                        },
                        (_NftTransaction::CretaeAuction(tt),_NftTransaction::CretaeAuction(xx))=>{
                            tt.transfer.tx==xx.transfer.tx                        },
                        (_NftTransaction::OnlyTransfer(tt),_NftTransaction::OnlyTransfer(xx))=>{
                            tt.tx==xx.tx
                        },
                        (_NftTransaction::FixedSell(tt),_NftTransaction::FixedSell(xx))=>{
                            tt.transfer.tx==xx.transfer.tx                        },
                        (_NftTransaction::PurchaseCart(tt),_NftTransaction::PurchaseCart(xx))=>{
                            tt.transfer.tx==xx.transfer.tx                        },
                        (_,_)=>false
                    }
                }){
                    db_nfts_transactions.push(t.to_owned())
                }
            }
            
            let update=sqlx::query(
                 r#"UPDATE "User" SET nfts_transactions =$2 WHERE wallet_address = $1 "#
            )
                .bind(wallet_address)
                .bind(serde_json::to_value(db_nfts_transactions).unwrap())
                .execute(conn).await;
            
            if let Ok(res) = update {
                Some(res)
            }else {
                None
            }
        }else {
   
            let mut nfts_holding:Vec<Collection>=vec![];
            let mut db_nfts_transactions:Vec<NFTtransaction>=vec![];
            let mut token_transactions:Vec<TokenSwap>=vec![];

            db_nfts_transactions.extend(nfts_transactions.iter().cloned());

            let user=User{
                wallet_address:wallet_address.to_string(),
                nfts_holding:nfts_holding,
                nfts_transactions:db_nfts_transactions,
                token_transactions:token_transactions,
            };

            if let Some(res) =insert_user(user, conn).await {
                Some(res)
            }else {
                // Err("update nfts transaction err  || inster user err".into())
                None
            }

        }
}

    
pub async fn update_token_transaction(wallet_address:&str,conn:&mut PgConnection,token_transactions:Vec<TokenSwap>) -> Option<PgQueryResult> {
        
        if let Some(UserData) =search_user(wallet_address, conn).await  {
            
            let mut db_token_transactions=UserData.token_transactions;
            for t in & token_transactions{
                if !db_token_transactions.iter().any(|x| 
                    t.account==x.account && 
                    t.trade_type == x.trade_type &&
                    t.tx == x.tx &&
                    t.ts == x.ts &&
                    t.target_amount == x.target_amount &&
                    t.source_amount == x.source_amount &&
                    t.target_token == x.target_token &&
                    t.source_token == x.source_token 
                ){
                    db_token_transactions.push(t.to_owned())
                }
            }            
            let update=sqlx::query(
                 r#"UPDATE "User" SET token_transactions =$2 WHERE wallet_address = $1 "#
            )
                .bind(wallet_address)
                .bind(serde_json::to_value(db_token_transactions).unwrap())
                .execute(conn).await;
            
            if let Ok(res) = update {
                Some(res)
            }else {
                // Err("update nfts transaction err ".into())
                None
            }
        }else {
            
            let mut nfts_holding:Vec<Collection>=vec![];
            let mut nfts_transactions:Vec<NFTtransaction>=vec![];
            let mut db_token_transactions:Vec<TokenSwap>=vec![];

            db_token_transactions.extend(token_transactions.iter().cloned());

            let user=User{
                wallet_address:wallet_address.to_string(),
                nfts_holding:nfts_holding,
                nfts_transactions:nfts_transactions,
                token_transactions:db_token_transactions,
            };

            if let Some(res) = insert_user(user, conn).await {
                Some(res)
            }else {
                None
            }

        }
     
}



//////////////////////////////////////////////

pub async fn search_contract_createauctions(contract_address:&str,conn:&mut PgConnection) -> Option<ContractCreateAuctions> {
    
    let res=sqlx::query_as::<_,_ContractCreateAuctions>(
        r#"SELECT contract_address ,create_auction_transactions FROM "ContractsCreateAuction" WHERE contract_address = $1"#
    )
    .bind(contract_address)
    .fetch_one(conn).await;

    if let Ok(data)=res  {

        let contract_create_auctions=serde_json::from_value::<Vec<OnlyCreateAuction>>(data.create_auction_transactions).unwrap();
        Some(ContractCreateAuctions { contract_address: data.contract_address, create_auctions: contract_create_auctions })
    }else {
        None
    }
}

pub async fn inster_ContractsCreateAuction(contranct_create_auctions:ContractCreateAuctions,conn:&mut PgConnection) -> Option<PgQueryResult> {
    
    let insert=sqlx::query::<sqlx::Postgres>(
        r#"INSERT INTO "ContractsCreateAuction" (contract_address, create_auction_transactions) VALUES ($1,$2)"#
    )
    .bind(contranct_create_auctions.contract_address)
    .bind(serde_json::to_value(contranct_create_auctions.create_auctions).unwrap())
    .execute(conn).await;
    
    if let Ok(result) =insert  {
        Some(result)
    }else {
        None
    }
}

pub async fn update_contract_create_auctions(contract_address:&str,create_auctions:Vec<OnlyCreateAuction>, conn:&mut PgConnection) -> Option<PgQueryResult> {
    if let Some(contract) =search_contract_createauctions(contract_address, conn).await  {

        let day_now=Utc::now().date_naive();

        let mut contract_create_auctions=contract.create_auctions;

        // 删除非今天        
        contract_create_auctions.retain(|auction|{
            let ts=DateTime::parse_from_rfc3339(&auction.ts).unwrap().with_timezone(&Utc).date_naive();
            ts == day_now
        });

        contract_create_auctions.extend(create_auctions.iter().cloned());


        let update=sqlx::query(
            r#"UPDATE "ContractsCreateAuction" SET create_auction_transactions =$2 WHERE contract_address = $1 "#
        )
        .bind(contract_address)
        .bind(serde_json::to_value(contract_create_auctions).unwrap())
        .execute(conn).await;

        if let Ok(result) =update  {
            Some(result)
        }else {
            None
        }


    }else {
        let contranct_create_auctions=ContractCreateAuctions{
            contract_address:contract_address.to_string(),
            create_auctions:create_auctions.clone()
        };
        if let Some(result) =inster_ContractsCreateAuction(contranct_create_auctions, conn).await {
            Some(result)
        }else {
            None
        }
    }
}




mod tests{

    use sei_client::field_data::field_data_structions::{AcceptBidNft, NftAttribute, TransferNft, _NftTransaction};

    use super::*;
    #[tokio::test]
    async fn test1()  {
       
        let mut conn=client_db().await.unwrap().acquire().await.unwrap();
        let a=search_user("sei1krvjk3r790dcsqkr96ymd44v04w9zz5dlr66z7", &mut conn).await;
      
        println!("{:?}",a)

      


    }
}

