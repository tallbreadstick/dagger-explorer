use std::io;
use std::path::Path;

use rusqlite::{Connection, params};

use super::super::paths::{data_dir, thumbs_db_path};

pub struct ThumbnailRecord {
    pub width: u32,
    pub height: u32,
    pub png: Vec<u8>,
}

pub struct ThumbnailDb {
    conn: Connection,
}

impl ThumbnailDb {
    pub fn open() -> io::Result<Self> {
        let path = thumbs_db_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path).map_err(io::Error::other)?;
        let db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    fn migrate(&self) -> io::Result<()> {
        const SCHEMA_VERSION: i64 = 3;

        let version: i64 = self
            .conn
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap_or(0);

        if version < SCHEMA_VERSION {
            self.conn
                .execute_batch("DROP TABLE IF EXISTS thumbnails;")
                .map_err(io::Error::other)?;
            self.conn
                .pragma_update(None, "user_version", SCHEMA_VERSION)
                .map_err(io::Error::other)?;
        }

        self.conn
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS thumbnails (
                    path TEXT NOT NULL PRIMARY KEY,
                    mtime_ns INTEGER NOT NULL,
                    width INTEGER NOT NULL,
                    height INTEGER NOT NULL,
                    png BLOB NOT NULL
                );
                CREATE INDEX IF NOT EXISTS idx_thumbnails_mtime ON thumbnails(mtime_ns);",
            )
            .map_err(io::Error::other)
    }

    pub fn get(&self, path: &Path, mtime_ns: u128) -> io::Result<Option<ThumbnailRecord>> {
        let path_str = path.to_string_lossy();
        let mut stmt = self
            .conn
            .prepare(
                "SELECT width, height, png FROM thumbnails
                 WHERE path = ?1 AND mtime_ns = ?2",
            )
            .map_err(io::Error::other)?;

        let mut rows = stmt
            .query(params![path_str.as_ref(), mtime_ns as i64])
            .map_err(io::Error::other)?;

        if let Some(row) = rows.next().map_err(io::Error::other)? {
            let width: i64 = row.get(0).map_err(io::Error::other)?;
            let height: i64 = row.get(1).map_err(io::Error::other)?;
            let png: Vec<u8> = row.get(2).map_err(io::Error::other)?;
            return Ok(Some(ThumbnailRecord {
                width: width as u32,
                height: height as u32,
                png,
            }));
        }

        Ok(None)
    }

    pub fn put(
        &self,
        path: &Path,
        mtime_ns: u128,
        width: u32,
        height: u32,
        png: &[u8],
    ) -> io::Result<()> {
        let path_str = path.to_string_lossy();
        self.conn
            .execute(
                "INSERT INTO thumbnails (path, mtime_ns, width, height, png)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(path) DO UPDATE SET
                    mtime_ns = excluded.mtime_ns,
                    width = excluded.width,
                    height = excluded.height,
                    png = excluded.png",
                params![
                    path_str.as_ref(),
                    mtime_ns as i64,
                    width as i64,
                    height as i64,
                    png,
                ],
            )
            .map_err(io::Error::other)?;
        Ok(())
    }
}

impl Default for ThumbnailDb {
    fn default() -> Self {
        Self::open().unwrap_or_else(|error| {
            eprintln!(
                "[dagger-explorer] failed to open thumbs.db in {}: {error}",
                data_dir().display()
            );
            Self {
                conn: Connection::open_in_memory().expect("in-memory sqlite"),
            }
        })
    }
}
