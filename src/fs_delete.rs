use sqlite_loadable::prelude::*;
use sqlite_loadable::{Result, api};

use std::path::Path;

pub fn fs_delete(context: *mut sqlite3_context, values: &[*mut sqlite3_value]) -> Result<()> {
    let path = api::value_text_notnull(values.get(0).expect("1st must be a file name"))?;
    let exists = Path::new(path).exists();
    // Not deleted but hey, file does not exist
    if !exists {
        api::result_bool(context, false);
        return Ok(());
    }

    match std::fs::remove_file(path) {
        Ok(_) => {
            // File deleted by this command so return true
            api::result_bool(context, true);
            Ok(())
        }

        Err(_) => Err("Could not delete file".into()),
    }
}
