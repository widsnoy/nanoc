use std::path::PathBuf;

use analyzer::project::Project;
use dashmap::DashMap;
use parking_lot::RwLock;
use tower_lsp_server::jsonrpc::Result;
use tower_lsp_server::ls_types::*;
use tower_lsp_server::{Client, LanguageServer};
use vfs::{FileID, Vfs};

use crate::error::LspError;
use crate::lsp_features;

/// Airyc Language Server
#[derive(Debug)]
pub(crate) struct Backend {
    /// LSP 客户端连接
    client: Client,
    /// 项目管理器（使用 RwLock 实现内部可变性）
    project: RwLock<Project>,
    /// URI 到 FileID 的映射
    uri_to_file_id: DashMap<Uri, FileID>,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            project: RwLock::new(Project::default()),
            uri_to_file_id: DashMap::new(),
        }
    }

    /// 将 URI 转换为 FileID
    fn get_file_id(&self, uri: &Uri) -> Option<FileID> {
        self.uri_to_file_id.get(uri).map(|r| *r)
    }

    /// 使用闭包访问 LineIndex 和 Module
    fn with_module_and_line_index<F, R>(&self, uri: &Uri, f: F) -> Option<R>
    where
        F: FnOnce(&analyzer::module::Module, &tools::LineIndex) -> R,
    {
        let file_id = self.get_file_id(uri)?;
        let project = self.project.read();
        
        let module = project.modules.get(&file_id)?;
        let line_index = project.line_indexes.get(&file_id)?;
        
        Some(f(&module, &line_index))
    }

    /// 重新构建整个项目
    fn rebuild_project(&self) {
        let mut project = self.project.write();
        
        // 获取当前的 Vfs
        let vfs = std::mem::take(&mut project.vfs);
        
        // 重新初始化 Project
        *project = Project::default();
        project.full_initialize(vfs);
    }

    /// 扫描工作区目录下的所有 .airy 文件
    fn scan_workspace(&self, root_path: PathBuf) -> Vfs {
        let vfs = Vfs::default();
        
        if let Ok(entries) = std::fs::read_dir(&root_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("airy") {
                    if let Ok(text) = std::fs::read_to_string(&path) {
                        vfs.new_file(path, text);
                    }
                }
            }
        }
        
        vfs
    }
}

impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        // 初始化扫描 WorkSpace 下所有 .airy 文件
        #[allow(deprecated)]
        if let Some(root_uri) = params.root_uri {
            if let Some(root_path) = root_uri.to_file_path() {
                let vfs = self.scan_workspace(root_path.into_owned());
                
                // 构建 URI 到 FileID 的映射
                vfs.for_each_file(|file_id, file| {
                    // 将 PathBuf 转换为 file:// URI
                    let path_str = file.path.to_string_lossy();
                    let uri_str = if cfg!(windows) {
                        format!("file:///{}", path_str.replace('\\', "/"))
                    } else {
                        format!("file://{}", path_str)
                    };
                    
                    if let Ok(uri) = uri_str.parse::<Uri>() {
                        self.uri_to_file_id.insert(uri, file_id);
                    }
                });
                
                // 初始化项目
                let mut project = self.project.write();
                project.full_initialize(vfs);
            }
        }

        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: "airyc-language-server".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
                        change: Some(TextDocumentSyncKind::FULL),
                        save: Some(TextDocumentSyncSaveOptions::SaveOptions(SaveOptions {
                            include_text: Some(false),
                        })),
                        ..Default::default()
                    },
                )),
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(
                        SemanticTokensOptions {
                            legend: SemanticTokensLegend {
                                token_types: crate::lsp_features::semantic_tokens::LEGEND_TYPE
                                    .to_vec(),
                                token_modifiers:
                                    crate::lsp_features::semantic_tokens::LEGEND_MODIFIER.to_vec(),
                            },
                            full: Some(SemanticTokensFullOptions::Bool(true)),
                            range: None,
                            ..Default::default()
                        },
                    ),
                ),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                // completion_provider: Some(CompletionOptions {
                //     trigger_characters: Some(vec![".".to_string(), "->".to_string()]),
                //     all_commit_characters: None,
                //     resolve_provider: Some(false),
                //     work_done_progress_options: WorkDoneProgressOptions {
                //         work_done_progress: None,
                //     },
                //     completion_item: None,
                // }),
                ..Default::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {}

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let text = params.text_document.text;

        // 将 URI 转换为路径
        let path = match uri.to_file_path() {
            Some(p) => p.into_owned(),
            None => return,
        };

        // 检查文件是否已在 Project 中
        {
            let project = self.project.read();
            if let Some(file_id) = self.get_file_id(&uri) {
                // 文件已存在，更新内容
                project.vfs.update_file(&file_id, text);
            } else {
                // 新文件，添加到 VFS
                let file_id = project.vfs.new_file(path, text);
                self.uri_to_file_id.insert(uri.clone(), file_id);
            }
        };

        // 重新分析整个项目
        self.rebuild_project();

        // 发布诊断信息
        if let Some((diagnostics, uri_clone)) = self.with_module_and_line_index(&uri, |module, line_index| {
            // 将 SemanticError 转换为 LspError
            let errors: Vec<LspError> = module
                .semantic_errors
                .iter()
                .map(|e| LspError::Semantic(e.clone()))
                .collect();
            
            let diagnostics = lsp_features::diagnostics::compute_diagnostics(
                &errors,
                line_index,
            );
            
            (diagnostics, uri.clone())
        }) {
            self.client
                .publish_diagnostics(uri_clone, diagnostics, None)
                .await;
        }
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.clone();

        if let Some(change) = params.content_changes.first() {
            // 获取 FileID
            if let Some(file_id) = self.get_file_id(&uri) {
                // 更新文件内容
                let project = self.project.read();
                project.vfs.update_file(&file_id, change.text.clone());
                drop(project);
                
                // 重新分析整个项目
                self.rebuild_project();
            }
        }
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let uri = params.text_document.uri;
        
        if self.get_file_id(&uri).is_some() {
            // 重新分析整个项目
            self.rebuild_project();
            
            // 发布诊断信息
            if let Some((diagnostics, uri_clone)) = self.with_module_and_line_index(&uri, |module, line_index| {
                // 将 SemanticError 转换为 LspError
                let errors: Vec<LspError> = module
                    .semantic_errors
                    .iter()
                    .map(|e| LspError::Semantic(e.clone()))
                    .collect();
                
                let diagnostics = lsp_features::diagnostics::compute_diagnostics(
                    &errors,
                    line_index,
                );
                
                (diagnostics, uri.clone())
            }) {
                self.client
                    .publish_diagnostics(uri_clone, diagnostics, None)
                    .await;
            }
        }
    }

    async fn did_close(&self, _params: DidCloseTextDocumentParams) {
        // 文件关闭时不做处理，保留在 Project 中
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let uri = params.text_document.uri;

        let tokens = match self.with_module_and_line_index(&uri, |module, line_index| {
            lsp_features::semantic_tokens::compute_semantic_tokens(module, line_index)
        }) {
            Some(t) => t,
            None => return Ok(None),
        };

        Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
            result_id: None,
            data: tokens,
        })))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        Ok(self.with_module_and_line_index(&uri, |module, line_index| {
            lsp_features::goto_definition::goto_definition(
                uri.clone(),
                position,
                line_index,
                module,
            )
        }).flatten())
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;

        Ok(self.with_module_and_line_index(&uri, |module, line_index| {
            lsp_features::references::get_references(
                uri.clone(),
                position,
                line_index,
                module,
            )
        }).flatten())
    }

    async fn completion(&self, _params: CompletionParams) -> Result<Option<CompletionResponse>> {
        Ok(None)
        // let uri = params.text_document_position.text_document.uri;
        //
        // let doc = match self.documents.get(&uri) {
        //     Some(doc) => doc,
        //     None => return Ok(None),
        // };
        //
        // Ok(lsp_features::completion::completion(
        //     params.text_document_position.position,
        //     params.context,
        //     &doc.line_index,
        //     &doc.module,
        //     &doc.text,
        // ))
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        Ok(self.with_module_and_line_index(&uri, |module, line_index| {
            lsp_features::hover::hover(
                position,
                line_index,
                module,
            )
        }).flatten())
    }
}
