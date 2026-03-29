use chrono::{DateTime, Datelike, Local, Timelike};
use std::time::SystemTime;

pub struct VFileTime {
    time: SystemTime,
}

impl VFileTime {
    pub fn new(time: SystemTime) -> VFileTime {
        Self { time }
    }

    pub fn to_string(&self) -> String {
        let utc_time: DateTime<Local> = self.time.into();
        format!(
            "{}-{:02}-{:02} {:02}:{:02}:{:02}",
            utc_time.year(),
            utc_time.month(),
            utc_time.day(),
            utc_time.hour(),
            utc_time.minute(),
            utc_time.second()
        )
        .to_string()
    }
}
