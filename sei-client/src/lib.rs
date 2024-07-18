pub mod apis;
pub mod field_data;

#[cfg(test)]
mod tests{
    use super::*;
    use apis::_apis::{get_contract_info, get_last_height, get_transaction_txs_by_event, get_transaction_txs_by_tx, get_txs_by_block_height, get_wallet_balances};

    #[tokio::test]
    async fn test1(){
        
        let a=get_wallet_balances("sei1krvjk3r790dcsqkr96ymd44v04w9zz5dlr66z7").await;
        println!("{:?}",a);

        
        // let a=get_contract_info("sei1epa25hnlexec6674zmznhtza6xc37ww6jx59vh3c3va9wjdvfd6qdw3wns".to_string() ).await;
        // println!("{:?}",a);

        // let c=get_transaction_txs_by_tx("D82CB8B6733534AB6A77488A2DB93DA6305A52A9D5B312E08B5C0F477CA725DB").await;
        // println!("{:?}",c);
 
    }

    


}

