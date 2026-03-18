use std::collections::HashMap;
use std::sync::Mutex;

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

struct Backend {
    client: Client,
    documents: Mutex<HashMap<Url, String>>,
}

impl Backend {
    fn new(client: Client) -> Self {
        Self {
            client,
            documents: Mutex::new(HashMap::new()),
        }
    }

    async fn diagnose(&self, uri: &Url, text: &str) {
        let diagnostics = match archflow_core::parse_dsl(text) {
            Ok(_) => vec![],
            Err(e) => {
                let (line, message) = match &e {
                    archflow_core::error::ArchflowError::ParseError { line, message } => {
                        (*line, message.clone())
                    }
                    other => (1, other.to_string()),
                };
                let line = if line > 0 { line - 1 } else { 0 };
                vec![Diagnostic {
                    range: Range {
                        start: Position::new(line as u32, 0),
                        end: Position::new(line as u32, u32::MAX),
                    },
                    severity: Some(DiagnosticSeverity::ERROR),
                    source: Some("archflow".to_string()),
                    message,
                    ..Default::default()
                }]
            }
        };

        self.client
            .publish_diagnostics(uri.clone(), diagnostics, None)
            .await;
    }

    fn get_completions(&self, text: &str, position: Position) -> Vec<CompletionItem> {
        let lines: Vec<&str> = text.lines().collect();
        let line_idx = position.line as usize;
        if line_idx >= lines.len() {
            return vec![];
        }
        let line = lines[line_idx];
        let col = position.character as usize;
        let before_cursor = if col <= line.len() {
            &line[..col]
        } else {
            line
        };
        let trimmed = before_cursor.trim();

        let mut items = vec![];

        // Top-level keywords
        if trimmed.is_empty()
            || matches!(
                trimmed,
                "t" | "ti" | "d" | "di" | "th" | "u" | "us" | "c" | "cl"
            )
        {
            for (kw, detail) in [
                ("title: ", "Set diagram title"),
                ("direction: ", "Set layout direction (TB or LR)"),
                ("theme: ", "Set theme"),
                ("use ", "Import a provider"),
                ("cluster ", "Define a cluster group"),
            ] {
                if kw.starts_with(trimmed) || trimmed.is_empty() {
                    items.push(CompletionItem {
                        label: kw.trim().to_string(),
                        kind: Some(CompletionItemKind::KEYWORD),
                        detail: Some(detail.to_string()),
                        insert_text: Some(kw.to_string()),
                        ..Default::default()
                    });
                }
            }
        }

        // After "use "
        if trimmed.starts_with("use ") && !trimmed.contains(" from ") {
            for provider in ["aws", "gcp", "k8s"] {
                items.push(CompletionItem {
                    label: provider.to_string(),
                    kind: Some(CompletionItemKind::MODULE),
                    detail: Some("Provider".to_string()),
                    ..Default::default()
                });
            }
        }

        // After "direction: "
        if trimmed.starts_with("direction:") {
            for dir in ["TB", "LR"] {
                items.push(CompletionItem {
                    label: dir.to_string(),
                    kind: Some(CompletionItemKind::ENUM_MEMBER),
                    ..Default::default()
                });
            }
        }

        // After "theme: "
        if trimmed.starts_with("theme:") {
            for t in ["default", "dark", "minimal", "ocean", "sunset", "forest"] {
                items.push(CompletionItem {
                    label: t.to_string(),
                    kind: Some(CompletionItemKind::ENUM_MEMBER),
                    ..Default::default()
                });
            }
        }

        // Provider prefix completions (e.g., "aws:" triggers icon list)
        if let Some(colon_pos) = trimmed.rfind(':') {
            let prefix = &trimmed[..colon_pos];
            // Check if prefix looks like a provider (last word before colon)
            let provider = prefix.split_whitespace().last().unwrap_or("");
            if !provider.is_empty()
                && provider
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
            {
                let icons = get_provider_icons(provider);
                for icon in icons {
                    items.push(CompletionItem {
                        label: icon.clone(),
                        kind: Some(CompletionItemKind::VALUE),
                        detail: Some(format!("{}:{}", provider, icon)),
                        ..Default::default()
                    });
                }
            }
        }

        // Cluster type completions: "cluster:aws:" or "cluster:gcp:"
        if let Some(rest) = trimmed.strip_prefix("cluster:") {
            if let Some(colon2) = rest.find(':') {
                let provider = &rest[..colon2];
                let types = get_cluster_types(provider);
                for t in types {
                    items.push(CompletionItem {
                        label: t.clone(),
                        kind: Some(CompletionItemKind::ENUM_MEMBER),
                        detail: Some(format!("Cluster type for {}", provider)),
                        ..Default::default()
                    });
                }
            }
        }

        items
    }
}

