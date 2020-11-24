
// TODO factor out the common part into one macro

#[macro_export]
macro_rules! log_debug {
  ($($arg:tt)*) => ({
    // use chrono::Local;
    // use ansi_term::Color::Yellow;
    // println!("[{} {} {}] {}", Local::now(), Yellow.paint("DEBUG"), file!(), format!($($arg)*));
  })
}

#[macro_export]
macro_rules! log_info {
  ($($arg:tt)*) => ({
    use chrono::Local;
    use ansi_term::Color::Green;
    println!("[{} {} {}] {}", Local::now(), Green.paint("INFO"), file!(), format!($($arg)*));
  })
}

#[macro_export]
macro_rules! log_warn {
  ($($arg:tt)*) => ({
    use chrono::Local;
    use ansi_term::Color::Purple;
    println!("[{} {} {}] {}", Local::now(), Purple.paint("WARN"), file!(), format!($($arg)*));
  })
}

#[macro_export]
macro_rules! log_error {
  ($($arg:tt)*) => ({
    use chrono::Local;
    use ansi_term::Color::Red;
    println!("[{} {} {}] {}", Local::now(), Red.paint("ERROR"), file!(), format!($($arg)*));
  })
}