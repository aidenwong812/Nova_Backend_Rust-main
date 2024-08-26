pub mod apis;
pub mod field_data;

#[cfg(test)]
mod tests{
    use super::*;
    use apis::_apis::{get_nft_owner,get_contract_info, get_last_height, get_transaction_txs_by_event, get_transaction_txs_by_tx, get_txs_by_block_height, get_wallet_balances};

    #[tokio::test]
    async fn test1(){
        
        // let a=get_wallet_balances("sei1krvjk3r790dcsqkr96ymd44v04w9zz5dlr66z7").await;
        // println!("{:?}",a);
        let a=get_nft_owner("sei1u64thag5ltz7sjte7ggdd0k8wzr59p32vlswh89wswmz46relcqq6my5h6","1").await;
        println!("{:?}",a);
    }

    


}

