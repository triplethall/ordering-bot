use anyhow::Result;
use chrono::{DateTime, Utc};
use tokio_postgres::Client as PgClient;

/// Минимальные структуры данных (вместо внешнего модуля models)
#[derive(Debug, Clone)]
pub struct Order {
    pub id: i32,
    pub user_id: i64,
    pub username: String,
    pub name: String,
    pub contacts: String,
    pub text: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct TestAuthor {
    pub id: i32,
    pub user_id: i64,
    pub username: String,
    pub name: String,
    pub contacts: String,
    pub text: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct Pupil {
    pub id: i32,
    pub user_id: i64,
    pub username: String,
    pub name: String,
    pub contacts: String,
    pub text: String,
    pub created_at: DateTime<Utc>,
}

/// Создаёт все таблицы (идемпотентно)
pub async fn create_tables(db: &PgClient) -> Result<()> {
    // Таблица заказов
    db.execute(
        r#"
        CREATE TABLE IF NOT EXISTS ob_orders (
            id SERIAL PRIMARY KEY,
            user_id BIGINT NOT NULL,
            username TEXT NOT NULL,
            name TEXT NOT NULL,
            contacts TEXT NOT NULL,
            text TEXT NOT NULL,
            created_at TIMESTAMPTZ DEFAULT NOW()
        )
        "#,
        &[],
    )
    .await?;

    // Таблица авторов тестов
    db.execute(
        r#"
        CREATE TABLE IF NOT EXISTS ob_test_authors (
            id SERIAL PRIMARY KEY,
            user_id BIGINT NOT NULL,
            username TEXT NOT NULL,
            name TEXT NOT NULL,
            contacts TEXT NOT NULL,
            text TEXT NOT NULL,
            created_at TIMESTAMPTZ DEFAULT NOW()
        )
        "#,
        &[],
    )
    .await?;

    // Таблица учеников
    db.execute(
        r#"
        CREATE TABLE IF NOT EXISTS ob_pupils (
            id SERIAL PRIMARY KEY,
            user_id BIGINT NOT NULL,
            username TEXT NOT NULL,
            name TEXT NOT NULL,
            contacts TEXT NOT NULL,
            text TEXT NOT NULL,
            created_at TIMESTAMPTZ DEFAULT NOW()
        )
        "#,
        &[],
    )
    .await?;

    // Таблица состояний диалога (для бота)
    db.execute(
        r#"
        CREATE TABLE IF NOT EXISTS ob_user_state (
            user_id BIGINT PRIMARY KEY,
            state INT4 NOT NULL DEFAULT 0,
            language TEXT NOT NULL DEFAULT 'ru',
            created_at TIMESTAMPTZ DEFAULT NOW()
        )
        "#,
        &[],
    )
    .await?;

    // Добавляем колонку language если её нет (миграция)
    db.execute(
        "ALTER TABLE ob_user_state ADD COLUMN IF NOT EXISTS language TEXT NOT NULL DEFAULT 'ru'",
        &[],
    )
    .await?;

    // Таблица кэширования изображений
    db.execute(
        r#"
        CREATE TABLE IF NOT EXISTS ob_image_cache (
            name TEXT PRIMARY KEY,
            subfolder TEXT NOT NULL,
            file_id BIGINT NOT NULL,
            access_hash BIGINT NOT NULL,
            file_reference BYTEA NOT NULL,
            created_at TIMESTAMPTZ DEFAULT NOW()
        )
        "#,
        &[],
    )
    .await?;

    println!("✅ Таблицы созданы/проверены");
    Ok(())
}

// ===== ORDER =====

/// Создаёт новый заказ
pub async fn create_order(
    db: &PgClient,
    user_id: i64,
    username: &str,
    name: &str,
    contacts: &str,
    text: &str,
) -> Result<i32> {
    let row = db
        .query_one(
            r#"
            INSERT INTO ob_orders (user_id, username, name, contacts, text)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id
            "#,
            &[&user_id, &username, &name, &contacts, &text],
        )
        .await?;

    Ok(row.try_get(0)?)
}

/// Получает заказ по ID
pub async fn get_order_by_id(db: &PgClient, id: i32) -> Result<Option<Order>> {
    let row = db
        .query_opt(
            "SELECT id, user_id, username, name, contacts, text, created_at FROM ob_orders WHERE id = $1",
            &[&id],
        )
        .await?
        .ok_or_else(|| anyhow::anyhow!("Заказ с ID {} не найден", id))?;

    Ok(Some(Order {
        id: row.try_get("id")?,
        user_id: row.try_get("user_id")?,
        username: row.try_get("username")?,
        name: row.try_get("name")?,
        contacts: row.try_get("contacts")?,
        text: row.try_get("text")?,
        created_at: row.try_get("created_at")?,
    }))
}

/// Получает все заказы пользователя
pub async fn get_orders_by_user(db: &PgClient, user_id: i64) -> Result<Vec<Order>> {
    let rows = db
        .query(
            "SELECT id, user_id, username, name, contacts, text, created_at FROM ob_orders WHERE user_id = $1 ORDER BY created_at DESC",
            &[&user_id],
        )
        .await?;

    let orders = rows
        .iter()
        .map(|row| {
            Ok(Order {
                id: row.try_get("id")?,
                user_id: row.try_get("user_id")?,
                username: row.try_get("username")?,
                name: row.try_get("name")?,
                contacts: row.try_get("contacts")?,
                text: row.try_get("text")?,
                created_at: row.try_get("created_at")?,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(orders)
}

/// Обновляет поля заказа (кроме created_at)
pub async fn update_order(
    db: &PgClient,
    id: i32,
    username: Option<&str>,
    name: Option<&str>,
    contacts: Option<&str>,
    text: Option<&str>,
) -> Result<()> {
    // Сохраняем флаги наличия значений ДО получения значений
    let has_username = username.is_some();
    let has_name = name.is_some();
    let has_contacts = contacts.is_some();
    let has_text = text.is_some();

    if !has_username && !has_name && !has_contacts && !has_text {
        return Ok(());
    }

    let query = format!(
        "UPDATE ob_orders SET {} WHERE id = $1",
        [
            username.map(|_| "username = $2"),
            name.map(|_| "name = $3"),
            contacts.map(|_| "contacts = $4"),
            text.map(|_| "text = $5"),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(", ")
    );

    let username = username.unwrap_or("");
    let name = name.unwrap_or("");
    let contacts = contacts.unwrap_or("");
    let text = text.unwrap_or("");

    let params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = vec![
        &id,
        &username,
        &name,
        &contacts,
        &text,
    ];

    db.execute(
        &query,
        &params[..(1
            + has_username as usize
            + has_name as usize
            + has_contacts as usize
            + has_text as usize)],
    )
    .await?;

    Ok(())
}

// ===== LANGUAGE =====

/// Получает язык пользователя
pub async fn get_user_language(db: &PgClient, user_id: i64) -> Result<String> {
    let row = db.query_opt(
        "SELECT language FROM ob_user_state WHERE user_id = $1",
        &[&user_id],
    ).await?;
    
    match row {
        Some(r) => Ok(r.get::<_, String>("language")),
        None => Ok("ru".to_string()), // Default to Russian
    }
}

/// Устанавливает язык пользователя
pub async fn set_user_language(db: &PgClient, user_id: i64, language: &str) -> Result<()> {
    db.execute(
        r#"
        INSERT INTO ob_user_state (user_id, state, language)
        VALUES ($1, 0, $2)
        ON CONFLICT (user_id) DO UPDATE SET language = $2
        "#,
        &[&user_id, &language],
    )
    .await?;
    Ok(())
}

// ===== TEST_AUTHOR =====
