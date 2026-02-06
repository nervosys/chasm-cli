// Copyright (c) 2024-2028 Nervosys LLC
// SPDX-License-Identifier: Apache-2.0
//! Edge Caching Module
//!
//! Provides CDN integration and edge caching for API responses.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;
use tokio::sync::RwLock;

// ============================================================================
// Cache Configuration
// ============================================================================

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Enable caching
    pub enabled: bool,
    /// Default TTL in seconds
    pub default_ttl_seconds: u64,
    /// Maximum cache size in MB
    pub max_size_mb: u64,
    /// Cache backend
    pub backend: CacheBackend,
    /// CDN configuration
    pub cdn: Option<CdnConfig>,
    /// Cache rules by path pattern
    pub rules: Vec<CacheRule>,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_ttl_seconds: 300,
            max_size_mb: 100,
            backend: CacheBackend::Memory,
            cdn: None,
            rules: vec![
                CacheRule {
                    pattern: "/api/stats".to_string(),
                    ttl_seconds: 60,
                    cache_control: "public, max-age=60".to_string(),
                    vary: vec!["Accept".to_string()],
                    private: false,
                },
                CacheRule {
                    pattern: "/api/sessions".to_string(),
                    ttl_seconds: 30,
                    cache_control: "private, max-age=30".to_string(),
                    vary: vec!["Authorization".to_string()],
                    private: true,
                },
            ],
        }
    }
}

/// Cache backend type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CacheBackend {
    /// In-memory cache
    Memory,
    /// Redis cache
    Redis(String),
    /// Memcached
    Memcached(String),
    /// File-based cache
    File(String),
}

/// CDN configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CdnConfig {
    /// CDN provider
    pub provider: CdnProvider,
    /// CDN base URL
    pub base_url: String,
    /// API key for cache invalidation
    pub api_key: Option<String>,
    /// Zone ID
    pub zone_id: Option<String>,
}

/// CDN provider
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CdnProvider {
    Cloudflare,
    Fastly,
    CloudFront,
    Akamai,
    BunnyCDN,
    Custom,
}

/// Cache rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheRule {
    /// URL pattern (glob or regex)
    pub pattern: String,
    /// TTL in seconds
    pub ttl_seconds: u64,
    /// Cache-Control header
    pub cache_control: String,
    /// Vary headers
    pub vary: Vec<String>,
    /// Whether cache is private (per-user)
    pub private: bool,
}

// ============================================================================
// Cache Entry
// ============================================================================

/// Cached entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    /// Cache key
    pub key: String,
    /// Cached value (serialized)
    pub value: Vec<u8>,
    /// Content type
    pub content_type: String,
    /// ETag
    pub etag: String,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Expires timestamp
    pub expires_at: DateTime<Utc>,
    /// Cache headers
    pub headers: HashMap<String, String>,
    /// Hit count
    pub hits: u64,
    /// Size in bytes
    pub size_bytes: usize,
}

impl CacheEntry {
    /// Check if entry is expired
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    /// Check if entry is stale (within grace period)
    pub fn is_stale(&self, grace_seconds: i64) -> bool {
        let grace_time = self.expires_at + Duration::seconds(grace_seconds);
        Utc::now() > self.expires_at && Utc::now() <= grace_time
    }

    /// Get remaining TTL in seconds
    pub fn remaining_ttl(&self) -> i64 {
        (self.expires_at - Utc::now()).num_seconds().max(0)
    }
}

// ============================================================================
// Edge Cache Manager
// ============================================================================

/// Manages edge caching operations
pub struct EdgeCacheManager {
    config: CacheConfig,
    cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
    stats: Arc<RwLock<CacheStats>>,
}

/// Cache statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CacheStats {
    /// Total requests
    pub requests: u64,
    /// Cache hits
    pub hits: u64,
    /// Cache misses
    pub misses: u64,
    /// Stale hits (served stale while revalidating)
    pub stale_hits: u64,
    /// Bytes served from cache
    pub bytes_served: u64,
    /// Current cache size
    pub current_size_bytes: u64,
    /// Number of entries
    pub entry_count: usize,
    /// Evictions
    pub evictions: u64,
}

impl CacheStats {
    /// Get hit rate
    pub fn hit_rate(&self) -> f64 {
        if self.requests == 0 {
            0.0
        } else {
            self.hits as f64 / self.requests as f64
        }
    }
}

