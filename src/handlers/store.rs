use protobuf::{Message};
use async_nats::Client;
use anyhow::{ Result};
use crate::proto::vault_store::{StoreVaultItem, StoreVaultItemResponse};
use crate::store::Store;

pub async fn store(db: Store, nc: Client, msg: async_nats::Message) -> Result<()> {
        let request = StoreVaultItem::parse_from_bytes(&msg.payload)?;

        // check slot is within inventory size
        let size = match db.vault_size(&request.uuid).await {
            Ok(n) => n,
            Err(e) => {
                println!("vault size: {}", e);
                send_error_reply(&nc, &msg,"Could not fetch inventory size." ).await?;
                return Err(e);
            }
        };

        if request.slot >= size {
            send_error_reply(&nc, &msg,"Slot not unlocked." ).await?;
            return Ok(());
        }

        let existing = match db.get_item(&request.uuid, request.slot).await {
            Ok(n) => n.is_some(),
            Err(e) => {
                println!("existing: {}", e);
                send_error_reply(&nc, &msg,"Could not check slot." ).await?;
                return Err(e);
            },
        };

        if existing {
            send_error_reply(&nc, &msg,"Item already in slot." ).await?;
            return Ok(());
        }

        match db.store_item(&request.uuid, request.slot, &request.item.unwrap()).await {
            Ok(_) => {}
            Err(e) => {
                println!("store_item: {}", e);
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