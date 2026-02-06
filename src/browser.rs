// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: Apache-2.0
//! Browser authentication detection for web-based LLM providers
//!
//! This module reads browser cookies (without opening windows) to detect
//! which cloud LLM providers the user is authenticated with.

use anyhow::{anyhow, Context, Result};
use colored::Colorize;
use rusqlite::{Connection, OpenFlags};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

// Suppress dead code warnings for fields used in debugging
#[allow(dead_code)]
/// Supported browser types for cookie extraction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BrowserType {
    Chrome,
    Edge,
    Firefox,
    Brave,
    Vivaldi,
    Opera,
}

impl BrowserType {
    pub fn name(&self) -> &'static str {
        match self {
            BrowserType::Chrome => "Chrome",
            BrowserType::Edge => "Edge",
            BrowserType::Firefox => "Firefox",
            BrowserType::Brave => "Brave",
            BrowserType::Vivaldi => "Vivaldi",
            BrowserType::Opera => "Opera",
        }
    }

    /// Get the default profile path for this browser
    #[cfg(windows)]
    pub fn profile_path(&self) -> Option<PathBuf> {
        let local_app_data = dirs::data_local_dir()?;
        let roaming_app_data = dirs::data_dir()?;

        let path = match self {
            BrowserType::Chrome => local_app_data.join("Google/Chrome/User Data/Default"),
            BrowserType::Edge => local_app_data.join("Microsoft/Edge/User Data/Default"),
            BrowserType::Brave => {
                local_app_data.join("BraveSoftware/Brave-Browser/User Data/Default")
            }
            BrowserType::Vivaldi => local_app_data.join("Vivaldi/User Data/Default"),
            BrowserType::Opera => roaming_app_data.join("Opera Software/Opera Stable"),
            BrowserType::Firefox => {
                // Firefox uses random profile directories
                // Select the profile with the largest cookies.sqlite (most likely active)
                let profiles_dir = roaming_app_data.join("Mozilla/Firefox/Profiles");
                if profiles_dir.exists() {
                    if let Ok(entries) = fs::read_dir(&profiles_dir) {
                        let mut best_profile: Option<(PathBuf, u64)> = None;

                        for entry in entries.flatten() {
                            let profile_path = entry.path();
                            let cookies_path = profile_path.join("cookies.sqlite");

                            if cookies_path.exists() {
                                if let Ok(metadata) = fs::metadata(&cookies_path) {
                                    let size = metadata.len();
                                    if best_profile.as_ref().map_or(true, |(_, s)| size > *s) {
                                        best_profile = Some((profile_path, size));
                                    }
                                }
                            }
                        }

                        if let Some((path, _)) = best_profile {
                            return Some(path);
                        }
                    }
                }
                return None;
            }
        };

        if path.exists() {
            Some(path)
        } else {
            None
        }
    }

    #[cfg(not(windows))]
    pub fn profile_path(&self) -> Option<PathBuf> {
        let home = dirs::home_dir()?;

        let path = match self {
            BrowserType::Chrome => {
                #[cfg(target_os = "macos")]
                {
                    home.join("Library/Application Support/Google/Chrome/Default")
                }
                #[cfg(target_os = "linux")]
                {
                    home.join(".config/google-chrome/Default")
                }
            }
            BrowserType::Edge => {
                #[cfg(target_os = "macos")]
                {
                    home.join("Library/Application Support/Microsoft Edge/Default")
                }
                #[cfg(target_os = "linux")]
                {
                    home.join(".config/microsoft-edge/Default")
                }
            }
            BrowserType::Firefox => {
                #[cfg(target_os = "macos")]
                let profiles_dir = home.join("Library/Application Support/Firefox/Profiles");
                #[cfg(target_os = "linux")]
                let profiles_dir = home.join(".mozilla/firefox");

                if profiles_dir.exists() {
                    if let Ok(entries) = fs::read_dir(&profiles_dir) {
                        for entry in entries.flatten() {
                            let name = entry.file_name().to_string_lossy().to_string();
                            if name.ends_with(".default-release") || name.ends_with(".default") {
                                return Some(entry.path());
                            }
                        }
                    }
                }
                return None;
            }
            BrowserType::Brave => {
                #[cfg(target_os = "macos")]
                {
                    home.join("Library/Application Support/BraveSoftware/Brave-Browser/Default")
                }
                #[cfg(target_os = "linux")]
                {
                    home.join(".config/BraveSoftware/Brave-Browser/Default")
                }
            }
            _ => return None,
        };

        if path.exists() {
            Some(path)
        } else {
            None
        }
    }

    /// Get the cookie database path for Chromium-based browsers
    pub fn cookies_path(&self) -> Option<PathBuf> {
        let profile = self.profile_path()?;

        match self {
            BrowserType::Firefox => {
                let path = profile.join("cookies.sqlite");
                if path.exists() {
                    Some(path)
                } else {
                    None
                }
            }
            _ => {
                // Chromium-based browsers store cookies in Network/Cookies (newer) or Cookies (older)
                let network_path = profile.join("Network/Cookies");
                if network_path.exists() {
                    return Some(network_path);
                }
                let old_path = profile.join("Cookies");
                if old_path.exists() {
                    Some(old_path)
                } else {
                    None
                }
            }
        }
    }

    /// Get the Local State file path (contains encryption key for Chromium browsers)
    /// Reserved for future cookie decryption implementation
    #[cfg(windows)]
    #[allow(dead_code)]
    pub fn local_state_path(&self) -> Option<PathBuf> {
        let local_app_data = dirs::data_local_dir()?;
        let roaming_app_data = dirs::data_dir()?;

        let path = match self {
            BrowserType::Chrome => local_app_data.join("Google/Chrome/User Data/Local State"),
            BrowserType::Edge => local_app_data.join("Microsoft/Edge/User Data/Local State"),
            BrowserType::Brave => {
                local_app_data.join("BraveSoftware/Brave-Browser/User Data/Local State")
            }
            BrowserType::Vivaldi => local_app_data.join("Vivaldi/User Data/Local State"),
            BrowserType::Opera => roaming_app_data.join("Opera Software/Opera Stable/Local State"),
            BrowserType::Firefox => return None, // Firefox doesn't use this
        };

        if path.exists() {
            Some(path)
        } else {
            None
        }
    }
}

