pub mod apis;
pub mod field_data;

#[cfg(test)]
mod tests{
    use super::*;
    use apis::_apis::{get_contract_info, get_last_height, get_transaction_txs_by_event, get_transaction_txs_by_tx, get_txs_by_block_height, get_wallet_balances};

    #[tokio::test]
    async fn test1(){
        
        // let a=get_wallet_balances("sei1krvjk3r790dcsqkr96ymd44v04w9zz5dlr66z7").await;
        // println!("{:?}",a);
        let list1 = vec![1, 2, 3, 4];
        let list2 = vec![5, 6, 7, 8];
    
        for (item1, item2) in list1.iter().zip(list2.iter()) {
            println!("item1: {}, item2: {}", item1, item2);
        }
        
       
 
    }

    


}

