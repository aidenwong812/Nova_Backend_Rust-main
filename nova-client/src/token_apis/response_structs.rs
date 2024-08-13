use std::collections::HashMap;

use chrono::{DateTime, Datelike, NaiveDate, NaiveDateTime, Utc};
use sei_client::field_data::field_data_structions::TokenSwap;
use serde::{Serialize, Deserialize};
use serde_json::Value;

#[derive(Serialize, Deserialize,Clone,Debug)]
pub struct TokenRouteData{
        id:String,
    pub swaps:Vec<SwapData>,
    pub denom_in:String,
    pub decimals_in:u8,
    pub price_in:Value,
    pub value_in:Value,
    pub amount_in:Value,
    pub denom_out:Value,
    pub decimals_out:u8,
    pub price_out:Value,
    pub value_out:Value,
    pub amount_out:String,
    pub price_difference:Value,
    pub price_impact :Value,
}

#[derive(Serialize, Deserialize,Clone,Debug)]
pub struct SwapData{
    contract_addr:String,
    from:String,
    to:String,
    #[serde(rename = "type")]
    _type:String,
    illiquid:bool,
}

#[derive(Serialize, Deserialize,Clone,Debug)]
pub struct UserTokenHolding{
    pub name:String,
    pub demon:String,
    pub decimals:Option<u8>,
    pub logo_url:Option<String>,
    pub amount:String,
    pub worth_usei:Option<String>,
    pub buy_price:Option<String>,
    pub token_transactions:Vec<TokenSwap>

}

#[derive(Serialize, Deserialize,Clone,Debug)]
pub struct UserTopToken{
    pub name:String,
    pub demon:String,
    pub decimals:Option<u8>,
    pub logo_url:Option<String>,
    pub amount:String,
    pub worth_usei:Option<String>,
    pub price_fluctuation:Option<f64> ,  // unrealized_gains / buy price
    pub unrealized_gains:Option<String>,
}

#[derive(Serialize, Deserialize,Clone,Debug)]
pub struct UserTopLossTokenInfo{
    pub top_gainers:Vec<UserTopToken>,
    pub top_looser:Vec<UserTopToken>,
    pub unkonw:Vec<UserTopToken>
}

#[derive(Serialize, Deserialize,Clone,Debug)]
pub struct _TokenTransaction{
    pub transaction_amount:usize,
    pub total_volume:String,
}

