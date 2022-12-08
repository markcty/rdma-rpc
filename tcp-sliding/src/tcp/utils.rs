use colored::Colorize;

pub fn _color_log_test(action: &str) {
    println!("[client]{}", action);
    println!(
        "{}, {}, {}, {}, {}, {}, and some normal text.",
        format!("Bold").bold(),
        format!("Red").red(),
        format!("Yellow").yellow(),
        format!("Green Strikethrough").green().strikethrough(),
        format!("Blue Underline").blue().underline(),
        format!("Purple Italics").purple().italic()
    );
}
pub fn client_prefix(action: &str) -> colored::ColoredString {
    format!("[Client][{}]", action).green()
}
pub fn server_prefix(action: &str) -> colored::ColoredString {
    format!("[Server][{}]", action).blue()
}

pub fn assemble_cur_buffer(mut data: &[u8], inner: &[u8], base: usize) {
    let len = inner.len();
    for i in 0..len {
        data[i + base] = inner[i];
    }
    return;
}
