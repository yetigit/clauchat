use anyhow::{Context, Result};
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Define a struct to hold the pricing information for a model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPricing {
    pub model_name: String,
    pub input_cost_per_million: f64,
    pub output_cost_per_million: f64,
    pub max_prompt_tokens: usize,
    pub max_output_tokens: usize,
}

/// Fetch and parse model pricing from a markdown table at the given URL
pub async fn fetch_model_pricing(
    model_name: Option<&str>,
) -> Result<Option<HashMap<String, ModelPricing>>> {
    let url =
        "https://raw.githubusercontent.com/AgentOps-AI/tokencost/refs/heads/main/pricing_table.md";
    // Fetch the markdown content
    debug!("Fetching pricing data from {}", url);
    let response = match reqwest::get(url).await {
        Ok(response) => response,
        Err(e) => {
            debug!("Could not fetch pricing data from {}, Error: {}", url, e);
            return Ok(None);
        }
    };

    let markdown_content = response
        .text()
        .await
        .context("Failed to extract text from response")?;

    // Parse the markdown table and extract pricing information
    Ok(Some(
        parse_pricing_table(&markdown_content, model_name).unwrap(),
    ))
}

/// Parse a markdown table containing model pricing information
fn parse_pricing_table(
    markdown: &str,
    model_name: Option<&str>,
) -> Result<HashMap<String, ModelPricing>> {
    let mut models = HashMap::new();
    let mut lines = markdown.lines();
    let mut found_table_header = false;

    // Find the table header
    while let Some(line) = lines.next() {
        if line.starts_with("| Model Name") {
            found_table_header = true;
            // Skip the separator line
            let _ = lines.next();
            break;
        }
    }

    if !found_table_header {
        return Err(anyhow::anyhow!(
            "Could not find the pricing table header in the markdown"
        ));
    }

    // Process table rows
    for line in lines {
        if let Some(_model_name) = model_name {
            if !line.contains(_model_name) {
                continue;
            }
        }
        // Stop if we reach a line that's not part of the table
        if !line.starts_with('|') {
            break;
        }

        // Split the line into columns, trim whitespace and remove the pipe characters
        let columns: Vec<&str> = line
            .split('|')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        // Make sure we have enough columns
        if columns.len() >= 5 {
            // Extract the model name
            let model_name = columns[0].trim().to_string();

            // Parse the pricing information
            // Note: Handle possible variations in the formatting
            let input_cost = parse_cost(columns[1])?;
            let output_cost = parse_cost(columns[2])?;

            // Parse the token limits
            let max_prompt_tokens = parse_token_limit(columns[3])?;
            let max_output_tokens = parse_token_limit(columns[4])?;

            // Create the model pricing entry
            let pricing = ModelPricing {
                model_name: model_name.clone(),
                input_cost_per_million: input_cost,
                output_cost_per_million: output_cost,
                max_prompt_tokens,
                max_output_tokens,
            };

            // Use the model name as the key
            models.insert(model_name, pricing);
        }
    }

    if models.is_empty() {
        return Err(anyhow::anyhow!(
            "No model pricing information found in the table"
        ));
    }

    Ok(models)
}

/// Parse a cost string like "$15.00" or "15.00" to a f64
fn parse_cost(cost_str: &str) -> Result<f64> {
    // Remove the dollar sign and any other non-numeric characters except the decimal point
    let cleaned = cost_str
        .replace('$', "")
        .chars()
        .filter(|c| c.is_digit(10) || *c == '.')
        .collect::<String>();

    // Parse the string to a f64
    match cost_str.to_string().as_str() {
        "nan" | "n/a" | "unlimited" | "--" | "-" | "" => {
            return Ok(-1.0);
        }
        _ => {} // Continue with normal parsing
    }
    cleaned
        .parse::<f64>()
        .context(format!("Failed to parse cost value: {}", cost_str))
}

/// Parse a token limit string like "200k" or "200,000" to a usize
fn parse_token_limit(limit_str: &str) -> Result<usize> {
    let limit_str = limit_str.trim().to_lowercase();

    // Handle special cases
    match limit_str.as_str() {
        "nan" | "n/a" | "unlimited" | "--" | "-" | "" => {
            // Return a very large number or a specific sentinel value
            // to represent unlimited/unknown
            return Ok(usize::MAX); // Or choose another appropriate value
        }
        _ => {} // Continue with normal parsing
    }

    // Handle "k" suffix (thousands)
    if limit_str.ends_with('k') {
        let base = limit_str.trim_end_matches('k').replace(',', "");

        let base_value = base.parse::<f64>().context(format!(
            "Failed to parse token limit with k suffix: {}",
            limit_str
        ))?;

        return Ok((base_value * 1000.0) as usize);
    }

    // Handle "M" suffix (millions)
    if limit_str.ends_with('m') {
        let base = limit_str.trim_end_matches('m').replace(',', "");

        let base_value = base.parse::<f64>().context(format!(
            "Failed to parse token limit with M suffix: {}",
            limit_str
        ))?;

        return Ok((base_value * 1_000_000.0) as usize);
    }

    // Handle regular numbers with possible commas
    let cleaned = limit_str.replace(',', "");
    cleaned
        .parse::<usize>()
        .context(format!("Failed to parse token limit: {}", limit_str))
}