/// Get commonly known icons for a provider (hardcoded subset for completions).
/// In the future, this could load from local manifest files.
fn get_provider_icons(provider: &str) -> Vec<String> {
    match provider {
        "aws" => vec![
            "EC2",
            "RDS",
            "S3",
            "Lambda",
            "ELB",
            "CloudFront",
            "SQS",
            "SNS",
            "DynamoDB",
            "ElastiCache",
            "ECS",
            "EKS",
            "ECR",
            "IAM",
            "Cognito",
            "CloudWatch",
            "CloudFormation",
            "Route53",
            "ApiGateway",
            "Bedrock",
            "SageMaker",
            "Kinesis",
            "Redshift",
            "Athena",
            "Glue",
            "EMR",
        ],
        "gcp" => vec![
            "compute-engine",
            "cloud-sql",
            "cloud-storage",
            "cloud-run",
            "bigquery",
            "gke",
            "vertex-ai",
            "cloud-spanner",
            "alloydb",
            "looker",
            "apigee",
            "anthos",
        ],
        "k8s" => vec![
            "pod",
            "deployment",
            "service",
            "ingress",
            "stateful-set",
            "config-map",
            "secret",
            "daemon-set",
            "replica-set",
            "job",
            "cron-job",
            "namespace",
            "node",
            "persistent-volume",
            "persistent-volume-claim",
            "service-account",
        ],
        _ => vec![],
    }
    .into_iter()
    .map(|s| s.to_string())
    .collect()
}

fn get_cluster_types(provider: &str) -> Vec<String> {
    match provider {
        "aws" => vec!["region", "vpc", "subnet", "account", "cloud"],
        "gcp" => vec!["region", "vpc", "subnet", "project", "zone"],
        "k8s" => vec!["cluster", "namespace"],
        _ => vec![],
    }
    .into_iter()
    .map(|s| s.to_string())
    .collect()
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![":".to_string(), " ".to_string()]),
                    ..Default::default()
                }),
                diagnostic_provider: Some(DiagnosticServerCapabilities::Options(
                    DiagnosticOptions::default(),
                )),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Archflow LSP initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        self.documents
            .lock()
            .unwrap()
            .insert(uri.clone(), text.clone());
        self.diagnose(&uri, &text).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        if let Some(change) = params.content_changes.into_iter().last() {
            let text = change.text;
            self.documents
                .lock()
                .unwrap()
                .insert(uri.clone(), text.clone());
            self.diagnose(&uri, &text).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        self.documents.lock().unwrap().remove(&uri);
        self.client.publish_diagnostics(uri, vec![], None).await;
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = &params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let docs = self.documents.lock().unwrap();
        let text = match docs.get(uri) {
            Some(t) => t.clone(),
            None => return Ok(None),
        };
        drop(docs);

        let items = self.get_completions(&text, position);
        if items.is_empty() {
            Ok(None)
        } else {
            Ok(Some(CompletionResponse::Array(items)))
        }
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(Backend::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}
