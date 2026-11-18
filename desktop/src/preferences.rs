use gtk::gio::prelude::*;
use gtk::{gio, glib};
use std::path::{Path, PathBuf};

fn vocabulary_path() -> Result<PathBuf, String> {
    match Path::new(env!("CARGO_MANIFEST_DIR")).parent() {
        Some(project_directory) => Ok(project_directory.join("keywords.txt")),
        None => Err("Cannot locate the project vocabulary file.".to_string()),
    }
}

pub fn load_vocabulary() -> Result<(String, Option<String>), String> {
    match vocabulary_path() {
        Ok(path) => load_vocabulary_at(&path),
        Err(error) => Err(error),
    }
}

fn load_vocabulary_at(path: &Path) -> Result<(String, Option<String>), String> {
    let file = gio::File::for_path(path);
    match file.load_contents(gio::Cancellable::NONE) {
        Ok((contents, etag)) => match String::from_utf8(contents.to_vec()) {
            Ok(text) => Ok((text, etag.map(|value| value.to_string()))),
            Err(error) => Err(format!("Vocabulary must contain UTF-8 text: {error}")),
        },
        Err(error) => Err(format!("Cannot read vocabulary: {error}")),
    }
}

pub fn save_vocabulary(
    contents: &str,
    expected_etag: Option<&str>,
) -> Result<Option<String>, String> {
    match vocabulary_path() {
        Ok(path) => save_vocabulary_at(&path, contents, expected_etag),
        Err(error) => Err(error),
    }
}

fn save_vocabulary_at(
    path: &Path,
    contents: &str,
    expected_etag: Option<&str>,
) -> Result<Option<String>, String> {
    let file = gio::File::for_path(path);
    match file.replace_contents(
        contents.as_bytes(),
        expected_etag,
        false,
        gio::FileCreateFlags::NONE,
        gio::Cancellable::NONE,
    ) {
        Ok(etag) => Ok(etag.map(|value| value.to_string())),
        Err(error) => {
            if error.matches(gio::IOErrorEnum::WrongEtag) == true {
                Err(
                    "Vocabulary changed outside Quill. Reload it before saving your edits."
                        .to_string(),
                )
            } else {
                Err(format!("Cannot save vocabulary: {error}"))
            }
        }
    }
}

fn preferences_path() -> Result<PathBuf, String> {
    preferences_path_from(
        std::env::var_os("XDG_CONFIG_HOME"),
        std::env::var_os("HOME"),
    )
}

fn preferences_path_from(
    configured_home: Option<std::ffi::OsString>,
    home: Option<std::ffi::OsString>,
) -> Result<PathBuf, String> {
    if let Some(directory) = configured_home {
        let path = PathBuf::from(directory);
        if path.is_absolute() == true {
            return Ok(path.join("quill/preferences.ini"));
        }
    }
    match home {
        Some(directory) => {
            let path = PathBuf::from(directory);
            if path.is_absolute() == true {
                Ok(path.join(".config/quill/preferences.ini"))
            } else {
                Err("Cannot locate Quill preferences: HOME must be an absolute path.".to_string())
            }
        }
        None => Err("Cannot locate Quill preferences: HOME is not set.".to_string()),
    }
}

pub fn load_typing_speed() -> Result<u32, String> {
    match preferences_path() {
        Ok(path) => load_typing_speed_at(&path),
        Err(error) => Err(error),
    }
}

fn load_typing_speed_at(path: &Path) -> Result<u32, String> {
    let preferences = match load_preferences_at(path) {
        Ok((preferences, _)) => preferences,
        Err(error) => return Err(error),
    };
    if preferences.has_group("Statistics") == false {
        return Ok(40);
    }
    match preferences.integer("Statistics", "typing_words_per_minute") {
        Ok(speed) => {
            if (10..=200).contains(&speed) == true {
                Ok(speed as u32)
            } else {
                Err("Typing speed must be between 10 and 200 words per minute.".to_string())
            }
        }
        Err(error) => Err(format!("Cannot read typing speed: {error}")),
    }
}

fn load_preferences_at(path: &Path) -> Result<(glib::KeyFile, Option<String>), String> {
    let file = gio::File::for_path(path);
    let (contents, etag) = match file.load_contents(gio::Cancellable::NONE) {
        Ok(result) => result,
        Err(error) => {
            if error.matches(gio::IOErrorEnum::NotFound) == true {
                return Ok((glib::KeyFile::new(), None));
            } else {
                return Err(format!("Cannot read Quill preferences: {error}"));
            }
        }
    };
    let text = match std::str::from_utf8(&contents) {
        Ok(text) => text,
        Err(error) => {
            return Err(format!(
                "Quill preferences must contain UTF-8 text: {error}"
            ));
        }
    };
    if text.trim().is_empty() == true {
        return Err("Quill preferences are empty.".to_string());
    }
    let preferences = glib::KeyFile::new();
    match preferences.load_from_data(text, glib::KeyFileFlags::KEEP_COMMENTS) {
        Ok(()) => Ok((preferences, etag.map(|value| value.to_string()))),
        Err(error) => Err(format!("Cannot parse Quill preferences: {error}")),
    }
}

