mod proto;
mod handlers;
mod store;

use futures::StreamExt;
use anyhow::Result;
use std::env;
use std::future::Future;
use std::time::Duration;
use tokio::task;

use sqlx::{Pool, Postgres};
use sqlx::postgres::PgPoolOptions;
use tokio::task::JoinSet;
use tracing::{error, Level};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::format::FmtSpan;
use crate::store::Store;

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

async fn handle_requests<F, Fut>(nc: async_nats::Client, subject: &str, f: F) -> Result<()>
where
    F: Fn(async_nats::Client, async_nats::Message) -> Fut + Send + Clone + Sync + 'static,
    Fut:  Future<Output = Result<()>> + Send + 'static,
{
    let subject = subject.to_string();

    let mut subscription = nc.subscribe(subject).await?;

    while let Some(msg) = subscription.next().await {

        let nc = nc.clone();
        let f = f.clone();

        task::spawn(tokio::time::timeout(Duration::from_millis(300), async move {
            let msg = msg;
            if let Err(e) = f(nc, msg.clone()).await {
                error!("Error: {}", e.to_string());
            };
        }));
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
    let db_url = match env::var("DATABASE_URL") {
        Ok(value) => value,
        Err(e) => {
            return Err(anyhow::anyhow!("Couldn't read DATABASE_URL environment variable: {}", e));
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

fn setup_logging(app_name: &str) {
    // Initialize the tracing subscriber with a custom configuration
    tracing_subscriber::fmt()
        // Include thread IDs
        .with_thread_ids(true)
        // Include span events (enter/exit of spans)
        .with_span_events(FmtSpan::FULL)
        // Use a custom environment filter
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive(Level::INFO.into())
                // Add specific module levels
                .add_directive((app_name.to_string()+"=debug").parse().unwrap())
        )
        // Pretty printing for development
        .pretty()
        // Initialize the subscriber
        .init();
}

#[tokio::main]
async fn main() -> Result<()> {

    // get the app name, used for group and such
    let app_name = match get_app_name() {
        Some(name) => name,
        None => { return Err(anyhow::anyhow!("Could not  determine application name.")); },
    };

    setup_logging(app_name.as_str());

    let cooldown_sec = match env::var("COOLDOWN_SECS") {
        Ok(value) => value.parse::<i64>().unwrap(),
        Err(e) => {
            return Err(anyhow::anyhow!("Couldn't read COOLDOWN_SECS environment variable: {}", e));
        },
    };

    // connect to db
    let db = connect_to_database().await?;
    let store = Store::new(db.clone());

    // connect to nats
    let nc = connect_to_nats().await?;

    let mut set: JoinSet<()> = JoinSet::new();

    let _nc = nc.clone();
    let _store = store.clone();
    set.spawn(async move {
        handle_requests(_nc, "vault.store", move|_nc, msg| {
            handlers::store::store(_store.clone(), _nc, msg)
        }).await.expect("vault.store");
    });

    let _nc = nc.clone();
    let _store = store.clone();
    set.spawn(async move {
        handle_requests(_nc, "vault.remove", move|_nc, msg| {
            handlers::remove::remove(_store.clone(), _nc, msg, cooldown_sec)
        }).await.expect("vault.remove");
    });

    let _nc = nc.clone();
    let _store = store.clone();
    set.spawn(async move {
        handle_requests(_nc, "vault.get",  move|_nc, msg| {
            handlers::get::get(_store.clone(), _nc, msg)
        }).await.expect("vault.get");
    });

    set.join_all().await;
    Ok(())
}
