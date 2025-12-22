use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use serde_json;

/// Core identity identifier (e.g., "foo.bar")
/// Always normalized to Unix style in memory (using / separator, no file extension)
pub type NoteId = String;

pub type LinkId = String;

/// Core internal coordinate system (0-based)
/// Does not directly use LSP Position to avoid coupling
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Point {
    pub line: usize,
    pub col: usize,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TextRange {
    pub start: Point,
    pub end: Point,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Heading {
    /// 标题层级 (1-6)
    pub level: u8,
    
    /// 标题文本 (e.g. "Project Ideas")
    /// 去除了 # 号和空格
    pub text: String,
    
    /// 标题在文件中的位置范围
    pub range: TextRange,
    
    // 未来 V1 可能需要：
    // pub parent_heading: Option<usize>, // 用于构建树状大纲
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    /// 唯一标识 (e.g. "foo.bar")
    pub id: NoteId,

    /// 文件路径 (绝对路径)
    pub path: Option<PathBuf>,

    /// 标题 (优先来自 Frontmatter，其次是 H1，最后是文件名)
    pub title: Option<String>,

    /// 元数据 (JSON Value)
    pub frontmatter: Option<serde_json::Value>,

    /// 所有的链接 (Outlinks)
    pub links: Vec<Link>,

    /// 所有的标题 (大纲结构)
    pub headings: Vec<Heading>,
}
/// Link entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Link {
    pub source_note_id: NoteId,
    pub target_note_id: NoteId,
    pub range: TextRange,
    pub kind: LinkKind,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LinkKind {
    WikiLink,     // [[target]]
    MarkdownLink, // [label](target)
}