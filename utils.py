async def handle_callback_and_clear_keyboard(client, event):
    msg = await event.get_message()

    # если это медиа-сообщение (фото/док/и т.д.)
    if msg.media:
        # убираем кнопки у старого
        await client.edit_message(
            msg.peer_id,
            msg.id,
            file=msg.media,
            text=msg.message + "\n🔽🔽🔽" or None,
            buttons=None
        )
    else:
        # обычный текст
        await client.edit_message(
            msg.peer_id,
            msg.id,
            text=msg.message + "\n🔽🔽🔽" or None,
            buttons=None
        )