pub mod bot;
pub mod bot_config;
pub mod db;
pub mod i18n;

use bot_config::{BotConfig, DbConnectionConfig};
use std::env;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let is_production = env::var("BOT_ENV")
        .map(|v| v == "production")
        .unwrap_or(false);

    let db_section = if is_production {
        "database.production"
    } else {
        "database.local"
    };

    println!(
        "🔧 Среда: {}",
        if is_production { "production" } else { "local" }
    );

    // Читаем конфиг из файла
    let settings = config::Config::builder()
        .add_source(config::File::with_name("config"))
        .build()?;

    let db_cfg: DbConnectionConfig = settings.get(db_section)?;

    println!("✅ Подключаюсь к БД {}:{}", db_cfg.host, db_cfg.port);

    // Подключаемся к БД
    let (client, connection) = tokio_postgres::connect(&db_cfg.database_url(), tokio_postgres::NoTls).await?;

    // Фоновый таск для соединения
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("⚠️ Соединение с БД разорвано: {}", e);
        }
    });

    let db_client = Arc::new(client);

    // Загружаем конфиг бота
    let bot_cfg = BotConfig::load(&db_client, "test").await?;

    let owner = bot_cfg.owner_comment.clone().unwrap_or_else(|| "не указан".to_string());
    println!("\n✅ Конфиг бота '{}':", bot_cfg.name);
    println!("   API ID: {}", bot_cfg.api_id);
    println!("   Владелец: {}", owner);

    // Запускаем бота
    println!("\n🚀 Запускаю бота...");
    bot::run(db_client.clone(), bot_cfg).await?;

    Ok(())
}