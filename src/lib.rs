mod fs_delete;
mod fs_exists;
mod fs_list;
mod fs_new;
mod fs_mk_dir;

use fs_exists::*;
use fs_delete::*;
use fs_list::*;
use fs_new::*;
use fs_mk_dir::*;

use sqlite_loadable::{Result, define_scalar_function, define_table_function, prelude::*};

#[sqlite_entrypoint]
pub fn sqlite3_sqlitefilesystem_init(db: *mut sqlite3) -> Result<()> {
    define_scalar_function(
        db,
        "fs_exists",
        1,
        fs_exists,
        FunctionFlags::UTF8 | FunctionFlags::DETERMINISTIC,
    )?;

    define_scalar_function(
        db,
        "fs_new",
        1,
        fs_new,
        FunctionFlags::UTF8 | FunctionFlags::DETERMINISTIC,
    )?;

    define_scalar_function(
        db,
        "fs_mk_dir",
        1,
        fs_mk_dir,
        FunctionFlags::UTF8 | FunctionFlags::DETERMINISTIC,
    )?;

    define_scalar_function(
      db,
      "fs_delete",
      1,
      fs_delete,
      FunctionFlags::UTF8 | FunctionFlags::DETERMINISTIC,
  )?;

    define_table_function::<ListTable>(db, "fs_list", None)?;
    Ok(())
}
