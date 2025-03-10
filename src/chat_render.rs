use eframe::egui::{self, Color32, RichText, TextFormat, Ui};
use log::{debug, info, error};
use std::ops::Range;

use crate::syntax_lit::SyntaxHighlighter;

/// Support for rendering different types of message content
pub struct ChatRenderer;

impl ChatRenderer {

    /// Render highlighted code into a UI
    fn render_highlighted_code(
        ui: &mut egui::Ui,
        code: &str,
        language: Option<&str>,
        is_dark_mode: bool,
    ) {
        let highlighted = SyntaxHighlighter::highlight_code(code, language, is_dark_mode);
        
        // Determine background color based on theme
        let bg_color = if is_dark_mode {
            Color32::from_rgb(40, 44, 52)
        } else {
            Color32::from_rgb(240, 240, 240)
        };
        
        // Create a frame for the code block
        let code_frame = egui::Frame::none()
            .fill(bg_color)
            .stroke(egui::Stroke::new(1.0, Color32::from_gray(100)))
            .inner_margin(egui::epaint::Marginf::same(8.0))
            .corner_radius(4.0)
            ;
            
        code_frame.show(ui, |ui| {
            // Show language if available
            if let Some(lang) = language {
                ui.label(
                    RichText::new(lang)
                        .color(if is_dark_mode { Color32::LIGHT_GRAY } else { Color32::DARK_GRAY })
                        .small()
                );
                ui.separator();
            }
            
            // Render the highlighted code
            let mut job = egui::text::LayoutJob::default();
            
            for (text, color) in highlighted {
                let text_format = TextFormat {
                    font_id: egui::FontId::monospace(14.0),
                    color,
                    ..Default::default()
                };
                
                job.append(&text, 0.0, text_format);
            }
            
            ui.label(job);
        });
    }

    /// Renders chat message content with special formatting for code blocks
    pub fn render_message_content(ui: &mut Ui, content: &str) {
        let mut last_end = 0;
        
        // Find code blocks using markdown syntax ```
        for (block_range, language) in Self::find_code_blocks(content) {
            // Render text before code block
            if last_end < block_range.start {
                ui.label(RichText::new(&content[last_end..block_range.start]));
            }
            
            // Render code block with special formatting
            let code_content = ChatRenderer::extract_code(&content[block_range.clone()], language.as_deref());
            ChatRenderer::render_highlighted_code(ui, &code_content, language.as_deref(), true);
            last_end = block_range.end;
        }
        
        // Render remaining text after last code block
        if last_end < content.len() {
            ui.label(RichText::new(&content[last_end..]));
        }
    }
    
    /// Find code blocks in the message content
    fn find_code_blocks(content: &str) -> Vec<(Range<usize>, Option<String>)> {
        let mut blocks = Vec::new();
        let mut in_code_block = false;
        let mut start_idx = 0;
        let mut language = None;
        
        for (i, line) in content.lines().enumerate() {
            let line_start = content.lines().take(i).map(|l| l.len() + 1).sum();
            
            if line.trim().starts_with("```") {
                if !in_code_block {
                    // Start of code block
                    in_code_block = true;
                    start_idx = line_start;
                    
                    // Extract language if specified
                    let lang = line.trim().strip_prefix("```").unwrap_or("").trim();
                    language = if lang.is_empty() { None } else { Some(lang.to_string()) };
                } else {
                    // End of code block
                    in_code_block = false;
                    let end_idx = line_start + line.len() + 1;
                    blocks.push((start_idx..end_idx, language.take()));
                }
            }
        }
        
        blocks
    }

    fn extract_code(text: &str, lang : Option<&str>) -> String {
        // Find the start marker position
        let start_marker = format!("```{}", lang.unwrap_or(""));
        let start_pos = match text.find(&start_marker) {
            Some(pos) => pos + start_marker.len(),
            None => return String::new(), // Start marker not found
        };
        
        // Find the end marker position
        let end_marker = "```";
        let end_pos = match text[start_pos..].find(end_marker) {
            Some(pos) => start_pos + pos,
            None => return String::new(), // End marker not found
        };
        
        // Extract and trim the content between markers
        text[start_pos..end_pos].trim().to_string()
    }
    
}
