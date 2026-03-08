use anyhow::Result;

use hippoclaudus::config::Config;
use hippoclaudus::discover_sessions;
use hippoclaudus::state::SyncState;

use crate::helpers::{file_mtime, session_id_from_path};

pub fn cmd_list(config: &Config, days: u32) -> Result<()> {
    let state = SyncState::open(config)?;
    let sessions = discover_sessions(config, Some(days));

    let mut count = 0;
    for path in &sessions {
        let session_id = session_id_from_path(path);
        let mtime = file_mtime(path);

        if state.is_processed(&session_id, mtime) {
            continue;
        }

        let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);

        println!(
            "{}\t{}\t{:.0}\t{}KB",
            session_id,
            path.display(),
            mtime,
            size / 1024
        );
        count += 1;
    }

    if count == 0 {
        println!("No unprocessed sessions in the last {} days.", days);
    } else {
        println!("\n{} unprocessed sessions.", count);
    }
    Ok(())
}
