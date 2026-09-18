use calamine::{open_workbook_auto, Data, Reader};
use excel_gui_app::excel::process_file;
use rust_xlsxwriter::Workbook;
use std::fs;

#[test]
fn adds_fio_and_account_columns() {
    let dir = std::env::temp_dir().join("excel_processor_test");
    fs::create_dir_all(&dir).unwrap();
    let input = dir.join("sample.xlsx");

    let mut workbook = Workbook::new();
    let sheet = workbook.add_worksheet();
    sheet.write_string(0, 0, "№").unwrap();
    sheet.write_string(0, 1, "Комментарий").unwrap();
    sheet.write_string(1, 0, "1").unwrap();
    sheet
        .write_string(1, 1, "1234567890 Иванов И.И.")
        .unwrap();
    sheet.write_string(2, 0, "2").unwrap();
    sheet
        .write_string(
            2,
            1,
            "л/с 9876543210 Петров П.П. г. Казань, доп. сведения",
        )
        .unwrap();
    workbook.save(&input).unwrap();

    let result = process_file(&input, |_current, _total| {}).unwrap();
    assert!(result.output_path.exists());
    assert_eq!(result.processed_rows, 2);

    let mut written = open_workbook_auto(&result.output_path).unwrap();
    let range = written.worksheet_range("Sheet1").unwrap();
    let rows: Vec<Vec<String>> = range
        .rows()
        .map(|row| {
            row.iter()
                .map(|cell| match cell {
                    Data::Empty => String::new(),
                    other => other.to_string(),
                })
                .collect()
        })
        .collect();

    assert_eq!(rows[0][2], "ФИО");
    assert_eq!(rows[0][3], "Л/с");
    assert_eq!(rows[1][2], "Иванов И.И.");
    assert_eq!(rows[1][3], "1234567890");
    assert_eq!(rows[2][2], "Петров П.П.");
    assert_eq!(rows[2][3], "9876543210");

    let _ = fs::remove_file(&input);
    let _ = fs::remove_file(&result.output_path);
}
