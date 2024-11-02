use crate::proto::vault_item::{VaultItem, VaultItemEnchantment};
use anyhow::Result;
use protobuf::SpecialFields;
use sqlx::types::Uuid;
use chrono::{Datelike, NaiveDateTime, Utc};
use sqlx::PgPool;

#[derive(Clone)]
pub struct Store {
    db: PgPool
}

impl Store {
    pub fn new(db: PgPool) -> Self {
        Store { db }
    }

    pub async fn vault_size(&self, player: &String) -> Result<i32> {
        struct T {
            pub created: NaiveDateTime
        }
        let re : Option<T> = sqlx::query_as!(
            T,
            r#"
               SELECT
                 created
               FROM
                 players
               WHERE
                 player = $1
            ;"#,
            Uuid::parse_str(&player)?,
        ).fetch_optional(&self.db)
            .await?;

        // if we got a date, calculate number of months since for number of slots.
        if re.is_some() {
            let signup = re.unwrap().created;
            let now = Utc::now().naive_utc();

            // Get year and month differences
            let year_diff = now.year() - signup.year();
            let month_diff = now.month() as i32 - signup.month() as i32;

            // Calculate total months
            let mut total_months = year_diff * 12 + month_diff;

            // Adjust for day of month if needed
            if now.day() < signup.day() {
                total_months -= 1
            }

            return Ok(total_months+1)
        }

        // create new record
        let _ = sqlx::query!(
            r#"
               INSERT INTO players (player) VALUES ($1)
            ;"#,
            Uuid::parse_str(&player)?,
        ).execute(&self.db)
            .await?;

        // return 1 (default)
        Ok(1)
    }

    pub async fn get_item(&self, player: &String, slot: i32) -> Result<Option<VaultItem>> {
        struct T {
            pub type_: String,
            pub amount: i32,
            pub durability: i32,
            pub display_name: Option<String>,
            pub custom_model_data: Option<i32>,
            pub lore: Vec<String>,
            pub enchants: Vec<String>,
            pub flags: Vec<String>
        }
        let re = sqlx::query_as!(
            T,
            r#"
            SELECT
                type as type_, amount, durability,
                display_name, custom_model_data, lore, enchants, flags
            FROM
                items
            WHERE
                player = $1 AND
                slot = $2
            ;"#,
            Uuid::parse_str(&player)?,
            slot,
        )
            .fetch_optional(&self.db)
            .await?;

        if re.is_none() {
            return Ok(None);
        }
        let re: T = re.unwrap();

        let mut enchants = Vec::new();
        for raw in re.enchants {
            let mut enchant = VaultItemEnchantment::new();
            let parts: Vec<&str> = raw.split(',').collect();
            enchant.name = parts[0].to_string();
            enchant.level = parts[1].to_string().parse::<i32>()?;
            enchants.push(enchant);
        }

        Ok(Some(VaultItem {
            type_: re.type_,
            amount: re.amount,
            durability: re.durability,

            display_name: re.display_name,
            custom_model_data: re.custom_model_data,
            lore: re.lore,
            enchants,
            flags: re.flags,
            special_fields: SpecialFields::new(),
        }))
    }

    pub async fn store_item(&self, player: &String, slot: i32, item: &VaultItem) -> Result<bool> {
        let enchants = item.enchants.iter().map(|e| {
            (e.name.clone() + "," + e.level.to_string().as_str()).to_string()
        }).collect::<Vec<String>>();
        let _ = sqlx::query_as!(
            T,
            r#"
            INSERT INTO items (
                player, slot,
                type, amount, durability,
                display_name, custom_model_data, lore, enchants, flags
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ;"#,
            Uuid::parse_str(&player)?,
            slot,
            item.type_,
            item.amount,
            item.durability,
            item.display_name,
            item.custom_model_data,
            item.lore.as_slice(),
            &enchants,
            item.flags.as_slice(),
        )
            .fetch_optional(&self.db)
            .await;

        Ok(true)
    }

    pub async fn remove_item(&self, player: &String, slot: i32) -> Result<()> {
        let _ = sqlx::query!(
            r#"
            DELETE FROM items
            WHERE player = $1 AND slot = $2
            ;"#,
            Uuid::parse_str(&player)?,
            slot,
        )
            .execute(&self.db)
            .await;

        Ok(()) // should really return if an item was removed... so a bool
    }
}