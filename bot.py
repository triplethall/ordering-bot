import os
import random
import json
import asyncio

from telethon import TelegramClient, events, Button
from telethon.events import StopPropagation

from alarm import info, debug, alarm, set_log_queue, preview_answer
from sql_utils import create_order, set_input_mode, get_input_mode, update_order_answers, notify_new_order
from utils import handle_callback_and_clear_keyboard

# Пути к файлам
CONFIG_PATH = r"C:\Bots\commonData\ordering\cfg_token.cfg"

temp_msg = None
user_data = {}
# Функция для загрузки конфига
def load_config():
    with open(CONFIG_PATH, 'r', encoding='utf-8') as f:
        return json.load(f)

#функция для создания клиента из конфига
async def create_client():
    config = load_config()
    api_id = config['api_id']
    api_hash = config['api_hash']
    bot_token = config['token']

    client = TelegramClient(r"C:\Bots\commonData\ordering\bot_session.session", api_id, api_hash)
    await client.start(bot_token=bot_token)
    return client

async def main(broadcast_queue):
    client = await create_client()
    info.put("Бот запущен.")
    @client.on(events.CallbackQuery)
    async def callback_handler(event):
        await event.answer()
        user_id = event.chat_id
        await handle_callback_and_clear_keyboard(client, event)
        data = event.data.decode('utf-8') if event.data else ''
        order_id = user_data.get(user_id)

        if data == "yes":
            await client.send_message(user_id, "💬 Как я могу к вам обращаться?")
            set_input_mode(order_id, 1)


    @client.on(events.NewMessage(pattern=r'/start(?:\s+(\d+))?$'))
    async def start_handler(event):
        sender = await event.get_sender()
        user_id = sender.id
        username = sender.username
        w8 = await client.send_message(user_id, "⏳")
        order_id = create_order(user_id, username)
        global user_data
        user_data[user_id] = order_id
        info.put(f'{user_id} начал заполнение заявки #{order_id}')
        PIC_PATH = r"C:\Bots\commonData\ordering\pics\start.png"
        buttons = [Button.inline("✅ Да!", "yes")]
        await client.send_file(user_id,
                               PIC_PATH,
                               caption = "👋🏻 Привет! Я бот, принимающий запросы на создание "
                                         "других ботов, помогаю быстро оформить запрос "
                                         "без звонков и переписок. "
                                         "\nЗадам пару вопросов по вашему заказу?",
                               buttons = buttons)


        try:
            await event.delete()
            await w8.delete()
        except:
            pass
        raise StopPropagation

    @client.on(events.NewMessage())
    async def text_handler(event):
        sender = await event.get_sender()
        user_id = sender.id
        global user_data
        order_id = user_data.get(user_id)

        if order_id is not None:
            if event.media is None:
                answer = event.text
                if get_input_mode(order_id) == 1:
                    await client.send_message(user_id, "💬 Опишите задачу, которую хотите решить")
                    update_order_answers(order_id, answer_1 = answer)
                    set_input_mode(order_id, 2)
                elif get_input_mode(order_id) == 2:
                    await client.send_message(user_id, "💬 Оставьте контактные данные (опционально, если нет то ответ придет на этот аккаунт Telegram)")
                    update_order_answers(order_id, answer_2=answer)
                    set_input_mode(order_id, 3)
                elif get_input_mode(order_id) == 3:
                    FIN_PATH = r"C:\Bots\commonData\ordering\pics\fin.png"
                    w8 = await client.send_message(user_id, "⏳")
                    await client.send_file(user_id,
                                     FIN_PATH,
                                     caption="Все, что вы мне сообщили, я отправлю разработчику. "
                                        "Он подумает, что можно с этим сделать, "
                                        "и свяжется с вами. Спасибо!")
                    update_order_answers(order_id, answer_3=answer)
                    user_data.pop(user_id)
                    set_input_mode(order_id, 4)
                    await notify_new_order(client, order_id)
                    try:
                        await w8.delete()
                    except:
                        pass
                else:
                    await client.send_message(user_id, r"Сначала нажмите /start !")
                    await event.delete()
            else:
                await client.send_message(user_id, r"Этот бот не принимает файлы!")
                await event.delete()
        else:
            await client.send_message(user_id, r"Сначала нужно нажать /start !")
            await event.delete()


        raise StopPropagation

    await client.run_until_disconnected()

def run_main_sync(l_queue):
    set_log_queue(l_queue)

    
    info.put("Процесс бота успешно запущен и настроил логирование.")
    asyncio.run(main(l_queue))
