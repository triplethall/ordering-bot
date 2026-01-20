import sqlite3
from datetime import datetime

conn = sqlite3.connect(r"C:\Bots\commonData\ordering\orders.db")
conn.execute("""
CREATE TABLE IF NOT EXISTS orders (
    order_id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    username TEXT NOT NULL,
    date TEXT NOT NULL,
    answer_1 TEXT,
    answer_2 TEXT,
    answer_3 TEXT,
    input_mode INTEGER NOT NULL DEFAULT 0
);
""")
conn.commit()


async def notify_new_order(client,order_id: int):
    """
    Отправляет уведомление о завершённом заказе с HTML разметкой.
    """
    cur = conn.cursor()
    cur.execute("""
        SELECT user_id, username, answer_1, answer_2, answer_3 
        FROM orders WHERE order_id = ?
    """, (order_id,))

    row = cur.fetchone()
    if not row:
        return False

    user_id, username, answer_1, answer_2, answer_3 = row

    # HTML сообщение
    message = f"""🚀 <b>Новый заказ #{order_id}!</b>

👤 Пользователь: <code>@{username}</code> / <code>{user_id}</code>
🧑 Представился как: <b>{answer_1}</b>

📋 Ему нужно:
<blockquote>{answer_2}</blockquote>

📞 Контакты:
<blockquote>{answer_3}</blockquote>"""

    try:
        await client.send_message(!YOUR_ID!, message, parse_mode='html')
        return True
    except Exception:
        return False

def create_order(user_id: int, username: str | None):
    """Создаёт заказ, заполняет user_id/username/date, возвращает order_id."""
    norm_username = username if username else "0"
    cur = conn.cursor()
    cur.execute(
        "INSERT INTO orders (user_id, username, date) VALUES (?, ?, ?)",
        (user_id, norm_username, datetime.now().isoformat())
    )
    conn.commit()
    return cur.lastrowid

def get_input_mode(order_id: int) -> int | None:
    """Возвращает input_mode по order_id или None если заказа нет."""
    cur = conn.cursor()
    cur.execute("SELECT input_mode FROM orders WHERE order_id = ?", (order_id,))
    row = cur.fetchone()
    return row[0] if row else None

def set_input_mode(order_id: int, input_mode: int) -> bool:
    """Устанавливает input_mode по order_id. Возвращает True если обновлено."""
    cur = conn.cursor()
    cur.execute(
        "UPDATE orders SET input_mode = ? WHERE order_id = ?",
        (input_mode, order_id)
    )
    conn.commit()
    return cur.rowcount > 0  


def is_order_complete(order_id: int) -> bool:
    """
    Проверяет, заполнены ли все три ответа в заказе.
    Возвращает True если order_id существует И все answer_1/2/3 не NULL.
    """
    cur = conn.cursor()
    cur.execute("""
        SELECT answer_1, answer_2, answer_3 
        FROM orders 
        WHERE order_id = ? 
        AND answer_1 IS NOT NULL 
        AND answer_2 IS NOT NULL 
        AND answer_3 IS NOT NULL
    """, (order_id,))

    row = cur.fetchone()
    return row is not None

def update_order_answers(order_id: int,
                         answer_1: str | None = None,
                         answer_2: str | None = None,
                         answer_3: str | None = None) -> bool:
    """
    Обновляет ответы в заказе. Заполняет только переданные параметры.
    Возвращает True если заказ найден и обновлён.
    """
    updates = []
    params = []

    if answer_1 is not None:
        updates.append("answer_1 = ?")
        params.append(answer_1)
    if answer_2 is not None:
        updates.append("answer_2 = ?")
        params.append(answer_2)
    if answer_3 is not None:
        updates.append("answer_3 = ?")
        params.append(answer_3)

    if not updates:
        return False  # Нечего обновлять

    params.append(order_id)
    set_clause = ", ".join(updates)

    cur = conn.cursor()
    cur.execute(f"UPDATE orders SET {set_clause} WHERE order_id = ?", params)
    conn.commit()

    return cur.rowcount > 0
