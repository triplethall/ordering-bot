use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup, MessageId, ChatId, InputFile};
use std::sync::Arc;
use tokio_postgres::Client as PgClient;
use anyhow::Result;

use crate::bot_config::BotConfig;
use crate::db;

fn main_menu() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![
            InlineKeyboardButton::callback("📝 Сделать заказ", "menu_order"),
            InlineKeyboardButton::callback("📋 Записаться на тесты", "menu_test"),
        ],
    ])
}

// Отправляет картинку с текстом (умная отправка)
async fn send_smart_with_image(
    bot: &Bot,
    db: &PgClient,
    user_id: i64,
    image_path: &str,
    caption: &str,
    markup: Option<InlineKeyboardMarkup>,
) -> Result<Message> {
    let chat_id = ChatId(user_id);
    
    // Удаляем старое сообщение
    if let Some(old_id) = get_previous_message_id(db, user_id).await? {
        bot.delete_message(chat_id, MessageId(old_id)).await.ok();
    }
    
    // Отправляем картинку
    let input_file = InputFile::file(image_path);
    let mut msg = bot.send_photo(chat_id, input_file).caption(caption);
    if let Some(kb) = markup {
        msg = msg.reply_markup(kb);
    }
    let sent = msg.await?;
    
    // Сохраняем ID
    save_message_id(db, user_id, sent.id.0 as i32).await?;
    
    Ok(sent)
}

// Состояния FSM:
// state = 0: начало, ждём команду
// state = 1: заказ - ждём текст задачи
// state = 2: заказ - ждём контакты (заказ уже создан, ждём контакты)
// state = 3: тесты - ждём канал
// state = 4: тесты - ждём контакты (автор уже создан, ждём контакты)

// Получает состояние пользователя
async fn get_user_state(db: &PgClient, user_id: i64) -> Result<i32> {
    let row = db.query_opt(
        "SELECT state FROM ob_user_state WHERE user_id = $1",
        &[&user_id],
    ).await?;
    
    match row {
        Some(r) => Ok(r.get::<_, i32>("state")),
        None => Ok(0),
    }
}

// Сохраняет состояние
async fn save_user_state(db: &PgClient, user_id: i64, state: i32) -> Result<()> {
    db.execute(
        r#"
        INSERT INTO ob_user_state (user_id, state)
        VALUES ($1, $2)
        ON CONFLICT (user_id) DO UPDATE SET state = $2
        "#,
        &[&user_id, &state],
    )
    .await?;
    Ok(())
}

// Получает предыдущий message_id пользователя
async fn get_previous_message_id(db: &PgClient, user_id: i64) -> Result<Option<i32>> {
    let row = db.query_opt(
        "SELECT message_id FROM ob_user_messages WHERE user_id = $1",
        &[&user_id],
    ).await?;
    
    match row {
        Some(r) => Ok(Some(r.get::<_, i64>("message_id") as i32)),
        None => Ok(None),
    }
}

// Сохраняет message_id
async fn save_message_id(db: &PgClient, user_id: i64, message_id: i32) -> Result<()> {
    db.execute(
        r#"
        INSERT INTO ob_user_messages (user_id, message_id)
        VALUES ($1, $2)
        ON CONFLICT (user_id) DO UPDATE SET message_id = $2, updated_at = NOW()
        "#,
        &[&user_id, &(message_id as i64)],
    )
    .await?;
    Ok(())
}

// Умное отправка: удаляет старое сообщение, отправляет новое
async fn send_smart(
    bot: &Bot,
    db: &PgClient,
    user_id: i64,
    text: &str,
    markup: Option<InlineKeyboardMarkup>,
) -> Result<Message> {
    let chat_id = ChatId(user_id);
    
    // Удаляем старое сообщение
    if let Some(old_id) = get_previous_message_id(db, user_id).await? {
        bot.delete_message(chat_id, MessageId(old_id)).await.ok();
    }
    
    // Отправляем новое
    let mut msg = bot.send_message(chat_id, text);
    if let Some(kb) = markup {
        msg = msg.reply_markup(kb);
    }
    let sent = msg.await?;
    
    // Сохраняем ID
    save_message_id(db, user_id, sent.id.0 as i32).await?;
    
    Ok(sent)
}

