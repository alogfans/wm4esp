use std::{thread::sleep, time::Duration};

use crate::error::Result;
use esp_idf_svc::sntp::{EspSntp, SyncStatus};

pub fn start_sntp_service() -> Result<()> {
    let sntp_service = EspSntp::new_default()?;
    while sntp_service.get_sync_status() == SyncStatus::Reset {
        sleep(Duration::from_secs(1));
    }
    Ok(())
}
