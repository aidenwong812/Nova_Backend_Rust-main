
use actix_web::{get, http::header, post, web::{self, Data}, HttpRequest, HttpResponse, Responder, Result};
use serde::{Deserialize, Serialize};

use crate::params_structs::{DontHaveData, Wallet};
use sqlx::PgPool;

use nova_client::{nft_apis::user::{get_user_income_holding_nfts, get_user_nfts_holidng, get_user_top_nfts, get_user_trade_info_nfts}, token_apis::user::{get_user_tokens_holding, get_user_top_tokens, get_user_trade_info_tokens}, utils::sync_address_transactions::sync};


#[get("/user/get_holding_nfts/{wallet_address}")]
pub async fn get_holding_nfts(
    path:web::Path<String>,
    nova_db_pool:Data<PgPool>,
    req: HttpRequest,
    
    ) -> Result<impl Responder>  {

    let wallet_address=path.into_inner();
    let mut conn=nova_db_pool.acquire().await.unwrap();

    // 设置 CORS 头部
    let cors_allow_origin = match req.headers().get("Origin") {
        Some(origin) => {
            let origin_str = origin.to_str().unwrap_or("*");
            header::HeaderValue::from_str(origin_str).unwrap_or_else(|_| header::HeaderValue::from_static("*"))
        }
        None => header::HeaderValue::from_static("*"),
    };

    if let Some(user_nfts_holding_data)=get_user_nfts_holidng(&wallet_address,&mut conn).await{
        Ok(
            HttpResponse::Ok().json(user_nfts_holding_data))
    }else {
        Ok(HttpResponse::Ok().json(DontHaveData{wallet_address:wallet_address.clone(),result:None}))
    }
}

#[get("/user/get_income_nfts/{wallet_address}")]
pub async fn get_income_nfts(
    path:web::Path<String>,
    nova_db_pool:Data<PgPool>,
) -> Result<impl Responder> {
    
    let wallet_address=path.into_inner();
    let mut conn=nova_db_pool.acquire().await.unwrap();


    if let Some(user_nfts_income_data) =get_user_income_holding_nfts(&wallet_address, &mut conn).await  {
        Ok(
            HttpResponse::Ok().json(user_nfts_income_data))
    }else {
        Ok(HttpResponse::Ok().json(DontHaveData{wallet_address:wallet_address.clone(),result:None}))
    }

}

#[get("/user/get_holding_nfts_top/{wallet_address}")]
pub async fn get_holding_nfts_top(
    path:web::Path<String>,
    nova_db_pool:Data<PgPool>,
) -> Result<impl Responder> {

    let wallet_address=path.into_inner();
    let mut conn=nova_db_pool.acquire().await.unwrap();


    if let Some(user_holding_nfts_top_data) =get_user_top_nfts(&wallet_address, &mut conn).await  {
       
        Ok(
            HttpResponse::Ok().json(user_holding_nfts_top_data))
    }else {
        Ok(HttpResponse::Ok().json(DontHaveData{wallet_address:wallet_address.clone(),result:None}))
    }
}

#[get("/user/get_nfts_trade_info/{wallet_address}")]
pub async fn get_nfts_trades_info(
    path:web::Path<String>,
    nova_db_pool:Data<PgPool>,
) -> Result<impl Responder> {

    let wallet_address=path.into_inner();
    let mut conn=nova_db_pool.acquire().await.unwrap();

    if let Some(user_nfts_trades_data) =get_user_trade_info_nfts(&wallet_address, &mut conn).await  {
        // println!("{:?}",user_nfts_trades_data);
        Ok(
            HttpResponse::Ok().json(user_nfts_trades_data))
    }else {
        Ok(HttpResponse::Ok().json(DontHaveData{wallet_address:wallet_address.clone(),result:None}))
    }
}

#[get("/user/get_holding_tokens/{wallet_address}")]
pub async fn get_user_token_hold(
    path:web::Path<String>,
    nova_db_pool:Data<PgPool>,
) -> Result<impl Responder>{

    let wallet_address=path.into_inner();
    let mut conn=nova_db_pool.acquire().await.unwrap();

    if let Some(token_holding_info)=get_user_tokens_holding(&wallet_address,&mut conn).await{
        Ok(
            HttpResponse::Ok().json(token_holding_info)
        )
    }else {
        Ok(HttpResponse::Ok().json(DontHaveData{wallet_address:wallet_address.clone(),result:None}))
    }
}