impl EdgeCacheManager {
    /// Create a new edge cache manager
    pub fn new(config: CacheConfig) -> Self {
        Self {
            config,
            cache: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(CacheStats::default())),
        }
    }

    /// Generate cache key from request
    pub fn generate_key(&self, path: &str, query: Option<&str>, vary_headers: &HashMap<String, String>) -> String {
        let mut key = path.to_string();
        
        if let Some(q) = query {
            key.push('?');
            key.push_str(q);
        }

        // Include vary headers in key
        let rule = self.get_rule(path);
        if let Some(rule) = rule {
            for header in &rule.vary {
                if let Some(value) = vary_headers.get(header) {
                    key.push_str(&format!("|{}:{}", header, value));
                }
            }
        }

        // Hash for consistent key length
        format!("cache:{:x}", md5_hash(&key))
    }

    /// Get cache rule for path
    fn get_rule(&self, path: &str) -> Option<&CacheRule> {
        self.config.rules.iter().find(|r| path.starts_with(&r.pattern))
    }

    /// Get entry from cache
    pub async fn get(&self, key: &str) -> Option<CacheEntry> {
        let mut stats = self.stats.write().await;
        stats.requests += 1;

        let cache = self.cache.read().await;
        if let Some(entry) = cache.get(key) {
            if !entry.is_expired() {
                stats.hits += 1;
                stats.bytes_served += entry.size_bytes as u64;
                return Some(entry.clone());
            } else if entry.is_stale(60) {
                // Stale-while-revalidate
                stats.stale_hits += 1;
                stats.bytes_served += entry.size_bytes as u64;
                return Some(entry.clone());
            }
        }

        stats.misses += 1;
        None
    }

    /// Set entry in cache
    pub async fn set(&self, key: String, value: Vec<u8>, content_type: String, path: &str) {
        let rule = self.get_rule(path);
        let ttl = rule.map(|r| r.ttl_seconds).unwrap_or(self.config.default_ttl_seconds);
        let cache_control = rule
            .map(|r| r.cache_control.clone())
            .unwrap_or_else(|| format!("public, max-age={}", ttl));

        let entry = CacheEntry {
            key: key.clone(),
            size_bytes: value.len(),
            value,
            content_type,
            etag: generate_etag(&key),
            created_at: Utc::now(),
            expires_at: Utc::now() + Duration::seconds(ttl as i64),
            headers: HashMap::from([("Cache-Control".to_string(), cache_control)]),
            hits: 0,
        };

        // Check size limits
        self.evict_if_needed(entry.size_bytes).await;

        let mut cache = self.cache.write().await;
        let mut stats = self.stats.write().await;

        stats.current_size_bytes += entry.size_bytes as u64;
        stats.entry_count = cache.len() + 1;

        cache.insert(key, entry);
    }

    /// Evict entries if cache is full
    async fn evict_if_needed(&self, new_entry_size: usize) {
        let max_size = self.config.max_size_mb * 1024 * 1024;
        let stats = self.stats.read().await;
        
        if stats.current_size_bytes + new_entry_size as u64 <= max_size {
            return;
        }
        drop(stats);

        // Evict expired entries first
        self.evict_expired().await;

        // If still over limit, evict LRU entries
        let stats = self.stats.read().await;
        if stats.current_size_bytes + new_entry_size as u64 > max_size {
            drop(stats);
            self.evict_lru((max_size / 4) as usize).await; // Evict 25%
        }
    }

    /// Evict expired entries
    async fn evict_expired(&self) {
        let mut cache = self.cache.write().await;
        let mut stats = self.stats.write().await;

        let expired_keys: Vec<_> = cache
            .iter()
            .filter(|(_, entry)| entry.is_expired() && !entry.is_stale(60))
            .map(|(k, _)| k.clone())
            .collect();

        for key in expired_keys {
            if let Some(entry) = cache.remove(&key) {
                stats.current_size_bytes -= entry.size_bytes as u64;
                stats.evictions += 1;
            }
        }
        stats.entry_count = cache.len();
    }

    /// Evict LRU entries to free space
    async fn evict_lru(&self, bytes_to_free: usize) {
        let mut cache = self.cache.write().await;
        let mut stats = self.stats.write().await;

        // Sort by hits (ascending) and created_at (ascending)
        let mut entries: Vec<_> = cache.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        entries.sort_by(|a, b| a.1.hits.cmp(&b.1.hits).then(a.1.created_at.cmp(&b.1.created_at)));

        let mut freed = 0usize;
        for (key, entry) in entries {
            if freed >= bytes_to_free {
                break;
            }
            cache.remove(&key);
            freed += entry.size_bytes;
            stats.current_size_bytes -= entry.size_bytes as u64;
            stats.evictions += 1;
        }
        stats.entry_count = cache.len();
    }

    /// Invalidate cache entry
    pub async fn invalidate(&self, key: &str) {
        let mut cache = self.cache.write().await;
        let mut stats = self.stats.write().await;

        if let Some(entry) = cache.remove(key) {
            stats.current_size_bytes -= entry.size_bytes as u64;
            stats.entry_count = cache.len();
        }

        // Also invalidate on CDN if configured
        if let Some(cdn) = &self.config.cdn {
            self.invalidate_cdn(cdn, key).await;
        }
    }

    /// Invalidate cache entries by prefix
    pub async fn invalidate_prefix(&self, prefix: &str) {
        let mut cache = self.cache.write().await;
        let mut stats = self.stats.write().await;

        let keys_to_remove: Vec<_> = cache
            .keys()
            .filter(|k| k.starts_with(prefix))
            .cloned()
            .collect();

        for key in keys_to_remove {
            if let Some(entry) = cache.remove(&key) {
                stats.current_size_bytes -= entry.size_bytes as u64;
            }
        }
        stats.entry_count = cache.len();
    }

    /// Clear all cache
    pub async fn clear(&self) {
        let mut cache = self.cache.write().await;
        let mut stats = self.stats.write().await;

        cache.clear();
        stats.current_size_bytes = 0;
        stats.entry_count = 0;
    }

    /// Invalidate on CDN
    async fn invalidate_cdn(&self, cdn: &CdnConfig, key: &str) {
        match cdn.provider {
            CdnProvider::Cloudflare => self.invalidate_cloudflare(cdn, key).await,
            CdnProvider::Fastly => self.invalidate_fastly(cdn, key).await,
            CdnProvider::CloudFront => self.invalidate_cloudfront(cdn, key).await,
            _ => {}
        }
    }

    async fn invalidate_cloudflare(&self, _cdn: &CdnConfig, _key: &str) {
        // In a real implementation, call Cloudflare API
        // POST https://api.cloudflare.com/client/v4/zones/{zone_id}/purge_cache
    }

    async fn invalidate_fastly(&self, _cdn: &CdnConfig, _key: &str) {
        // In a real implementation, call Fastly API
        // POST https://api.fastly.com/service/{service_id}/purge/{surrogate_key}
    }

    async fn invalidate_cloudfront(&self, _cdn: &CdnConfig, _key: &str) {
        // In a real implementation, call CloudFront API
        // CreateInvalidation
    }

    /// Get cache statistics
    pub async fn get_stats(&self) -> CacheStats {
        self.stats.read().await.clone()
    }

    /// Get cache headers for response
    pub fn get_cache_headers(&self, path: &str, etag: &str) -> HashMap<String, String> {
        let mut headers = HashMap::new();

        if let Some(rule) = self.get_rule(path) {
            headers.insert("Cache-Control".to_string(), rule.cache_control.clone());
            if !rule.vary.is_empty() {
                headers.insert("Vary".to_string(), rule.vary.join(", "));
            }
        } else {
            headers.insert(
                "Cache-Control".to_string(),
                format!("public, max-age={}", self.config.default_ttl_seconds),
            );
        }

        headers.insert("ETag".to_string(), format!("\"{}\"", etag));
        headers
    }
}

// ============================================================================
// Utility Functions
// ============================================================================

fn md5_hash(input: &str) -> u128 {
    use std::hash::Hasher;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    input.hash(&mut hasher);
    hasher.finish() as u128
}

fn generate_etag(key: &str) -> String {
    format!("{:x}", md5_hash(&format!("{}{}", key, Utc::now().timestamp())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cache_set_get() {
        let config = CacheConfig::default();
        let cache = EdgeCacheManager::new(config);

        let key = cache.generate_key("/api/stats", None, &HashMap::new());
        cache.set(key.clone(), b"test data".to_vec(), "application/json".to_string(), "/api/stats").await;

        let entry = cache.get(&key).await;
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().value, b"test data");
    }

    #[tokio::test]
    async fn test_cache_invalidation() {
        let config = CacheConfig::default();
        let cache = EdgeCacheManager::new(config);

        let key = cache.generate_key("/api/test", None, &HashMap::new());
        cache.set(key.clone(), b"test".to_vec(), "text/plain".to_string(), "/api/test").await;

        assert!(cache.get(&key).await.is_some());

        cache.invalidate(&key).await;
        // Entry is removed
        let stats = cache.get_stats().await;
        assert_eq!(stats.entry_count, 0);
    }
}
