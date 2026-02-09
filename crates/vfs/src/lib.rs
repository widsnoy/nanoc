use std::{collections::HashMap, ops::Deref, path::PathBuf};

use thunderdome::{Arena, Index};

#[derive(Debug, Default)]
pub struct Vfs {
    pub files: Arena<VirtulFile>,

    /// 基于绝对路径的索引
    pub index: HashMap<PathBuf, FileID>,
}

#[derive(Debug, Default)]
pub struct VirtulFile {
    /// 文件的绝对路径
    pub path: PathBuf,
    pub text: String,
}

impl VirtulFile {
    pub fn new(path: PathBuf, text: String) -> Self {
        Self { path, text }
    }
}

#[derive(Default, Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct FileID(pub Index);

impl FileID {
    pub fn none() -> Self {
        Self(Index::DANGLING)
    }
}

impl Deref for FileID {
    type Target = Index;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Vfs {
    pub fn get_file_id_by_path(&self, path: &PathBuf) -> Option<&FileID> {
        self.index.get(path)
    }

    pub fn get_file_by_file_id(&self, id: &FileID) -> Option<&VirtulFile> {
        self.files.get(**id)
    }

    pub fn get_file_mut_by_file_id(&mut self, id: &FileID) -> Option<&mut VirtulFile> {
        self.files.get_mut(**id)
    }

    /// 添加文件到 VFS（使用绝对路径）
    pub fn new_file(&mut self, path: PathBuf, text: String) -> FileID {
        let file = VirtulFile::new(path.clone(), text);
        let id = FileID(self.files.insert(file));
        self.index.insert(path, id);
        id
    }

    pub fn remove_file(&mut self, file_id: &FileID) -> bool {
        let Some(file) = self.files.remove(**file_id) else {
            return false;
        };
        self.index.remove(&file.path).is_some()
    }

    pub fn update_file(&mut self, file_id: &FileID, text: String) -> bool {
        if let Some(file) = self.get_file_mut_by_file_id(file_id) {
            file.text = text;
            true
        } else {
            false
        }
    }
}
