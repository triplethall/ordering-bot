import sys
import queue
import threading
from PyQt5 import QtWidgets, uic, QtCore, QtGui
from alarm import info

# --- Глобальная переменная для очереди сообщений ---
# Эта очередь будет использоваться для передачи строковых сообщений из других потоков
# в поток GUI для отображения
# в textBrowser.
message_queue = queue.Queue()

class Ui_MainWindow(QtWidgets.QMainWindow):
    """
    Класс главного окна приложения.

    Он загружает UI из файла, инициализирует соединения сигналов и слотов
    и реализует функциональность приложения.
    """

    message_received = QtCore.pyqtSignal(str)

    def __init__(self, *args, **kwargs):
      
        super(Ui_MainWindow, self).__init__(*args, **kwargs)
        info.put("--- СОЗДАНИЕ ЭКЗЕМПЛЯРА ОКНА Ui_MainWindow ---")
        
        uic.loadUi(r"C:\Bots\commonData\ordering\gui\ui.ui", self)

        
        self.setWindowFlag(QtCore.Qt.FramelessWindowHint)
        self.setWindowTitle("TT: Site+Bot")
        app_icon = QtGui.QIcon(r"C:\Bots\commonData\ordering\gui\tray.png")
        self.setWindowIcon(app_icon)


        self.tray_icon = QtWidgets.QSystemTrayIcon(self)
        self.tray_icon.setIcon(QtGui.QIcon(r"C:\Bots\commonData\ordering\gui\tray.png"))


        tray_menu = QtWidgets.QMenu()
        exit_action = tray_menu.addAction("Выход")
        exit_action.triggered.connect(self.close)

        self.tray_icon.setContextMenu(tray_menu)
        self.tray_icon.show()

        self.pushButton.clicked.connect(self.hide)

        self.pushButton_2.clicked.connect(self.close)

        self.tray_icon.activated.connect(self.on_tray_icon_activated)

        self.message_received.connect(self.append_message)

        self.queue_thread = threading.Thread(target=self.process_queue, daemon=True)
        self.queue_thread.start()

        self.drag_pos = None

    def process_queue(self):
        """
        Функция, которая выполняется в отдельном потоке и постоянно ждет сообщений
        в очереди. При получении сообщения она испускает сигнал для обновления GUI.
        """
        while True:
            message = message_queue.get()
            if message is None:
                break
            self.message_received.emit(message)

    def closeEvent(self, event: QtGui.QCloseEvent):
        
        info.put("Окно получило команду на закрытие, останавливаем внутренние потоки...")

        
        message_queue.put(None)

        self.queue_thread.join(timeout=1)

        event.accept()

    @QtCore.pyqtSlot(str)
    def append_message(self, message):
        """
        Слот, который добавляет полученное сообщение в textBrowser.
        Этот метод выполняется в основном потоке GUI.
        """
        self.textBrowser.append(message)

    def on_tray_icon_activated(self, reason):
        """
        Обрабатывает клики по иконке в системном трее.
        """
        if reason == QtWidgets.QSystemTrayIcon.Trigger:
            self.show()

    def hideEvent(self, event):
        """
        Переопределяется для корректной обработки сворачивания в трей.
        Когда окно скрывается, мы просто показываем иконку в трее.
        """
        self.tray_icon.show()
        super().hideEvent(event)

    def showEvent(self, event):
        """Переопределяет стандартное событие показа, чтобы скрыть иконку трея при показе окна."""
        self.tray_icon.hide()
        super().showEvent(event)

    def mousePressEvent(self, event: QtGui.QMouseEvent):
        """
        Переопределяет событие нажатия мыши для реализации перетаскивания окна.
        """
        if event.button() == QtCore.Qt.LeftButton:
            if self.pushButton.geometry().contains(event.pos()) or \
                    self.pushButton_2.geometry().contains(event.pos()) or \
                    self.textBrowser.geometry().contains(event.pos()):
                super().mousePressEvent(event)
                return

            # Сохраняем начальную позицию курсора относительно окна.
            self.drag_pos = event.globalPos() - self.frameGeometry().topLeft()
            event.accept()

    def mouseMoveEvent(self, event: QtGui.QMouseEvent):
        """
        Переопределяет событие перемещения мыши.
        """
        
        if event.buttons() == QtCore.Qt.LeftButton and self.drag_pos is not None:
            self.move(event.globalPos() - self.drag_pos)
            event.accept()

    def mouseReleaseEvent(self, event: QtGui.QMouseEvent):
        """
        Переопределяет событие отпускания мыши.
        """
        self.drag_pos = None
        event.accept()


def start_gui_thread():
    
    app = QtWidgets.QApplication.instance()  
    if not app:  
        app = QtWidgets.QApplication(sys.argv)


    app.setQuitOnLastWindowClosed(True)

    window = Ui_MainWindow()
    window.show()
    app.exec_()
