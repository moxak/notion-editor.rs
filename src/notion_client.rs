
use serde_json::{json, Value};

// Struct to hold Notion API state
pub struct NotionClient {
    pub token: String,
    pub base_url: String,
    pub current_page_id: String,
}

impl NotionClient {
    pub fn new(token: String) -> Self {
        NotionClient {
            token,
            base_url: "https://api.notion.com/v1".to_string(),
            current_page_id: String::new(),
        }
    }

    // Get page content from Notion API
    pub fn get_page_content(&self, page_id: &str) -> Result<String, Box<dyn std::error::Error>> {
        if self.token.is_empty() {
            return Err("No API token provided".into());
        }

        let client = reqwest::blocking::Client::new();
        let url = format!("{}/blocks/{}/children", self.base_url, page_id);
        
        let response = client.get(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Notion-Version", "2022-06-28")
            .send()?
            .text()?;

        // Parse the response to extract text content
        let parsed: Value = serde_json::from_str(&response)?;
        let mut content = String::new();

        if let Some(results) = parsed.get("results").and_then(|r| r.as_array()) {
            for block in results {
                if let Some(block_type) = block.get("type").and_then(|t| t.as_str()) {
                    if block_type == "paragraph" {
                        if let Some(paragraph) = block.get("paragraph") {
                            if let Some(text) = paragraph.get("rich_text").and_then(|t| t.as_array()) {
                                for text_part in text {
                                    if let Some(text_content) = text_part.get("plain_text").and_then(|t| t.as_str()) {
                                        content.push_str(text_content);
                                    }
                                }
                                content.push('\n');
                            }
                        }
                    }
                }
            }
        }

        Ok(content)
    }

    // Update page content
    pub fn update_page_content(&self, page_id: &str, content: &str) -> Result<(), Box<dyn std::error::Error>> {
        if self.token.is_empty() {
            return Err("No API token provided".into());
        }

        // First, archive existing blocks
        let client = reqwest::blocking::Client::new();
        let url = format!("{}/blocks/{}/children", self.base_url, page_id);
        
        let response = client.get(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Notion-Version", "2022-06-28")
            .send()?
            .text()?;

        let parsed: Value = serde_json::from_str(&response)?;
        
        if let Some(results) = parsed.get("results").and_then(|r| r.as_array()) {
            for block in results {
                if let Some(block_id) = block.get("id").and_then(|i| i.as_str()) {
                    // Archive each block
                    let block_url = format!("{}/blocks/{}", self.base_url, block_id);
                    let _archive_response = client.patch(&block_url)
                        .header("Authorization", format!("Bearer {}", self.token))
                        .header("Notion-Version", "2022-06-28")
                        .json(&json!({ "archived": true }))
                        .send()?;
                }
            }
        }

        // Parse the content into paragraphs
        let paragraphs: Vec<&str> = content.split('\n').collect();
        
        // Build the request body
        let mut blocks = Vec::new();
        for paragraph in paragraphs {
            if !paragraph.is_empty() {
                let block = json!({
                    "object": "block",
                    "type": "paragraph",
                    "paragraph": {
                        "rich_text": [{
                            "type": "text",
                            "text": {
                                "content": paragraph
                            }
                        }]
                    }
                });
                blocks.push(block);
            }
        }

        let request_body = json!({
            "children": blocks
        });

        // Send the request to add new content
        let _response = client.patch(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Notion-Version", "2022-06-28")
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()?;

        Ok(())
    }

    // Get a list of databases
    pub fn list_databases(&self) -> Result<Vec<(String, String)>, Box<dyn std::error::Error>> {
        if self.token.is_empty() {
            return Err("No API token provided".into());
        }

        let client = reqwest::blocking::Client::new();
        let url = format!("{}/search", self.base_url);
        
        let request_body = json!({
            "filter": {
                "value": "database",
                "property": "object"
            }
        });
        
        let response = client.post(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Notion-Version", "2022-06-28")
            .json(&request_body)
            .send()?
            .text()?;

        // Parse the response to extract database IDs and titles
        let parsed: Value = serde_json::from_str(&response)?;
        let mut databases = Vec::new();

        if let Some(results) = parsed.get("results").and_then(|r| r.as_array()) {
            for db in results {
                if let Some(id) = db.get("id").and_then(|i| i.as_str()) {
                    let title = if let Some(title_obj) = db.get("title") {
                        if let Some(title_arr) = title_obj.as_array() {
                            let mut full_title = String::new();
                            for part in title_arr {
                                if let Some(text) = part.get("plain_text").and_then(|t| t.as_str()) {
                                    full_title.push_str(text);
                                }
                            }
                            full_title
                        } else {
                            "Untitled Database".to_string()
                        }
                    } else {
                        "Untitled Database".to_string()
                    };
                    
                    databases.push((id.to_string(), title));
                }
            }
        }

        Ok(databases)
    }

    // Query items in a database
    pub fn query_database(&self, database_id: &str) -> Result<Vec<(String, String)>, Box<dyn std::error::Error>> {
        if self.token.is_empty() {
            return Err("No API token provided".into());
        }

        let client = reqwest::blocking::Client::new();
        let url = format!("{}/databases/{}/query", self.base_url, database_id);
        
        let response = client.post(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Notion-Version", "2022-06-28")
            .json(&json!({}))  // Empty query for now
            .send()?
            .text()?;

        // Parse the response to extract page IDs and titles
        let parsed: Value = serde_json::from_str(&response)?;
        let mut pages = Vec::new();

        if let Some(results) = parsed.get("results").and_then(|r| r.as_array()) {
            for page in results {
                if let Some(id) = page.get("id").and_then(|i| i.as_str()) {
                    let title = if let Some(properties) = page.get("properties") {
                        // Try to find a title property
                        let mut page_title = String::new();
                        
                        for (prop_name, prop_value) in properties.as_object().unwrap() {
                            if let Some(prop_type) = prop_value.get("type").and_then(|t| t.as_str()) {
                                if prop_type == "title" {
                                    if let Some(title_array) = prop_value.get("title").and_then(|t| t.as_array()) {
                                        for part in title_array {
                                            if let Some(text) = part.get("plain_text").and_then(|t| t.as_str()) {
                                                page_title.push_str(text);
                                            }
                                        }
                                    }
                                    break;
                                }
                            }
                        }
                        
                        if page_title.is_empty() {
                            "Untitled Page".to_string()
                        } else {
                            page_title
                        }
                    } else {
                        "Untitled Page".to_string()
                    };
                    
                    pages.push((id.to_string(), title));
                }
            }
        }

        Ok(pages)
    }
}