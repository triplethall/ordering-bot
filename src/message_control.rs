use anyhow::Result;
use grammers_client::{
    types::{Media, Message, Peer},
    Client,
};
use grammers_tl_types as tl;
use tokio_postgres::Client as PgClient;

/// Отправляет "умное" сообщение: удаляет предыдущее, отправляет новое
pub async fn send_smart(
    client: &Client,
    db: &PgClient,
    user_id: i64,
    text: &str,
    buttons: Option<tl::enums::ReplyMarkup>,
    file: Option<Media>,
    parse_mode: Option<&str>,
) -> Result<Message> {
    // 1. Получаем и удаляем старое сообщение (если есть)
    cleanup_previous_message(client, db, user_id).await?;

    // 2. Отправляем новое сообщение
    let peer = Peer::User(user_id);
    let user = client.get_entity(peer).await?;

    let msg = if let Some(media) = file {
        client
            .send_media(user, media)
            .caption(text)
            .reply_markup(buttons)
            .await?
    } else {
        client
            .send_message(user, text)
            .reply_markup(buttons)
            .parse_mode(parse_mode.unwrap_or(""))
            .await?
    };

    // 3. Сохраняем новый message_id
    save_message_id(db, user_id, msg.id()).await?;

    Ok(msg)
}

/// Редактирует сообщение "умно": если нельзя отредактировать — отправляет новое
pub async fn edit_smart(
    client: &Client,
    db: &PgClient,
    user_id: i64,
    message_id: i64,
    text: &str,
    buttons: Option<tl::enums::ReplyMarkup>,
    file: Option<Media>,
    parse_mode: Option<&str>,
) -> Result<Message> {
    // Пытаемся отредактировать
    let peer = Peer::User(user_id);
    let user = client.get_entity(peer).await?;

    match client
        .edit_message_text(user, message_id, text)
        .reply_markup(buttons)
        .parse_mode(parse_mode.unwrap_or(""))
        .await
    {
        Ok(msg) => {
            // Успешно отредактировали — обновляем время в БД
            update_message_timestamp(db, user_id).await?;
            Ok(msg)
        }
        Err(_) => {
            // Не удалось отредактировать — отправляем новое сообщение
            send_smart(client, db, user_id, text, buttons, file, parse_mode).await
        }
    }
}

// ===== ВСПОМОГАТЕЛЬНЫЕ ФУНКЦИИ =====

/// Удаляет предыдущее сообщение пользователя (если есть)
async fn cleanup_previous_message(client: &Client, db: &PgClient, user_id: i64) -> Result<()> {
    // Получаем старый message_id
    let old_id = get_previous_message_id(db, user_id).await?;

    if let Some(msg_id) = old_id {
        // Пытаемся удалить старое сообщение
        let peer = Peer::User(user_id);
        if let Err(e) = client.delete_messages(peer, &[msg_id]).await {
            tracing::warn!("Не удалось удалить сообщение {}: {}", msg_id, e);
        }
    }

    Ok(())
}

/// Получает предыдущий message_id из БД
async fn get_previous_message_id(db: &PgClient, user_id: i64) -> Result<Option<i64>> {
    let row = db
        .query_opt(
            "SELECT message_id FROM ob_user_messages WHERE user_id = $1",
            &[&user_id],
        )
        .await?
        .map(|r| r.try_get("message_id"));

    match row {
        Some(Ok(id)) => Ok(Some(id)),
        Some(Err(e)) => Err(e.into()),
        None => Ok(None),
    }
}

/// Сохраняет новый message_id в БД (вставка или обновление)
async fn save_message_id(db: &PgClient, user_id: i64, message_id: i64) -> Result<()> {
    db.execute(
        r#"
        INSERT INTO ob_user_messages (user_id, message_id)
        VALUES ($1, $2)
        ON CONFLICT (user_id) DO UPDATE
        SET message_id = $2, updated_at = NOW()
        "#,
        &[&user_id, &message_id],
    )
    .await?;

    Ok(())
}

/// Обновляет только временну́ю метку (для редактирования без смены ID)
async fn update_message_timestamp(db: &PgClient, user_id: i64) -> Result<()> {
    db.execute(
        "UPDATE ob_user_messages SET updated_at = NOW() WHERE user_id = $1",
        &[&user_id],
    )
    .await?;

    Ok(())
}
