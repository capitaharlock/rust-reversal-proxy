use tokio_postgres::{Client, Error, NoTls};
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;

const MAX_CONNECTION_ATTEMPTS: u8 = 5;
const RETRY_DELAY_SECONDS: u64 = 5;

pub async fn init_db(client: &Client) -> Result<(), Error> {
    client.batch_execute(include_str!("schema.sql")).await
}

pub async fn connect_to_postgres(database_url: &str) -> Result<Client, Error> {
    for attempt in 1..=MAX_CONNECTION_ATTEMPTS {
        match tokio_postgres::connect(database_url, NoTls).await {
            Ok((client, connection)) => {
                tokio::spawn(async move {
                    if let Err(e) = connection.await {
                        eprintln!("Postgres connection error: {}", e);
                    }
                });
                return Ok(client);
            }
            Err(e) => {
                eprintln!("Failed to connect to Postgres (attempt {}/{}): {}", attempt, MAX_CONNECTION_ATTEMPTS, e);
                if attempt == MAX_CONNECTION_ATTEMPTS {
                    return Err(e);
                }
                sleep(Duration::from_secs(RETRY_DELAY_SECONDS)).await;
            }
        }
    }
    unreachable!()
}

pub async fn load_api_keys(client: &Client) -> Result<HashMap<String, bool>, Error> {
    let rows = client.query("SELECT key FROM api_keys", &[]).await?;
    Ok(rows.into_iter().map(|row| (row.get(0), true)).collect())
}