use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::path::PathBuf;

use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use thunderdome::{Arena, Index};
use tools::LineIndex;

/// 虚拟文件系统，支持并发访问
#[derive(Debug)]
pub struct Vfs {
    /// 内部数据，使用 RwLock 保护
    inner: RwLock<VfsInner>,
}

/// VFS 内部数据结构
#[derive(Debug)]
struct VfsInner {
    /// 文件存储
    files: Arena<VirtulFile>,
    /// 路径到文件 ID 的映射
    index: HashMap<PathBuf, FileID>,
}

impl Default for Vfs {
    fn default() -> Self {
        Self {
            inner: RwLock::new(VfsInner {
                files: Arena::new(),
                index: HashMap::new(),
            }),
        }
    }
}

#[derive(Debug)]
pub struct VirtulFile {
    /// 文件的绝对路径
    pub path: PathBuf,
    pub text: String,
    pub line_index: LineIndex,
}

impl VirtulFile {
    pub fn new(path: PathBuf, text: String) -> Self {
        let line_index = LineIndex::from_text(&text);
        Self {
            path,
            text,
            line_index,
        }
    }
}

utils::define_id_type!(FileID);

/// 只读文件引用守卫
///
/// 持有 VFS 的读锁，保证在守卫存在期间数据不会被修改
pub struct VfsFileRef<'a> {
    guard: RwLockReadGuard<'a, VfsInner>,
    index: Index,
}

impl<'a> VfsFileRef<'a> {
    fn new(guard: RwLockReadGuard<'a, VfsInner>, index: Index) -> Option<Self> {
        if guard.files.get(index).is_some() {
            Some(Self { guard, index })
        } else {
            None
        }
    }
}

impl<'a> Deref for VfsFileRef<'a> {
    type Target = VirtulFile;

    fn deref(&self) -> &Self::Target {
        self.guard.files.get(self.index).expect("Invalid FileID")
    }
}

/// 可写文件引用守卫
///
/// 持有 VFS 的写锁，保证独占访问
pub struct VfsFileMut<'a> {
    guard: RwLockWriteGuard<'a, VfsInner>,
    index: Index,
}

impl<'a> VfsFileMut<'a> {
    fn new(guard: RwLockWriteGuard<'a, VfsInner>, index: Index) -> Option<Self> {
        if guard.files.get(index).is_some() {
            Some(Self { guard, index })
        } else {
            None
        }
    }
}

impl<'a> Deref for VfsFileMut<'a> {
    type Target = VirtulFile;

    fn deref(&self) -> &Self::Target {
        self.guard.files.get(self.index).expect("Invalid FileID")
    }
}

impl<'a> DerefMut for VfsFileMut<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.guard
            .files
            .get_mut(self.index)
            .expect("Invalid FileID")
    }
}

impl Vfs {
    /// 根据路径获取文件 ID
    pub fn get_file_id_by_path(&self, path: &PathBuf) -> Option<FileID> {
        let inner = self.inner.read();
        inner.index.get(path).copied()
    }

