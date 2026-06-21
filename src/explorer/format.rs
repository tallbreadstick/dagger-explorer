pub fn format_with_commas(value: u64) -> String {
    let s = value.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (index, ch) in s.chars().rev().enumerate() {
        if index > 0 && index % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out.chars().rev().collect()
}

/// Format byte count as `"#,### KB"` (always kilobytes, comma-separated).
pub fn format_size_kb(bytes: u64) -> String {
    let kb = bytes.div_ceil(1024);
    format!("{} KB", format_with_commas(kb))
}

pub fn item_count_label(count: usize) -> String {
    if count == 1 {
        "1 item".to_string()
    } else {
        format!("{count} items")
    }
}
