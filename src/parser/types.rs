/// Represents different types of code decorations
#[derive(Debug, Clone)]
pub enum Decoration {
    Inline {
        line: usize,
        local_value: String,
        committed_value: String,
    },
    Block {
        start_line: usize,
        end_line: usize,
        local_content: String,
        committed_content: String,
    },
    Partial {
        line: usize,
        replacements: Vec<PartialReplacement>,
    },
}

/// Represents a partial replacement within a string
#[derive(Debug, Clone)]
pub struct PartialReplacement {
    pub start: usize,
    pub end: usize,
    pub local_value: String,
    pub committed_value: String,
}