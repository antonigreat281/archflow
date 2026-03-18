//! Archflow DSL Parser
//!
//! Grammar (strict):
//! ```text
//! diagram       = line*
//! line          = comment | use_stmt | metadata | cluster_block | edge_chain | node_decl
//! comment       = ('#' | '//') TEXT
//! use_stmt      = 'use' IDENT ('from' SOURCE)?
//! metadata      = 'title' ':' TEXT
//!               | 'direction' ':' ('TB' | 'LR')
//!               | 'theme' ':' IDENT
//! cluster_block = 'cluster' (':' IDENT ':' IDENT)? LABEL '{' line* '}'
//! edge_chain    = node_ref ('>>' node_ref)+ (':' TEXT)?
//! node_ref      = (IDENT ':' IDENT SPACE)? LABEL
//! node_decl     = node_ref   (standalone node on its own line)
//! ```

use std::collections::HashMap;

use crate::error::ArchflowError;
use crate::model::{ClusterDef, DiagramIR, EdgeDef, Metadata, NodeDef};

/// Parse an Archflow DSL string into a DiagramIR.
pub fn parse_dsl(input: &str) -> Result<DiagramIR, ArchflowError> {
    let mut parser = Parser::new(input);
    parser.parse()
}

struct Parser<'a> {
    lines: Vec<&'a str>,
    pos: usize,
    // State
    title: Option<String>,
    direction: String,
    theme: String,
    provider_sources: HashMap<String, Option<String>>,
    nodes: Vec<NodeDef>,
    node_ids: HashMap<String, usize>, // label -> index in nodes vec
    clusters: Vec<ClusterDef>,
    edges: Vec<EdgeDef>,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            lines: input.lines().collect(),
            pos: 0,
            title: None,
            direction: "TB".to_string(),
            theme: "default".to_string(),
            provider_sources: HashMap::new(),
            nodes: Vec::new(),
            node_ids: HashMap::new(),
            clusters: Vec::new(),
            edges: Vec::new(),
        }
    }

    fn parse(&mut self) -> Result<DiagramIR, ArchflowError> {
        while self.pos < self.lines.len() {
            self.parse_line()?;
        }

        if self.nodes.is_empty() {
            return Err(ArchflowError::ParseError {
                line: self.lines.len(),
                message: "Diagram must have at least one node".into(),
            });
        }

        Ok(DiagramIR {
            version: "1.0.0".to_string(),
            metadata: Metadata {
                title: self.title.clone(),
                direction: self.direction.clone(),
                theme: self.theme.clone(),
                custom_theme: None,
                provider_sources: self.provider_sources.clone(),
                node_render_modes: HashMap::new(),
            },
            nodes: self.nodes.clone(),
            clusters: self.clusters.clone(),
            edges: self.edges.clone(),
        })
    }

    fn parse_line(&mut self) -> Result<(), ArchflowError> {
        let line_num = self.pos + 1;
        let line = self.lines[self.pos];
        let trimmed = line.trim();
        self.pos += 1;

        // Empty or comment
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("//") {
            return Ok(());
        }

        // use statement
        if trimmed.starts_with("use ") || trimmed == "use" {
            return self.parse_use(trimmed, line_num);
        }

        // metadata: title, direction, theme
        if let Some(rest) = strip_prefix_ci(trimmed, "title:") {
            self.title = Some(rest.trim().to_string());
            return Ok(());
        }
        if let Some(rest) = strip_prefix_ci(trimmed, "direction:") {
            let dir = rest.trim().to_uppercase();
            if dir != "TB" && dir != "LR" {
                return Err(ArchflowError::ParseError {
                    line: line_num,
                    message: format!("Invalid direction '{}'. Must be TB or LR", rest.trim()),
                });
            }
            self.direction = dir;
            return Ok(());
        }
        if let Some(rest) = strip_prefix_ci(trimmed, "theme:") {
            self.theme = rest.trim().to_string();
            return Ok(());
        }

        // cluster block
        if trimmed.starts_with("cluster") && trimmed.ends_with('{') {
            return self.parse_cluster(trimmed, line_num);
        }

        // edge chain (contains >>)
        if trimmed.contains(">>") {
            return self.parse_edge_chain(trimmed, line_num);
        }

        // standalone node declaration
        if !trimmed.contains('{') && !trimmed.contains('}') {
            self.ensure_node(trimmed, line_num)?;
            return Ok(());
        }

        // closing brace (inside cluster parsing, shouldn't reach here)
        if trimmed == "}" {
            return Ok(());
        }

        Err(ArchflowError::ParseError {
            line: line_num,
            message: format!("Unexpected syntax: '{}'", trimmed),
        })
    }

    fn parse_use(&mut self, trimmed: &str, line_num: usize) -> Result<(), ArchflowError> {
        // use IDENT [from SOURCE]
        let rest = if trimmed.len() > 4 {
            trimmed[4..].trim()
        } else {
            ""
        };
        let (provider, source) = if let Some(from_idx) = rest.find(" from ") {
            let p = rest[..from_idx].trim();
            let s = rest[from_idx + 6..].trim();
            if s.is_empty() {
                return Err(ArchflowError::ParseError {
                    line: line_num,
                    message: "'use ... from' requires a source".into(),
                });
            }
            (p.to_string(), Some(s.to_string()))
        } else {
            (rest.to_string(), None)
        };

        if provider.is_empty()
            || !provider
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-')
        {
            return Err(ArchflowError::ParseError {
                line: line_num,
                message: format!("Invalid provider name: '{}'", provider),
            });
        }

        self.provider_sources.insert(provider, source);
        Ok(())
    }

    fn parse_cluster(&mut self, trimmed: &str, line_num: usize) -> Result<(), ArchflowError> {
        // cluster:provider:type Label {
        // cluster Label {
        let without_brace = trimmed[..trimmed.len() - 1].trim();

        let (provider, cluster_type, label) = if let Some(rest) =
            without_brace.strip_prefix("cluster:")
        {
            let first_colon = rest.find(':').ok_or_else(|| ArchflowError::ParseError {
                line: line_num,
                message: "Expected cluster:provider:type Label {".into(),
            })?;
            let provider = &rest[..first_colon];
            let after_provider = &rest[first_colon + 1..];
            let space_idx = after_provider
                .find(' ')
                .ok_or_else(|| ArchflowError::ParseError {
                    line: line_num,
                    message: "Expected cluster:provider:type Label {".into(),
                })?;
            let ctype = &after_provider[..space_idx];
            let label = after_provider[space_idx + 1..].trim();
            (
                Some(provider.to_string()),
                Some(ctype.to_string()),
                label.to_string(),
            )
        } else {
            // cluster Label
            let label = without_brace
                .strip_prefix("cluster")
                .unwrap()
                .trim()
                .to_string();
            (None, None, label)
        };

        if label.is_empty() {
            return Err(ArchflowError::ParseError {
                line: line_num,
                message: "Cluster must have a label".into(),
            });
        }

        let cluster_id = to_id(&label);
        let mut children = Vec::new();

        // Parse lines inside cluster until }
        while self.pos < self.lines.len() {
            let inner_line_num = self.pos + 1;
            let inner = self.lines[self.pos].trim();
            self.pos += 1;

            if inner == "}" {
                break;
            }
            if inner.is_empty() || inner.starts_with('#') || inner.starts_with("//") {
                continue;
            }

            // Edge chains inside clusters
            if inner.contains(">>") {
                self.parse_edge_chain(inner, inner_line_num)?;
                // Collect node IDs from the edge chain that were just added
                // (they're already in self.nodes)
                continue;
            }

            let node_id = self.ensure_node(inner, inner_line_num)?;
            children.push(node_id);
        }

        self.clusters.push(ClusterDef {
            id: cluster_id,
            label,
            children,
            provider,
            cluster_type,
            icon_svg: None,
            style: None,
        });

        Ok(())
    }

    fn parse_edge_chain(&mut self, trimmed: &str, line_num: usize) -> Result<(), ArchflowError> {
        let parts: Vec<&str> = trimmed.split(">>").collect();
        if parts.len() < 2 {
            return Err(ArchflowError::ParseError {
                line: line_num,
                message: "Edge chain requires at least two nodes".into(),
            });
        }

        for j in 0..parts.len() - 1 {
            let from_raw = parts[j].trim();
            let from_id = self.ensure_node(from_raw, line_num)?;

            let to_part = parts[j + 1].trim();

            // Last segment might have : edge_label
            let (to_raw, edge_label) = if j == parts.len() - 2 {
                parse_edge_label(to_part)
            } else {
                (to_part, None)
            };

            let to_id = self.ensure_node(to_raw, line_num)?;

            self.edges.push(EdgeDef {
                from: from_id,
                to: to_id,
                label: edge_label.map(|s| s.to_string()),
                style: None,
            });
        }

        Ok(())
    }

    /// Ensure a node exists, returning its ID. Creates it if not yet seen.
    fn ensure_node(&mut self, raw: &str, _line_num: usize) -> Result<String, ArchflowError> {
        let spec = parse_node_spec(raw);
        let id = to_id(&spec.label);

        if id.is_empty() {
            return Err(ArchflowError::ParseError {
                line: _line_num,
                message: format!("Empty node label in: '{}'", raw),
            });
        }

        if !self.node_ids.contains_key(&spec.label) {
            let idx = self.nodes.len();
            self.nodes.push(NodeDef {
                id: id.clone(),
                label: spec.label.clone(),
                provider: spec.provider,
                icon: spec.icon,
                icon_svg: None,
                style: None,
            });
            self.node_ids.insert(spec.label, idx);
        }

        Ok(id)
    }
}

