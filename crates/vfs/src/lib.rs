use std::{collections::HashMap, ffi::OsStr, ops::Deref, path::PathBuf};

use thunderdome::{Arena, Index};

#[derive(Debug, Default)]
pub struct Vfs {
    pub files: Arena<VirtulFile>,

    /// 基于工作区的相对路径
    pub index: HashMap<PathBuf, FileID>,
}

#[derive(Debug, Default)]
pub struct VirtulFile {
    pub path: PathBuf,
    pub text: String,
}

impl VirtulFile {
    pub fn new(path: PathBuf, text: String) -> Self {
        Self { path, text }
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
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
    /// 创建一个新的 VFS，递归读取 workspace 目录下的所有 .airy 文件
    pub fn new(workspace: &PathBuf) -> Result<Self, std::io::Error> {
        let mut vfs = Self {
            files: Arena::new(),
            index: HashMap::new(),
        };

        // 使用 walkdir 递归遍历目录
        for entry in walkdir::WalkDir::new(workspace) {
            let entry = entry?;
            let path = entry.path();

            // 只处理 .airy 文件
            if path.is_file() && path.extension() == Some(OsStr::new("airy")) {
                let text = std::fs::read_to_string(path)?;
                // 计算相对于 workspace 的相对路径
                let relative_path = path
                    .strip_prefix(workspace)
                    .expect("path should be under workspace")
                    .to_path_buf();
                vfs.new_file(relative_path, text);
            }
        }

        Ok(vfs)
    }

    pub fn get_file_id_by_path(&self, path: &PathBuf) -> Option<&FileID> {
        self.index.get(path)
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
