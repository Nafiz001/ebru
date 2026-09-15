//! The gallery: prints on disk, and a SQLite table describing them.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use marbling::Op;
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;

use crate::patterns::Tally;

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Print {
    pub id: i64,
    /// "Taraklı no. 4"
    pub name: String,
    pub pattern: String,
    /// Unix seconds.
    pub made_at: i64,
    /// How many people had a hand in it; 0 if the tray made it alone.
    pub people: u32,
    pub ground: u8,
    pub tally: Tally,
}

/// A print on its way to the gallery.
pub struct Draft {
    pub pattern: &'static str,
    pub people: u32,
    pub tally: Tally,
    pub recipe: Recipe,
    pub image: Vec<u8>,
    pub small: Vec<u8>,
    pub made_at: i64,
}

/// Everything needed to watch a print being made again.
#[derive(Serialize, serde::Deserialize)]
pub struct Recipe {
    pub ground: u8,
    pub ops: Vec<Op>,
}

#[derive(Clone)]
pub struct Store {
    db: Arc<Mutex<Connection>>,
    dir: PathBuf,
}

impl Store {
    pub fn open(data: &Path) -> rusqlite::Result<Store> {
        let dir = data.join("prints");
        std::fs::create_dir_all(&dir).map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
        let db = Connection::open(data.join("ebru.sqlite3"))?;
        db.execute_batch(
            "PRAGMA journal_mode = WAL;
             CREATE TABLE IF NOT EXISTS prints (
                 id INTEGER PRIMARY KEY,
                 name TEXT NOT NULL,
                 pattern TEXT NOT NULL,
                 made_at INTEGER NOT NULL,
                 people INTEGER NOT NULL,
                 ground INTEGER NOT NULL,
                 tally TEXT NOT NULL
             );",
        )?;
        Ok(Store { db: Arc::new(Mutex::new(db)), dir })
    }

    pub fn image_path(&self, id: i64, small: bool) -> PathBuf {
        self.dir.join(if small { format!("{id}-small.png") } else { format!("{id}.png") })
    }

    fn recipe_path(&self, id: i64) -> PathBuf {
        self.dir.join(format!("{id}.json"))
    }

    /// Records a print and writes its files. Blocking; call from a worker thread.
    pub fn save(&self, print: Draft) -> Result<Print, Box<dyn std::error::Error + Send + Sync>> {
        let Draft { pattern, people, tally, recipe, image, small, made_at } = print;
        let db = self.db.lock().expect("store lock");
        let count: i64 = db.query_row("SELECT COUNT(*) FROM prints WHERE pattern = ?1", [pattern], |r| r.get(0))?;
        let name = format!("{pattern} no. {}", count + 1);
        db.execute(
            "INSERT INTO prints (name, pattern, made_at, people, ground, tally) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![name, pattern, made_at, people, recipe.ground, serde_json::to_string(&tally)?],
        )?;
        let id = db.last_insert_rowid();
        drop(db);

        std::fs::write(self.image_path(id, false), image)?;
        std::fs::write(self.image_path(id, true), small)?;
        std::fs::write(self.recipe_path(id), serde_json::to_vec(&recipe)?)?;

        Ok(Print { id, name, pattern: pattern.to_string(), made_at, people, ground: recipe.ground, tally })
    }

    pub fn list(&self, before: Option<i64>, limit: u32) -> rusqlite::Result<Vec<Print>> {
        let db = self.db.lock().expect("store lock");
        let mut query = db.prepare(
            "SELECT id, name, pattern, made_at, people, ground, tally FROM prints
             WHERE id < ?1 ORDER BY id DESC LIMIT ?2",
        )?;
        let rows = query.query_map(params![before.unwrap_or(i64::MAX), limit], row_to_print)?;
        rows.collect()
    }

    pub fn get(&self, id: i64) -> rusqlite::Result<Option<Print>> {
        let db = self.db.lock().expect("store lock");
        db.query_row("SELECT id, name, pattern, made_at, people, ground, tally FROM prints WHERE id = ?1", [id], row_to_print)
            .optional()
    }

    pub fn recipe(&self, id: i64) -> std::io::Result<Vec<u8>> {
        std::fs::read(self.recipe_path(id))
    }
}

fn row_to_print(row: &rusqlite::Row) -> rusqlite::Result<Print> {
    let tally: String = row.get(6)?;
    Ok(Print {
        id: row.get(0)?,
        name: row.get(1)?,
        pattern: row.get(2)?,
        made_at: row.get(3)?,
        people: row.get(4)?,
        ground: row.get(5)?,
        tally: serde_json::from_str(&tally).unwrap_or_default(),
    })
}
