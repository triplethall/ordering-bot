use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

/// Supported languages
pub const LANG_RU: &str = "ru";
pub const LANG_EN: &str = "en";

/// Get language display name
pub fn lang_name(lang: &str) -> &str {
    match lang {
        LANG_EN => "English",
        _ => "Русский",
    }
}

/// Translations struct
pub struct Translations {
    pub welcome: &'static str,
    pub choose_action: &'static str,
    pub choose_language: &'static str,
    pub order_btn: &'static str,
    pub test_btn: &'static str,
    pub order_task: &'static str,
    pub order_contacts: &'static str,
    pub test_channel: &'static str,
    pub test_contacts: &'static str,
    pub order_saved: &'static str,
    pub test_registered: &'static str,
    pub write_start: &'static str,
    pub lang_changed: &'static str,
}

/// Get translations for a language
pub fn t(lang: &str) -> Translations {
    match lang {
        LANG_EN => Translations {
            welcome: "Welcome!",
            choose_action: "Choose an action:",
            choose_language: "Select language:",
            order_btn: "📝 Make order",
            test_btn: "📋 Sign up for tests",
            order_task: "Describe your order:",
            order_contacts: "Enter your contacts:",
            test_channel: "Enter your channel link:",
            test_contacts: "Enter your contacts:",
            order_saved: "✅ Order saved! We will contact you.",
            test_registered: "✅ You are signed up for tests!",
            write_start: "Write /start",
            lang_changed: "✅ Language changed!",
        },
        _ => Translations {
            welcome: "Добро пожаловать!",
            choose_action: "Выберите действие:",
            choose_language: "Выберите язык:",
            order_btn: "📝 Сделать заказ",
            test_btn: "📋 Записаться на тесты",
            order_task: "Опишите ваш заказ:",
            order_contacts: "Введите ваши контакты:",
            test_channel: "Введите ссылку на ваш канал:",
            test_contacts: "Введите ваши контакты:",
            order_saved: "✅ Заказ сохранён! Мы свяжемся с вами.",
            test_registered: "✅ Вы записаны на тесты!",
            write_start: "Напишите /start",
            lang_changed: "✅ Язык изменён!",
        },
    }
}

/// Language selection keyboard
pub fn lang_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![
            InlineKeyboardButton::callback("🇷🇺 Русский", "lang_ru"),
            InlineKeyboardButton::callback("🇬🇧 English", "lang_en"),
        ],
    ])
}

/// Main menu keyboard
pub fn main_menu(lang: &str) -> InlineKeyboardMarkup {
    let tr = t(lang);
    InlineKeyboardMarkup::new(vec![
        vec![
            InlineKeyboardButton::callback(tr.order_btn, "menu_order"),
            InlineKeyboardButton::callback(tr.test_btn, "menu_test"),
        ],
    ])
}