#![allow(unused_assignments)]

mod recursive_type;

use std::{collections::HashMap, fmt::Debug};

use vfs::FileID;

use crate::{error::AnalyzeError, module::Module};

pub use recursive_type::RecursiveTypeChecker;

/// Project 级别的检查
pub trait ProjectChecker: Send + Sync + Debug {
    fn check_project(
        &mut self,
        modules: &HashMap<FileID, Module>,
    ) -> HashMap<FileID, Vec<AnalyzeError>>;
}
