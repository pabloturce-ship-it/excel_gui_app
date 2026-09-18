//! Графический интерфейс: выбор файла и отображение хода обработки.

use eframe::egui::{
    self, FontFamily, FontId, ProgressBar, RichText, Slider, TextStyle, ThemePreference,
};
// Предполагаем, что эти структуры объявлены в вашем модуле excel
// ТАК НАДО:
use excel_gui_app::excel::{self, ProcessResult};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

const DEFAULT_FONT_SIZE: f32 = 16.0;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([560.0, 400.0])
            .with_min_inner_size([440.0, 320.0])
            .with_title("Обработка Excel"),
        ..Default::default()
    };

    eframe::run_native(
        "Обработка Excel",
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_theme(ThemePreference::System);
            apply_font_size(&cc.egui_ctx, DEFAULT_FONT_SIZE);
            Ok(Box::new(App::default()))
        }),
    )
}

fn apply_font_size(ctx: &egui::Context, size: f32) {
    ctx.all_styles_mut(|style| {
        style.text_styles.insert(
            TextStyle::Small,
            FontId::new((size * 0.85).round(), FontFamily::Proportional),
        );
        style
            .text_styles
            .insert(TextStyle::Body, FontId::new(size, FontFamily::Proportional));
        style.text_styles.insert(
            TextStyle::Button,
            FontId::new(size, FontFamily::Proportional),
        );
        style.text_styles.insert(
            TextStyle::Heading,
            FontId::new((size * 1.35).round(), FontFamily::Proportional),
        );
        style.text_styles.insert(
            TextStyle::Monospace,
            FontId::new(size, FontFamily::Monospace),
        );
    });
}

enum JobEvent {
    Progress { current: usize, total: usize },
    Done(ProcessResult),
    Error(String),
}

struct App {
    selected_file: Option<PathBuf>,
    status: String,
    status_is_error: bool,
    progress: f32,
    progress_text: String,
    busy: bool,
    font_size: f32,
    receiver: Option<Receiver<JobEvent>>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            selected_file: None,
            status: "Выберите Excel-файл (.xlsx или .xls).".to_string(),
            status_is_error: false,
            progress: 0.0,
            progress_text: String::new(),
            busy: false,
            font_size: DEFAULT_FONT_SIZE,
            receiver: None,
        }
    }
}

impl App {
    fn pick_file(&mut self) {
        if self.busy {
            return;
        }

        let picked = rfd::FileDialog::new()
            .add_filter("Excel", &["xlsx", "xls", "xlsm"])
            .set_title("Выберите файл Excel")
            .pick_file();

        if let Some(path) = picked {
            self.status = format!("Выбран файл:\n{}", path.display());
            self.status_is_error = false;
            self.progress = 0.0;
            self.progress_text.clear();
            self.selected_file = Some(path);
        }
    }

    fn start_processing(&mut self) {
        if self.busy {
            return;
        }

        let Some(path) = self.selected_file.clone() else {
            self.status = "Сначала выберите файл.".to_string();
            self.status_is_error = true;
            return;
        };

        let (sender, receiver) = mpsc::channel();
        self.receiver = Some(receiver);
        self.busy = true;
        self.status_is_error = false;
        self.progress = 0.0;
        self.progress_text = "Запуск…".to_string();
        self.status = "Обработка файла…".to_string();

        thread::spawn(move || {
            let sender_for_progress = sender.clone();
            let result = excel::process_file(&path, move |current, total| {
                let _ = sender_for_progress.send(JobEvent::Progress { current, total });
            });

            match result {
                Ok(done) => {
                    let _ = sender.send(JobEvent::Done(done));
                }
                Err(message) => {
                    let _ = sender.send(JobEvent::Error(message));
                }
            }
        });
    }

    fn poll_job(&mut self) {
        let Some(receiver) = self.receiver.as_ref() else {
            return;
        };

        loop {
            match receiver.try_recv() {
                Ok(JobEvent::Progress { current, total }) => {
                    let total = total.max(1);
                    self.progress = current as f32 / total as f32;
                    self.progress_text = format!("Строка {current} из {total}");
                    self.status = format!("Идёт обработка: строка {current} из {total}");
                }
                Ok(JobEvent::Done(result)) => {
                    self.busy = false;
                    self.receiver = None;
                    self.progress = 1.0;
                    self.progress_text = "Готово".to_string();
                    self.status_is_error = false;
                    self.status = format!(
                        "Готово.\nОбработано строк: {}.\nПропущено строк: {}.\nЛистов: {}.\nСохранено:\n{}",
                        result.processed_rows,
                        result.skipped_rows,
                        result.sheets,
                        result.output_path.display()
                    );
                    break;
                }
                Ok(JobEvent::Error(message)) => {
                    self.busy = false;
                    self.receiver = None;
                    self.progress = 0.0;
                    self.progress_text.clear();
                    self.status_is_error = true;
                    self.status = format!("Ошибка: {message}");
                    break;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.busy = false;
                    self.receiver = None;
                    self.status_is_error = true;
                    self.status = "Ошибка: обработка прервалась.".to_string();
                    break;
                }
            }
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_job();
        if self.busy {
            ctx.request_repaint();
        }

        // В eframe метод называется `update` (раньше был `ui`), а панель объявляется здесь
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(8.0);
            ui.heading("Обработка Excel");
            ui.add_space(4.0);
            ui.label("Программа найдёт таблицу, добавит столбцы «ФИО» и «Л/с» и сохранит новый файл.");
            ui.add_space(12.0);

            // Слайдер изменения шрифта
            let old_font_size = self.font_size;
            ui.add(
                Slider::new(&mut self.font_size, 12.0..=28.0)
                    .text("Размер шрифта")
                    .integer(),
            );
            // Применяем настройки шрифта только если ползунок сдвинулся (оптимизация)
            if (self.font_size - old_font_size).abs() > 0.1 {
                apply_font_size(ui.ctx(), self.font_size);
            }

            ui.add_space(12.0);

            // Кнопки управления
            ui.horizontal(|ui| {
                let pick = ui.add_enabled(!self.busy, egui::Button::new("Выбрать файл"));
                if pick.clicked() {
                    self.pick_file();
                }

                let run = ui.add_enabled(
                    !self.busy && self.selected_file.is_some(),
                    egui::Button::new("Обработать"),
                );
                if run.clicked() {
                    self.start_processing();
                }
            });

            ui.add_space(12.0);

            // Отображение пути выбранного файла
            if let Some(path) = &self.selected_file {
                ui.label(RichText::new("Файл:").strong());
                ui.label(path.display().to_string());
            } else {
                ui.label("Файл ещё не выбран.");
            }

            ui.add_space(12.0);
            
            // Прогресс-бар
            ui.add(
                ProgressBar::new(self.progress)
                    .text(&self.progress_text)
                    .desired_width(f32::INFINITY),
            );
            ui.add_space(8.0);

            // Статусный текст (ошибки подсвечиваются красным)
            let color = if self.status_is_error {
                ui.visuals().error_fg_color
            } else {
                ui.visuals().text_color()
            };
            ui.label(RichText::new(&self.status).color(color));
        });
    }
}
