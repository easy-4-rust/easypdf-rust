//! MCP tool definitions and dispatch.
//!
//! Each tool maps a high-level PDF operation (read text, convert to markdown,
//! create, merge, split, extract metadata, get page count) to the
//! corresponding sub-crate API.

use serde::{Deserialize, Serialize};

use super::error::{McpError, Result};

// ---------------------------------------------------------------------------
// MCP wire types
// ---------------------------------------------------------------------------

/// A tool definition returned by `tools/list`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Unique tool name.
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// JSON Schema for the tool's input parameters.
    pub input_schema: serde_json::Value,
}

/// The result of a `tools/call` invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// Content blocks to return to the LLM.
    pub content: Vec<ContentBlock>,
    /// If `true`, the tool execution failed.
    #[serde(rename = "isError", skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

/// A single content block inside a [`ToolResult`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    /// Plain text content.
    #[serde(rename = "text")]
    Text {
        /// The text payload.
        text: String,
    },
}

// ---------------------------------------------------------------------------
// Tool definitions
// ---------------------------------------------------------------------------

/// Return the complete list of tools exposed by this MCP server.
#[must_use]
pub fn tool_definitions() -> Vec<ToolDefinition> {
    vec![
        pdf_read_text(),
        pdf_to_markdown(),
        pdf_create_text(),
        pdf_merge(),
        pdf_split(),
        pdf_metadata(),
        pdf_page_count(),
    ]
}

fn pdf_read_text() -> ToolDefinition {
    ToolDefinition {
        name: "pdf_read_text".to_string(),
        description: "Extract text content from a PDF file".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Absolute path to the PDF file"
                },
                "start_page": {
                    "type": "integer",
                    "description": "Start page (0-based, default 0)"
                },
                "end_page": {
                    "type": "integer",
                    "description": "End page (exclusive, default = last page)"
                }
            },
            "required": ["path"]
        }),
    }
}

fn pdf_to_markdown() -> ToolDefinition {
    ToolDefinition {
        name: "pdf_to_markdown".to_string(),
        description: "Convert a PDF to Markdown with table detection and structure preservation"
            .to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Absolute path to the PDF file"
                },
                "profile": {
                    "type": "string",
                    "enum": ["fast", "balanced", "thorough"],
                    "description": "Conversion profile (default: balanced)"
                }
            },
            "required": ["path"]
        }),
    }
}

fn pdf_create_text() -> ToolDefinition {
    ToolDefinition {
        name: "pdf_create_text".to_string(),
        description: "Create a new PDF containing text content".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Absolute output path for the new PDF"
                },
                "text": {
                    "type": "string",
                    "description": "Text content to write into the PDF"
                },
                "title": {
                    "type": "string",
                    "description": "Document title (optional)"
                }
            },
            "required": ["path", "text"]
        }),
    }
}

fn pdf_merge() -> ToolDefinition {
    ToolDefinition {
        name: "pdf_merge".to_string(),
        description: "Merge multiple PDF files into one".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "inputs": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Absolute paths to the input PDF files"
                },
                "output": {
                    "type": "string",
                    "description": "Absolute path for the merged output PDF"
                }
            },
            "required": ["inputs", "output"]
        }),
    }
}

fn pdf_split() -> ToolDefinition {
    ToolDefinition {
        name: "pdf_split".to_string(),
        description: "Split a PDF into individual page files".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Absolute path to the PDF file to split"
                },
                "output_dir": {
                    "type": "string",
                    "description": "Absolute path to the output directory"
                }
            },
            "required": ["path", "output_dir"]
        }),
    }
}

fn pdf_metadata() -> ToolDefinition {
    ToolDefinition {
        name: "pdf_metadata".to_string(),
        description: "Extract metadata (title, author, etc.) from a PDF file".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Absolute path to the PDF file"
                }
            },
            "required": ["path"]
        }),
    }
}

fn pdf_page_count() -> ToolDefinition {
    ToolDefinition {
        name: "pdf_page_count".to_string(),
        description: "Get the number of pages in a PDF file".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Absolute path to the PDF file"
                }
            },
            "required": ["path"]
        }),
    }
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

/// Dispatch a tool call by name, validating parameters and executing the
/// corresponding PDF operation.
///
/// # Errors
///
/// Returns `McpError::InvalidParams` if the tool name is unknown or
/// parameters are invalid. Returns `McpError::Internal` / `McpError::Pdf`
/// if the underlying PDF operation fails.
pub fn dispatch_tool(name: &str, args: &serde_json::Value) -> Result<ToolResult> {
    match name {
        "pdf_read_text" => execute_pdf_read_text(args),
        "pdf_to_markdown" => execute_pdf_to_markdown(args),
        "pdf_create_text" => execute_pdf_create_text(args),
        "pdf_merge" => execute_pdf_merge(args),
        "pdf_split" => execute_pdf_split(args),
        "pdf_metadata" => execute_pdf_metadata(args),
        "pdf_page_count" => execute_pdf_page_count(args),
        other => Err(McpError::InvalidParams(format!(
            "Unknown tool: {other}. Available tools: pdf_read_text, pdf_to_markdown, pdf_create_text, pdf_merge, pdf_split, pdf_metadata, pdf_page_count"
        ))),
    }
}

