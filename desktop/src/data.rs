use rusqlite::{Connection, OpenFlags, params};
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq)]
pub struct Dictation {
    pub id: i64,
    pub created_at: i64,
    pub text: String,
    pub word_count: i64,
    pub duration_seconds: f64,
}

#[derive(Debug, Default)]
pub struct Statistics {
    pub word_count: i64,
    pub dictation_count: i64,
    pub duration_seconds: f64,
}

impl Statistics {
    pub fn estimated_seconds_saved(&self, typing_words_per_minute: f64) -> f64 {
        if typing_words_per_minute.is_finite() == false || typing_words_per_minute <= 0.0 {
            return 0.0;
        }
        let typing_seconds = self.word_count as f64 * 60.0 / typing_words_per_minute;
        let seconds_saved = typing_seconds - self.duration_seconds;
        if seconds_saved > 0.0 {
            seconds_saved
        } else {
            0.0
        }
    }
}

pub fn database_path() -> Result<PathBuf, String> {
    database_path_from(std::env::var_os("XDG_DATA_HOME"), std::env::var_os("HOME"))
}

fn database_path_from(
    configured_data_home: Option<std::ffi::OsString>,
    home: Option<std::ffi::OsString>,
) -> Result<PathBuf, String> {
    if let Some(directory) = configured_data_home {
        let path = PathBuf::from(directory);
        if path.is_absolute() == true {
            return Ok(path.join("quill/history.sqlite3"));
        }
    }
    match home {
        Some(directory) => {
            let path = PathBuf::from(directory);
            if path.is_absolute() == true {
                Ok(path.join(".local/share/quill/history.sqlite3"))
            } else {
                Err("Cannot locate Quill history: HOME must be an absolute path.".to_string())
            }
        }
        None => Err("Cannot locate Quill history: HOME is not set.".to_string()),
    }
}

fn open_database(path: &Path) -> Result<Option<Connection>, String> {
    match path.try_exists() {
        Ok(false) => return Ok(None),
        Ok(true) => {}
        Err(error) => return Err(format!("Cannot access Quill history: {error}")),
    }
    let connection = match Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY) {
        Ok(connection) => connection,
        Err(error) => return Err(format!("Cannot open Quill history: {error}")),
    };
    match connection.busy_timeout(Duration::from_millis(250)) {
        Ok(()) => {}
        Err(error) => return Err(format!("Cannot configure Quill history: {error}")),
    }
    let version: i64 = match connection.query_row("PRAGMA user_version", [], |row| row.get(0)) {
        Ok(version) => version,
        Err(error) => return Err(format!("Cannot read Quill history schema: {error}")),
    };
    if version > 1 {
        return Err(format!(
            "Unsupported Quill history schema version: {version}"
        ));
    }
    Ok(Some(connection))
}

pub fn load_statistics() -> Result<Statistics, String> {
    match database_path() {
        Ok(path) => load_statistics_at(&path),
        Err(error) => Err(error),
    }
}

fn load_statistics_at(path: &Path) -> Result<Statistics, String> {
    let connection = match open_database(path) {
        Ok(Some(connection)) => connection,
        Ok(None) => return Ok(Statistics::default()),
        Err(error) => return Err(error),
    };
    let totals: (i64, i64, f64) = match connection.query_row(
        "SELECT COALESCE(SUM(word_count), 0), COUNT(*), COALESCE(SUM(duration_seconds), 0.0) FROM dictations",
        [],
        |row| Ok((row.get(0), row.get(1), row.get(2))),
    ) {
        Ok((Ok(words), Ok(count), Ok(seconds))) => (words, count, seconds),
        Ok(_) => return Err("Quill history contains invalid statistics.".to_string()),
        Err(error) => return Err(format!("Cannot read Quill statistics: {error}")),
    };
    Ok(Statistics {
        word_count: totals.0,
        dictation_count: totals.1,
        duration_seconds: totals.2,
    })
}

pub fn list_dictations(search: &str, limit: u32) -> Result<Vec<Dictation>, String> {
    match database_path() {
        Ok(path) => list_dictations_at(&path, search, limit),
        Err(error) => Err(error),
    }
}