// ─── Helpers ───

struct NodeSpec {
    label: String,
    provider: Option<String>,
    icon: Option<String>,
}

/// Parse "provider:icon Label" or just "Label"
fn parse_node_spec(raw: &str) -> NodeSpec {
    let trimmed = raw.trim();
    // Match pattern: lowercase_provider:PascalIcon rest_of_label
    // e.g., "aws:EC2 Web Server"
    if let Some(colon_idx) = trimmed.find(':') {
        let prefix = &trimmed[..colon_idx];
        // Provider must be lowercase alphanumeric
        if !prefix.is_empty()
            && prefix
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        {
            let after_colon = &trimmed[colon_idx + 1..];
            // Find the space that separates icon name from label
            if let Some(space_idx) = after_colon.find(' ') {
                let icon_part = &after_colon[..space_idx];
                let label = after_colon[space_idx + 1..].trim();
                if !icon_part.is_empty() && !label.is_empty() {
                    return NodeSpec {
                        label: label.to_string(),
                        provider: Some(prefix.to_string()),
                        icon: Some(icon_part.to_lowercase()),
                    };
                }
            }
            // No space: "aws:EC2" — icon is the type, label derived from it
            if !after_colon.is_empty() {
                return NodeSpec {
                    label: after_colon.to_string(),
                    provider: Some(prefix.to_string()),
                    icon: Some(after_colon.to_lowercase()),
                };
            }
        }
    }

    NodeSpec {
        label: trimmed.to_string(),
        provider: None,
        icon: None,
    }
}

