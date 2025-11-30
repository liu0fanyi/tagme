pub fn format_file_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

pub fn format_timestamp(ts: i64) -> String {
    if ts <= 0 { return "Unknown".to_string(); }
    const SECONDS_PER_MINUTE: i64 = 60;
    const SECONDS_PER_HOUR: i64 = 3600;
    const SECONDS_PER_DAY: i64 = 86400;
    let total_days = ts / SECONDS_PER_DAY;
    let remaining_after_days = ts % SECONDS_PER_DAY;
    let hours = remaining_after_days / SECONDS_PER_HOUR;
    let remaining_after_hours = remaining_after_days % SECONDS_PER_HOUR;
    let minutes = remaining_after_hours / SECONDS_PER_MINUTE;
    let seconds = remaining_after_hours % SECONDS_PER_MINUTE;
    format!("{} days, {:02}:{:02}:{:02}", total_days, hours, minutes, seconds)
}
