use sqlite_loadable::prelude::*;
use sqlite_loadable::{Result, api};

use std::fs::File;
use std::path::Path;

pub fn fs_new(context: *mut sqlite3_context, values: &[*mut sqlite3_value]) -> Result<()> {
    let path = api::value_text_notnull(values.get(0).expect("1st must be a file name"))?;
    match File::create(path) {
        Ok(_) => {
            let exists = Path::new(path).exists();
            api::result_bool(context, exists);

            Ok(())
        }

        Err(_) => Err("Could not create file".into()),
    }
}