pub mod notion_client;

// Include the Slint UI definitions and bindings
slint::include_modules!();
use std::env;
use std::fs;
use std::sync::Mutex;
use std::rc::Rc;
// Ensure NotionClient is imported correctly
use crate::notion_client::NotionClient;
use rfd::FileDialog;
use slint::{SharedString, ModelRc, VecModel};

fn count_lines(text: &str) -> usize {
    text.lines().count()
}

fn load_notion_token() -> Option<String> {
    // Try to load from environment variable
    if let Ok(token) = env::var("NOTION_TOKEN") {
        return Some(token);
    }
    
    // Try to load from a config file
    let home_dir = dirs::home_dir()?;
    let config_path = home_dir.join(".notion_token");
    fs::read_to_string(config_path).ok()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize Notion client with token if available
    let token = load_notion_token().unwrap_or_default();
    let notion_client = Rc::new(Mutex::new(NotionClient::new(token)));
    
    // Initialize UI
    let app = NotionEditor::new()?;
    
    // Set initial state
    app.set_connected(!notion_client.lock().unwrap().token.is_empty());
    app.set_document_title(SharedString::from("Notion Editor"));
    
    // Handle token setup
    {
        let editor_handle = app.as_weak();
        let client = notion_client.clone();
        
        app.on_set_token(move |token_str| {
            if let Some(editor) = editor_handle.upgrade() {
                let mut notion = client.lock().unwrap();
                notion.token = token_str.to_string();
                
                // Save token to config file for future use
                if let Some(home_dir) = dirs::home_dir() {
                    let config_path = home_dir.join(".notion_token");
                    let _ = fs::write(config_path, &notion.token);
                }
                
                editor.set_connected(!notion.token.is_empty());
            }
        });
    }
    
    // Handle database listing
    {
        let editor_handle = app.as_weak();
        let client = notion_client.clone();
        
        app.on_list_databases(move || {
            if let Some(editor) = editor_handle.upgrade() {
                let notion = client.lock().unwrap();
                
                match notion.list_databases() {
                    Ok(databases) => {
                        // Convert to format needed by UI
                        let mut db_model = Vec::new();
                        for (id, title) in databases {
                            db_model.push(DatabaseListItem {
                                id: SharedString::from(id),
                                title: SharedString::from(title),
                            });
                        }
                        
                        editor.set_databases(ModelRc::new(VecModel::from(db_model)));
                        editor.set_status_message(SharedString::from("Databases loaded successfully"));
                    },
                    Err(e) => {
                        editor.set_status_message(SharedString::from(format!("Error: {}", e)));
                    }
                }
            }
        });
    }
    
    // Handle database selection and page query
    {
        let editor_handle = app.as_weak();
        let client = notion_client.clone();
        
        app.on_select_database(move |db_id| {
            if let Some(editor) = editor_handle.upgrade() {
                let notion = client.lock().unwrap();
                
                match notion.query_database(&db_id.to_string()) {
                    Ok(pages) => {
                        // Convert to format needed by UI
                        let mut page_model = Vec::new();
                        for (id, title) in pages {
                            page_model.push(PageListItem {
                                id: SharedString::from(id),
                                title: SharedString::from(title),
                            });
                        }
                        
                        editor.set_pages(ModelRc::new(VecModel::from(page_model)));
                        editor.set_status_message(SharedString::from("Pages loaded successfully"));
                    },
                    Err(e) => {
                        editor.set_status_message(SharedString::from(format!("Error: {}", e)));
                    }
                }
            }
        });
    }
    
    // Handle page selection and content loading
    {
        let editor_handle = app.as_weak();
        let client = notion_client.clone();
        
        app.on_select_page(move |page_id| {
            if let Some(editor) = editor_handle.upgrade() {
                let mut notion = client.lock().unwrap();
                notion.current_page_id = page_id.to_string();
                
                match notion.get_page_content(&page_id.to_string()) {
                    Ok(content) => {
                        editor.set_document_content(SharedString::from(content));
                        editor.set_status_message(SharedString::from("Page content loaded"));
                        
                        // Update line count
                        // let line_count = count_lines(&content);
                        let line_count = 1;
                        editor.set_line_count(line_count);
                    },
                    Err(e) => {
                        editor.set_status_message(SharedString::from(format!("Error: {}", e)));
                    }
                }
            }
        });
    }
    
    // Handle content saving
    {
        let editor_handle = app.as_weak();
        let client = notion_client.clone();
        
        app.on_save_content(move || {
            if let Some(editor) = editor_handle.upgrade() {
                let notion = client.lock().unwrap();
                let content = editor.get_document_content().to_string();
                
                if notion.current_page_id.is_empty() {
                    editor.set_status_message(SharedString::from("No page selected"));
                    return;
                }
                
                match notion.update_page_content(&notion.current_page_id, &content) {
                    Ok(_) => {
                        editor.set_status_message(SharedString::from("Content saved to Notion"));
                    },
                    Err(e) => {
                        editor.set_status_message(SharedString::from(format!("Error: {}", e)));
                    }
                }
            }
        });
    }
    
    // Handle line counting
    {
        let editor_handle = app.as_weak();
        
        app.on_count_lines(move || {
            if let Some(editor) = editor_handle.upgrade() {
                let content = editor.get_document_content().to_string();
                let line_count = count_lines(&content);
                editor.set_line_count(line_count as i32);
            }
        });
    }
    
    // Also support local file operations
    {
        let editor_handle = app.as_weak();
        
        app.on_open_file(move || {
            if let Some(editor) = editor_handle.upgrade() {
                if let Some(path_buf) = FileDialog::new().pick_file() {
                    if let Ok(content) = fs::read_to_string(&path_buf) {
                        let content_clone = content.clone();
                        editor.set_document_content(SharedString::from(content_clone));
                        
                        // Update the title to the file name
                        if let Some(name) = path_buf.file_name()
                            .and_then(|n| n.to_str().map(|s| s.to_string()))
                        {
                            editor.set_document_title(SharedString::from(name));
                        }
                        
                        // Update line count
                        let line_count = count_lines(&content);
                        editor.set_line_count(line_count as i32);
                    }
                }
            }
        });
    }
    
    {
        let editor_handle = app.as_weak();
        
        app.on_save_file(move || {
            if let Some(editor) = editor_handle.upgrade() {
                if let Some(path_buf) = FileDialog::new().save_file() {
                    let content = editor.get_document_content().to_string();
                    if let Err(e) = fs::write(&path_buf, content) {
                        editor.set_status_message(SharedString::from(format!("Error saving file: {}", e)));
                    } else {
                        // Update the title to file name
                        if let Some(name) = path_buf.file_name()
                            .and_then(|n| n.to_str().map(|s| s.to_string()))
                        {
                            editor.set_document_title(SharedString::from(name));
                        }
                        editor.set_status_message(SharedString::from("File saved locally"));
                    }
                }
            }
        });
    }

    // Start the application
    app.run()?;
    Ok(())
}