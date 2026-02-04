use dashmap::DashMap;
use tower_lsp_server::jsonrpc::Result;
use tower_lsp_server::ls_types::*;
use tower_lsp_server::{Client, LanguageServer};

use crate::document::Document;

/// Airyc Language Server
#[derive(Debug)]
pub struct Backend {
    /// LSP 客户端连接
    client: Client,
    /// 文档管理器
    documents: DashMap<Uri, Document>,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: DashMap::new(),
        }
    }
}

impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: "airyc-language-server".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
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
                ..Default::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Airyc Language Server initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;

        self.client
            .log_message(
                MessageType::INFO,
                format!("Document opened: {}", uri.as_str()),
            )
            .await;

        // 创建文档
        let document = Document::new(text);
        self.documents.insert(uri, document);

        // TODO: 发布诊断信息
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;

        self.client
            .log_message(
                MessageType::INFO,
                format!("Document changed: {}", uri.as_str()),
            )
            .await;

        if let Some(change) = params.content_changes.first() {
            // 更新文档内容
            if let Some(mut doc) = self.documents.get_mut(&uri) {
                doc.update(change.text.clone());
            }

            //TODO: 重新发布诊断信息
        }
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let uri = params.text_document.uri;
        self.client
            .log_message(
                MessageType::INFO,
                format!("Document saved: {}", uri.as_str()),
            )
            .await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        self.documents.remove(&uri);

        self.client
            .log_message(
                MessageType::INFO,
                format!("Document closed: {}", uri.as_str()),
            )
            .await;
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let uri = params.text_document.uri;

        // 获取文档
        let doc = match self.documents.get(&uri) {
            Some(doc) => doc,
            None => return Ok(None),
        };

        // 计算语义 tokens
        let tokens = doc.compute_semantic_tokens();

        Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
            result_id: None,
            data: tokens,
        })))
    }

    async fn goto_definition(
        &self,
        _params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        Ok(None)
    }

    async fn references(&self, _params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        Ok(None)
    }

    async fn completion(&self, _params: CompletionParams) -> Result<Option<CompletionResponse>> {
        Ok(None)
    }

    async fn hover(&self, _params: HoverParams) -> Result<Option<Hover>> {
        Ok(None)
    }
}
