use async_nats::Client;
use protobuf::MessageField;
use crate::proto::vault::{Vault, VaultSlot};
use crate::proto::vault_get::{GetVault, GetVaultResponse};
use crate::store::Store;
use protobuf::{Message};

pub async fn get(db: Store, nc: Client, msg: async_nats::Message) -> anyhow::Result<()> {
    let request = GetVault::parse_from_bytes(&msg.payload).unwrap();

    if let Some(reply) = msg.reply {

        // get vault size
        let size = match db.vault_size(&request.uuid).await {
            Ok(size) => size,
            Err(e) => {
                return Err(e);
            }
        };

        // Get each vault item
        let mut vault = Vault::new();
        for n in 0..size {
            let mut slot = VaultSlot::new();
            slot.is_locked = false;
            slot.cooldown_seconds = 0;

            let item = match db.get_item(&request.uuid, n).await {
                Ok(item) => item,
                Err(e) => {
                    return Err(e);
                }
            };

            if item.is_some() {
                slot.item = MessageField::some(item.unwrap());
            }
            vault.slots.push(slot);
        }

        // build response and send
        let mut resp = GetVaultResponse::new();
        resp.vault = MessageField::some(vault);
        let encoded: Vec<u8> = resp.write_to_bytes().unwrap();
        nc.publish(reply, encoded.into()).await?;
    }

    return Ok(());
}