pub fn load_language() -> Result<String, String> {
    match preferences_path() {
        Ok(path) => load_language_at(&path),
        Err(error) => Err(error),
    }
}

fn load_language_at(path: &Path) -> Result<String, String> {
    let preferences = match load_preferences_at(path) {
        Ok((preferences, _)) => preferences,
        Err(error) => return Err(error),
    };
    if preferences.has_group("Transcription") == false {
        return Ok("en".to_string());
    }
    match preferences.string("Transcription", "language") {
        Ok(language) => {
            let language = language.trim();
            if language == "en" || language == "auto" {
                Ok(language.to_string())
            } else {
                Err("Transcription language must be en or auto.".to_string())
            }
        }
        Err(error) => {
            if error.matches(glib::KeyFileError::KeyNotFound) == true {
                Ok("en".to_string())
            } else {
                Err(format!("Cannot read transcription language: {error}"))
            }
        }
    }
}

pub fn save_language(language: &str) -> Result<(), String> {
    match preferences_path() {
        Ok(path) => save_language_at(&path, language),
        Err(error) => Err(error),
    }
}

fn save_language_at(path: &Path, language: &str) -> Result<(), String> {
    if language != "en" && language != "auto" {
        return Err("Transcription language must be en or auto.".to_string());
    }
    let (preferences, etag) = match load_preferences_at(path) {
        Ok(result) => result,
        Err(error) => return Err(error),
    };
    preferences.set_string("Transcription", "language", language);
    save_preferences_at(path, &preferences, etag.as_deref())
}

pub fn save_typing_speed(words_per_minute: u32) -> Result<(), String> {
    match preferences_path() {
        Ok(path) => save_typing_speed_at(&path, words_per_minute),
        Err(error) => Err(error),
    }
}

fn save_typing_speed_at(path: &Path, words_per_minute: u32) -> Result<(), String> {
    if (10..=200).contains(&words_per_minute) == false {
        return Err("Typing speed must be between 10 and 200 words per minute.".to_string());
    }
    let (preferences, etag) = match load_preferences_at(path) {
        Ok(result) => result,
        Err(error) => return Err(error),
    };
    preferences.set_integer(
        "Statistics",
        "typing_words_per_minute",
        words_per_minute as i32,
    );
    save_preferences_at(path, &preferences, etag.as_deref())
}

