use protobuf::{Message};
use async_nats::Client;
use anyhow::{ Result};
use chrono::{Duration, Utc};
use tracing::error;
use crate::proto::vault_remove::{RemoveVaultItem, RemoveVaultItemResponse};
use crate::store::Store;

#[tracing::instrument]
pub async fn remove(db: Store, nc: Client, msg: async_nats::Message, cooldown_sec: i64) -> Result<()> {
    let request = RemoveVaultItem::parse_from_bytes(&msg.payload)?;

    let existing = match db.get_slot(&request.uuid, request.slot).await {
        Ok(n) => n.item.is_some(),
        Err(e) => {
            error!("Error: {}", e.to_string());
            send_error_reply(&nc, &msg,"Could not check slot." ).await?;
            return Err(e);
        },
    };

    if !existing {
        send_error_reply(&nc, &msg,"No Item in the slot." ).await?;
        return Ok(());
    }

    let cooldown_expire = Utc::now().naive_utc() + Duration::seconds(cooldown_sec);
    match db.remove_item(&request.uuid, request.slot, cooldown_expire).await {
        Ok(_) => {}
        Err(e) => {
            error!("Error: {}", e.to_string());
            send_error_reply(&nc, &msg,"Remove Item failed." ).await?;
            return Err(e);
        }
    }

    // reply
    let mut resp = RemoveVaultItemResponse::new();
    resp.success = true;
    resp.error = None;
    let encoded: Vec<u8> = resp.write_to_bytes()?;
    nc.publish(msg.reply.unwrap(), encoded.into()).await?;

    Ok(())
}

async fn send_error_reply(nc: &Client, msg: &async_nats::Message, error: &str) -> Result<()> {
    let mut resp = RemoveVaultItemResponse::new();
    resp.success = false;
    resp.error = Some(error.to_string());

    // Serialize to bytes
    let encoded: Vec<u8> = resp.write_to_bytes()?;

    // send reply
    nc.publish(msg.clone().reply.unwrap(), encoded.into()).await?;

    Ok(())
}