#[get("/user/get_holding_tokens_top/{wallet_address}")]
pub async fn get_user_tokens_top(
    path:web::Path<String>,
    nova_db_pool:Data<PgPool>,
) -> Result<impl Responder>{

    let wallet_address=path.into_inner();
    let mut conn=nova_db_pool.acquire().await.unwrap();

    if let Some(token_holding_info)=get_user_top_tokens(&wallet_address,&mut conn).await{
        Ok(
            HttpResponse::Ok().json(token_holding_info)
        )
    }else {
        Ok(HttpResponse::Ok().json(DontHaveData{wallet_address:wallet_address.clone(),result:None}))
    }
}

#[get("/user/get_tokens_trade_info/{wallet_address}")]
pub async fn get_user_trades_info(
    path:web::Path<String>,
    nova_db_pool:Data<PgPool>,
) -> Result<impl Responder>{

    let wallet_address=path.into_inner();
    let mut conn=nova_db_pool.acquire().await.unwrap();

    if let Some(token_holding_info)=get_user_trade_info_tokens(&wallet_address,&mut conn).await{
        Ok(
            HttpResponse::Ok().json(token_holding_info)
        )
    }else {
        Ok(HttpResponse::Ok().json(DontHaveData{wallet_address:wallet_address.clone(),result:None}))
    }
}




//sync address transaction
#[get("/address/{wallet_address}/post_transactions_async")]
pub async fn post_sync_address_transaction(
    path:web::Path<String>,
    nova_db_pool:Data<PgPool>,
) -> Result<impl Responder>{ 
    let wallet_address=path.into_inner();
    let mut conn=nova_db_pool.acquire().await.unwrap();

    if sync(&wallet_address, conn).await.is_ok(){

        #[derive(Deserialize,Serialize,Debug)]
        pub struct Rp{
            pub info:String
        }

        Ok(HttpResponse::Ok().json(Rp{info:"sync sucess".to_string()}))
    }else {
        Ok(HttpResponse::Ok().json(DontHaveData{wallet_address:wallet_address.clone(),result:None}))
    }
}

#[actix_web::test]
async fn test_route_nfts() {
    
    use std::env;
    use dotenv::dotenv;

    dotenv().ok();
    let nova_database_url = std::env::var("DATABASE_URL").unwrap();
    let nova_db_pool=PgPool::connect(&nova_database_url).await.unwrap();

    let app = actix_web::test::init_service(
        actix_web::App::new()
            .app_data(Data::new(nova_db_pool))
            .service(get_holding_nfts)
            .service(get_income_nfts)
            .service(get_holding_nfts_top)
            ,
    )
    .await;

    let wallet_address="sei1hyg745gqqlnjrdh2q684aly4wz875h6hadh3ek";


    // test get user nfts holding
    let get_user_holding_nfts_url=format!("/user/get_holding_nfts/{}",wallet_address);
    let get_user_holding_nfts_resp = actix_web::test::TestRequest::get()
        .uri(&get_user_holding_nfts_url)
        .to_request();
    let get_user_holding_nfts_resp = actix_web::test::call_service(&app, get_user_holding_nfts_resp).await;
    assert!(get_user_holding_nfts_resp.status().is_success());
    


    // test get user income nfts
    // let get_user_income_nfts_url=format!("/user/get_income_nfts/{}",wallet_address);
    // let get_user_income_nfts_resp = actix_web::test::TestRequest::get()
    //     .uri(&get_user_income_nfts_url)
    //     .to_request();
    // let get_user_income_nfts_resp = actix_web::test::call_service(&app, get_user_income_nfts_resp).await;
    // assert!(get_user_income_nfts_resp.status().is_success());

    //  // test get user holding nfts top
    //  let get_user_holding_nfts_top_url=format!("/user/get_holding_nfts_top/{}",wallet_address);
    //  let get_user_holding_nfts_top_resp = actix_web::test::TestRequest::get()
    //      .uri(&get_user_holding_nfts_top_url)
    //      .to_request();
    //  let get_user_holding_nfts_top_resp = actix_web::test::call_service(&app, get_user_holding_nfts_top_resp).await;
    //  assert!(get_user_holding_nfts_top_resp.status().is_success());

}


