use async_nats::Client;
use protobuf::MessageField;
use crate::proto::vault::{Vault, VaultSlot};
use crate::proto::vault_get::{GetVault, GetVaultResponse};
use crate::store::Store;
use protobuf::{Message};

pub async fn get(db: Store, nc: Client, msg: async_nats::Message) -> anyhow::Result<()> {
    let request = GetVault::parse_from_bytes(&msg.payload).unwrap();
    // todo - trace logging

    if let Some(reply) = msg.reply {

        /*let name = &request.subject[6..];
        client
            .publish(reply, format!("hello, {}", name).into())
            .await?;*/

        let mut vault = Vault::new();
        for n in 0..2 {
            let mut slot = VaultSlot::new();
            slot.is_locked = false;
            slot.cooldown_seconds = 0;
            vault.slots.push(slot);
        }
        for n in 2..5 {
            let mut slot = VaultSlot::new();
            slot.is_locked = false;
            slot.cooldown_seconds = 500;
            vault.slots.push(slot);
        }
        for n in 5..8 {
            let mut slot = VaultSlot::new();
            slot.is_locked = true;
            slot.cooldown_seconds = 0;
            vault.slots.push(slot);
        }

        let mut resp = GetVaultResponse::new();
        resp.vault = MessageField::some(vault);

        // Serialize to bytes
        let encoded: Vec<u8> = resp.write_to_bytes().unwrap();

        futures::executor::block_on(
            nc.publish(reply, encoded.into())
        ).expect("Could not send reply to get vault.");
    }

    return Ok(());
}