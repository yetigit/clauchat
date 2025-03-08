use eframe::egui::{self, Color32, RichText, TextFormat, Ui};
use log::{debug, info, error};
use std::ops::Range;

/// Support for rendering different types of message content
pub struct ChatRenderer;

impl ChatRenderer {
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
            Self::render_code_block(ui, &content[block_range.clone()], language);
            
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

    fn extract_code(text: &str, lang : &str) -> String {
        // Find the start marker position
        let start_marker = format!("```{}", lang);
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
    
    /// Render a code block with special formatting
    fn render_code_block(ui: &mut Ui, code_text: &str, language: Option<String>) {
        let code_frame = egui::Frame::new()
            .fill(Color32::from_rgb(40, 44, 52))
            .stroke(egui::Stroke::new(1.0, Color32::from_gray(100)))
            .inner_margin(egui::epaint::Marginf::same(8.0))
            .corner_radius(4.0);
            
        let lang = language.clone().unwrap_or(String::new());
        code_frame.show(ui, |ui| {
            // Display language if available
            if !lang.is_empty() {
                ui.label(RichText::new(&lang).color(Color32::from_rgb(220, 220, 170)).small());
                ui.separator();
            }
            // Display code content with monospace font
            // debug!("Code text: {}", code_text);
            let code_content = ChatRenderer::extract_code(code_text, &lang);
                
            ui.add(egui::TextEdit::multiline(&mut code_content.to_string())
                .font(egui::TextStyle::Monospace)
                .text_color(Color32::from_rgb(220, 220, 220))
                .desired_width(f32::INFINITY)
                .frame(false)
                .interactive(false));
        });
    }
}
