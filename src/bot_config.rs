use anyhow::{bail, Result};
use serde::Deserialize;
use tokio_postgres::Client as PgClient;

/// Конфиг подключения к БД из файла config (TOML)
#[derive(Debug, Deserialize)]
pub struct DbConnectionConfig {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub user: String,
    pub password: String,
}

impl DbConnectionConfig {
    pub fn database_url(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.user, self.password, self.host, self.port, self.database
        )
    }
}

/// Конфиг бота из таблицы tg_configs
#[derive(Debug, Clone)]
pub struct BotConfig {
    pub name: String,
    pub api_id: i64,
    pub api_hash: String,
    pub bot_token: String,
    pub owner_comment: Option<String>,
}

impl BotConfig {
    pub async fn load(db: &PgClient, bot_name: &str) -> Result<Self> {
        let row = db
            .query_opt(
                r#"
                SELECT name, apiid, apihash, token, owner
                FROM tg_configs
                WHERE name = $1
                "#,
                &[&bot_name],
            )
            .await?
            .ok_or_else(|| anyhow::anyhow!("Конфиг бота '{}' не найден в БД", bot_name))?;

        // Безопасное получение значений с явным указанием типа
        let name: String = row.try_get(0)?;
        let api_id: i64 = row.try_get(1)?;
        let api_hash: String = row.try_get(2)?;
        let bot_token: String = row.try_get(3)?;
        let owner_comment: Option<String> = row.try_get(4)?;

        Ok(Self {
            name,
            api_id,
            api_hash,
            bot_token,
            owner_comment,
        })
    }
}

/// Общие настройки из таблицы tg_common
#[derive(Debug, Clone)]
pub struct CommonConfig {
    pub caching_channel_id: i64,
    pub main_user_id: i64,
}

impl CommonConfig {
    pub async fn load(db: &PgClient) -> Result<Self> {
        let row = db
            .query_opt(
                r#"
                SELECT
                    MAX(CASE WHEN name = 'caching_channel' THEN id END) AS caching_channel_id,
                    MAX(CASE WHEN name = 'main_id' THEN id END) AS main_user_id
                FROM tg_common
                "#,
                &[],
            )
            .await?
            .ok_or_else(|| anyhow::anyhow!("Не найдены общие настройки в БД"))?;

        let caching_channel_id: i64 = row.try_get("caching_channel_id")?;
        let main_user_id: i64 = row.try_get("main_user_id")?;

        // Проверка на NULL для обязательных полей
        if caching_channel_id == 0 {
            bail!("Не найден канал кэширования в tg_common");
        }
        if main_user_id == 0 {
            bail!("Не найден main_id в tg_common");
        }

        Ok(Self {
            caching_channel_id,
            main_user_id,
        })
    }
}
