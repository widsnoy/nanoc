use std::path::PathBuf;

use analyzer::checker::RecursiveTypeChecker;
use analyzer::project::Project;
use dashmap::DashMap;
use parking_lot::RwLock;
use tower_lsp_server::jsonrpc::Result;
use tower_lsp_server::ls_types::*;
use tower_lsp_server::{Client, LanguageServer};
use vfs::{FileID, Vfs};

use crate::lsp_features;

/// Airyc Language Server
#[derive(Debug)]
pub(crate) struct Backend {
    /// LSP 客户端连接
    client: Client,
    /// 项目管理器（使用 RwLock 实现内部可变性）
    project: RwLock<Project>,
    /// virtul file system
    vfs: Vfs,
    /// URI 到 FileID 的映射
    uri_to_file_id: DashMap<Uri, FileID>,
    /// FileID 到 URI 的反向映射
    file_id_to_uri: DashMap<FileID, Uri>,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        let project = Project::new().with_checker::<RecursiveTypeChecker>();
        Self {
            client,
            project: RwLock::new(project),
            uri_to_file_id: DashMap::new(),
            file_id_to_uri: DashMap::new(),
            vfs: Default::default(),
        }
    }

    /// 将 URI 转换为 FileID
    fn get_file_id(&self, uri: &Uri) -> Option<FileID> {
        self.uri_to_file_id.get(uri).map(|r| *r)
    }

    /// 将 FileID 转换为 URI
    fn get_uri_by_file_id(&self, file_id: FileID) -> Option<Uri> {
        self.file_id_to_uri.get(&file_id).map(|r| r.clone())
    }

    /// 使用闭包访问 LineIndex 和 Module
    fn with_module_and_line_index<F, R>(&self, uri: &Uri, f: F) -> Option<R>
    where
        F: FnOnce(&analyzer::module::Module, &tools::LineIndex) -> R,
    {
        let file_id = self.get_file_id(uri)?;
        let project = self.project.read();

        let module = project.modules.get(&file_id)?;
        let file = self.vfs.get_file_by_file_id(&file_id)?;
        let line_index = &file.line_index;

        Some(f(module, line_index))
    }

    /// 重新构建整个项目
    fn rebuild_project(&self) {
        let mut project = self.project.write();
        project.full_initialize(&self.vfs);
    }

    /// 发布所有文件的诊断信息
    async fn publish_all_diagnostics(&self) {
        // 收集所有需要发布的诊断信息
        let diagnostics_to_publish = {
            let project = self.project.read();

            let mut result = Vec::new();

            // 遍历 uri_to_file_id 映射
            for entry in self.uri_to_file_id.iter() {
                let uri = entry.key().clone();
                let file_id = *entry.value();

                if let Some(module) = project.modules.get(&file_id)
                    && let Some(file) = self.vfs.get_file_by_file_id(&file_id)
                {
                    let diagnostics = lsp_features::diagnostics::compute_diagnostics(
                        &module.semantic_errors,
                        &file.line_index,
                    );

                    result.push((uri, diagnostics));
                }
            }

            result
        }; // project 锁在这里释放

        // 发布所有诊断信息
        for (uri, diagnostics) in diagnostics_to_publish {
            self.client
                .publish_diagnostics(uri, diagnostics, None)
                .await;
        }
    }

    /// 扫描工作区目录下的所有 .airy 文件
    fn scan_workspace(&self, root_path: PathBuf) {
        if let Ok(entries) = std::fs::read_dir(&root_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file()
                    && path.extension().and_then(|s| s.to_str()) == Some("airy")
                    && let Ok(text) = std::fs::read_to_string(&path)
                {
                    self.vfs.new_file(path, text);
                }
            }
        }
    }
}

impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        // 初始化扫描 WorkSpace 下所有 .airy 文件
        if let Some(root_uri) = params.workspace_folders
            && let Some(uri) = root_uri.first()
            && let Some(root) = uri.uri.to_file_path()
        {
            self.scan_workspace(root.to_path_buf());

            // 构建 URI 到 FileID 的映射
            self.vfs.for_each_file(|file_id, file| {
                // 将 PathBuf 转换为 file:// URI
                let path_str = file.path.to_string_lossy();
                let uri_str = if cfg!(windows) {
                    format!("file:///{}", path_str.replace('\\', "/"))
                } else {
                    format!("file://{}", path_str)
                };

                if let Ok(uri) = uri_str.parse::<Uri>() {
                    self.uri_to_file_id.insert(uri.clone(), file_id);
                    self.file_id_to_uri.insert(file_id, uri);
                }
            });

            // 初始化项目
            let mut project = self.project.write();
            project.full_initialize(&self.vfs);
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
                // semantic_tokens_provider: Some(
                //     SemanticTokensServerCapabilities::SemanticTokensOptions(
                //         SemanticTokensOptions {
                //             legend: SemanticTokensLegend {
                //                 token_types: crate::lsp_features::semantic_tokens::LEGEND_TYPE
                //                     .to_vec(),
                //                 token_modifiers:
                //                     crate::lsp_features::semantic_tokens::LEGEND_MODIFIER.to_vec(),
                //             },
                //             full: Some(SemanticTokensFullOptions::Bool(true)),
                //             range: None,
                //             ..Default::default()
                //         },
                //     ),
                // ),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                workspace_symbol_provider: Some(OneOf::Left(true)),
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

    async fn initialized(&self, _: InitializedParams) {
        // 发布所有文件的诊断信息
        self.publish_all_diagnostics().await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;

        // 将 URI 转换为路径
        let path = match uri.to_file_path() {
            Some(p) => p.into_owned(),
            None => return,
        };

        // 检查文件是否已在 Project 中
        if let Some(file_id) = self.get_file_id(&uri) {
            // 文件已存在，更新内容
            self.vfs.update_file(&file_id, text);
        } else {
            // 新文件，添加到 VFS
            let file_id = self.vfs.new_file(path, text);
            self.uri_to_file_id.insert(uri.clone(), file_id);
            self.file_id_to_uri.insert(file_id, uri.clone());
        };

        // 重新分析整个项目
        self.rebuild_project();

        // 发布诊断信息
        self.publish_all_diagnostics().await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.clone();

        if let Some(change) = params.content_changes.into_iter().next() {
            // 获取 FileID
            if let Some(file_id) = self.get_file_id(&uri) {
                // 更新文件内容
                self.vfs.update_file(&file_id, change.text);

                // 假设只改当前文件，可以只重新分析单文件
                // 但是需要新增切换 watch 的文件时候，全量分析
                self.rebuild_project();
            } else {
                self.client
                    .log_message(MessageType::ERROR, "expect {uri} in vfs")
                    .await;
            }
        }
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let uri = params.text_document.uri;

        if self.get_file_id(&uri).is_some() {
            // 重新分析整个项目
            self.rebuild_project();

            // 发布所有文件的诊断信息（因为跨文件依赖可能影响其他文件）
            self.publish_all_diagnostics().await;
        }
    }

    async fn did_close(&self, _params: DidCloseTextDocumentParams) {
        // FIXME: 文件关闭时不做处理，保留在 Project 中
    }

    async fn did_change_watched_files(&self, _params: DidChangeWatchedFilesParams) {
        // TODO
    }

    // async fn semantic_tokens_full(
    //     &self,
    //     params: SemanticTokensParams,
    // ) -> Result<Option<SemanticTokensResult>> {
    //     let uri = params.text_document.uri;
    //
    //     let tokens = match self.with_module_and_line_index(&uri, |module, line_index| {
    //         lsp_features::semantic_tokens::compute_semantic_tokens(module, line_index)
    //     }) {
    //         Some(t) => t,
    //         None => return Ok(None),
    //     };
    //
    //     Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
    //         result_id: None,
    //         data: tokens,
    //     })))
    // }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let file_id = match self.get_file_id(&uri) {
            Some(id) => id,
            None => return Ok(None),
        };

        let project = self.project.read();
        let module = match project.modules.get(&file_id) {
            Some(m) => m,
            None => return Ok(None),
        };

        Ok(lsp_features::goto_definition::goto_definition(
            uri,
            position,
            module,
            &project,
            &self.vfs,
            |file_id| self.get_uri_by_file_id(file_id),
        ))
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;

        let file_id = match self.get_file_id(&uri) {
            Some(id) => id,
            None => return Ok(None),
        };

        let project = self.project.read();
        let module = match project.modules.get(&file_id) {
            Some(m) => m,
            None => return Ok(None),
        };

        Ok(lsp_features::references::get_references(
            uri,
            position,
            module,
            &project,
            &self.vfs,
            |file_id| self.get_uri_by_file_id(file_id),
        ))
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

        Ok(self
            .with_module_and_line_index(&uri, |module, line_index| {
                lsp_features::hover::hover(position, line_index, module)
            })
            .flatten())
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let uri = params.text_document.uri;

        Ok(self
            .with_module_and_line_index(&uri, |module, line_index| {
                lsp_features::document_symbols::compute_document_symbols(module, line_index)
            })
            .flatten())
    }

    async fn symbol(
        &self,
        params: WorkspaceSymbolParams,
    ) -> Result<Option<WorkspaceSymbolResponse>> {
        let project = self.project.read();

        Ok(lsp_features::workspace_symbols::search_workspace_symbols(
            &params.query,
            &project,
            &self.vfs,
            |file_id| self.get_uri_by_file_id(file_id),
        ))
    }
}