/// Web LLM provider authentication info
#[derive(Debug, Clone)]
pub struct ProviderAuth {
    pub name: &'static str,
    pub domain: &'static str,
    pub auth_cookie_names: &'static [&'static str],
    #[allow(dead_code)]
    pub description: &'static str,
}

/// Known web LLM providers and their authentication cookies
pub const WEB_LLM_PROVIDERS: &[ProviderAuth] = &[
    ProviderAuth {
        name: "ChatGPT",
        domain: "chatgpt.com", // Also checked: openai.com, chat.openai.com
        auth_cookie_names: &[
            "__Secure-next-auth.session-token",
            "_puid",
            "__cf_bm",
            "cf_clearance",
        ],
        description: "OpenAI ChatGPT",
    },
    ProviderAuth {
        name: "Claude",
        domain: "claude.ai",
        auth_cookie_names: &["sessionKey", "__cf_bm"],
        description: "Anthropic Claude",
    },
    ProviderAuth {
        name: "Gemini",
        domain: "gemini.google.com",
        auth_cookie_names: &["SID", "HSID", "SSID"],
        description: "Google Gemini",
    },
    ProviderAuth {
        name: "Perplexity",
        domain: "perplexity.ai",
        auth_cookie_names: &["pplx.visitor-id", "__Secure-next-auth.session-token"],
        description: "Perplexity AI",
    },
    ProviderAuth {
        name: "DeepSeek",
        domain: "chat.deepseek.com",
        auth_cookie_names: &["token", "sessionid"],
        description: "DeepSeek Chat",
    },
    ProviderAuth {
        name: "Poe",
        domain: "poe.com",
        auth_cookie_names: &["p-b", "p-lat"],
        description: "Quora Poe",
    },
    ProviderAuth {
        name: "HuggingChat",
        domain: "huggingface.co",
        auth_cookie_names: &["token", "hf-chat"],
        description: "HuggingFace Chat",
    },
    ProviderAuth {
        name: "Copilot",
        domain: "copilot.microsoft.com",
        auth_cookie_names: &["_U", "MUID"],
        description: "Microsoft Copilot",
    },
    ProviderAuth {
        name: "Mistral",
        domain: "chat.mistral.ai",
        auth_cookie_names: &["__Secure-next-auth.session-token"],
        description: "Mistral Le Chat",
    },
    ProviderAuth {
        name: "Cohere",
        domain: "coral.cohere.com",
        auth_cookie_names: &["session", "auth_token"],
        description: "Cohere Coral",
    },
    ProviderAuth {
        name: "Groq",
        domain: "groq.com",
        auth_cookie_names: &["__Secure-next-auth.session-token"],
        description: "Groq Cloud",
    },
    ProviderAuth {
        name: "Phind",
        domain: "phind.com",
        auth_cookie_names: &["__Secure-next-auth.session-token", "phind-session"],
        description: "Phind AI",
    },
    ProviderAuth {
        name: "Character.AI",
        domain: "character.ai",
        auth_cookie_names: &["token", "web-next-auth.session-token"],
        description: "Character.AI",
    },
    ProviderAuth {
        name: "You.com",
        domain: "you.com",
        auth_cookie_names: &["stytch_session", "youchat_session"],
        description: "You.com AI",
    },
    ProviderAuth {
        name: "Pi",
        domain: "pi.ai",
        auth_cookie_names: &["__Secure-next-auth.session-token"],
        description: "Inflection Pi",
    },
];