// ---------------------------------------------------------------------------
// Tool implementations
// ---------------------------------------------------------------------------

fn text_result(text: String) -> ToolResult {
    ToolResult {
        content: vec![ContentBlock::Text { text }],
        is_error: None,
    }
}

#[cfg(test)]
fn error_result(message: String) -> ToolResult {
    ToolResult {
        content: vec![ContentBlock::Text { text: message }],
        is_error: Some(true),
    }
}

/// Extract a required string field from JSON args.
fn require_string(args: &serde_json::Value, field: &str) -> Result<String> {
    args[field]
        .as_str()
        .map(String::from)
        .ok_or_else(|| McpError::invalid_params(format!("Missing or invalid '{field}' parameter")))
}

/// Validate that a path is absolute and does not contain path traversal.
fn validate_absolute_path(path: &str, label: &str) -> Result<()> {
    if path.contains("..") {
        return Err(McpError::invalid_params(format!(
            "{label} path must not contain '..' traversal"
        )));
    }
    if !std::path::Path::new(path).is_absolute() {
        return Err(McpError::invalid_params(format!(
            "{label} path must be absolute, got: {path}"
        )));
    }
    Ok(())
}

fn execute_pdf_read_text(args: &serde_json::Value) -> Result<ToolResult> {
    let path = require_string(args, "path")?;
    validate_absolute_path(&path, "input")?;

    let mut reader = easypdf_reader::PdfReader::open(&path)?;

    if let Some(start) = args["start_page"].as_u64() {
        let start = usize::try_from(start)
            .map_err(|_| McpError::invalid_params("start_page is too large"))?;
        let end = args["end_page"]
            .as_u64()
            .map(|e| {
                usize::try_from(e).map_err(|_| McpError::invalid_params("end_page is too large"))
            })
            .transpose()?
            .unwrap_or(usize::MAX);
        reader = reader.pages(start..end);
    }

    let text = reader.extract_text()?;
    Ok(text_result(text))
}

fn execute_pdf_to_markdown(args: &serde_json::Value) -> Result<ToolResult> {
    let path = require_string(args, "path")?;
    validate_absolute_path(&path, "input")?;

    let profile_str = args["profile"].as_str().unwrap_or("balanced");
    let profile = match profile_str {
        "thorough" => easypdf_markdown::MarkdownProfile::Llm,
        _ => easypdf_markdown::MarkdownProfile::Gfm,
    };

    let result = easypdf_markdown::PdfMarkdownBuilder::new(&path)
        .profile(profile)
        .do_convert()?;
    Ok(text_result(result.into_markdown()))
}

fn execute_pdf_create_text(args: &serde_json::Value) -> Result<ToolResult> {
    let path = require_string(args, "path")?;
    validate_absolute_path(&path, "output")?;
    let text = require_string(args, "text")?;

    let title = args["title"].as_str().unwrap_or("Untitled");

    let mut writer = easypdf_writer::PdfWriter::new(title);
    writer.add_page(
        easypdf_core::PageSize::A4,
        easypdf_core::Orientation::Portrait,
    )?;
    writer.write_text(&easypdf_core::PdfText::new(&text), 72.0, 700.0)?;
    writer.finish(&path)?;

    Ok(text_result(format!("PDF created at {path}")))
}

fn execute_pdf_merge(args: &serde_json::Value) -> Result<ToolResult> {
    let output = require_string(args, "output")?;
    validate_absolute_path(&output, "output")?;

    let inputs = args["inputs"]
        .as_array()
        .ok_or_else(|| McpError::invalid_params("'inputs' must be an array of paths"))?;

    if inputs.is_empty() {
        return Err(McpError::invalid_params("'inputs' must not be empty"));
    }

    let input_paths: Vec<String> = inputs
        .iter()
        .map(|v| {
            v.as_str()
                .map(String::from)
                .ok_or_else(|| McpError::invalid_params("Each input must be a string path"))
        })
        .collect::<Result<Vec<_>>>()?;

    for p in &input_paths {
        validate_absolute_path(p, "input")?;
    }

    easypdf_reader::PdfManipulator::merge_files(&input_paths, &output)?;

    Ok(text_result(format!(
        "Merged {} files into {output}",
        input_paths.len()
    )))
}

