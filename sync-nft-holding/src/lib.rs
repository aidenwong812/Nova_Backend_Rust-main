use anyhow::Result;
use sqlx::PgConnection;

pub async fn get_collection_list(conn:&mut PgConnection) -> Result<Vec<String>> {
    let collection_list:Vec<String>=sqlx::query_scalar(
        r#"
        SELECT contract_address
        FROM "ContractsCreateAuction"
        "#).fetch_all(conn).await?;
    

    Ok(collection_list)
}