/// Result of checking browser authentication
#[derive(Debug, Clone)]
pub struct BrowserAuthResult {
    pub browser: BrowserType,
    pub provider: String,
    pub authenticated: bool,
    #[allow(dead_code)]
    pub cookies_found: Vec<String>,
}

/// Scan browsers for authenticated LLM providers
pub fn scan_browser_auth() -> Vec<BrowserAuthResult> {
    scan_browser_auth_internal(false)
}

/// Scan browsers for authenticated LLM providers with verbose output
pub fn scan_browser_auth_verbose() -> Vec<BrowserAuthResult> {
    scan_browser_auth_internal(true)
}

fn scan_browser_auth_internal(verbose: bool) -> Vec<BrowserAuthResult> {
    let mut results = Vec::new();

    let browsers = [
        BrowserType::Edge,
        BrowserType::Chrome,
        BrowserType::Brave,
        BrowserType::Firefox,
        BrowserType::Vivaldi,
        BrowserType::Opera,
    ];

    for browser in browsers {
        if let Some(cookies_path) = browser.cookies_path() {
            if verbose {
                println!(
                    "      {} {} cookies: {}",
                    "->".dimmed(),
                    browser.name(),
                    cookies_path.display()
                );
            }
            match scan_browser_cookies_internal(&browser, &cookies_path, verbose) {
                Ok(browser_results) => results.extend(browser_results),
                Err(e) => {
                    if verbose {
                        println!("        {} Direct access failed: {}", "!".yellow(), e);
                        println!("        {} Trying copy method...", "->".dimmed());
                    }
                    // Browser might be open and locking the database
                    // Try copying to temp file
                    match scan_browser_cookies_with_copy_internal(&browser, &cookies_path, verbose)
                    {
                        Ok(browser_results) => results.extend(browser_results),
                        Err(e2) => {
                            if verbose {
                                println!("        {} Copy method also failed: {}", "x".red(), e2);
                            }
                        }
                    }
                }
            }
        }
    }

    results
}

/// Get list of installed browsers
pub fn get_installed_browsers() -> Vec<BrowserType> {
    let browsers = [
        BrowserType::Edge,
        BrowserType::Chrome,
        BrowserType::Brave,
        BrowserType::Firefox,
        BrowserType::Vivaldi,
        BrowserType::Opera,
    ];

    browsers
        .into_iter()
        .filter(|b| b.profile_path().is_some())
        .collect()
}

