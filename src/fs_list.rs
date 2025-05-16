//! cargo build --example series
//! sqlite3 :memory: '.read examples/test.sql'

use chrono::{DateTime, Local};
use sqlite_loadable::prelude::*;
use sqlite_loadable::table::ConstraintOperator;
use sqlite_loadable::{
    Result, api,
    table::{BestIndexError, IndexInfo, VTab, VTabArguments, VTabCursor},
};

use std::fs::Permissions;
use std::mem;
use std::os::unix::fs::PermissionsExt;
use std::time::SystemTime;

static CREATE_SQL: &str = "CREATE TABLE x(path, is_dir, bytes, is_file, created, modified, accessed, is_sym_link, readonly, extension, file_name, input hidden)";
enum Columns {
    Path,
    IsDir,
    Bytes,
    IsFile,
    Created,
    Modified,
    Accessed,
    IsSymLink,
    Permissions,
    Extension,
    FileName,
    Input,
}

fn column(index: i32) -> Option<Columns> {
    match index {
        0 => Some(Columns::Path),
        1 => Some(Columns::IsDir),
        2 => Some(Columns::Bytes),
        3 => Some(Columns::IsFile),
        4 => Some(Columns::Created),
        5 => Some(Columns::Modified),
        6 => Some(Columns::Accessed),
        7 => Some(Columns::IsSymLink),
        8 => Some(Columns::Permissions),
        9 => Some(Columns::Extension),
        10 => Some(Columns::FileName),
        11 => Some(Columns::Input),
        _ => None,
    }
}

#[repr(C)]
pub struct ListTable {
    /// must be first
    base: sqlite3_vtab,
}

impl<'vtab> VTab<'vtab> for ListTable {
    type Aux = ();
    type Cursor = ListDirectoryCursor;

    fn connect(
        _db: *mut sqlite3,
        _aux: Option<&Self::Aux>,
        _args: VTabArguments,
    ) -> Result<(String, ListTable)> {
        let vtab = ListTable {
            base: unsafe { mem::zeroed() },
        };
        Ok((CREATE_SQL.to_owned(), vtab))
    }

    fn destroy(&self) -> Result<()> {
        Ok(())
    }

    fn best_index(&self, mut info: IndexInfo) -> core::result::Result<(), BestIndexError> {
        // Check for input
        let mut has_input = false;
        for mut constraint in info.constraints() {
            match column(constraint.column_idx()) {
                Some(Columns::Input) => {
                    if constraint.usable() && constraint.op() == Some(ConstraintOperator::EQ) {
                        constraint.set_omit(true);
                        constraint.set_argv_index(1);
                        has_input = true;
                    } else {
                        return Err(BestIndexError::Constraint);
                    }
                }
                _ => todo!(),
            }
        }

        if !has_input {
            return Err(BestIndexError::Error);
        }

        info.set_estimated_cost(100000.0);
        info.set_estimated_rows(100000);
        info.set_idxnum(1);

        Ok(())
    }

    fn open(&mut self) -> Result<ListDirectoryCursor> {
        Ok(ListDirectoryCursor::new())
    }
}

struct DirectoryEntry {
    pub path: String,

    pub is_dir: bool,
    pub is_file: bool,
    pub is_sym_link: bool,

    pub bytes: i64,

    pub created: std::io::Result<SystemTime>,
    pub modified: std::io::Result<SystemTime>,
    pub accessed: std::io::Result<SystemTime>,

    pub permissions: Permissions,

    pub extension: String,
    pub file_name: String,
}

#[repr(C)]
pub struct ListDirectoryCursor {
    /// Base class. Must be first
    base: sqlite3_vtab_cursor,
    input: Option<String>,
    directory_entries: Option<Vec<DirectoryEntry>>,
    idx: usize,
}

impl ListDirectoryCursor {
    fn new() -> ListDirectoryCursor {
        ListDirectoryCursor {
            base: unsafe { mem::zeroed() },
            input: None,
            directory_entries: None,
            idx: 0,
        }
    }
}

