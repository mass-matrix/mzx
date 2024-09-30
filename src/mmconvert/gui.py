import os
import subprocess
import sys
from PySide6.QtGui import QDropEvent
from PySide6.QtWidgets import (
    QApplication,
    QMainWindow,
    QMessageBox,
    QVBoxLayout,
    QWidget,
    QCheckBox,
    QTextEdit,
    QSizePolicy,
)
from . import convert_raw_file, get_vendor


class MainWindow(QMainWindow):
    def __init__(self):
        super().__init__()
        self.initUI()

    def initUI(self):
        self.setWindowTitle("EZConvert")
        self.setGeometry(100, 100, 600, 400)

        self.central_widget = QWidget()
        self.setCentralWidget(self.central_widget)

        self.layout = QVBoxLayout(self.central_widget)

        self.peakpicking_checkbox = QCheckBox("Enable Peakpicking", self)
        self.layout.addWidget(self.peakpicking_checkbox)

        # Remove Zeros Options
        self.removezeros_checkbox = QCheckBox("Remove Zeros", self)
        self.layout.addWidget(self.removezeros_checkbox)

        # Create a blank panel at the bottom
        self.blank_panel = QWidget(self)
        self.blank_panel.setSizePolicy(QSizePolicy.Expanding, QSizePolicy.Expanding)
        self.blank_panel.setMinimumHeight(self.height() // 2)
        self.layout.addWidget(self.blank_panel)

        # Log output
        self.log_text_edit = QTextEdit(self)
        self.log_text_edit.setReadOnly(True)
        self.layout.addWidget(self.log_text_edit)

    def dragEnterEvent(self, event):
        if event.mimeData().hasUrls():
            event.accept()
        else:
            event.ignore()

    def dropEvent(self, event: QDropEvent):
        for url in event.mimeData().urls():
            path = url.toLocalFile()
            if os.path.isfile(path):
                self.convert(path)

    def convert(self, path):
        if not self.is_docker_running():
            self.show_popup("Docker is not running. Please start Docker and try again.")
            return

        # get the folder of the file
        folder = os.path.dirname(path)

        # get the basename of the file without folder
        basename = os.path.basename(path)

        # check if peakpicking is enabled
        peakpicking = self.peakpicking_checkbox.isChecked()
        removezeros = self.removezeros_checkbox.isChecked()

        options = ""
        options += ' --filter "peakPicking true 1-"' if peakpicking else ""
        options += ' --filter "zeroSamples removeExtra"' if removezeros else ""

        # call the command to convert the file with the output file as parameter
        # command = f"docker run --rm -v {folder}:/data chambm/pwiz-skyline-i-agree-to-the-vendor-licenses:latest wine msconvert {basename} {options}"
        vendor = get_vendor(path)
        _outfile = convert_raw_file(path, vendor)

        # process = subprocess.Popen(
        #     command,
        #     shell=True,
        #     stdout=subprocess.PIPE,
        #     stderr=subprocess.PIPE,
        #     text=True,
        #     bufsize=1,
        # )
        # # Read the output line by line
        # for line in process.stdout:
        #     line = line.strip()
        #     if line == "":
        #         continue
        #     self.log_text_edit.append(line)
        #     QApplication.processEvents()  # Update the GUI

        # # Read the error output line by line
        # for line in process.stderr:
        #     line = line.strip()
        #     if line == "":
        #         continue
        #     self.log_text_edit.append(line)
        #     QApplication.processEvents()  # Update the GUI

    def is_docker_running(self):
        try:
            subprocess.run(
                ["docker", "info"],
                check=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )
            return True
        except subprocess.CalledProcessError:
            return False

    def show_popup(self, message):
        msg = QMessageBox()
        msg.setIcon(QMessageBox.Warning)
        msg.setText(message)
        msg.setWindowTitle("Warning")
        msg.setStandardButtons(QMessageBox.Ok)
        msg.exec()


def main():
    app = QApplication(sys.argv)
    window = MainWindow()
    window.show()
    sys.exit(app.exec())


if __name__ == "__main__":
    main()