fn save_preferences_at(
    path: &Path,
    preferences: &glib::KeyFile,
    expected_etag: Option<&str>,
) -> Result<(), String> {
    let directory = match path.parent() {
        Some(directory) => directory,
        None => return Err("Cannot locate the Quill preferences directory.".to_string()),
    };
    match std::fs::create_dir_all(directory) {
        Ok(()) => {}
        Err(error) => {
            return Err(format!(
                "Cannot create Quill preferences directory: {error}"
            ));
        }
    }
    let file = gio::File::for_path(path);
    match file.replace_contents(
        preferences.to_data().as_bytes(),
        expected_etag,
        false,
        gio::FileCreateFlags::PRIVATE,
        gio::Cancellable::NONE,
    ) {
        Ok(_) => Ok(()),
        Err(error) => {
            if error.matches(gio::IOErrorEnum::WrongEtag) == true {
                Err("Preferences changed outside Quill. Try saving again.".to_string())
            } else {
                Err(format!("Cannot save Quill preferences: {error}"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

    struct TestDirectory {
        path: PathBuf,
    }

    impl TestDirectory {
        fn new() -> Self {
            let sequence = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "quill-preferences-{}-{sequence}",
                std::process::id()
            ));
            std::fs::create_dir(&path).unwrap();
            Self { path }
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.path).unwrap();
        }
    }

    #[test]
    fn resolves_config_path_without_changing_environment() {
        assert_eq!(
            preferences_path_from(Some("/config".into()), None).unwrap(),
            PathBuf::from("/config/quill/preferences.ini")
        );
        for configured in [None, Some("".into()), Some("relative".into())] {
            assert_eq!(
                preferences_path_from(configured, Some("/home/person".into())).unwrap(),
                PathBuf::from("/home/person/.config/quill/preferences.ini")
            );
        }
        assert!(preferences_path_from(None, None).is_err() == true);
    }

    #[test]
    fn defaults_only_when_missing_and_roundtrips_valid_speeds() {
        let directory = TestDirectory::new();
        let path = directory.path.join("quill/preferences.ini");
        assert_eq!(load_typing_speed_at(&path).unwrap(), 40);
        assert!(path.exists() == false);
        for speed in [10, 40, 200] {
            save_typing_speed_at(&path, speed).unwrap();
            assert_eq!(load_typing_speed_at(&path).unwrap(), speed);
        }
    }

    #[test]
    fn language_and_typing_speed_preserve_each_other_and_unrelated_keys() {
        let directory = TestDirectory::new();
        let path = directory.path.join("preferences.ini");
        assert_eq!(load_language_at(&path).unwrap(), "en");
        save_language_at(&path, "auto").unwrap();
        assert_eq!(load_typing_speed_at(&path).unwrap(), 40);
        std::fs::write(&path, "# Keep this comment\n[Statistics]\ntyping_words_per_minute=100\n\n[Other]\nvalue=keep\n").unwrap();
        assert_eq!(load_language_at(&path).unwrap(), "en");
        save_language_at(&path, "en").unwrap();
        assert_eq!(load_typing_speed_at(&path).unwrap(), 100);
        save_typing_speed_at(&path, 80).unwrap();
        assert_eq!(load_language_at(&path).unwrap(), "en");
        save_language_at(&path, "auto").unwrap();
        assert_eq!(load_language_at(&path).unwrap(), "auto");
        assert_eq!(load_typing_speed_at(&path).unwrap(), 80);
        let contents = std::fs::read_to_string(&path).unwrap();
        assert!(contents.contains("# Keep this comment") == true);
        assert!(contents.contains("value=keep") == true);
    }

    #[test]
    fn invalid_language_and_malformed_files_are_not_overwritten() {
        let directory = TestDirectory::new();
        let path = directory.path.join("preferences.ini");
        save_language_at(&path, "en").unwrap();
        for language in ["", "unknown", "English"] {
            assert!(save_language_at(&path, language).is_err() == true);
            assert_eq!(load_language_at(&path).unwrap(), "en");
        }
        std::fs::write(&path, "[Transcription]\nlanguage=unknown\n").unwrap();
        assert!(load_language_at(&path).is_err() == true);
        for contents in ["", "not ini"] {
            std::fs::write(&path, contents).unwrap();
            assert!(save_language_at(&path, "en").is_err() == true);
            assert!(save_typing_speed_at(&path, 80).is_err() == true);
            assert_eq!(std::fs::read_to_string(&path).unwrap(), contents);
        }
    }

    #[test]
    fn preferences_reject_concurrent_external_changes() {
        let directory = TestDirectory::new();
        let path = directory.path.join("preferences.ini");
        save_language_at(&path, "en").unwrap();
        let (preferences, etag) = load_preferences_at(&path).unwrap();
        preferences.set_string("Transcription", "language", "auto");
        std::thread::sleep(std::time::Duration::from_millis(20));
        let external = "[Transcription]\nlanguage=en\n# External edit\n";
        std::fs::write(&path, external).unwrap();
        assert!(save_preferences_at(&path, &preferences, etag.as_deref()).is_err() == true);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), external);
    }

    #[test]
    fn invalid_speed_does_not_overwrite_valid_preferences() {
        let directory = TestDirectory::new();
        let path = directory.path.join("preferences.ini");
        save_typing_speed_at(&path, 60).unwrap();
        for speed in [0, 9, 201, u32::MAX] {
            assert!(save_typing_speed_at(&path, speed).is_err() == true);
            assert_eq!(load_typing_speed_at(&path).unwrap(), 60);
        }
    }

    #[test]
    fn malformed_settings_are_visible_errors() {
        let directory = TestDirectory::new();
        let path = directory.path.join("preferences.ini");
        for contents in [
            "",
            "not ini",
            "[Statistics]\n",
            "[Statistics]\ntyping_words_per_minute=fast",
            "[Statistics]\ntyping_words_per_minute=-1",
            "[Statistics]\ntyping_words_per_minute=201",
        ] {
            std::fs::write(&path, contents).unwrap();
            assert!(load_typing_speed_at(&path).is_err() == true);
        }
        std::fs::write(&path, [0xff]).unwrap();
        assert!(load_typing_speed_at(&path).is_err() == true);
    }

    #[test]
    fn vocabulary_preserves_text_and_rejects_external_changes() {
        let directory = TestDirectory::new();
        let path = directory.path.join("keywords.txt");
        std::fs::write(&path, "Quill\r\n  exact spacing  \n\n").unwrap();
        let (original, original_etag) = load_vocabulary_at(&path).unwrap();
        assert_eq!(original, "Quill\r\n  exact spacing  \n\n");
        assert!(original_etag.is_some() == true);
        let new_etag = save_vocabulary_at(&path, "Changed\n\n", original_etag.as_deref()).unwrap();
        assert_eq!(load_vocabulary_at(&path).unwrap().0, "Changed\n\n");
        assert!(new_etag.is_some() == true);
        // A distinct modification timestamp makes the ETag conflict deterministic.
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(&path, "External edit\n").unwrap();
        let error = save_vocabulary_at(&path, "Overwrite", new_etag.as_deref()).unwrap_err();
        assert!(error.contains("changed outside Quill") == true);
        assert_eq!(load_vocabulary_at(&path).unwrap().0, "External edit\n");
    }

    #[test]
    fn vocabulary_read_errors_never_create_or_replace_files() {
        let directory = TestDirectory::new();
        let path = directory.path.join("keywords.txt");
        assert!(load_vocabulary_at(&path).is_err() == true);
        assert!(path.exists() == false);
        std::fs::write(&path, [0xff]).unwrap();
        assert!(load_vocabulary_at(&path).is_err() == true);
        assert_eq!(std::fs::read(&path).unwrap(), vec![0xff]);
    }
}