/// Parse edge label from the last segment: "Node B : some label" → ("Node B", Some("some label"))
/// Must handle provider:icon prefix correctly.
fn parse_edge_label(part: &str) -> (&str, Option<&str>) {
    let trimmed = part.trim();

    // Check if there's a provider:icon prefix
    if let Some(colon_idx) = trimmed.find(':') {
        let prefix = &trimmed[..colon_idx];
        if !prefix.is_empty()
            && prefix
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        {
            // This colon is part of provider:icon, look for another colon after the label
            let after_provider = &trimmed[colon_idx + 1..];
            if let Some(space_idx) = after_provider.find(' ') {
                let rest_after_icon = &after_provider[space_idx + 1..];
                // Look for " : " in the rest (edge label separator)
                if let Some(label_colon) = rest_after_icon.find(" : ") {
                    let node_end = colon_idx + 1 + space_idx + 1 + label_colon;
                    let edge_label = rest_after_icon[label_colon + 3..].trim();
                    return (&trimmed[..node_end], Some(edge_label));
                }
            }
            // No edge label with provider prefix
            return (trimmed, None);
        }
    }

    // No provider prefix — simple " : " split
    if let Some(idx) = trimmed.find(" : ") {
        let node = trimmed[..idx].trim();
        let label = trimmed[idx + 3..].trim();
        (node, Some(label))
    } else {
        (trimmed, None)
    }
}

/// Convert label to a lowercase ID: "Web Server" → "web_server"
fn to_id(label: &str) -> String {
    label
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect::<String>()
        .split('_')
        .filter(|s| !s.is_empty())
        .collect::<Vec<&str>>()
        .join("_")
}

