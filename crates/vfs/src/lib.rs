use std::{
    collections::HashMap,
    ffi::OsStr,
    ops::Deref,
    path::{Path, PathBuf},
};

use thunderdome::{Arena, Index};

#[derive(Debug, Default)]
pub struct Vfs {
    pub files: Arena<VirtulFile>,

    /// 绝对路径到 FileID 的映射
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

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct FileID(Index);

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
    /// 创建一个空的 VFS
    pub fn new() -> Self {
        Self {
            files: Arena::new(),
            index: HashMap::new(),
        }
    }

    pub fn _from_workspace(workspace: &PathBuf) -> Result<Self, std::io::Error> {
        let mut vfs = Self::new();

        // 使用 walkdir 递归遍历目录
        for entry in walkdir::WalkDir::new(workspace) {
            let entry = entry?;
            let path = entry.path();

            // 只处理 .airy 文件
            if path.is_file() && path.extension() == Some(OsStr::new("airy")) {
                let text = std::fs::read_to_string(path)?;
                // 使用绝对路径
                let absolute_path = path.canonicalize()?;
                vfs.new_file(absolute_path, text);
            }
        }

        Ok(vfs)
    }

    /// 按绝对路径查找文件
    pub fn get_file_id_by_path(&self, path: &Path) -> Option<FileID> {
        self.index.get(path).copied()
    }

    pub fn get_file_by_file_id(&self, id: &FileID) -> Option<&VirtulFile> {
        self.files.get(**id)
    }

    pub fn get_file_mut_by_file_id(&mut self, id: &FileID) -> Option<&mut VirtulFile> {
        self.files.get_mut(**id)
    }

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