/// Scan a browser's cookie database for LLM provider authentication
#[allow(dead_code)]
fn scan_browser_cookies(
    browser: &BrowserType,
    cookies_path: &PathBuf,
) -> Result<Vec<BrowserAuthResult>> {
    scan_browser_cookies_internal(browser, cookies_path, false)
}

fn scan_browser_cookies_internal(
    browser: &BrowserType,
    cookies_path: &PathBuf,
    verbose: bool,
) -> Result<Vec<BrowserAuthResult>> {
    let mut results = Vec::new();

    // Open database read-only
    let conn = Connection::open_with_flags(
        cookies_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .context("Failed to open cookie database")?;

    // Get all cookies grouped by domain
    let cookies = match *browser {
        BrowserType::Firefox => get_firefox_cookies(&conn)?,
        _ => get_chromium_cookies(&conn)?,
    };

    if verbose {
        println!(
            "        {} Found {} domains with cookies",
            "->".dimmed(),
            cookies.len()
        );

        // Show domains that might match our providers
        let llm_domains: Vec<_> = cookies
            .keys()
            .filter(|d| {
                let dl = d.to_lowercase();
                dl.contains("openai")
                    || dl.contains("claude")
                    || dl.contains("anthropic")
                    || dl.contains("google")
                    || dl.contains("perplexity")
                    || dl.contains("deepseek")
                    || dl.contains("poe")
                    || dl.contains("huggingface")
                    || dl.contains("microsoft")
                    || dl.contains("copilot")
                    || dl.contains("mistral")
                    || dl.contains("cohere")
                    || dl.contains("groq")
                    || dl.contains("phind")
                    || dl.contains("character")
            })
            .collect();

        if !llm_domains.is_empty() {
            println!("        {} LLM-related domains found:", "->".dimmed());
            for domain in &llm_domains {
                let cookie_names = cookies
                    .get(*domain)
                    .map(|v| v.join(", "))
                    .unwrap_or_default();
                println!(
                    "          {} {} -> [{}]",
                    "*".dimmed(),
                    domain,
                    cookie_names.dimmed()
                );
            }
        }
    }

    // Check each provider
    for provider in WEB_LLM_PROVIDERS {
        // Domain matching needs to handle:
        // - Exact match: "chat.openai.com"
        // - Dot-prefixed: ".openai.com"
        // - Parent domain: "openai.com" matches ".openai.com"
        let domain_cookies: Vec<&String> = cookies
            .iter()
            .filter(|(domain, _)| {
                let domain_clean = domain.trim_start_matches('.');
                let provider_domain = provider.domain.trim_start_matches('.');
                domain_clean.ends_with(provider_domain) || provider_domain.ends_with(domain_clean)
            })
            .flat_map(|(_, names)| names)
            .collect();

        let found_auth_cookies: Vec<String> = provider
            .auth_cookie_names
            .iter()
            .filter(|name| {
                domain_cookies
                    .iter()
                    .any(|c| c == *name || c.contains(*name))
            })
            .map(|s| s.to_string())
            .collect();

        let authenticated = !found_auth_cookies.is_empty();

        if verbose && !domain_cookies.is_empty() {
            println!(
                "        {} {}: domain cookies={:?}, auth cookies={:?}, authenticated={}",
                "->".dimmed(),
                provider.name,
                domain_cookies.iter().take(5).collect::<Vec<_>>(),
                found_auth_cookies,
                authenticated
            );
        }

        results.push(BrowserAuthResult {
            browser: *browser,
            provider: provider.name.to_string(),
            authenticated,
            cookies_found: found_auth_cookies,
        });
    }

    Ok(results)
}

/// Copy cookie database to temp file and scan (for when browser has lock)
#[allow(dead_code)]
fn scan_browser_cookies_with_copy(
    browser: &BrowserType,
    cookies_path: &PathBuf,
) -> Result<Vec<BrowserAuthResult>> {
    scan_browser_cookies_with_copy_internal(browser, cookies_path, false)
}

fn scan_browser_cookies_with_copy_internal(
    browser: &BrowserType,
    cookies_path: &PathBuf,
    verbose: bool,
) -> Result<Vec<BrowserAuthResult>> {
    let temp_dir = std::env::temp_dir();
    let temp_path = temp_dir.join(format!("csm_cookies_{}.db", uuid::Uuid::new_v4()));

    // Copy the database file
    fs::copy(cookies_path, &temp_path).context("Failed to copy cookie database")?;

    // Also copy the journal/wal files if they exist
    let wal_path = cookies_path.with_extension("db-wal");
    if wal_path.exists() {
        let _ = fs::copy(&wal_path, temp_path.with_extension("db-wal"));
    }
    let shm_path = cookies_path.with_extension("db-shm");
    if shm_path.exists() {
        let _ = fs::copy(&shm_path, temp_path.with_extension("db-shm"));
    }

    // Firefox uses -wal and -shm without the .db prefix
    let ff_wal = cookies_path.with_file_name(format!(
        "{}-wal",
        cookies_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
    ));
    if ff_wal.exists() {
        let _ = fs::copy(
            &ff_wal,
            temp_dir.join(format!("csm_cookies_{}.db-wal", uuid::Uuid::new_v4())),
        );
    }

    if verbose {
        println!(
            "        {} Copied to temp: {}",
            "->".dimmed(),
            temp_path.display()
        );
    }

    let result = scan_browser_cookies_internal(browser, &temp_path, verbose);

    // Clean up temp files
    let _ = fs::remove_file(&temp_path);
    let _ = fs::remove_file(temp_path.with_extension("db-wal"));
    let _ = fs::remove_file(temp_path.with_extension("db-shm"));

    result
}

/// Get cookies from Chromium-based browser database
fn get_chromium_cookies(conn: &Connection) -> Result<HashMap<String, Vec<String>>> {
    let mut cookies: HashMap<String, Vec<String>> = HashMap::new();

    // Query cookie names grouped by host
    let mut stmt = conn.prepare(
        "SELECT host_key, name FROM cookies WHERE host_key LIKE '%.%' GROUP BY host_key, name",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;

    for row in rows.flatten() {
        let (host, name) = row;
        cookies.entry(host).or_default().push(name);
    }

    Ok(cookies)
}

/// Get cookies from Firefox database
fn get_firefox_cookies(conn: &Connection) -> Result<HashMap<String, Vec<String>>> {
    let mut cookies: HashMap<String, Vec<String>> = HashMap::new();

    let mut stmt = conn
        .prepare("SELECT host, name FROM moz_cookies WHERE host LIKE '%.%' GROUP BY host, name")?;

    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;

    for row in rows.flatten() {
        let (host, name) = row;
        cookies.entry(host).or_default().push(name);
    }

    Ok(cookies)
}

/// Summary of authenticated providers across all browsers
/// Reserved for future programmatic access to auth state
#[allow(dead_code)]
#[derive(Debug, Default)]
pub struct AuthSummary {
    pub browsers_checked: Vec<BrowserType>,
    pub authenticated_providers: HashMap<String, Vec<BrowserType>>,
    pub total_providers_authenticated: usize,
}

/// Extracted cookie with its value
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ExtractedCookie {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub browser: BrowserType,
}

/// Provider credentials extracted from browser cookies
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct ProviderCredentials {
    pub provider: String,
    pub session_token: Option<String>,
    pub cookies: HashMap<String, String>,
    pub browser: Option<BrowserType>,
}

/// Extract actual cookie values for a specific provider
pub fn extract_provider_cookies(provider_name: &str) -> Option<ProviderCredentials> {
    let provider_auth = WEB_LLM_PROVIDERS
        .iter()
        .find(|p| p.name.eq_ignore_ascii_case(provider_name))?;

    // For ChatGPT, try multiple domains since cookies can be on either
    let domains_to_try: Vec<&str> = if provider_name.eq_ignore_ascii_case("chatgpt") {
        vec!["chatgpt.com", "openai.com", "chat.openai.com"]
    } else {
        vec![provider_auth.domain]
    };

    let browsers = [
        BrowserType::Edge,
        BrowserType::Chrome,
        BrowserType::Brave,
        BrowserType::Firefox,
        BrowserType::Vivaldi,
        BrowserType::Opera,
    ];

    for browser in browsers {
        if let Some(cookies_path) = browser.cookies_path() {
            for domain in &domains_to_try {
                // Try to extract cookies
                if let Ok(cookies) = extract_cookies_for_domain(&browser, &cookies_path, domain) {
                    if !cookies.is_empty() {
                        let mut creds = ProviderCredentials {
                            provider: provider_name.to_string(),
                            session_token: None,
                            cookies: HashMap::new(),
                            browser: Some(browser),
                        };

                        for cookie in &cookies {
                            // Check if this is a session token cookie
                            if provider_auth
                                .auth_cookie_names
                                .iter()
                                .any(|name| cookie.name.contains(name))
                                && (cookie.name.contains("session")
                                    || cookie.name.contains("token"))
                            {
                                creds.session_token = Some(cookie.value.clone());
                            }
                            creds
                                .cookies
                                .insert(cookie.name.clone(), cookie.value.clone());
                        }

                        if creds.session_token.is_some() || !creds.cookies.is_empty() {
                            return Some(creds);
                        }
                    }
                }
            }
        }
    }

    None
}

/// Extract all cookies for a domain from a browser
fn extract_cookies_for_domain(
    browser: &BrowserType,
    cookies_path: &PathBuf,
    domain: &str,
) -> Result<Vec<ExtractedCookie>> {
    // Try direct access first
    match extract_cookies_internal(browser, cookies_path, domain) {
        Ok(cookies) => Ok(cookies),
        Err(_) => {
            // Browser might be locking the file, try copy method
            extract_cookies_with_copy(browser, cookies_path, domain)
        }
    }
}

fn extract_cookies_internal(
    browser: &BrowserType,
    cookies_path: &PathBuf,
    domain: &str,
) -> Result<Vec<ExtractedCookie>> {
    let conn = Connection::open_with_flags(
        cookies_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .context("Failed to open cookie database")?;

    match browser {
        BrowserType::Firefox => extract_firefox_cookie_values(&conn, domain, browser),
        _ => extract_chromium_cookie_values(&conn, domain, browser),
    }
}

fn extract_cookies_with_copy(
    browser: &BrowserType,
    cookies_path: &PathBuf,
    domain: &str,
) -> Result<Vec<ExtractedCookie>> {
    let temp_dir = std::env::temp_dir();
    let temp_path = temp_dir.join(format!("csm_cookies_extract_{}.db", uuid::Uuid::new_v4()));

    fs::copy(cookies_path, &temp_path).context("Failed to copy cookie database")?;

    // Copy journal files
    let wal_path = cookies_path.with_extension("db-wal");
    if wal_path.exists() {
        let _ = fs::copy(&wal_path, temp_path.with_extension("db-wal"));
    }
    let shm_path = cookies_path.with_extension("db-shm");
    if shm_path.exists() {
        let _ = fs::copy(&shm_path, temp_path.with_extension("db-shm"));
    }

    let result = extract_cookies_internal(browser, &temp_path, domain);

    // Clean up
    let _ = fs::remove_file(&temp_path);
    let _ = fs::remove_file(temp_path.with_extension("db-wal"));
    let _ = fs::remove_file(temp_path.with_extension("db-shm"));

    result
}

/// Extract cookie values from Firefox database
fn extract_firefox_cookie_values(
    conn: &Connection,
    domain: &str,
    browser: &BrowserType,
) -> Result<Vec<ExtractedCookie>> {
    let mut cookies = Vec::new();

    let mut stmt =
        conn.prepare("SELECT name, value, host FROM moz_cookies WHERE host LIKE ? OR host LIKE ?")?;

    let domain_pattern = format!("%{}", domain);
    let dot_domain_pattern = format!("%.{}", domain);

    let rows = stmt.query_map([&domain_pattern, &dot_domain_pattern], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;

    for row in rows.flatten() {
        let (name, value, host) = row;
        if !value.is_empty() {
            cookies.push(ExtractedCookie {
                name,
                value,
                domain: host,
                browser: *browser,
            });
        }
    }

    Ok(cookies)
}

/// Extract cookie values from Chromium-based browser database
/// Note: Chromium encrypts cookie values, this returns encrypted values
/// Full decryption requires platform-specific crypto APIs
fn extract_chromium_cookie_values(
    conn: &Connection,
    domain: &str,
    browser: &BrowserType,
) -> Result<Vec<ExtractedCookie>> {
    let mut cookies = Vec::new();

    // Chromium stores encrypted values in encrypted_value column
    // We try to get the plaintext value first, then fall back to encrypted
    let mut stmt = conn.prepare(
        "SELECT name, value, encrypted_value, host_key FROM cookies WHERE host_key LIKE ? OR host_key LIKE ?"
    )?;

    let domain_pattern = format!("%{}", domain);
    let dot_domain_pattern = format!("%.{}", domain);

    let rows = stmt.query_map([&domain_pattern, &dot_domain_pattern], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Vec<u8>>(2)?,
            row.get::<_, String>(3)?,
        ))
    })?;

    for row in rows.flatten() {
        let (name, value, encrypted_value, host) = row;

        // Try plaintext value first (older Chrome versions or some cookies)
        let cookie_value = if !value.is_empty() {
            value
        } else if !encrypted_value.is_empty() {
            // Try to decrypt the cookie value
            match decrypt_chromium_cookie(&encrypted_value, browser) {
                Ok(decrypted) => decrypted,
                Err(_) => continue, // Skip cookies we can't decrypt
            }
        } else {
            continue;
        };

        if !cookie_value.is_empty() {
            cookies.push(ExtractedCookie {
                name,
                value: cookie_value,
                domain: host,
                browser: *browser,
            });
        }
    }

    Ok(cookies)
}

/// Decrypt Chromium cookie value
/// On Windows, uses DPAPI. On macOS, uses Keychain. On Linux, may be stored in plain text or use secret service.
#[cfg(windows)]
fn decrypt_chromium_cookie(encrypted_value: &[u8], browser: &BrowserType) -> Result<String> {
    use windows::Win32::Security::Cryptography::{CryptUnprotectData, CRYPT_INTEGER_BLOB};

    // Check for v10/v20 prefix (AES-GCM encrypted)
    if encrypted_value.len() > 3 && &encrypted_value[0..3] == b"v10"
        || &encrypted_value[0..3] == b"v20"
    {
        // Get the encryption key from Local State file
        if let Some(key) = get_chromium_encryption_key(browser) {
            return decrypt_aes_gcm(&encrypted_value[3..], &key);
        }
    }

    // Try DPAPI decryption (older format)
    unsafe {
        #[allow(unused_mut)]
        let input = CRYPT_INTEGER_BLOB {
            cbData: encrypted_value.len() as u32,
            pbData: encrypted_value.as_ptr() as *mut u8,
        };
        let mut output = CRYPT_INTEGER_BLOB {
            cbData: 0,
            pbData: std::ptr::null_mut(),
        };

        let result = CryptUnprotectData(&input, None, None, None, None, 0, &mut output);

        if result.is_ok() && !output.pbData.is_null() {
            let slice = std::slice::from_raw_parts(output.pbData, output.cbData as usize);
            let decrypted = String::from_utf8_lossy(slice).to_string();
            // Note: We should free output.pbData with LocalFree, but it's a small leak
            // for a short-lived operation. The windows crate doesn't expose LocalFree directly.
            return Ok(decrypted);
        }
    }

    Err(anyhow!("Failed to decrypt cookie"))
}

#[cfg(windows)]
fn get_chromium_encryption_key(browser: &BrowserType) -> Option<Vec<u8>> {
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
    use windows::Win32::Security::Cryptography::{CryptUnprotectData, CRYPT_INTEGER_BLOB};

    let local_state_path = browser.local_state_path()?;
    let local_state_content = fs::read_to_string(&local_state_path).ok()?;
    let local_state: serde_json::Value = serde_json::from_str(&local_state_content).ok()?;

    let encrypted_key_b64 = local_state
        .get("os_crypt")?
        .get("encrypted_key")?
        .as_str()?;

    let encrypted_key = BASE64.decode(encrypted_key_b64).ok()?;

    // Remove "DPAPI" prefix (5 bytes)
    if encrypted_key.len() <= 5 || &encrypted_key[0..5] != b"DPAPI" {
        return None;
    }

    let encrypted_key = &encrypted_key[5..];

    // Decrypt using DPAPI
    unsafe {
        #[allow(unused_mut)]
        let input = CRYPT_INTEGER_BLOB {
            cbData: encrypted_key.len() as u32,
            pbData: encrypted_key.as_ptr() as *mut u8,
        };
        let mut output = CRYPT_INTEGER_BLOB {
            cbData: 0,
            pbData: std::ptr::null_mut(),
        };

        let result = CryptUnprotectData(&input, None, None, None, None, 0, &mut output);

        if result.is_ok() && !output.pbData.is_null() {
            let key = std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
            // Note: We should free output.pbData with LocalFree, but it's a small leak
            // for a short-lived operation. The windows crate doesn't expose LocalFree directly.
            return Some(key);
        }
    }

    None
}

#[cfg(windows)]
fn decrypt_aes_gcm(encrypted_data: &[u8], key: &[u8]) -> Result<String> {
    use aes_gcm::{
        aead::{Aead, KeyInit},
        Aes256Gcm, Nonce,
    };

    if encrypted_data.len() < 12 + 16 {
        return Err(anyhow!("Encrypted data too short"));
    }

    // First 12 bytes are nonce
    let nonce = Nonce::from_slice(&encrypted_data[0..12]);
    let ciphertext = &encrypted_data[12..];

    let cipher =
        Aes256Gcm::new_from_slice(key).map_err(|e| anyhow!("Failed to create cipher: {}", e))?;

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| anyhow!("Decryption failed: {}", e))?;

    String::from_utf8(plaintext).map_err(|e| anyhow!("Invalid UTF-8 in decrypted cookie: {}", e))
}

