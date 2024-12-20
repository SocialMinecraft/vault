use protobuf::{Message};
use async_nats::Client;
use anyhow::{ Result};
use tracing::error;
use crate::proto::vault_store::{StoreVaultItem, StoreVaultItemResponse};
use crate::store::Store;

#[tracing::instrument]
pub async fn store(db: Store, nc: Client, msg: async_nats::Message) -> Result<()> {
     let request = StoreVaultItem::parse_from_bytes(&msg.payload)?;

    // check slot is within inventory size
    let size = match db.vault_size(&request.uuid).await {
        Ok(n) => n,
        Err(e) => {
            error!("Error: {}", e.to_string());
            send_error_reply(&nc, &msg,"Could not fetch inventory size." ).await?;
            return Err(e);
        }
    };

    if request.slot >= size {
        send_error_reply(&nc, &msg,"Slot not unlocked." ).await?;
        return Ok(());
    }

    let existing = match db.get_slot(&request.uuid, request.slot).await {
        Ok(n) => n,
        Err(e) => {
            error!("Error: {}", e.to_string());
            send_error_reply(&nc, &msg,"Could not check slot." ).await?;
            return Err(e);
        },
    };

    if existing.item.is_some() {
        send_error_reply(&nc, &msg,"Item already in slot." ).await?;
        return Ok(());
    }

    if existing.cooldown_seconds > 0 {
        send_error_reply(&nc, &msg,"Slot on cooldown." ).await?;
        return Ok(());
    }

    match db.store_item(&request.uuid, request.slot, &request.item.unwrap()).await {
        Ok(_) => {}
        Err(e) => {
            error!("Error: {}", e.to_string());
            send_error_reply(&nc, &msg,"StoreItem failed." ).await?;
            return Err(e);
        }
    }

    // reply
    let mut resp = StoreVaultItemResponse::new();
    resp.success = true;
    resp.error = None;
    let encoded: Vec<u8> = resp.write_to_bytes()?;
    nc.publish(msg.reply.unwrap(), encoded.into()).await.expect("Failed to publish message");

    Ok(())
}

async fn send_error_reply(nc: &Client, msg: &async_nats::Message, error: &str) -> Result<()> {
    let mut resp = StoreVaultItemResponse::new();
    resp.success = false;
    resp.error = Some(error.to_string());

    // Serialize to bytes
    let encoded: Vec<u8> = resp.write_to_bytes()?;

    // send reply
    nc.publish(msg.clone().reply.unwrap(), encoded.into()).await.expect("Failed to publish message");

    Ok(())
}