fn list_dictations_at(path: &Path, search: &str, limit: u32) -> Result<Vec<Dictation>, String> {
    let connection = match open_database(path) {
        Ok(Some(connection)) => connection,
        Ok(None) => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let mut statement = match connection.prepare(
        "SELECT id, created_at, text, word_count, duration_seconds FROM dictations
         WHERE instr(lower(text), lower(?1)) > 0
         ORDER BY created_at DESC, id DESC LIMIT ?2",
    ) {
        Ok(statement) => statement,
        Err(error) => return Err(format!("Cannot read Quill dictations: {error}")),
    };
    let mut rows = match statement.query(params![search, limit]) {
        Ok(rows) => rows,
        Err(error) => return Err(format!("Cannot search Quill dictations: {error}")),
    };
    let mut dictations = Vec::new();
    loop {
        let row = match rows.next() {
            Ok(Some(row)) => row,
            Ok(None) => break,
            Err(error) => return Err(format!("Cannot read Quill dictation: {error}")),
        };
        match (row.get(0), row.get(1), row.get(2), row.get(3), row.get(4)) {
            (Ok(id), Ok(created_at), Ok(text), Ok(word_count), Ok(duration_seconds)) => {
                dictations.push(Dictation {
                    id,
                    created_at,
                    text,
                    word_count,
                    duration_seconds,
                });
            }
            _ => return Err("Quill history contains an invalid dictation.".to_string()),
        }
    }
    Ok(dictations)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_DATABASE: AtomicU64 = AtomicU64::new(0);

    struct TestDatabase {
        path: PathBuf,
    }

    impl TestDatabase {
        fn new() -> Self {
            let unique_id = NEXT_DATABASE.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "quill-history-test-{}-{unique_id}.sqlite3",
                std::process::id()
            ));
            Self { path }
        }

        fn populate(&self) {
            let connection = Connection::open(&self.path).unwrap();
            connection
                .execute_batch(
                    "PRAGMA user_version = 1;
                 CREATE TABLE dictations (id INTEGER PRIMARY KEY, created_at INTEGER NOT NULL,
                 text TEXT NOT NULL, word_count INTEGER NOT NULL, duration_seconds REAL NOT NULL);
                 INSERT INTO dictations VALUES (1, 100, 'Hello World', 2, 1.5);
                 INSERT INTO dictations VALUES (2, 200, '100%_literal', 2, 2.5);
                 INSERT INTO dictations VALUES (3, 200, 'Latest words here', 3, 3.0);",
                )
                .unwrap();
        }
    }

    impl Drop for TestDatabase {
        fn drop(&mut self) {
            match std::fs::remove_file(&self.path) {
                Ok(()) => {}
                Err(error) => {
                    if error.kind() != std::io::ErrorKind::NotFound {
                        panic!("Cannot clean test database: {error}");
                    }
                }
            }
        }
    }

    #[test]
    fn resolves_absolute_data_home_and_home_fallback() {
        assert_eq!(
            database_path_from(Some("/data".into()), None).unwrap(),
            PathBuf::from("/data/quill/history.sqlite3")
        );
        for configured in [None, Some("".into()), Some("relative".into())] {
            assert_eq!(
                database_path_from(configured, Some("/home/person".into())).unwrap(),
                PathBuf::from("/home/person/.local/share/quill/history.sqlite3")
            );
        }
        assert!(database_path_from(None, None).is_err() == true);
    }

    #[test]
    fn calculates_totals_and_nonnegative_time_saved() {
        let database = TestDatabase::new();
        database.populate();
        let statistics = load_statistics_at(&database.path).unwrap();
        assert_eq!(statistics.word_count, 7);
        assert_eq!(statistics.dictation_count, 3);
        assert_eq!(statistics.duration_seconds, 7.0);
        assert_eq!(statistics.estimated_seconds_saved(30.0), 7.0);
        for speed in [0.0, -1.0, f64::NAN, f64::INFINITY, 120.0] {
            assert_eq!(statistics.estimated_seconds_saved(speed), 0.0);
        }
    }

    #[test]
    fn searches_literal_text_and_orders_and_limits_results() {
        let database = TestDatabase::new();
        database.populate();
        let records = list_dictations_at(&database.path, "", 2).unwrap();
        assert_eq!(
            records.iter().map(|record| record.id).collect::<Vec<_>>(),
            vec![3, 2]
        );
        assert_eq!(
            list_dictations_at(&database.path, "hello", 10).unwrap()[0].text,
            "Hello World"
        );
        assert_eq!(
            list_dictations_at(&database.path, "%_", 10).unwrap()[0].id,
            2
        );
        assert!(
            list_dictations_at(&database.path, "' OR 1=1 --", 10)
                .unwrap()
                .is_empty()
                == true
        );
        assert!(
            list_dictations_at(&database.path, "", 0)
                .unwrap()
                .is_empty()
                == true
        );
        assert_eq!(records[0].created_at, 200);
        assert_eq!(records[0].word_count, 3);
        assert_eq!(records[0].duration_seconds, 3.0);
    }

    #[test]
    fn missing_database_returns_empty_without_creating_file() {
        let database = TestDatabase::new();
        assert_eq!(
            load_statistics_at(&database.path).unwrap().dictation_count,
            0
        );
        assert!(
            list_dictations_at(&database.path, "", 10)
                .unwrap()
                .is_empty()
                == true
        );
        assert!(database.path.exists() == false);
    }

    #[test]
    fn corrupt_database_missing_table_and_new_schema_are_errors() {
        let corrupt = TestDatabase::new();
        std::fs::write(&corrupt.path, "not a database").unwrap();
        let missing_table = TestDatabase::new();
        Connection::open(&missing_table.path).unwrap();
        let newer = TestDatabase::new();
        newer.populate();
        Connection::open(&newer.path)
            .unwrap()
            .execute_batch("PRAGMA user_version = 2")
            .unwrap();
        for database in [&corrupt, &missing_table, &newer] {
            assert!(load_statistics_at(&database.path).is_err() == true);
            assert!(list_dictations_at(&database.path, "", 10).is_err() == true);
        }
        assert!(
            load_statistics_at(&newer.path)
                .unwrap_err()
                .contains("version: 2")
                == true
        );
    }
}