impl VTabCursor for ListDirectoryCursor {
    fn filter(
        &mut self,
        _idx_num: i32,
        _idx_str: Option<&str>,
        values: &[*mut sqlite3_value],
    ) -> Result<()> {
        // Get the path or use the current directory
        let path = match values.get(0) {
            Some(p) => {
                if let Ok(s) = api::value_text(p) {
                    s.to_owned()
                } else {
                    String::from("./")
                }
            }
            None => {
                // String::from("./")
                return Err(format!("No path name provided!").into());
            }
        };

        if std::fs::read_dir(path.clone()).is_err() {
            return Err(format!("Directory {} not found", path).into());
        }

        // Check, if we can read the entry
        let f = std::fs::read_dir(path.clone());
        if f.is_err() {
            match f {
                Err(e) => return Err(e.to_string().into()),
                _ => {}
            }
        }

        let paths = std::fs::read_dir(path.clone()).unwrap();
        let mut entries: Vec<DirectoryEntry> = Vec::new();
        for entry in paths {
            if let Err(_) = entry {
                continue;
            }

            let e = entry.unwrap();
            let file = e.path();
            let file_name = match e.file_name().to_str() {
                Some(s) => s.to_owned(),
                None => String::from(""),
            };

            let (is_dir, size, is_file, is_sym_link, accessed, created, permissions, modified) =
                if let Ok(meta) = file.metadata() {
                    (
                        meta.is_dir(),
                        meta.len(),
                        meta.is_file(),
                        meta.is_symlink(),
                        meta.accessed(),
                        meta.created(),
                        meta.permissions(),
                        meta.modified(),
                    )
                } else {
                    (
                        false,
                        0,
                        false,
                        false,
                        Ok(SystemTime::now()),
                        Ok(SystemTime::now()),
                        Permissions::from_mode(0),
                        Ok(SystemTime::now()),
                    )
                };

            let extension = match file.extension() {
                Some(ext) => match ext.to_str() {
                    Some(e) => e.to_owned(),
                    None => String::from(""),
                },
                None => String::from(""),
            };

            let entry = DirectoryEntry {
                path: format!("{}", file.display()),
                is_dir,
                bytes: size as i64,
                is_file,
                created,
                modified,
                accessed,
                is_sym_link,
                permissions,
                extension,
                file_name,
            };
            entries.push(entry);
        }

        self.directory_entries = Some(entries);
        self.input = Some(path);
        self.idx = 0;

        Ok(())
    }

    fn next(&mut self) -> Result<()> {
        self.idx += 1;
        Ok(())
    }

    fn eof(&self) -> bool {
        match &self.directory_entries {
            Some(chars) => chars.get(self.idx).is_none(),
            None => true,
        }
    }

    fn column(&self, context: *mut sqlite3_context, i: i32) -> Result<()> {
        match column(i) {
            Some(Columns::Path) => {
                api::result_text(
                    context,
                    self.directory_entries
                        .as_ref()
                        .unwrap()
                        .get(self.idx)
                        .unwrap()
                        .path
                        .as_str(),
                )?;
            }

            Some(Columns::Input) => {
                api::result_text(context, self.input.as_ref().unwrap())?;
            }

            Some(Columns::IsDir) => {
                api::result_bool(
                    context,
                    self.directory_entries
                        .as_ref()
                        .unwrap()
                        .get(self.idx)
                        .unwrap()
                        .is_dir,
                );
            }

            Some(Columns::Bytes) => {
                api::result_int64(
                    context,
                    self.directory_entries
                        .as_ref()
                        .unwrap()
                        .get(self.idx)
                        .unwrap()
                        .bytes,
                );
            }

            Some(Columns::IsFile) => {
                api::result_bool(
                    context,
                    self.directory_entries
                        .as_ref()
                        .unwrap()
                        .get(self.idx)
                        .unwrap()
                        .is_file,
                );
            }

            Some(Columns::IsSymLink) => {
                api::result_bool(
                    context,
                    self.directory_entries
                        .as_ref()
                        .unwrap()
                        .get(self.idx)
                        .unwrap()
                        .is_sym_link,
                );
            }

            Some(Columns::Created) => {
                if let Ok(value) = self
                    .directory_entries
                    .as_ref()
                    .unwrap()
                    .get(self.idx)
                    .unwrap()
                    .created
                {
                    let datetime: DateTime<Local> = value.into();
                    api::result_text(context, format!("{}", datetime.format("%Y-%m%d %T")))?;
                } else {
                    api::result_null(context);
                }
            }

            Some(Columns::Modified) => {
                if let Ok(value) = self
                    .directory_entries
                    .as_ref()
                    .unwrap()
                    .get(self.idx)
                    .unwrap()
                    .modified
                {
                    let datetime: DateTime<Local> = value.into();
                    api::result_text(context, format!("{}", datetime.format("%Y-%m%d %T")))?;
                } else {
                    api::result_null(context);
                }
            }

            Some(Columns::Accessed) => {
                if let Ok(value) = self
                    .directory_entries
                    .as_ref()
                    .unwrap()
                    .get(self.idx)
                    .unwrap()
                    .accessed
                {
                    let datetime: DateTime<Local> = value.into();
                    api::result_text(context, format!("{}", datetime.format("%Y-%m%d %T")))?;
                } else {
                    api::result_null(context);
                }
            }

            Some(Columns::Permissions) => {
                api::result_bool(
                    context,
                    self.directory_entries
                        .as_ref()
                        .unwrap()
                        .get(self.idx)
                        .unwrap()
                        .permissions
                        .readonly(),
                );
            }

            Some(Columns::Extension) => {
                api::result_text(
                    context,
                    self.directory_entries
                        .as_ref()
                        .unwrap()
                        .get(self.idx)
                        .unwrap()
                        .extension
                        .as_str(),
                )?;
            }

            Some(Columns::FileName) => {
                api::result_text(
                    context,
                    self.directory_entries
                        .as_ref()
                        .unwrap()
                        .get(self.idx)
                        .unwrap()
                        .file_name
                        .as_str(),
                )?;
            }
            None => (),
        }
        Ok(())
    }

    fn rowid(&self) -> Result<i64> {
        Ok(self.idx as i64)
    }
}
