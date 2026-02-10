// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! Browser & Automation Integrations
//!
//! Chrome, Arc, Firefox, Safari, Playwright, Puppeteer

use super::IntegrationResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// =============================================================================
// Browser Control
// =============================================================================

/// Browser tab
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserTab {
    pub id: String,
    pub window_id: String,
    pub url: String,
    pub title: String,
    pub favicon_url: Option<String>,
    pub is_active: bool,
    pub is_pinned: bool,
    pub is_muted: bool,
    pub index: u32,
}

/// Browser window
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserWindow {
    pub id: String,
    pub tabs: Vec<BrowserTab>,
    pub is_focused: bool,
    pub is_incognito: bool,
    pub bounds: Option<WindowBounds>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowBounds {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Bookmark
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bookmark {
    pub id: String,
    pub title: String,
    pub url: Option<String>,
    pub parent_id: Option<String>,
    pub is_folder: bool,
    pub children: Vec<Bookmark>,
    pub date_added: Option<String>,
}

/// History item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryItem {
    pub id: String,
    pub url: String,
    pub title: String,
    pub visit_count: u32,
    pub last_visit: String,
}

/// Browser provider trait
#[async_trait::async_trait]
pub trait BrowserProvider: Send + Sync {
    // Tab management
    async fn list_tabs(&self) -> IntegrationResult;
    async fn open_url(&self, url: &str, new_tab: bool) -> IntegrationResult;
    async fn close_tab(&self, tab_id: &str) -> IntegrationResult;
    async fn activate_tab(&self, tab_id: &str) -> IntegrationResult;
    async fn reload_tab(&self, tab_id: &str) -> IntegrationResult;
    async fn duplicate_tab(&self, tab_id: &str) -> IntegrationResult;
    async fn pin_tab(&self, tab_id: &str, pinned: bool) -> IntegrationResult;
    async fn mute_tab(&self, tab_id: &str, muted: bool) -> IntegrationResult;

    // Window management
    async fn list_windows(&self) -> IntegrationResult;
    async fn create_window(&self, url: Option<&str>, incognito: bool) -> IntegrationResult;
    async fn close_window(&self, window_id: &str) -> IntegrationResult;

    // Bookmarks
    async fn get_bookmarks(&self) -> IntegrationResult;
    async fn create_bookmark(
        &self,
        title: &str,
        url: &str,
        folder_id: Option<&str>,
    ) -> IntegrationResult;
    async fn delete_bookmark(&self, bookmark_id: &str) -> IntegrationResult;

    // History
    async fn get_history(&self, max_results: u32) -> IntegrationResult;
    async fn search_history(&self, query: &str, max_results: u32) -> IntegrationResult;
    async fn delete_history(&self, url: &str) -> IntegrationResult;

    // Page interaction
    async fn get_page_content(&self, tab_id: &str) -> IntegrationResult;
    async fn execute_script(&self, tab_id: &str, script: &str) -> IntegrationResult;
    async fn take_screenshot(&self, tab_id: &str) -> IntegrationResult;
}

// =============================================================================
// Arc Browser Specific
// =============================================================================

/// Arc space
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArcSpace {
    pub id: String,
    pub name: String,
    pub color: String,
    pub icon: Option<String>,
    pub tabs: Vec<BrowserTab>,
}

/// Arc browser provider trait
#[async_trait::async_trait]
pub trait ArcProvider: BrowserProvider {
    async fn list_spaces(&self) -> IntegrationResult;
    async fn create_space(&self, name: &str, color: &str) -> IntegrationResult;
    async fn switch_space(&self, space_id: &str) -> IntegrationResult;
    async fn move_tab_to_space(&self, tab_id: &str, space_id: &str) -> IntegrationResult;
    async fn get_little_arc_tabs(&self) -> IntegrationResult;
}

// =============================================================================
// Browser Automation
// =============================================================================

/// Page element
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageElement {
    pub selector: String,
    pub tag_name: String,
    pub text: Option<String>,
    pub attributes: HashMap<String, String>,
    pub is_visible: bool,
    pub bounding_box: Option<BoundingBox>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundingBox {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Automation action
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AutomationAction {
    Navigate {
        url: String,
    },
    Click {
        selector: String,
    },
    DoubleClick {
        selector: String,
    },
    RightClick {
        selector: String,
    },
    Type {
        selector: String,
        text: String,
    },
    Clear {
        selector: String,
    },
    Select {
        selector: String,
        value: String,
    },
    Check {
        selector: String,
        checked: bool,
    },
    Hover {
        selector: String,
    },
    Focus {
        selector: String,
    },
    Scroll {
        x: i32,
        y: i32,
    },
    ScrollTo {
        selector: String,
    },
    WaitFor {
        selector: String,
        timeout_ms: u64,
    },
    WaitForNavigation {
        timeout_ms: u64,
    },
    Screenshot {
        path: String,
        full_page: bool,
    },
    Pdf {
        path: String,
    },
    Evaluate {
        script: String,
    },
    SetViewport {
        width: u32,
        height: u32,
    },
    SetCookie {
        name: String,
        value: String,
        domain: String,
    },
    Delay {
        ms: u64,
    },
}

/// Automation script
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationScript {
    pub name: String,
    pub description: Option<String>,
    pub start_url: String,
    pub actions: Vec<AutomationAction>,
    pub headless: bool,
    pub viewport: Option<(u32, u32)>,
    pub timeout_ms: u64,
}

/// Browser automation provider trait (Playwright/Puppeteer)
#[async_trait::async_trait]
pub trait AutomationProvider: Send + Sync {
    // Session management
    async fn create_session(&self, headless: bool) -> IntegrationResult;
    async fn close_session(&self, session_id: &str) -> IntegrationResult;

    // Navigation
    async fn navigate(&self, session_id: &str, url: &str) -> IntegrationResult;
    async fn go_back(&self, session_id: &str) -> IntegrationResult;
    async fn go_forward(&self, session_id: &str) -> IntegrationResult;
    async fn reload(&self, session_id: &str) -> IntegrationResult;

    // Interaction
    async fn click(&self, session_id: &str, selector: &str) -> IntegrationResult;
    async fn type_text(&self, session_id: &str, selector: &str, text: &str) -> IntegrationResult;
    async fn fill(&self, session_id: &str, selector: &str, value: &str) -> IntegrationResult;
    async fn select_option(
        &self,
        session_id: &str,
        selector: &str,
        value: &str,
    ) -> IntegrationResult;
    async fn check(&self, session_id: &str, selector: &str) -> IntegrationResult;
    async fn uncheck(&self, session_id: &str, selector: &str) -> IntegrationResult;

    // Waiting
    async fn wait_for_selector(
        &self,
        session_id: &str,
        selector: &str,
        timeout_ms: u64,
    ) -> IntegrationResult;
    async fn wait_for_navigation(&self, session_id: &str, timeout_ms: u64) -> IntegrationResult;

    // Extraction
    async fn get_text(&self, session_id: &str, selector: &str) -> IntegrationResult;
    async fn get_attribute(
        &self,
        session_id: &str,
        selector: &str,
        attribute: &str,
    ) -> IntegrationResult;
    async fn get_html(&self, session_id: &str, selector: Option<&str>) -> IntegrationResult;
    async fn query_selector_all(&self, session_id: &str, selector: &str) -> IntegrationResult;

    // Capture
    async fn screenshot(&self, session_id: &str, path: &str, full_page: bool) -> IntegrationResult;
    async fn pdf(&self, session_id: &str, path: &str) -> IntegrationResult;

    // Scripts
    async fn run_script(&self, script: &AutomationScript) -> IntegrationResult;
    async fn evaluate(&self, session_id: &str, script: &str) -> IntegrationResult;
}

// =============================================================================
// Web Scraping
// =============================================================================

/// Scrape configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrapeConfig {
    pub url: String,
    pub selectors: HashMap<String, String>,
    pub wait_for: Option<String>,
    pub pagination: Option<PaginationConfig>,
    pub headers: Option<HashMap<String, String>>,
    pub cookies: Option<Vec<Cookie>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationConfig {
    pub next_selector: String,
    pub max_pages: u32,
    pub delay_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cookie {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: Option<String>,
    pub expires: Option<i64>,
    pub http_only: Option<bool>,
    pub secure: Option<bool>,
}

/// Scrape result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrapeResult {
    pub url: String,
    pub data: HashMap<String, Vec<String>>,
    pub page_title: String,
    pub scraped_at: String,
}

/// Scraping provider trait
#[async_trait::async_trait]
pub trait ScrapingProvider: Send + Sync {
    async fn scrape(&self, config: &ScrapeConfig) -> IntegrationResult;
    async fn scrape_text(&self, url: &str) -> IntegrationResult;
    async fn scrape_links(&self, url: &str) -> IntegrationResult;
    async fn scrape_images(&self, url: &str) -> IntegrationResult;
    async fn scrape_tables(&self, url: &str) -> IntegrationResult;
}

// =============================================================================
// Reading & Later
// =============================================================================

/// Saved article
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedArticle {
    pub id: String,
    pub url: String,
    pub title: String,
    pub excerpt: Option<String>,
    pub content: Option<String>,
    pub author: Option<String>,
    pub published_at: Option<String>,
    pub saved_at: String,
    pub reading_time: Option<u32>,
    pub is_read: bool,
    pub is_favorited: bool,
    pub tags: Vec<String>,
}

/// Read later provider trait (Pocket, Instapaper, Readwise)
#[async_trait::async_trait]
pub trait ReadLaterProvider: Send + Sync {
    async fn save_url(
        &self,
        url: &str,
        title: Option<&str>,
        tags: Option<&[String]>,
    ) -> IntegrationResult;
    async fn list_articles(&self, unread_only: bool, limit: u32) -> IntegrationResult;
    async fn get_article(&self, article_id: &str) -> IntegrationResult;
    async fn archive_article(&self, article_id: &str) -> IntegrationResult;
    async fn delete_article(&self, article_id: &str) -> IntegrationResult;
    async fn favorite_article(&self, article_id: &str, favorite: bool) -> IntegrationResult;
    async fn tag_article(&self, article_id: &str, tags: &[String]) -> IntegrationResult;
}
