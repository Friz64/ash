use std::{
    collections::HashMap,
    fs, io,
    path::{Path, PathBuf},
};
use tracing::trace;

#[derive(Default)]
pub struct _Vfs(HashMap<PathBuf, Vec<u8>>);

impl _Vfs {
    pub fn _put(&mut self, path: impl Into<PathBuf>, content: impl Into<Vec<u8>>) {
        let path = path.into();
        assert!(path.is_relative());
        self.0.insert(path, content.into());
    }

    pub fn _sync_to(&self, target: impl AsRef<Path>) -> io::Result<()> {
        let target = target.as_ref();
        fs::create_dir_all(target)?;

        fn visit(
            dir: &Path,
            base: &Path,
            files: &mut Vec<PathBuf>,
            dirs: &mut Vec<PathBuf>,
        ) -> io::Result<()> {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                let stripped = path.strip_prefix(base).unwrap();
                if path.is_dir() {
                    visit(&path, base, files, dirs)?;
                    dirs.push(stripped.into());
                } else {
                    files.push(stripped.into());
                }
            }

            Ok(())
        }

        let mut existing_files = Vec::new();
        let mut existing_dirs = Vec::new();
        visit(target, target, &mut existing_files, &mut existing_dirs)?;

        for existing_file in existing_files.into_iter() {
            if !self.0.contains_key(&existing_file) {
                let target_path = target.join(existing_file);
                trace!(?target_path, "removing file");
                fs::remove_file(target_path)?;
            }
        }

        existing_dirs.sort_by_key(|p| p.components().count());
        for existing_dir in existing_dirs.into_iter().rev() {
            if !(self.0.keys()).any(|file_path| file_path.starts_with(&existing_dir)) {
                let target_path = target.join(existing_dir);
                trace!(?target_path, "removing dir");
                fs::remove_dir(target_path)?;
            }
        }

        for (file_path, file_content) in &self.0 {
            let target_path = target.join(file_path);

            if let Some(parent) = target_path.parent() {
                trace!(?parent, "creating dir if not exists");
                fs::create_dir_all(parent)?;
            }

            trace!(?target_path, "writing file");
            fs::write(target_path, file_content)?;
        }

        Ok(())
    }
}