fn execute_pdf_split(args: &serde_json::Value) -> Result<ToolResult> {
    let path = require_string(args, "path")?;
    validate_absolute_path(&path, "input")?;
    let output_dir = require_string(args, "output_dir")?;
    validate_absolute_path(&output_dir, "output_dir")?;

    let manipulator = easypdf_reader::PdfManipulator::open(&path)?;
    let total_pages = manipulator.page_count();
    std::fs::create_dir_all(&output_dir)?;

    let mut output_paths = Vec::new();
    for i in 0..total_pages {
        let mut doc = manipulator.extract_pages(i..i + 1)?;
        let filename = format!("page_{:03}.pdf", i + 1);
        let out_path = std::path::Path::new(&output_dir).join(&filename);
        doc.save(&out_path)?;
        output_paths.push(out_path);
    }

    Ok(text_result(format!(
        "Split into {} files in {output_dir}: {}",
        output_paths.len(),
        output_paths
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    )))
}

fn execute_pdf_metadata(args: &serde_json::Value) -> Result<ToolResult> {
    let path = require_string(args, "path")?;
    validate_absolute_path(&path, "input")?;

    let reader = easypdf_reader::PdfReader::open(&path)?;
    let meta = reader.extract_metadata()?;
    let mut map = serde_json::Map::new();

    if let Some(title) = &meta.title {
        map.insert("title".to_string(), serde_json::json!(title));
    }
    if let Some(author) = &meta.author {
        map.insert("author".to_string(), serde_json::json!(author));
    }
    if let Some(subject) = &meta.subject {
        map.insert("subject".to_string(), serde_json::json!(subject));
    }
    if let Some(keywords) = &meta.keywords {
        map.insert("keywords".to_string(), serde_json::json!(keywords));
    }
    if let Some(creator) = &meta.creator {
        map.insert("creator".to_string(), serde_json::json!(creator));
    }
    if let Some(producer) = &meta.producer {
        map.insert("producer".to_string(), serde_json::json!(producer));
    }

    let json = serde_json::to_string_pretty(&serde_json::Value::Object(map))
        .unwrap_or_else(|_| "{}".to_string());
    Ok(text_result(json))
}

fn execute_pdf_page_count(args: &serde_json::Value) -> Result<ToolResult> {
    let path = require_string(args, "path")?;
    validate_absolute_path(&path, "input")?;

    let reader = easypdf_reader::PdfReader::open(&path)?;
    let count = reader.page_count()?;
    Ok(text_result(count.to_string()))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_count() {
        assert_eq!(tool_definitions().len(), 7);
    }

    #[test]
    fn tool_names_unique() {
        let defs = tool_definitions();
        let mut names: Vec<&str> = defs.iter().map(|d| d.name.as_str()).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), defs.len());
    }

    #[test]
    fn tool_definitions_serialize() {
        let defs = tool_definitions();
        let json = serde_json::to_string(&defs).unwrap();
        assert!(json.contains("pdf_read_text"));
        assert!(json.contains("pdf_to_markdown"));
        assert!(json.contains("pdf_create_text"));
        assert!(json.contains("pdf_merge"));
        assert!(json.contains("pdf_split"));
        assert!(json.contains("pdf_metadata"));
        assert!(json.contains("pdf_page_count"));
    }

    #[test]
    fn dispatch_unknown_tool() {
        let result = dispatch_tool("unknown_tool", &serde_json::json!({}));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, McpError::InvalidParams(_)));
        assert!(err.to_string().contains("Unknown tool"));
    }

    #[test]
    fn dispatch_missing_path() {
        let result = dispatch_tool("pdf_read_text", &serde_json::json!({}));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Missing"));
    }

    #[test]
    fn dispatch_relative_path_rejected() {
        let result = dispatch_tool(
            "pdf_read_text",
            &serde_json::json!({"path": "relative/file.pdf"}),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("absolute"));
    }

    #[test]
    fn dispatch_traversal_rejected() {
        let result = dispatch_tool(
            "pdf_read_text",
            &serde_json::json!({"path": "/tmp/../etc/passwd"}),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("traversal"));
    }

    #[test]
    fn tool_result_serialization() {
        let result = ToolResult {
            content: vec![ContentBlock::Text {
                text: "hello".to_string(),
            }],
            is_error: None,
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"text\""));
        assert!(json.contains("hello"));
        assert!(!json.contains("isError"));
    }

    #[test]
    fn tool_result_error_serialization() {
        let result = error_result("something failed".to_string());
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("isError"));
        assert!(json.contains("something failed"));
    }

    #[test]
    fn merge_empty_inputs_rejected() {
        let result = dispatch_tool(
            "pdf_merge",
            &serde_json::json!({"inputs": [], "output": "/tmp/out.pdf"}),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not be empty"));
    }

    #[test]
    fn create_text_missing_text_rejected() {
        let result = dispatch_tool(
            "pdf_create_text",
            &serde_json::json!({"path": "/tmp/out.pdf"}),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("text"));
    }
}
