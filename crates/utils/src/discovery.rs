use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use parser::parse::Parser;
use rowan::GreenNode;
use syntax::{AstNode, SyntaxNode, ast::Header};
use vfs::{FileID, Vfs};

pub use lexer::LexerError;
pub use parser::parse::ParserError;

/// 依赖发现结果
#[derive(Debug)]
pub struct DiscoveryResult {
    /// VFS（包含所有发现的文件）
    pub vfs: Vfs,
    /// 每个文件的 GreenNode
    pub green_trees: HashMap<FileID, GreenNode>,
    /// 入口文件的 FileID
    pub entry_file_id: FileID,
    /// 解析错误（按文件分组）
    pub parse_errors: HashMap<FileID, Vec<ParserError>>,
    /// 词法错误（按文件分组）
    pub lexer_errors: HashMap<FileID, Vec<LexerError>>,
}

/// 从入口文件开始，递归发现所有依赖
///
/// # 参数
/// - `entry_path`: 入口文件的路径（可以是相对路径或绝对路径）
///
/// # import 路径规则
/// - 相对于当前文件
/// - 支持 `../` 等相对路径
/// - 自动添加 `.airy` 后缀（如果没有）
///
/// # 返回
/// - `Ok(DiscoveryResult)`: 成功发现所有依赖
/// - `Err(String)`: 入口文件不存在或无法读取
pub fn discover_dependencies(entry_path: &Path) -> Result<DiscoveryResult, String> {
    // 规范化入口文件路径为绝对路径
    let entry_absolute = entry_path.canonicalize().map_err(|e| {
        format!(
            "Failed to read entry file '{}': {}",
            entry_path.display(),
            e
        )
    })?;

    // 创建 VFS 和结果容器
    let mut vfs = Vfs::new();
    let mut green_trees = HashMap::new();
    let mut parse_errors = HashMap::new();
    let mut lexer_errors = HashMap::new();

    // 递归发现依赖
    let mut visited = HashSet::new();
    let mut to_visit = vec![entry_absolute.clone()];

    while let Some(current_path) = to_visit.pop() {
        // 跳过已访问的文件
        if visited.contains(&current_path) {
            continue;
        }
        visited.insert(current_path.clone());

        // 读取文件内容
        let text = match fs::read_to_string(&current_path) {
            Ok(text) => text,
            Err(e) => {
                // 文件不存在，记录错误但继续处理其他文件
                eprintln!(
                    "Warning: Failed to read file '{}': {}",
                    current_path.display(),
                    e
                );
                continue;
            }
        };

        // 解析文件
        let parser = Parser::new(&text);
        let (green_node, p_errors, l_errors) = parser.parse();

        // 添加到 VFS
        let file_id = vfs.new_file(current_path.clone(), text);

        // 保存结果
        green_trees.insert(file_id, green_node.clone());
        if !p_errors.is_empty() {
            parse_errors.insert(file_id, p_errors);
        }
        if !l_errors.is_empty() {
            lexer_errors.insert(file_id, l_errors);
        }

        // 提取 import 语句
        let imports = extract_imports(&green_node, &current_path);
        to_visit.extend(imports);
    }

    // 4. 获取入口文件的 FileID
    let entry_file_id = vfs
        .get_file_id_by_path(&entry_absolute)
        .ok_or_else(|| "Entry file not found in VFS".to_string())?;

    Ok(DiscoveryResult {
        vfs,
        green_trees,
        entry_file_id,
        parse_errors,
        lexer_errors,
    })
}

/// 从 GreenNode 中提取所有 import 路径
///
/// # 参数
/// - `green_node`: 要分析的语法树
/// - `current_file`: 当前文件的绝对路径
///
/// # 返回
/// - 所有被 import 的文件的绝对路径列表
fn extract_imports(green_node: &GreenNode, current_file: &Path) -> Vec<PathBuf> {
    let root = SyntaxNode::new_root(green_node.clone());
    let current_dir = current_file.parent().unwrap();

    root.children()
        .flat_map(Header::cast)
        .filter_map(|header| {
            let path_node = header.path()?;
            let import_path_str = path_node.ident()?.to_string();

            // 解析相对路径
            let mut import_path = PathBuf::from(&import_path_str);

            // 自动添加 .airy 后缀（如果没有）
            if import_path.extension().is_none() {
                import_path = import_path.with_extension("airy");
            }

            // 相对于当前文件解析路径
            let resolved = current_dir.join(&import_path);

            // 规范化路径（处理 ../ 等）
            resolved.canonicalize().ok()
        })
        .collect()
}