#[cfg(not(windows))]
fn decrypt_chromium_cookie(encrypted_value: &[u8], _browser: &BrowserType) -> Result<String> {
    // On macOS, would need to access Keychain
    // On Linux, cookies may be stored in plain text or use secret service

    // Check if it's already plaintext
    if let Ok(s) = String::from_utf8(encrypted_value.to_vec()) {
        if s.chars()
            .all(|c| c.is_ascii_graphic() || c.is_ascii_whitespace())
        {
            return Ok(s);
        }
    }

    Err(anyhow!(
        "Cookie decryption not implemented for this platform"
    ))
}

/// Get a summary of all authenticated web LLM providers
/// Reserved for future programmatic access to auth state
#[allow(dead_code)]
pub fn get_auth_summary() -> AuthSummary {
    let results = scan_browser_auth();
    let mut summary = AuthSummary::default();

    // Track which browsers we checked
    let mut browsers_seen = std::collections::HashSet::new();

    for result in results {
        browsers_seen.insert(result.browser);

        if result.authenticated {
            summary
                .authenticated_providers
                .entry(result.provider)
                .or_default()
                .push(result.browser);
        }
    }

    summary.browsers_checked = browsers_seen.into_iter().collect();
    summary.total_providers_authenticated = summary.authenticated_providers.len();

    summary
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_browser_type_name() {
        assert_eq!(BrowserType::Chrome.name(), "Chrome");
        assert_eq!(BrowserType::Edge.name(), "Edge");
        assert_eq!(BrowserType::Firefox.name(), "Firefox");
    }

    #[test]
    fn test_get_installed_browsers() {
        let browsers = get_installed_browsers();
        // Should return a list (may be empty if no browsers installed)
        assert!(browsers.len() <= 6);
    }

    #[test]
    fn test_provider_auth_domains() {
        // Verify all providers have valid domains
        for provider in WEB_LLM_PROVIDERS {
            assert!(!provider.domain.is_empty());
            assert!(provider.domain.contains('.'));
            assert!(!provider.auth_cookie_names.is_empty());
        }
    }

    #[test]
    fn test_auth_summary_default() {
        let summary = AuthSummary::default();
        assert!(summary.browsers_checked.is_empty());
        assert!(summary.authenticated_providers.is_empty());
        assert_eq!(summary.total_providers_authenticated, 0);
    }
}
