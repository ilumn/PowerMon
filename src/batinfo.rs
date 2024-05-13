extern crate battery;

use std::fmt;
use std::io;
use std::thread;
use std::time::Duration;


pub fn get_battery_info() -> battery::Result<Option<battery::Battery>> {
    let manager = battery::Manager::new()?;
    let battery = match manager.batteries()?.next() {
        Some(Ok(battery)) => Some(battery),
        Some(Err(e)) => {
            eprintln!("Unable to access battery information");
            return Err(e);
        }
        None => {
            eprintln!("Unable to find any batteries");
            return Ok(None);
        }
    };

    Ok(battery)
}
