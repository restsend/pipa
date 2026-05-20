use std::fmt;

#[derive(Debug, Clone, Default)]
pub struct SourceLocation {
    pub filename: String,
    pub line: u32,
    pub column: u32,
}

impl SourceLocation {
    pub fn new(filename: impl Into<String>, line: u32, column: u32) -> Self {
        Self {
            filename: filename.into(),
            line,
            column,
        }
    }

    pub fn unknown() -> Self {
        Self {
            filename: "<anonymous>".to_string(),
            line: 0,
            column: 0,
        }
    }
}

impl fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}", self.filename, self.line, self.column)
    }
}

pub type LineNumberEntry = (u32, u32);

#[derive(Debug, Clone, Default)]
pub struct LineNumberTable {
    entries: Vec<LineNumberEntry>,
}

impl LineNumberTable {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn add_entry(&mut self, instruction_offset: u32, line_number: u32) {
        if let Some(pos) = self
            .entries
            .binary_search_by(|(off, _)| off.cmp(&instruction_offset))
            .ok()
        {
            self.entries[pos] = (instruction_offset, line_number);
        } else {
            let pos = self
                .entries
                .partition_point(|(off, _)| *off < instruction_offset);
            self.entries.insert(pos, (instruction_offset, line_number));
        }
    }

    pub fn lookup_line(&self, instruction_offset: u32) -> Option<u32> {
        match self
            .entries
            .binary_search_by(|(off, _)| off.cmp(&instruction_offset))
        {
            Ok(idx) => Some(self.entries[idx].1),
            Err(idx) if idx > 0 => Some(self.entries[idx - 1].1),
            _ => None,
        }
    }

    pub fn entries(&self) -> &[LineNumberEntry] {
        &self.entries
    }
}

#[derive(Debug, Clone)]
pub struct FrameInfo {
    pub function_name: String,
    pub location: SourceLocation,
}

impl FrameInfo {
    pub fn new(function_name: impl Into<String>, location: SourceLocation) -> Self {
        Self {
            function_name: function_name.into(),
            location,
        }
    }
}

pub struct TracebackFormatter;

impl TracebackFormatter {
    pub fn format(error_message: &str, frames: &[FrameInfo], error_type: Option<&str>) -> String {
        let mut result = String::new();

        if let Some(etype) = error_type {
            result.push_str(&format!("Uncaught {}: {}", etype, error_message));
        } else {
            result.push_str(&format!("Uncaught: {}", error_message));
        }
        result.push('\n');

        for frame in frames.iter().rev() {
            result.push_str(&format!(
                "    at {} ({}:{}:{})\n",
                frame.function_name,
                frame.location.filename,
                frame.location.line,
                frame.location.column
            ));
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line_number_table() {
        let mut table = LineNumberTable::new();

        table.add_entry(0, 1);
        table.add_entry(10, 2);
        table.add_entry(25, 5);

        assert_eq!(table.lookup_line(0), Some(1));
        assert_eq!(table.lookup_line(5), Some(1));
        assert_eq!(table.lookup_line(10), Some(2));
        assert_eq!(table.lookup_line(20), Some(2));
        assert_eq!(table.lookup_line(25), Some(5));
        assert_eq!(table.lookup_line(100), Some(5));
    }

    #[test]
    fn test_traceback_formatter() {
        let frames = vec![
            FrameInfo::new("inner", SourceLocation::new("test.js", 3, 5)),
            FrameInfo::new("outer", SourceLocation::new("test.js", 7, 2)),
            FrameInfo::new("global", SourceLocation::new("test.js", 10, 1)),
        ];

        let output = TracebackFormatter::format("x is not a function", &frames, Some("TypeError"));

        assert!(output.contains("Uncaught TypeError: x is not a function"));
        assert!(output.contains("at inner (test.js:3:5)"));
        assert!(output.contains("at outer (test.js:7:2)"));
        assert!(output.contains("at global (test.js:10:1)"));
    }
}
