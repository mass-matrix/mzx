import loguru
import os
import subprocess
import sys
from PySide6.QtCore import QSettings, QThread, Signal
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
from . import convert_raw_file, get_vendor, __version__


class MainWindow(QMainWindow):
    def __init__(self):
        super().__init__()
        self.initUI()

    def initUI(self):
        settings = QSettings("mzx", "app")
        self.central_widget = QWidget()
        layout = QVBoxLayout(self.central_widget)

        self.setWindowTitle(f"mzx Version {__version__}")
        self.setCentralWidget(self.central_widget)

        # Restore window geometry (position and size)
        geometry = settings.value("window_geometry")
        if geometry:
            self.restoreGeometry(geometry)
        else:
            self.setGeometry(100, 100, 600, 400)

        # Peak Picking Option
        self.peakpicking_checkbox = QCheckBox("Enable Peakpicking", self)
        self.peakpicking_checkbox.setChecked(settings.value("peakpicking", False, bool))
        layout.addWidget(self.peakpicking_checkbox)

        # Remove Zeros Option
        self.removezeros_checkbox = QCheckBox("Remove Zeros", self)
        self.removezeros_checkbox.setChecked(settings.value("removezeros", False, bool))
        layout.addWidget(self.removezeros_checkbox)

        # Create a blank panel at the bottom
        blank_panel = QWidget(self)
        blank_panel.setSizePolicy(
            QSizePolicy.Policy.Expanding, QSizePolicy.Policy.Expanding
        )
        blank_panel.setMinimumHeight(self.height() // 2)
        layout.addWidget(blank_panel)

        # Log output
        self.log_text_edit = QTextEdit(self)
        self.log_text_edit.setReadOnly(True)
        layout.addWidget(self.log_text_edit)
        self._layout = layout

    def closeEvent(self, event):
        settings = QSettings("mzx", "app")
        settings.setValue("window_geometry", self.saveGeometry())
        settings.setValue("peakpicking", self.peakpicking_checkbox.isChecked())
        settings.setValue("removezeros", self.removezeros_checkbox.isChecked())
        super().closeEvent(event)

    def dragEnterEvent(self, event: QDropEvent) -> None:
        if event.mimeData().hasUrls():
            event.accept()
            self.setStyleSheet("background-color: lightgreen;")
        else:
            event.ignore()

    def dragLeaveEvent(self, event: QDropEvent) -> None:
        self.setStyleSheet("")  # Reset background color on leave

    def dropEvent(self, event: QDropEvent) -> None:
        self.setStyleSheet("")  # Reset background color after drop
        for url in event.mimeData().urls():
            path = url.toLocalFile()
            if os.path.isfile(path):
                self.convert(path)

    def convert(self, path: str) -> None:
        if not self.is_docker_running():
            dialog = self.show_popup(
                "Docker is not running. Please start Docker and try again."
            )
            dialog.close()
            self.close()

        # get the folder of the file
        _folder = os.path.dirname(path)

        # get the basename of the file without folder
        _basename = os.path.basename(path)

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

    def is_docker_running(self) -> bool:
        try:
            subprocess.run(
                ["docker", "info"],
                check=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )
            return True
        except subprocess.CalledProcessError as e:
            loguru.logger.exception(e)
            return False

    def show_popup(self, message: str) -> QMessageBox:
        dialog = QMessageBox()
        dialog.setIcon(QMessageBox.Icon.Warning)
        dialog.setText(message)
        dialog.setWindowTitle("Warning")
        dialog.setStandardButtons(QMessageBox.StandardButton.Ok)
        dialog.show()
        return dialog


def main():
    app = QApplication(sys.argv)
    window = MainWindow()
    window.show()
    sys.exit(app.exec())


if __name__ == "__main__":
    main()
