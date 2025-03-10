use eframe::egui::Color32;
use log::{debug, error};
use std::sync::OnceLock;
use syntect::highlighting::{Theme, ThemeSet};
use syntect::parsing::{SyntaxReference, SyntaxSet};
use syntect::util::LinesWithEndings;

// Store the SyntaxSet and ThemeSet as statics to avoid loading them every time
static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
static THEME_SET: OnceLock<ThemeSet> = OnceLock::new();

pub struct SyntaxHighlighter;

impl SyntaxHighlighter {
    /// Get the syntax set, initializing it if necessary
    fn syntax_set() -> &'static SyntaxSet {
        SYNTAX_SET.get_or_init(|| SyntaxSet::load_defaults_newlines())
    }

    /// Get the theme set, initializing it if necessary
    fn theme_set() -> &'static ThemeSet {
        THEME_SET.get_or_init(|| ThemeSet::load_defaults())
    }

    /// Get the appropriate syntax for a given language
    fn get_syntax_for_language(language: &str) -> Option<&'static SyntaxReference> {
        let syntax_set = Self::syntax_set();
        
        // Try direct match first
        if let Some(syntax) = syntax_set.find_syntax_by_token(language) {
            return Some(syntax);
        }
        
        // Try common language aliases
        let language = match language.to_lowercase().as_str() {
            "js" => "javascript",
            "ts" => "typescript",
            "py" => "python",
            "rb" => "ruby",
            "rs" => "rust",
            "sh" => "bash",
            "c++" | "cpp" | "c" => "c++",
            "cs" => "csharp",
            _ => language,
        };
        
        syntax_set.find_syntax_by_token(language)
    }

    /// Get the current theme
    pub fn get_theme(is_dark_mode: bool) -> &'static Theme {
        let theme_set = Self::theme_set();
        if is_dark_mode {
            &theme_set.themes["base16-ocean.dark"]
        } else {
            &theme_set.themes["base16-ocean.light"] 
        }
    }

    /// Highlight a code block with the appropriate syntax
    pub fn highlight_code(
        code: &str, 
        language_name: Option<&str>,
        is_dark_mode: bool
    ) -> Vec<(String, Color32)> {
        let syntax_set = Self::syntax_set();
        let theme = Self::get_theme(is_dark_mode);
        
        // Determine the syntax to use
        let syntax = if let Some(lang) = language_name {
            Self::get_syntax_for_language(lang)
                .unwrap_or_else(|| syntax_set.find_syntax_plain_text())
        } else {
            // Try to detect the language if not specified
            syntax_set.find_syntax_by_first_line(code)
                .unwrap_or_else(|| syntax_set.find_syntax_plain_text())
        };
        
        let mut highlighter = syntect::easy::HighlightLines::new(syntax, theme);
        let mut result = Vec::new();
        
        // Process each line in the code
        for line in LinesWithEndings::from(code) {
            match highlighter.highlight_line(line, syntax_set) {
                Ok(highlights) => {
                    // Convert syntect colors to egui colors
                    for (style, text) in highlights {
                        let color = Color32::from_rgba_premultiplied(
                            style.foreground.r,
                            style.foreground.g,
                            style.foreground.b,
                            style.foreground.a,
                        );
                        result.push((text.to_string(), color));
                    }
                }
                Err(err) => {
                    error!("Syntax highlighting error: {}", err);
                    // Fall back to plain text if highlighting fails
                    result.push((line.to_string(), Color32::GRAY));
                }
            }
        }
        
        result
    }

}
