# Excel GUI Processor

A fast, lightweight, and cross-platform desktop application written in **Rust**. It provides a simple graphical user interface (GUI) to open, process, and update Microsoft Excel spreadsheets (`.xlsx`, `.xls`, `.xlsm`) without needing Microsoft Office installed.

## 🚀 Features

- **Cross-Platform:** Runs seamlessly on Windows, macOS, and Linux.
- **Modern GUI:** Built with `egui` (`eframe`), featuring dynamic font-size adjustment and system light/dark theme synchronization.
- **Non-blocking Architecture:** Heavy Excel processing runs in a background thread using channels (`std::sync::mpsc`) to keep the UI smooth and responsive.
- **Visual Progress:** Real-time progress bar showing the exact row being processed.
- **Excel Automation:** Dynamically parses worksheets, identifies tables, and appends new processed columns (e.g., "Full Name" / "ФИО" and "Account Number" / "Л/с").

## 🛠️ Tech Stack

- **Language:** Rust (Edition 2021)
- **GUI Framework:** [egui / eframe](https://github.com)
- **File Dialogs:** [rfd](https://github.com) (Native File Dialogs)
- **Excel Parser:** [calamine](https://github.com) (Pure Rust Excel Reader)
- **Excel Writer:** [rust_xlsxwriter](https://github.com) (Feature-rich Excel Writer)

## 📦 Installation & Running

### Prerequisites
Make sure you have the Rust toolchain installed. If not, get it from [rustup.rs](https://rustup.rs).

### Run Development Version
```bash
cargo run
```

### Build Production Binary
To compile a highly optimized, standalone executable:
```bash
cargo build --release
```
The executable will be generated at `target/release/excel_gui_app`.

## 📝 License

This project is open-source and available under the MIT License.
