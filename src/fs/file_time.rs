use chrono::{DateTime, Datelike, Local, Timelike};
use std::fmt::{Display, Formatter};
use std::time::SystemTime;

pub struct VFileTime {
    time: SystemTime,
}

impl Display for VFileTime {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let utc_time: DateTime<Local> = self.time.into();
        write!(
            f,
            "{}-{:02}-{:02} {:02}:{:02}:{:02}",
            utc_time.year(),
            utc_time.month(),
            utc_time.day(),
            utc_time.hour(),
            utc_time.minute(),
            utc_time.second()
        )
    }
}

impl VFileTime {
    pub fn new(time: SystemTime) -> VFileTime {
        Self { time }
    }
}