pub async fn run(_db: Arc<PgClient>, bot_cfg: BotConfig) -> Result<()> {
    println!("🤖 Запускаю бота '{}'...", bot_cfg.name);

    let bot = Bot::new(&bot_cfg.bot_token);
    println!("🚀 Бот готов: @{}", bot_cfg.name);

    // Long polling
    let mut offset: Option<i32> = None;

    loop {
        let updates = match bot.get_updates()
            .timeout(30)
            .offset(offset.unwrap_or(0) + 1)
            .send()
            .await {
            Ok(u) => u,
            Err(e) => {
                eprintln!("Ошибка get_updates: {}", e);
                tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                continue;
            }
        };

        for update in updates {
            offset = Some(update.id.0 as i32);

            // Обработка сообщений
            if let teloxide::types::UpdateKind::Message(ref msg) = update.kind {
                let chat_id = msg.chat.id;
                let msg_id = msg.id;
                
                if let Some(text) = msg.text() {
                    let user_id = msg.chat.id.0 as i64;
                    let username = msg.chat.username().map(|s| s.to_string()).unwrap_or_default();
                    let name = msg.chat.first_name().map(|s| s.to_string()).unwrap_or_else(|| "Unknown".to_string());
                    
                    // Проверяем состояние FSM
                    let state = get_user_state(&*_db, user_id).await.unwrap_or(0);
                    
                    match state {
                        // state = 0: ждём команду
                        0 => {
                            match text {
                                "/start" => {
                                    send_smart_with_image(&bot, &*_db, user_id, "assets/images/start.png", "Добро пожаловать!", Some(main_menu())).await.ok();
                                }
                                "/menu" => {
                                    send_smart_with_image(&bot, &*_db, user_id, "assets/images/start.png", "Выберите действие:", Some(main_menu())).await.ok();
                                }
                                _ => {
                                    send_smart(&bot, &*_db, user_id, "Напишите /start", None::<InlineKeyboardMarkup>).await.ok();
                                }
                            }
                        }
                        // state = 1: ввели задачу заказа - создаём заказ и спрашиваем контакты
                        1 => {
                            // Создаём заказ
                            db::create_order(&*_db, user_id, &username, &name, "", text).await.ok();
                            // Переходим к ожиданию контактов
                            save_user_state(&*_db, user_id, 2).await.ok();
                            send_smart_with_image(&bot, &*_db, user_id, "assets/images/order/contacts.png", "Введите ваши контакты:", None::<InlineKeyboardMarkup>).await.ok();
                        }
                        // state = 2: ввели контакты заказа - обновляем и завершаем
                        2 => {
                            // Обновляем контакты (берём последний заказ пользователя)
                            let orders = db::get_orders_by_user(&*_db, user_id).await.unwrap_or_default();
                            if let Some(order) = orders.first() {
                                db::update_order(&*_db, order.id, None, None, Some(text), None).await.ok();
                            }
                            save_user_state(&*_db, user_id, 0).await.ok();
                            send_smart(&bot, &*_db, user_id, "✅ Заказ сохранён! Мы свяжемся с вами.", None::<InlineKeyboardMarkup>).await.ok();
                        }
                        // state = 3: канал для тестов -> переход к контактам
                        3 => {
                            // Сохраняем канал и переходим к контактам
                            save_user_state(&*_db, user_id, 4).await.ok();
                            send_smart_with_image(&bot, &*_db, user_id, "assets/images/test/contacts.png", "Введите ваши контакты:", None::<InlineKeyboardMarkup>).await.ok();
                        }
                        // state = 4: контакты для тестов
                        4 => {
                            save_user_state(&*_db, user_id, 0).await.ok();
                            send_smart(&bot, &*_db, user_id, "✅ Вы записаны на тесты!", None::<InlineKeyboardMarkup>).await.ok();
                        }
                        _ => {}
                    }
                    
                    // Удаляем входящее сообщение пользователя
                    bot.delete_message(chat_id, msg_id).await.ok();
                }
            }

            // Callback (кнопки меню)
            if let teloxide::types::UpdateKind::CallbackQuery(q) = update.kind {
                if let Some(data) = q.data {
                    let user_id = q.from.id.0 as i64;
                    
                    match data.as_str() {
                        "menu_order" => {
                            // Переходим в состояние 1 (ожидание текста заказа)
                            save_user_state(&*_db, user_id, 1).await.ok();
                            send_smart_with_image(&bot, &*_db, user_id, "assets/images/order/task.png", "Опишите ваш заказ:", None::<InlineKeyboardMarkup>).await.ok();
                        }
                        "menu_test" => {
                            // Переходим в состояние 3 (ожидание канала)
                            save_user_state(&*_db, user_id, 3).await.ok();
                            send_smart_with_image(&bot, &*_db, user_id, "assets/images/test/channel.png", "Введите ссылку на ваш канал:", None::<InlineKeyboardMarkup>).await.ok();
                        }
                        _ => {}
                    }
                }
                bot.answer_callback_query(&q.id).await.ok();
            }
        }
    }
}