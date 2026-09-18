//! Чтение Excel (calamine) и запись нового файла (rust_xlsxwriter).

use crate::parse::extract_account_and_name;
use calamine::{open_workbook_auto, Data, Range, Reader};
use chrono::{DateTime, Local};
use rust_xlsxwriter::{Format, Workbook, Worksheet, XlsxError};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ProcessResult {
    pub output_path: PathBuf,
    pub processed_rows: usize,
    pub skipped_rows: usize,
    pub sheets: usize,
}

fn is_empty_cell(cell: &Data) -> bool {
    match cell {
        Data::Empty => true,
        Data::String(value) => value.trim().is_empty(),
        _ => false,
    }
}

fn cell_to_text(cell: &Data) -> String {
    match cell {
        Data::Empty => String::new(),
        Data::Int(value) => value.to_string(),
        Data::Float(value) if value.is_finite() && value.fract() == 0.0 => {
            format!("{:.0}", value)
        }
        Data::Float(value) => value.to_string(),
        Data::String(value) => value.clone(),
        Data::Bool(value) => value.to_string(),
        Data::DateTime(value) => value.to_string(),
        Data::DateTimeIso(value) => value.clone(),
        Data::DurationIso(value) => value.clone(),
        Data::Error(err) => format!("#{err:?}"),
    }
}

fn last_filled_cell(row: &[Data]) -> Option<&Data> {
    row.iter().rev().find(|cell| !is_empty_cell(cell))
}

fn write_cell(sheet: &mut Worksheet, row: u32, col: u16, cell: &Data) -> Result<(), XlsxError> {
    match cell {
        Data::Empty => Ok(()),
        Data::Int(value) => sheet.write_number(row, col, *value as f64).map(|_| ()),
        Data::Float(value) => sheet.write_number(row, col, *value).map(|_| ()),
        Data::Bool(value) => sheet.write_boolean(row, col, *value).map(|_| ()),
        Data::String(value) => sheet.write_string(row, col, value).map(|_| ()),
        other => sheet.write_string(row, col, cell_to_text(other)).map(|_| ()),
    }
}

/// Имя результата: исходное имя + «исправ» + дата и время.
pub fn output_path_for(input: &Path, now: DateTime<Local>) -> PathBuf {
    let stem = input
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "file".to_string());
    let parent = input.parent().unwrap_or_else(|| Path::new("."));
    let stamp = now.format("%Y-%m-%d_%H-%M-%S");
    parent.join(format!("{stem}_исправ_{stamp}.xlsx"))
}

fn process_sheet(
    worksheet: &mut Worksheet,
    range: &Range<Data>,
    header_format: &Format,
    mut on_row: impl FnMut(),
) -> Result<(usize, usize), String> {
    if range.is_empty() {
        return Ok((0, 0));
    }

    let (start_row, start_col) = range.start().unwrap_or((0, 0));
    let width = range.width();
    if width == 0 {
        return Ok((0, 0));
    }

    let fio_col = start_col as u16 + width as u16;
    let ls_col = fio_col + 1;

    worksheet
        .set_column_width(fio_col, 22.0)
        .map_err(|err| format!("Не удалось задать ширину столбцов: {err}"))?;
    worksheet
        .set_column_width(ls_col, 16.0)
        .map_err(|err| format!("Не удалось задать ширину столбцов: {err}"))?;

    let mut processed = 0usize;
    let mut skipped = 0usize;

    for (offset, row) in range.rows().enumerate() {
        let excel_row = start_row + offset as u32;

        for (col_offset, cell) in row.iter().enumerate() {
            let excel_col = start_col as u16 + col_offset as u16;
            write_cell(worksheet, excel_row, excel_col, cell)
                .map_err(|err| format!("Не удалось записать ячейку: {err}"))?;
        }

        let extracted = last_filled_cell(row)
            .map(cell_to_text)
            .and_then(|text| extract_account_and_name(&text));

        if offset == 0 && extracted.is_none() {
            worksheet
                .write_string_with_format(excel_row, fio_col, "ФИО", header_format)
                .map_err(|err| format!("Не удалось записать заголовки: {err}"))?;
            worksheet
                .write_string_with_format(excel_row, ls_col, "Л/с", header_format)
                .map_err(|err| format!("Не удалось записать заголовки: {err}"))?;
            skipped += 1;
            on_row();
            continue;
        }

        match extracted {
            Some(data) => {
                worksheet
                    .write_string(excel_row, fio_col, &data.full_name)
                    .map_err(|err| format!("Не удалось записать ФИО: {err}"))?;
                worksheet
                    .write_string(excel_row, ls_col, &data.personal_account)
                    .map_err(|err| format!("Не удалось записать Л/с: {err}"))?;
                processed += 1;
            }
            None => skipped += 1,
        }

        on_row();
    }

    Ok((processed, skipped))
}

/// Открывает книгу, дополняет каждую непустую таблицу столбцами «ФИО» и «Л/с».
pub fn process_file(
    path: &Path,
    mut on_progress: impl FnMut(usize, usize),
) -> Result<ProcessResult, String> {
    if !path.exists() {
        return Err(format!("Файл не найден: {}", path.display()));
    }

    let mut reader = open_workbook_auto(path)
        .map_err(|err| format!("Не удалось открыть Excel-файл: {err}"))?;

    let sheets: Vec<(String, Range<Data>)> = reader
        .worksheets()
        .into_iter()
        .filter(|(_, range)| !range.is_empty())
        .collect();

    let total_rows: usize = sheets.iter().map(|(_, range)| range.height()).sum();

    if sheets.is_empty() {
        return Err("В файле не найдена таблица с данными.".to_string());
    }

    on_progress(0, total_rows.max(1));

    let mut workbook = Workbook::new();
    let header_format = Format::new().set_bold();
    let mut processed_rows = 0usize;
    let mut skipped_rows = 0usize;
    let mut done_rows = 0usize;
    let sheet_count = sheets.len();

    for (name, range) in sheets {
        let worksheet = workbook.add_worksheet();

        let safe_name: String = name
            .chars()
            .map(|ch| match ch {
                '\\' | '/' | '*' | '?' | ':' | '[' | ']' => ' ',
                other => other,
            })
            .take(31)
            .collect();
        if !safe_name.is_empty() {
            worksheet
                .set_name(&safe_name)
                .map_err(|err| format!("Не удалось назвать лист «{safe_name}»: {err}"))?;
        }

        let (processed, skipped) = process_sheet(worksheet, &range, &header_format, || {
            done_rows += 1;
            on_progress(done_rows, total_rows.max(1));
        })?;
        processed_rows += processed;
        skipped_rows += skipped;
    }

    let output_path = output_path_for(path, Local::now());
    workbook
        .save(&output_path)
        .map_err(|err| format!("Не удалось сохранить файл: {err}"))?;

    Ok(ProcessResult {
        output_path,
        processed_rows,
        skipped_rows,
        sheets: sheet_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn builds_output_file_name() {
        let now = Local.with_ymd_and_hms(2026, 9, 18, 23, 53, 1).unwrap();
        let path = Path::new("/tmp/реестр.xlsx");
        let out = output_path_for(path, now);
        assert_eq!(
            out.file_name().unwrap().to_string_lossy(),
            "реестр_исправ_2026-09-18_23-53-01.xlsx"
        );
    }
}
