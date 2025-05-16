use sqlite_loadable::prelude::*;
use sqlite_loadable::{Result, api};

use std::fs::{self};
use std::path::Path;

pub fn fs_mk_dir(context: *mut sqlite3_context, values: &[*mut sqlite3_value]) -> Result<()> {
    let path = api::value_text_notnull(values.get(0).expect("1st must be a directory name"))?;
    match fs::create_dir(path) {
        Ok(_) => {
            let exists = Path::new(path).exists();
            api::result_bool(context, exists);

            Ok(())
        }

        Err(_) => Err("Could not create directory".into()),
    }
}