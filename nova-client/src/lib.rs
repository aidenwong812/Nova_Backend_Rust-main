pub mod nft_apis;
pub mod token_apis;
pub mod utils;

#[cfg(test)]
mod tests{
    use super::*;
    use db::client_db;
    #[tokio::test]
    async fn test_sync()  {
        let mut conn=client_db().await.unwrap().acquire().await.unwrap();
        let a=utils::sync_address_transactions::sync("sei16zjp47vwu48uvjdetc3rn477d8td5dlwnsd0n4",conn).await;
        println!("{:#?}",a);
    }
}
