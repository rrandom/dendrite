use serde::{Deserialize, Serialize};
use serde_json;
use std::path::PathBuf;
use uuid::Uuid;

/// Core identifier of a noate
/// Private and internal in the core library
/// The ONLY stable identifier across file change
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct NoteId(pub Uuid);

impl NoteId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for NoteId {
    fn default() -> Self {
        Self(Uuid::nil())
    }
}

// Changeable name of a note
pub type NoteKey = String;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModelId(pub &'static str);

/// Core internal coordinate system (0-based)
/// Uses u32 to match LSP Position type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Point {
    pub line: u32,
    pub col: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TextRange {
    pub start: Point,
    pub end: Point,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Heading {
    pub level: u8,
    pub text: String,
    pub range: TextRange,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Block {
    pub id: String,
    pub range: TextRange,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Note {
    #[allow(private_interfaces)]
    pub id: NoteId,
    pub vault_name: String,
    pub path: Option<PathBuf>,
    pub title: Option<String>,
    #[serde(with = "frontmatter_serde")]
    pub frontmatter: Option<serde_json::Value>,
    pub content_offset: u32,
    pub links: Vec<Link>,
    pub headings: Vec<Heading>,
    pub blocks: Vec<Block>,
    pub digest: Option<String>,
}
/// Link entity
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Link {
    #[allow(private_interfaces)]
    pub target: NoteId,
    pub raw_target: String,
    pub alias: Option<String>,
    pub anchor: Option<String>,
    pub range: TextRange,
    pub kind: LinkKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum WikiLinkFormat {
    #[default]
    AliasFirst, // [[alias|target]]
    TargetFirst, // [[target|alias]]
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LinkKind {
    WikiLink(WikiLinkFormat),
    EmbeddedWikiLink(WikiLinkFormat),
    MarkdownLink,  // [label](target)
    MarkdownImage, // ![alt](target)
    AutoLink,      // <http://example.com>
}

impl Default for LinkKind {
    fn default() -> Self {
        Self::WikiLink(WikiLinkFormat::default())
    }
}

/// Reference to a note for tree view
/// Used for serialization in LSP protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteRef {
    /// Note ID as string (UUID)
    pub id: String,
    /// Note key
    pub key: Option<NoteKey>,
    /// File path as URI string
    pub path: Option<String>,
    /// Vault name
    pub vault_name: Option<String>,
    /// Note title
    pub title: Option<String>,
}

/// Tree view structure for hierarchy display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeView {
    pub note: NoteRef,
    pub children: Vec<TreeView>,
}

mod frontmatter_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use serde_json;

    pub fn serialize<S>(value: &Option<serde_json::Value>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = match value {
            Some(v) => Some(v.to_string()),
            None => None,
        };
        s.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<serde_json::Value>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: Option<String> = Option::<String>::deserialize(deserializer)?;
        match s {
            Some(string) => serde_json::from_str(&string).map_err(serde::de::Error::custom),
            None => Ok(None),
        }
    }
}
