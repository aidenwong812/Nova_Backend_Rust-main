pub mod nft_apis;
pub mod token_apis;
use tokio;

mod tests{
    use sei_client::apis::_apis::get_ibc_info;
    use token_apis::user::{get_user_tokens_holding};

    use super::*;
    #[tokio::test]
    async fn test_routes()  {
                 // sei1huqsl8mypckr7tgqs636e7uwrfsvlq8mpmx4tz
         // sei1d649tnttdphknafag5xwz69fd55v9rllrnrt4h

         //sei1hrndqntlvtmx2kepr0zsfgr7nzjptcc72cr4ppk4yav58vvy7v3s4er8ed
        // let a=get_user_tokens_holding("sei1d649tnttdphknafag5xwz69fd55v9rllrnrt4h").await;
        // println!("{:#?}",a);
    }
}