/// Case-insensitive prefix strip: "Title: foo" with prefix "title:" → Some(" foo")
fn strip_prefix_ci<'b>(s: &'b str, prefix: &str) -> Option<&'b str> {
    if s.len() >= prefix.len() && s[..prefix.len()].eq_ignore_ascii_case(prefix) {
        Some(&s[prefix.len()..])
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_dsl() {
        let ir = parse_dsl("title: Hello\ndirection: LR\n\nNode A >> Node B").unwrap();
        assert_eq!(ir.metadata.title, Some("Hello".to_string()));
        assert_eq!(ir.metadata.direction, "LR");
        assert_eq!(ir.nodes.len(), 2);
        assert_eq!(ir.edges.len(), 1);
        assert_eq!(ir.edges[0].from, "node_a");
        assert_eq!(ir.edges[0].to, "node_b");
    }

    #[test]
    fn test_edge_chain() {
        let ir = parse_dsl("A >> B >> C >> D").unwrap();
        assert_eq!(ir.nodes.len(), 4);
        assert_eq!(ir.edges.len(), 3);
        assert_eq!(ir.edges[0].from, "a");
        assert_eq!(ir.edges[0].to, "b");
        assert_eq!(ir.edges[2].from, "c");
        assert_eq!(ir.edges[2].to, "d");
    }

    #[test]
    fn test_edge_with_label() {
        let ir = parse_dsl("Node A >> Node B : connects to").unwrap();
        assert_eq!(ir.edges.len(), 1);
        assert_eq!(ir.edges[0].label, Some("connects to".to_string()));
    }

    #[test]
    fn test_provider_node() {
        let ir = parse_dsl("use aws\naws:EC2 Web Server >> aws:RDS Database").unwrap();
        assert_eq!(ir.nodes.len(), 2);
        assert_eq!(ir.nodes[0].provider, Some("aws".to_string()));
        assert_eq!(ir.nodes[0].icon, Some("ec2".to_string()));
        assert_eq!(ir.nodes[0].label, "Web Server");
        assert_eq!(ir.nodes[0].id, "web_server");
        assert_eq!(ir.nodes[1].provider, Some("aws".to_string()));
        assert_eq!(ir.nodes[1].icon, Some("rds".to_string()));
        assert_eq!(ir.nodes[1].label, "Database");
    }

    #[test]
    fn test_use_statement() {
        let ir = parse_dsl("use aws\nuse gcp from github:org/repo\nNode A >> Node B").unwrap();
        assert_eq!(ir.metadata.provider_sources.len(), 2);
        assert_eq!(ir.metadata.provider_sources["aws"], None);
        assert_eq!(
            ir.metadata.provider_sources["gcp"],
            Some("github:org/repo".to_string())
        );
    }

    #[test]
    fn test_use_from_url() {
        let ir = parse_dsl("use aws from https://example.com/icons\nNode A >> Node B").unwrap();
        assert_eq!(
            ir.metadata.provider_sources["aws"],
            Some("https://example.com/icons".to_string())
        );
    }

    #[test]
    fn test_cluster() {
        let ir = parse_dsl("cluster My Group {\n  Node A\n  Node B\n}\nNode A >> Node B").unwrap();
        assert_eq!(ir.clusters.len(), 1);
        assert_eq!(ir.clusters[0].label, "My Group");
        assert_eq!(ir.clusters[0].id, "my_group");
        assert_eq!(ir.clusters[0].children, vec!["node_a", "node_b"]);
    }

    #[test]
    fn test_provider_cluster() {
        let ir = parse_dsl(
            "use aws\ncluster:aws:vpc Production VPC {\n  aws:EC2 Web\n}\naws:EC2 Web >> Node B",
        )
        .unwrap();
        assert_eq!(ir.clusters.len(), 1);
        assert_eq!(ir.clusters[0].provider, Some("aws".to_string()));
        assert_eq!(ir.clusters[0].cluster_type, Some("vpc".to_string()));
        assert_eq!(ir.clusters[0].label, "Production VPC");
        assert_eq!(ir.clusters[0].children, vec!["web"]);
    }

    #[test]
    fn test_comments() {
        let ir = parse_dsl("# This is a comment\n// Another\nNode A >> Node B").unwrap();
        assert_eq!(ir.nodes.len(), 2);
    }

    #[test]
    fn test_theme() {
        let ir = parse_dsl("theme: dark\nNode A >> Node B").unwrap();
        assert_eq!(ir.metadata.theme, "dark");
    }

    #[test]
    fn test_empty_diagram_error() {
        let result = parse_dsl("title: Empty\n# nothing here");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_direction_error() {
        let result = parse_dsl("direction: XY\nNode A >> Node B");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Invalid direction"));
    }

    #[test]
    fn test_use_empty_provider_error() {
        let result = parse_dsl("use \nNode A >> Node B");
        assert!(result.is_err());
    }

    #[test]
    fn test_use_from_empty_source_error() {
        let result = parse_dsl("use aws from \nNode A >> Node B");
        assert!(result.is_err());
    }

    #[test]
    fn test_cluster_no_label_error() {
        let result = parse_dsl("cluster {\n  Node A\n}");
        assert!(result.is_err());
    }

    #[test]
    fn test_node_dedup() {
        let ir = parse_dsl("Node A >> Node B\nNode B >> Node C\nNode A >> Node C").unwrap();
        assert_eq!(ir.nodes.len(), 3);
    }

    #[test]
    fn test_edge_label_with_provider() {
        let ir = parse_dsl("use aws\naws:EC2 Web >> aws:RDS DB : SQL queries").unwrap();
        assert_eq!(ir.edges.len(), 1);
        assert_eq!(ir.edges[0].label, Some("SQL queries".to_string()));
    }

    #[test]
    fn test_standalone_node() {
        let ir = parse_dsl("My Node\nMy Node >> Other").unwrap();
        assert_eq!(ir.nodes.len(), 2);
        assert_eq!(ir.nodes[0].label, "My Node");
    }

    #[test]
    fn test_version_is_set() {
        let ir = parse_dsl("A >> B").unwrap();
        assert_eq!(ir.version, "1.0.0");
    }

    #[test]
    fn test_to_id() {
        assert_eq!(to_id("Web Server"), "web_server");
        assert_eq!(to_id("API Gateway"), "api_gateway");
        assert_eq!(to_id("  Trimmed  "), "trimmed");
        assert_eq!(to_id("node-with-dashes"), "node_with_dashes");
    }

    #[test]
    fn test_full_diagram() {
        let dsl = r#"
title: AWS Web Architecture
direction: LR
theme: default
use aws

aws:ELB Load Balancer >> aws:EC2 Web Server >> aws:RDS Database
aws:EC2 Web Server >> aws:S3 Static Assets

cluster:aws:region US East 1 {
  aws:ELB Load Balancer
  aws:EC2 Web Server
  aws:RDS Database
  aws:S3 Static Assets
}

cluster:aws:vpc Production VPC {
  aws:EC2 Web Server
  aws:RDS Database
}
"#;
        let ir = parse_dsl(dsl).unwrap();
        assert_eq!(ir.metadata.title, Some("AWS Web Architecture".to_string()));
        assert_eq!(ir.metadata.direction, "LR");
        assert_eq!(ir.nodes.len(), 4);
        assert_eq!(ir.edges.len(), 3);
        assert_eq!(ir.clusters.len(), 2);
        assert_eq!(ir.clusters[0].provider, Some("aws".to_string()));
        assert_eq!(ir.clusters[0].cluster_type, Some("region".to_string()));
        assert!(ir.metadata.provider_sources.contains_key("aws"));
    }

    #[test]
    fn test_edges_inside_cluster() {
        let dsl = r#"
cluster Backend {
  API >> Database
  API >> Cache
}
"#;
        let ir = parse_dsl(dsl).unwrap();
        assert_eq!(ir.nodes.len(), 3);
        assert_eq!(ir.edges.len(), 2);
        assert_eq!(ir.clusters.len(), 1);
    }

    #[test]
    fn test_multiple_providers() {
        let dsl = r#"
use aws
use gcp from github:my-org/icons

aws:EC2 Compute >> gcp:cloud-sql Database
"#;
        let ir = parse_dsl(dsl).unwrap();
        assert_eq!(ir.nodes.len(), 2);
        assert_eq!(ir.nodes[0].provider, Some("aws".to_string()));
        assert_eq!(ir.nodes[1].provider, Some("gcp".to_string()));
        assert_eq!(ir.metadata.provider_sources.len(), 2);
    }
}