#[derive(Serialize, Deserialize,Clone,Debug)]
pub struct TokenTransaction{
    pub day:HashMap<String,_TokenTransaction>,
    pub week:HashMap<String,_TokenTransaction>,
    pub month:HashMap<String,_TokenTransaction>,
}impl TokenTransaction {
    pub fn new()->Self{
        TokenTransaction { 
            day: HashMap::new(), 
            week: HashMap::new(), 
            month: HashMap::new() }
    }

    pub fn add_data(&mut self,transactions:Vec<TokenSwap>,trade_type:&str) {
        
        let mut day_transactions:HashMap<NaiveDate,Vec<TokenSwap>>=HashMap::new();
        let mut week_transactions:HashMap<(i32,u32),Vec<TokenSwap>>=HashMap::new();
        let mut month_transactions:HashMap<(i32,u32),Vec<TokenSwap>>=HashMap::new();

        transactions.iter().for_each(|transaction|{
            let date:NaiveDateTime=DateTime::parse_from_rfc3339(&transaction.ts).unwrap().with_timezone(&Utc).naive_utc();

            day_transactions.entry(date.clone().date()).or_insert_with(Vec::new).push(transaction.clone());

            let iso_week=date.iso_week();
            week_transactions.entry((date.year(),iso_week.week())).or_insert_with(Vec::new).push(transaction.clone());

            month_transactions.entry((date.year(),date.month())).or_insert_with(Vec::new).push(transaction.clone());

        });

        if trade_type=="all"{
            day_transactions.into_iter().for_each(|(day,transactions)|{
                let mut sale_price:Vec<usize>=vec![];
                transactions.iter().for_each(|transaction|{
                    if transaction.source_token=="usei"{
                        sale_price.push(transaction.source_amount.parse::<usize>().unwrap())
                    }else if transaction.target_token=="usei" {
                        sale_price.push(transaction.target_amount.parse::<usize>().unwrap())
                    }
                    else {
                        sale_price.push(0)
                    }  
                });
                if sale_price.len()>0{
                    let volume:usize=sale_price.iter().sum();
                    self.day.entry(day.clone().to_string()).or_insert_with(||_TokenTransaction { transaction_amount: transactions.len(), total_volume: format!("{}usei",volume.to_string())});
                }else {
                    self.day.entry(day.clone().to_string()).or_insert_with(||_TokenTransaction { transaction_amount: transactions.len(), total_volume: "0usei".to_string()});

                }
            });

            week_transactions.into_iter().for_each(|(day,transactions)|{
                let mut sale_price:Vec<usize>=vec![];
                transactions.iter().for_each(|transaction|{
                    if transaction.source_token=="usei"{
                        sale_price.push(transaction.source_amount.parse::<usize>().unwrap())
                    }else if transaction.target_token=="usei" {
                        sale_price.push(transaction.target_amount.parse::<usize>().unwrap())
                    }
                    else {
                        sale_price.push(0)
                    }  
                });
                if sale_price.len()>0{
                    let volume:usize=sale_price.iter().sum();
                    self.day.entry(format!("{}-{}",day.clone().0.to_string(),day.clone().1.to_string())).or_insert_with(||_TokenTransaction { transaction_amount: transactions.len(), total_volume: format!("{}usei",volume.to_string())});
                }else {
                    self.day.entry(format!("{}-{}",day.clone().0.to_string(),day.clone().1.to_string())).or_insert_with(||_TokenTransaction { transaction_amount: transactions.len(), total_volume: "0usei".to_string()});

                }
            });

            month_transactions.into_iter().for_each(|(day,transactions)|{
                let mut sale_price:Vec<usize>=vec![];
                transactions.iter().for_each(|transaction|{
                    if transaction.source_token=="usei"{
                        sale_price.push(transaction.source_amount.parse::<usize>().unwrap())
                    }else if transaction.target_token=="usei" {
                        sale_price.push(transaction.target_amount.parse::<usize>().unwrap())
                    }
                    else {
                        sale_price.push(0)
                    }  
                });
                if sale_price.len()>0{
                    let volume:usize=sale_price.iter().sum();
                    self.day.entry(format!("{}-{}",day.clone().0.to_string(),day.clone().1.to_string())).or_insert_with(||_TokenTransaction { transaction_amount: transactions.len(), total_volume: format!("{}usei",volume.to_string())});
                }else {
                    self.day.entry(format!("{}-{}",day.clone().0.to_string(),day.clone().1.to_string())).or_insert_with(||_TokenTransaction { transaction_amount: transactions.len(), total_volume: "0usei".to_string()});

                }
            });

        }else if trade_type=="sell" {
            day_transactions.into_iter().for_each(|(day,transactions)|{
                let mut sale_price:Vec<usize>=vec![];
                transactions.iter().for_each(|transaction|{
                    sale_price.push(transaction.target_amount.parse::<usize>().unwrap())
                });
                if sale_price.len()>0{
                    let volume:usize=sale_price.iter().sum();
                    self.day.entry(day.clone().to_string()).or_insert_with(||_TokenTransaction { transaction_amount: transactions.len(), total_volume: format!("{}usei",volume.to_string())});
                }else {
                    self.day.entry(day.clone().to_string()).or_insert_with(||_TokenTransaction { transaction_amount: transactions.len(), total_volume: "0usei".to_string()});

                }
            });

            week_transactions.into_iter().for_each(|(day,transactions)|{
                let mut sale_price:Vec<usize>=vec![];
                transactions.iter().for_each(|transaction|{
                    sale_price.push(transaction.target_amount.parse::<usize>().unwrap())
                });
                if sale_price.len()>0{
                    let volume:usize=sale_price.iter().sum();
                    self.day.entry(format!("{}-{}",day.clone().0.to_string(),day.clone().1.to_string())).or_insert_with(||_TokenTransaction { transaction_amount: transactions.len(), total_volume: format!("{}usei",volume.to_string())});
                }else {
                    self.day.entry(format!("{}-{}",day.clone().0.to_string(),day.clone().1.to_string())).or_insert_with(||_TokenTransaction { transaction_amount: transactions.len(), total_volume: "0usei".to_string()});

                }
            });

            month_transactions.into_iter().for_each(|(day,transactions)|{
                let mut sale_price:Vec<usize>=vec![];
                transactions.iter().for_each(|transaction|{
                    sale_price.push(transaction.target_amount.parse::<usize>().unwrap())
                });
                if sale_price.len()>0{
                    let volume:usize=sale_price.iter().sum();
                    self.day.entry(format!("{}-{}",day.clone().0.to_string(),day.clone().1.to_string())).or_insert_with(||_TokenTransaction { transaction_amount: transactions.len(), total_volume: format!("{}usei",volume.to_string())});
                }else {
                    self.day.entry(format!("{}-{}",day.clone().0.to_string(),day.clone().1.to_string())).or_insert_with(||_TokenTransaction { transaction_amount: transactions.len(), total_volume: "0usei".to_string()});

                }
            });
        }else if trade_type=="buy" {
            day_transactions.into_iter().for_each(|(day,transactions)|{
                let mut sale_price:Vec<usize>=vec![];
                transactions.iter().for_each(|transaction|{
                    sale_price.push(transaction.source_amount.parse::<usize>().unwrap())
                });
                if sale_price.len()>0{
                    let volume:usize=sale_price.iter().sum();
                    self.day.entry(day.clone().to_string()).or_insert_with(||_TokenTransaction { transaction_amount: transactions.len(), total_volume: format!("{}usei",volume.to_string())});
                }else {
                    self.day.entry(day.clone().to_string()).or_insert_with(||_TokenTransaction { transaction_amount: transactions.len(), total_volume: "0usei".to_string()});

                }
            });

            week_transactions.into_iter().for_each(|(day,transactions)|{
                let mut sale_price:Vec<usize>=vec![];
                transactions.iter().for_each(|transaction|{
                    sale_price.push(transaction.source_amount.parse::<usize>().unwrap())
                });
                if sale_price.len()>0{
                    let volume:usize=sale_price.iter().sum();
                    self.day.entry(format!("{}-{}",day.clone().0.to_string(),day.clone().1.to_string())).or_insert_with(||_TokenTransaction { transaction_amount: transactions.len(), total_volume: format!("{}usei",volume.to_string())});
                }else {
                    self.day.entry(format!("{}-{}",day.clone().0.to_string(),day.clone().1.to_string())).or_insert_with(||_TokenTransaction { transaction_amount: transactions.len(), total_volume: "0usei".to_string()});

                }
            });

            month_transactions.into_iter().for_each(|(day,transactions)|{
                let mut sale_price:Vec<usize>=vec![];
                transactions.iter().for_each(|transaction|{
                    sale_price.push(transaction.source_amount.parse::<usize>().unwrap())
                });
                if sale_price.len()>0{
                    let volume:usize=sale_price.iter().sum();
                    self.day.entry(format!("{}-{}",day.clone().0.to_string(),day.clone().1.to_string())).or_insert_with(||_TokenTransaction { transaction_amount: transactions.len(), total_volume: format!("{}usei",volume.to_string())});
                }else {
                    self.day.entry(format!("{}-{}",day.clone().0.to_string(),day.clone().1.to_string())).or_insert_with(||_TokenTransaction { transaction_amount: transactions.len(), total_volume: "0usei".to_string()});

                }
            });
        }
    }
}

#[derive(Serialize, Deserialize,Clone,Debug)]
pub struct TokenTradeInfo{
    pub all:TokenTransaction,
    pub sell:TokenTransaction,
    pub buy:TokenTransaction,
}