    /// 获取文件的只读引用
    pub fn get_file_by_file_id(&self, id: &FileID) -> Option<VfsFileRef<'_>> {
        let guard = self.inner.read();
        VfsFileRef::new(guard, **id)
    }

    /// 获取文件的可写引用
    pub fn get_file_mut_by_file_id(&self, id: &FileID) -> Option<VfsFileMut<'_>> {
        let guard = self.inner.write();
        VfsFileMut::new(guard, **id)
    }

    /// 原子添加文件到 VFS（使用绝对路径）
    pub fn new_file(&self, path: PathBuf, text: String) -> FileID {
        let mut inner = self.inner.write();
        let file = VirtulFile::new(path.clone(), text);
        let id = FileID(inner.files.insert(file));
        inner.index.insert(path, id);
        id
    }

    /// 原子从 VFS 中删除文件
    pub fn remove_file(&self, file_id: &FileID) -> bool {
        let mut inner = self.inner.write();
        let Some(file) = inner.files.remove(**file_id) else {
            return false;
        };
        inner.index.remove(&file.path).is_some()
    }

    /// 原子更新文件内容
    pub fn update_file(&self, file_id: &FileID, text: String) -> bool {
        let mut inner = self.inner.write();
        if let Some(file) = inner.files.get_mut(**file_id) {
            file.line_index = LineIndex::from_text(&text);
            file.text = text;
            true
        } else {
            false
        }
    }

    /// 获取所有文件 ID 的快照
    pub fn file_ids(&self) -> Vec<FileID> {
        let inner = self.inner.read();
        inner.files.iter().map(|(idx, _)| FileID(idx)).collect()
    }

    /// 使用闭包遍历所有文件
    pub fn for_each_file<F>(&self, mut f: F)
    where
        F: FnMut(FileID, &VirtulFile),
    {
        let inner = self.inner.read();
        for (idx, file) in inner.files.iter() {
            f(FileID(idx), file);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_basic_operations() {
        let vfs = Vfs::default();
        let path = PathBuf::from("/test.airy");

        // 添加文件
        let file_id = vfs.new_file(path.clone(), "content".to_string());

        // 查询文件 ID
        assert_eq!(vfs.get_file_id_by_path(&path), Some(file_id));

        // 读取文件
        let file = vfs.get_file_by_file_id(&file_id).unwrap();
        assert_eq!(file.text, "content");
        assert_eq!(file.path, path);
        drop(file); // 显式释放守卫

        // 更新文件
        assert!(vfs.update_file(&file_id, "new content".to_string()));
        let file = vfs.get_file_by_file_id(&file_id).unwrap();
        assert_eq!(file.text, "new content");
        drop(file);

        // 删除文件
        assert!(vfs.remove_file(&file_id));
        assert!(vfs.get_file_by_file_id(&file_id).is_none());
    }

    #[test]
    fn test_concurrent_reads() {
        let vfs = Arc::new(Vfs::default());
        let file_id = vfs.new_file(PathBuf::from("/test.airy"), "content".to_string());

        // 10 个并发读线程
        let handles: Vec<_> = (0..10)
            .map(|_| {
                let vfs = Arc::clone(&vfs);
                thread::spawn(move || {
                    for _ in 0..100 {
                        let file = vfs.get_file_by_file_id(&file_id).unwrap();
                        assert_eq!(file.text, "content");
                    }
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }
    }

    #[test]
    fn test_concurrent_read_write() {
        let vfs = Arc::new(Vfs::default());
        let file_id = vfs.new_file(PathBuf::from("/test.airy"), "0".to_string());

        // 5 个读线程 + 5 个写线程
        let mut handles = vec![];

        // 读线程
        for _ in 0..5 {
            let vfs = Arc::clone(&vfs);
            handles.push(thread::spawn(move || {
                for _ in 0..50 {
                    let _file = vfs.get_file_by_file_id(&file_id);
                    thread::yield_now();
                }
            }));
        }

        // 写线程
        for i in 0..5 {
            let vfs = Arc::clone(&vfs);
            handles.push(thread::spawn(move || {
                for j in 0..50 {
                    vfs.update_file(&file_id, format!("thread-{}-{}", i, j));
                    thread::yield_now();
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }
    }

    #[test]
    fn test_atomicity() {
        // 测试 new_file 的原子性：files 和 index 同时可见
        let vfs = Arc::new(Vfs::default());

        let vfs1 = Arc::clone(&vfs);
        let handle1 = thread::spawn(move || {
            for i in 0..100 {
                let p = PathBuf::from(format!("/test{}.airy", i));
                vfs1.new_file(p, format!("content{}", i));
            }
        });

        let vfs2 = Arc::clone(&vfs);
        let handle2 = thread::spawn(move || {
            for i in 0..100 {
                let p = PathBuf::from(format!("/test{}.airy", i));
                // 如果能查到 ID，那么一定能读到文件
                if let Some(id) = vfs2.get_file_id_by_path(&p) {
                    let file = vfs2.get_file_by_file_id(&id);
                    assert!(file.is_some(), "Found ID but file is None!");
                }
            }
        });

        handle1.join().unwrap();
        handle2.join().unwrap();
    }

    #[test]
    fn test_iteration() {
        let vfs = Vfs::default();
        let id1 = vfs.new_file(PathBuf::from("/a.airy"), "a".to_string());
        let id2 = vfs.new_file(PathBuf::from("/b.airy"), "b".to_string());
        let id3 = vfs.new_file(PathBuf::from("/c.airy"), "c".to_string());

        // 测试 file_ids
        let ids = vfs.file_ids();
        assert_eq!(ids.len(), 3);
        assert!(ids.contains(&id1));
        assert!(ids.contains(&id2));
        assert!(ids.contains(&id3));

        // 测试 for_each_file
        let mut count = 0;
        vfs.for_each_file(|_, _| {
            count += 1;
        });
        assert_eq!(count, 3);
    }

    #[test]
    fn test_multiple_files() {
        let vfs = Vfs::default();

        // 添加多个文件
        let id1 = vfs.new_file(PathBuf::from("/file1.airy"), "content1".to_string());
        let id2 = vfs.new_file(PathBuf::from("/file2.airy"), "content2".to_string());

        // 验证可以同时访问不同文件
        let file1 = vfs.get_file_by_file_id(&id1).unwrap();
        let file2 = vfs.get_file_by_file_id(&id2).unwrap();

        assert_eq!(file1.text, "content1");
        assert_eq!(file2.text, "content2");
    }
}
