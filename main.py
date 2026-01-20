import asyncio
import queue
import threading
import time
import multiprocessing
from multiprocessing import Process, Queue  

from alarm import info,  set_log_queue
from flask_utils import start_site_in_thread
from gui import start_gui_thread, message_queue
from bot import run_main_sync  


def log_bridge(log_q: Queue, gui_q: queue.Queue):
    while True:
        message = log_q.get()
        gui_q.put(message)


def start_bot_process(log_queue):

    bot_process = Process(target=run_main_sync, args=(log_queue,))
    bot_process.start()
    info.put(f"Процесс бота запущен в фоне (PID: {bot_process.pid}).")
    return bot_process

if __name__ == "__main__":
    multiprocessing.freeze_support()
    log_queue = Queue()
    start_site_in_thread()
    set_log_queue(log_queue)

    bridge_thread = threading.Thread(target=log_bridge, args=(log_queue, message_queue), daemon=True)
    bridge_thread.start()

    gui_thread = threading.Thread(target=start_gui_thread)
    gui_thread.daemon = True
    gui_thread.start()


    bot_process = start_bot_process(log_queue)

    gui_thread.join()
    info.put("Окно GUI было закрыто.")

    
    info.put("Завершение процесса бота...")
    bot_process.terminate()
    bot_process.join()  

    info.put("Приложение полностью завершено.")



