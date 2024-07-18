pub mod nft_apis;
pub mod token_apis;
use tokio;

mod tests{
    use token_apis::user::token_swap_routes;

    use super::*;
    #[tokio::test]
    async fn test_routes()  {
        let amount:usize=100000;
        let a=token_swap_routes("usei", "factory/sei19d4r7e9e3j7xlksgea982clrwqpxvnqmsttn3v/jei", &amount).await;
        println!("{:?}",a);
    }
}