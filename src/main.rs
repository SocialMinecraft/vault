mod proto;

use futures::StreamExt;
use anyhow::Result;
use std::env;
use tokio::task;

use protobuf::Message;
use sqlx::{Pool, Postgres};
use sqlx::postgres::PgPoolOptions;
use tokio::task::JoinSet;

async fn connect_to_nats() -> Result<async_nats::Client> {
    // Get Nats Env Variable
    let nats_urls_env = match env::var("NATS_URL") {
        Ok(value) => value,
        Err(e) => {
            return Err(anyhow::anyhow!("Couldn't read NATS_URL environment variable: {}", e));
        },
    };

    let nats_urls : Vec<&str> = nats_urls_env.split(",").collect();

    // Connect to NATS server
    let client = async_nats::connect(nats_urls).await?;

    Ok(client)
}

/*async fn send_hello(nc: async_nats::Client, from: &str) -> Result<()> {

    // create test message
    let mut msg = Hello::new();
    msg.from = from.to_string();

    // Serialize the user to bytes
    let encoded: Vec<u8> = msg.write_to_bytes().unwrap();

    // send message
    let publisher_client = nc.clone();
    publisher_client.publish("hello", encoded.into()).await?;

    Ok(())
}*/

async fn handle_requests<F>(nc: async_nats::Client, subject: &str, f: F) -> Result<()>
where
    F: Fn(async_nats::Client, async_nats::Message) + Send + Copy + Sync + 'static
{
    let subject = subject.to_string();

    let mut subscription = nc.subscribe(subject).await?;

    while let Some(msg) = subscription.next().await {

        let nc = nc.clone();
        let f = f.clone();

        task::spawn(async move {
            let msg = msg;
            f(nc, msg.clone());
        });
    }

    Ok(())
}

fn get_app_name() -> Option<String> {
    env::current_exe()
        .ok()
        .and_then(|pb| pb.file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_owned()))
}

async fn connect_to_database() -> Result<Pool<Postgres>> {
    // Get Nats Env Variable
    let db_url = match env::var("DB_URL") {
        Ok(value) => value,
        Err(e) => {
            return Err(anyhow::anyhow!("Couldn't read DB_URL environment variable: {}", e));
        },
    };

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url).await?;

    sqlx::migrate!()
        .run(&pool)
        .await?;

    Ok(pool)
}

#[tokio::main]
async fn main() -> Result<()> {

    // get the app name, used for group and such
    let app_name = match get_app_name() {
        Some(name) => name,
        None => { return Err(anyhow::anyhow!("Could not  determine application name.")); },
    };

    // connect to db
    let db = connect_to_database().await?;

    // connect to nats
    let nc = connect_to_nats().await?;

    let mut set: JoinSet<()> = JoinSet::new();

    /*let _nc = nc.clone();
    set.spawn(async move {
        handle_requests(_nc, "hello", |_nc, msg| {
            let decoded_msg = Hello::parse_from_bytes(&msg.payload).unwrap();
            println!("Hello from: {}", decoded_msg.from);
        }).await.expect("Could not listen for messages on hello");
    });*/

    // send hello
    //send_hello(nc.clone(), &app_name.to_string()).await?;

    set.join_all().await;
    Ok(())
}
