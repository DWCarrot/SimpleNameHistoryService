use std::fmt::Debug;
use std::fmt::Display;
use std::time::Duration;
use std::time::SystemTime;

use serde::Serialize;
use serde::Serializer;
use serde::ser::SerializeStruct;
use sqlx::FromRow;
use sqlx::Row;
use sqlx::sqlite::SqliteRow;
use uuid::Uuid;

use crate::client::config::UseCacheConfig; 

#[derive(Debug)]
pub struct SystemTimeTransformationError<T: Debug>(T);

impl<T: Debug + Display> Display for SystemTimeTransformationError<T> {

    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("SystemTimeTransformationError[timestamp = {}]", &self.0))
    }
}

impl<T: Debug + Display> std::error::Error for SystemTimeTransformationError<T> {

}

fn systemtime_error(index: &str, raw: i64) -> sqlx::Error {

    sqlx::Error::ColumnDecode { index: index.to_string(), source: Box::new(SystemTimeTransformationError(raw)) }
}


pub(super) fn into_argument_uuid<'a>(v: &'a Uuid) -> &'a [u8] {
    v.as_bytes().as_slice()
}

pub(super) const CREATE_TABLE_NAMES: &'static str = 
"CREATE TABLE IF NOT EXISTS `names` (
    \"index\"	INTEGER NOT NULL UNIQUE,
    \"uuid\"	BLOB NOT NULL,
    \"name\"	TEXT NOT NULL,
    \"changedToAt\"	INTEGER,
    \"source\"	INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY(\"index\" AUTOINCREMENT)
)
";

pub(super) const CREATE_INDEX_NAMES: &'static str =
"CREATE INDEX IF NOT EXISTS `names_index_uuid` ON `names`(\"uuid\")";

pub(super) const QUERY_NAME_HISTORY: &'static str = 
"SELECT \"name\", \"changedToAt\"
FROM `names`
WHERE \"uuid\" = ?
ORDER BY \"changedToAt\"
";

pub(super) const INSERT_NAME: &'static str = 
"INSERT INTO `names`
(\"uuid\", \"name\", \"changedToAt\", \"source\")
VALUES (?, ?, ?, ?)
";

pub(super) const INSERT_FIRST_NAME: &'static str = 
"INSERT INTO `names`
(\"uuid\", \"name\", \"source\")
VALUES (?, ?, ?)
";


pub type NameHistory = Vec<NameHistoryElement>;


#[derive(Debug)]
pub struct NameHistoryElement {
    
    pub name: String,
    
    pub changed_to_at: Option<SystemTime>,
}

impl Serialize for NameHistoryElement {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer 
    {
        let mut s = serializer.serialize_struct("NameHistoryElement", 2)?;
        s.serialize_field("name", self.name.as_str())?;
        if let Some(changed_to_at) = &self.changed_to_at {
            match changed_to_at.duration_since(SystemTime::UNIX_EPOCH) {
                Ok(d) => {
                    let t = d.as_millis();
                    s.serialize_field("changedToAt", &t)?;
                }
                Err(e) => {
                    s.serialize_field("changedToAt", "")?;
                }
            }
        } else {
            s.skip_field("changedToAt")?;
        }
        s.end()
    }
}


impl<'r> FromRow<'r, SqliteRow> for NameHistoryElement {
    fn from_row(row: &'r SqliteRow) -> Result<Self, sqlx::Error> {
        let name = row.try_get("name")?;
        let changed_to_at_timestamp: Option<i64> = row.try_get("changedToAt")?;
        let changed_to_at = if let Some(changed_to_at_timestamp) = changed_to_at_timestamp {
            let v: u64 = unsafe { std::mem::transmute(changed_to_at_timestamp) };
            let d = Duration::from_millis(v);
            Some(SystemTime::UNIX_EPOCH.checked_add(d).ok_or_else(|| systemtime_error("changedToAt", changed_to_at_timestamp))?)
        } else {
            None
        };
        Ok(NameHistoryElement { name, changed_to_at })
    }
}

impl NameHistoryElement {

    pub fn new_initial(name: String) -> Self {
        Self { name, changed_to_at: None }
    }

    pub fn new(name: String, changed_to_at: SystemTime) -> Self {
        Self { name, changed_to_at: Some(changed_to_at) }
    }

    pub(super) fn into_argument_systemtime<'a>(v: &'a SystemTime) -> i64 {
        match v.duration_since(SystemTime::UNIX_EPOCH) {
            Ok(s) => {
                unsafe {
                    let d = s.as_millis() as u64;
                    std::mem::transmute(d)
                }
            } ,
            Err(e) => {
                0
            }
        }
    }
}


pub(super) const CREATE_TABLE_UPDATES: &'static str =
"CREATE TABLE IF NOT EXISTS `updates` (
    \"uuid\"	BLOB NOT NULL UNIQUE,
    \"update\"	INTEGER NOT NULL,
    \"changed\"	BOOLEAN NOT NULL,
    PRIMARY KEY(\"uuid\")
)
";

pub(super) const CREATE_INDEX_UPDATES: &'static str =
"CREATE INDEX IF NOT EXISTS `updates_index_uuid` ON `updates`(\"uuid\")";

pub(super) const QUERY_UPDATE: &'static str =
"SELECT \"update\", \"changed\"
FROM `updates`
WHERE \"uuid\" = ?
";

pub(super) const REFRESH_UPDATE: &'static str =
"UPDATE `updates`
SET \"update\" = ?, \"changed\" = ?
WHERE \"uuid\" = ?
";

pub(super) const NEW_UPDATE: &'static str =
"INSERT INTO `updates`
VALUES(?, ?, ?)
";


#[derive(Debug)]
pub struct Update {

    pub update: SystemTime,

    pub changed: bool
}

impl<'r> FromRow<'r, SqliteRow> for Update {
    fn from_row(row: &'r SqliteRow) -> Result<Self, sqlx::Error> {
        let update_timestamp: i64 = row.try_get("update")?;
        let update = {
            let v: u64 = unsafe { std::mem::transmute(update_timestamp) };
            let d = Duration::from_secs(v);
            SystemTime::UNIX_EPOCH.checked_add(d).ok_or_else(|| systemtime_error("update", update_timestamp))?
        };
        let changed = row.try_get("changed")?;
        Ok(Update { update, changed })
    }
}

impl Update {
    
    pub fn new(update: SystemTime, changed: bool) -> Self {
        Self {
            update,
            changed: true
        }
    }

    pub fn use_cache(&self, now: &SystemTime, config: &UseCacheConfig) -> bool {
        match now.duration_since(self.update) {
            Ok(d) => {
                if self.changed {
                    d < config.changed
                } else {
                    d < config.unchanged
                }
            },
            Err(_e) => {
                false
            }
        }
    }

    pub(super) fn into_argument_systemtime<'a>(v: &'a SystemTime) -> i64 {
        match v.duration_since(SystemTime::UNIX_EPOCH) {
            Ok(s) => {
                unsafe {
                    let d = s.as_secs();
                    std::mem::transmute(d)
                }
            } ,
            Err(e) => {
                0
            }
        }
    }
}


