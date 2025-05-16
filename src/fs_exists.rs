use sqlite_loadable::prelude::*;
use sqlite_loadable::{Result, api};

use std::path::Path;

pub fn fs_exists(context: *mut sqlite3_context, values: &[*mut sqlite3_value]) -> Result<()> {
    let path = api::value_text_notnull(values.get(0).expect("1st must be a file path"))?;
    let exists = Path::new(path).exists();
    api::result_bool(context, exists);
    Ok(())
}
