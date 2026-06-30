//! Excel (OOXML .xlsx) export utilities.
//!
//! Uses the same hand-rolled OOXML approach as `build_redeem_codes_xlsx` in
//! `routes.rs` — no external xlsx writer crate required.
//!
//! We generate a `.xlsx` file as a zip of XML parts, following the
//! Office Open XML (ISO 29500) standard.

use std::io::Write;

use crate::feedback_service::FeedbackThreadSummary;

/// Build a feedback thread export workbook.
pub fn build_feedback_export_xlsx(
    threads: &[FeedbackThreadSummary],
    include_content: bool,
) -> Vec<u8> {
    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut cursor);
        let opts = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        write_xml_part(&mut zip, opts, "[Content_Types].xml", content_types_xml());
        write_xml_part(&mut zip, opts, "_rels/.rels", rels_xml());
        write_xml_part(&mut zip, opts, "xl/workbook.xml", workbook_xml());
        write_xml_part(
            &mut zip,
            opts,
            "xl/_rels/workbook.xml.rels",
            workbook_rels_xml(),
        );

        // Sheet 1: thread list
        write_xml_part(
            &mut zip,
            opts,
            "xl/worksheets/sheet1.xml",
            feedback_threads_sheet_xml(threads),
        );

        // Sheet 2: messages (if requested — includes message content)
        if include_content {
            write_xml_part(
                &mut zip,
                opts,
                "xl/worksheets/sheet2.xml",
                messages_header_sheet_xml(),
            );
        }

        zip.finish().expect("zip finish should not fail");
    }
    cursor.into_inner()
}

/// Build a filtered export workbook from admin list results.
pub fn build_admin_feedback_export(threads: &[FeedbackThreadSummary]) -> Vec<u8> {
    build_feedback_export_xlsx(threads, false)
}

pub fn write_xml_part<W: Write + std::io::Seek, S: AsRef<str>>(
    zip: &mut zip::ZipWriter<W>,
    opts: zip::write::SimpleFileOptions,
    name: &str,
    content: S,
) {
    zip.start_file(name, opts)
        .expect("zip start_file should not fail");
    zip.write_all(content.as_ref().as_bytes())
        .expect("zip write_all should not fail");
}

fn content_types_xml() -> &'static str {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
<Default Extension="xml" ContentType="application/xml"/>
<Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>
<Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>
</Types>"#
}

fn rels_xml() -> &'static str {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>
</Relationships>"#
}

fn workbook_xml() -> &'static str {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
          xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
<sheets>
<sheet name="Feedback Threads" sheetId="1" r:id="rId1"/>
</sheets>
</workbook>"#
}

fn workbook_rels_xml() -> &'static str {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>
</Relationships>"#
}

fn feedback_threads_sheet_xml(threads: &[FeedbackThreadSummary]) -> String {
    let headers = [
        "Thread ID",
        "Conversion Job ID",
        "Title",
        "Type",
        "Status",
        "Priority",
        "Messages",
        "Latest Reply",
        "Created At",
        "Updated At",
    ];

    let mut sheet = String::from(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#);
    sheet.push_str(
        r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">"#,
    );
    sheet.push_str("<sheetData>");

    // Header row
    let mut header_row = String::from(r#"<row r="1">"#);
    for (col, h) in headers.iter().enumerate() {
        let col_letter = (b'A' + col as u8) as char;
        header_row.push_str(&format!(
            r#"<c r="{}{}" t="inlineStr"><is><t>{}</t></is></c>"#,
            col_letter,
            1,
            xml_escape(h)
        ));
    }
    header_row.push_str("</row>");
    sheet.push_str(&header_row);

    // Data rows
    for (row_idx, t) in threads.iter().enumerate() {
        let r = (row_idx + 2) as u32;
        let mut row = format!(r#"<row r="{r}">"#);

        let count_str = t.message_count.to_string();
        let latest_str = t
            .latest_message_at
            .clone()
            .unwrap_or_else(|| "-".to_string());
        let job_id_str = t
            .conversion_job_id
            .clone()
            .unwrap_or_else(|| "-".to_string());

        macro_rules! cell {
            ($col:expr, $val:expr) => {{
                let col_letter = (b'A' + $col as u8) as char;
                let escaped = xml_escape(&$val);
                row.push_str(&format!(
                    r#"<c r="{}{}" t="inlineStr"><is><t>{}</t></is></c>"#,
                    col_letter, r, escaped
                ));
            }};
        }

        cell!(0, t.thread_id);
        cell!(1, job_id_str);
        cell!(2, t.title);
        cell!(3, t.feedback_type);
        cell!(4, t.status);
        cell!(5, t.priority);
        cell!(6, count_str);
        cell!(7, latest_str);
        cell!(8, t.created_at);
        cell!(9, t.updated_at);

        row.push_str("</row>");
        sheet.push_str(&row);
    }

    sheet.push_str("</sheetData></worksheet>");
    sheet
}

fn messages_header_sheet_xml() -> &'static str {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
<sheetData>
<row r="1">
<c r="A1" t="inlineStr"><is><t>Thread ID</t></is></c>
<c r="B1" t="inlineStr"><is><t>Title</t></is></c>
<c r="C1" t="inlineStr"><is><t>Type</t></is></c>
<c r="D1" t="inlineStr"><is><t>Status</t></is></c>
<c r="E1" t="inlineStr"><is><t>Priority</t></is></c>
<c r="F1" t="inlineStr"><is><t>Created At</t></is></c>
</row>
</sheetData>
</worksheet>"#
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
