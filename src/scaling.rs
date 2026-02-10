// Copyright (c) 2024-2028 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! Database Scaling Module
//!
//! Provides sharding, read replicas, and advanced database scaling capabilities.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use tokio::sync::RwLock;

// ============================================================================
// Sharding Configuration
// ============================================================================

/// Sharding strategy
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ShardingStrategy {
    /// Shard by hash of key
    Hash,
    /// Shard by range of values
    Range,
    /// Shard by tenant ID
    Tenant,
    /// Shard by time period
    Temporal,
    /// Shard by geographic region
    Geographic,
    /// Round-robin distribution
    RoundRobin,
    /// Custom sharding function
    Custom(String),
}

/// Shard configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardConfig {
    /// Shard ID
    pub id: String,
    /// Shard name
    pub name: String,
    /// Database connection string
    pub connection_string: String,
    /// Shard weight (for load balancing)
    pub weight: u32,
    /// Whether shard is active
    pub active: bool,
    /// Shard region
    pub region: Option<String>,
    /// Min range (for range sharding)
    pub range_min: Option<String>,
    /// Max range (for range sharding)
    pub range_max: Option<String>,
    /// Tenant IDs (for tenant sharding)
    pub tenant_ids: Vec<String>,
}

/// Sharding configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardingConfig {
    /// Sharding strategy
    pub strategy: ShardingStrategy,
    /// Number of virtual shards (for consistent hashing)
    pub virtual_shards: u32,
    /// Replication factor
    pub replication_factor: u32,
    /// Shards
    pub shards: Vec<ShardConfig>,
    /// Default shard ID
    pub default_shard: String,
}

impl Default for ShardingConfig {
    fn default() -> Self {
        Self {
            strategy: ShardingStrategy::Hash,
            virtual_shards: 256,
            replication_factor: 1,
            shards: vec![ShardConfig {
                id: "default".to_string(),
                name: "Default Shard".to_string(),
                connection_string: "sqlite://chasm.db".to_string(),
                weight: 100,
                active: true,
                region: None,
                range_min: None,
                range_max: None,
                tenant_ids: vec![],
            }],
            default_shard: "default".to_string(),
        }
    }
}

// ============================================================================
// Shard Router
// ============================================================================

/// Routes queries to appropriate shards
pub struct ShardRouter {
    config: ShardingConfig,
    ring: ConsistentHashRing,
}

impl ShardRouter {
    /// Create a new shard router
    pub fn new(config: ShardingConfig) -> Self {
        let ring = ConsistentHashRing::new(&config);
        Self { config, ring }
    }

    /// Get shard for a key
    pub fn get_shard(&self, key: &str) -> &ShardConfig {
        match self.config.strategy {
            ShardingStrategy::Hash => self.get_shard_by_hash(key),
            ShardingStrategy::Range => self.get_shard_by_range(key),
            ShardingStrategy::Tenant => self.get_shard_by_tenant(key),
            ShardingStrategy::Temporal => self.get_shard_by_time(key),
            ShardingStrategy::Geographic => self.get_shard_by_region(key),
            ShardingStrategy::RoundRobin => self.get_shard_round_robin(),
            ShardingStrategy::Custom(_) => self.get_default_shard(),
        }
    }

    /// Get shard by consistent hash
    fn get_shard_by_hash(&self, key: &str) -> &ShardConfig {
        let shard_id = self.ring.get_node(key);
        self.config
            .shards
            .iter()
            .find(|s| s.id == shard_id && s.active)
            .unwrap_or_else(|| self.get_default_shard())
    }

    /// Get shard by range
    fn get_shard_by_range(&self, key: &str) -> &ShardConfig {
        for shard in &self.config.shards {
            if !shard.active {
                continue;
            }
            let in_min = shard.range_min.as_ref().map(|m| key >= m.as_str()).unwrap_or(true);
            let in_max = shard.range_max.as_ref().map(|m| key < m.as_str()).unwrap_or(true);
            if in_min && in_max {
                return shard;
            }
        }
        self.get_default_shard()
    }

    /// Get shard by tenant ID
    fn get_shard_by_tenant(&self, tenant_id: &str) -> &ShardConfig {
        self.config
            .shards
            .iter()
            .find(|s| s.active && s.tenant_ids.contains(&tenant_id.to_string()))
            .unwrap_or_else(|| self.get_default_shard())
    }

    /// Get shard by time
    fn get_shard_by_time(&self, time_key: &str) -> &ShardConfig {
        // Parse time and route to appropriate shard
        // For simplicity, use range-based routing
        self.get_shard_by_range(time_key)
    }

    /// Get shard by region
    fn get_shard_by_region(&self, region: &str) -> &ShardConfig {
        self.config
            .shards
            .iter()
            .find(|s| s.active && s.region.as_deref() == Some(region))
            .unwrap_or_else(|| self.get_default_shard())
    }

    /// Get shard by round robin (stateless, uses current time)
    fn get_shard_round_robin(&self) -> &ShardConfig {
        let active_shards: Vec<_> = self.config.shards.iter().filter(|s| s.active).collect();
        if active_shards.is_empty() {
            return self.get_default_shard();
        }
        let idx = (Utc::now().timestamp_millis() as usize) % active_shards.len();
        active_shards[idx]
    }

    /// Get default shard
    fn get_default_shard(&self) -> &ShardConfig {
        self.config
            .shards
            .iter()
            .find(|s| s.id == self.config.default_shard)
            .unwrap_or(&self.config.shards[0])
    }

    /// Get all shards for scatter-gather query
    pub fn get_all_shards(&self) -> Vec<&ShardConfig> {
        self.config.shards.iter().filter(|s| s.active).collect()
    }
}

// ============================================================================
// Consistent Hash Ring
// ============================================================================

/// Consistent hash ring for shard distribution
struct ConsistentHashRing {
    ring: Vec<(u64, String)>,
}

impl ConsistentHashRing {
    fn new(config: &ShardingConfig) -> Self {
        let mut ring = Vec::new();

        for shard in &config.shards {
            if !shard.active {
                continue;
            }
            // Add virtual nodes for each shard
            let vnodes = (config.virtual_shards * shard.weight) / 100;
            for i in 0..vnodes {
                let key = format!("{}:{}", shard.id, i);
                let hash = Self::hash(&key);
                ring.push((hash, shard.id.clone()));
            }
        }

        ring.sort_by_key(|(hash, _)| *hash);
        Self { ring }
    }

    fn hash(key: &str) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        key.hash(&mut hasher);
        hasher.finish()
    }

    fn get_node(&self, key: &str) -> String {
        if self.ring.is_empty() {
            return "default".to_string();
        }

        let hash = Self::hash(key);

        // Binary search for the first node >= hash
        let idx = match self.ring.binary_search_by_key(&hash, |(h, _)| *h) {
            Ok(i) => i,
            Err(i) => {
                if i >= self.ring.len() {
                    0 // Wrap around
                } else {
                    i
                }
            }
        };

        self.ring[idx].1.clone()
    }
}

// ============================================================================
// Read Replica Configuration
// ============================================================================

/// Read replica configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicaConfig {
    /// Replica ID
    pub id: String,
    /// Replica name
    pub name: String,
    /// Connection string
    pub connection_string: String,
    /// Region
    pub region: Option<String>,
    /// Priority (lower = preferred)
    pub priority: u32,
    /// Maximum lag allowed (seconds)
    pub max_lag_seconds: u32,
    /// Whether replica is active
    pub active: bool,
    /// Current lag (updated dynamically)
    #[serde(skip)]
    pub current_lag_ms: u64,
}

/// Read replica manager
pub struct ReplicaManager {
    primary: String,
    replicas: Vec<ReplicaConfig>,
    health_status: Arc<RwLock<HashMap<String, ReplicaHealth>>>,
}

#[derive(Debug, Clone)]
struct ReplicaHealth {
    is_healthy: bool,
    last_check: DateTime<Utc>,
    lag_ms: u64,
    error_count: u32,
}

impl ReplicaManager {
    /// Create a new replica manager
    pub fn new(primary: String, replicas: Vec<ReplicaConfig>) -> Self {
        Self {
            primary,
            replicas,
            health_status: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get the best replica for read operations
    pub async fn get_read_replica(&self, preferred_region: Option<&str>) -> String {
        let health = self.health_status.read().await;

        // Filter healthy, active replicas
        let mut candidates: Vec<_> = self
            .replicas
            .iter()
            .filter(|r| {
                r.active
                    && health
                        .get(&r.id)
                        .map(|h| h.is_healthy && h.lag_ms < (r.max_lag_seconds as u64 * 1000))
                        .unwrap_or(false)
            })
            .collect();

        if candidates.is_empty() {
            return self.primary.clone();
        }

        // Prefer same region
        if let Some(region) = preferred_region {
            let regional: Vec<_> = candidates
                .iter()
                .filter(|r| r.region.as_deref() == Some(region))
                .copied()
                .collect();
            if !regional.is_empty() {
                candidates = regional;
            }
        }

        // Sort by priority and lag
        candidates.sort_by(|a, b| {
            let lag_a = health.get(&a.id).map(|h| h.lag_ms).unwrap_or(u64::MAX);
            let lag_b = health.get(&b.id).map(|h| h.lag_ms).unwrap_or(u64::MAX);
            a.priority.cmp(&b.priority).then(lag_a.cmp(&lag_b))
        });

        candidates
            .first()
            .map(|r| r.connection_string.clone())
            .unwrap_or_else(|| self.primary.clone())
    }

    /// Get primary connection for write operations
    pub fn get_primary(&self) -> &str {
        &self.primary
    }

    /// Update replica health status
    pub async fn update_health(&self, replica_id: &str, is_healthy: bool, lag_ms: u64) {
        let mut health = self.health_status.write().await;
        let entry = health.entry(replica_id.to_string()).or_insert(ReplicaHealth {
            is_healthy: true,
            last_check: Utc::now(),
            lag_ms: 0,
            error_count: 0,
        });

        entry.is_healthy = is_healthy;
        entry.last_check = Utc::now();
        entry.lag_ms = lag_ms;
        if !is_healthy {
            entry.error_count += 1;
        } else {
            entry.error_count = 0;
        }
    }

    /// Health check all replicas
    pub async fn health_check_all(&self) {
        for replica in &self.replicas {
            if !replica.active {
                continue;
            }

            // In a real implementation, ping the replica and measure lag
            let (is_healthy, lag_ms) = self.check_replica_health(&replica.connection_string).await;
            self.update_health(&replica.id, is_healthy, lag_ms).await;
        }
    }

    async fn check_replica_health(&self, _connection_string: &str) -> (bool, u64) {
        // Simulate health check
        (true, 50)
    }
}

// ============================================================================
// Connection Pool
// ============================================================================

/// Connection pool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolConfig {
    /// Minimum connections
    pub min_connections: u32,
    /// Maximum connections
    pub max_connections: u32,
    /// Connection timeout (seconds)
    pub connect_timeout_seconds: u32,
    /// Idle timeout (seconds)
    pub idle_timeout_seconds: u32,
    /// Max lifetime (seconds)
    pub max_lifetime_seconds: u32,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            min_connections: 5,
            max_connections: 20,
            connect_timeout_seconds: 30,
            idle_timeout_seconds: 300,
            max_lifetime_seconds: 1800,
        }
    }
}

// ============================================================================
// Database Scaling Manager
// ============================================================================

/// Manages all database scaling features
pub struct ScalingManager {
    shard_router: ShardRouter,
    replica_manager: ReplicaManager,
    pool_config: PoolConfig,
}

impl ScalingManager {
    /// Create a new scaling manager
    pub fn new(
        sharding_config: ShardingConfig,
        primary: String,
        replicas: Vec<ReplicaConfig>,
        pool_config: PoolConfig,
    ) -> Self {
        Self {
            shard_router: ShardRouter::new(sharding_config),
            replica_manager: ReplicaManager::new(primary, replicas),
            pool_config,
        }
    }

    /// Get connection for write operation
    pub fn get_write_connection(&self, key: &str) -> &str {
        let shard = self.shard_router.get_shard(key);
        &shard.connection_string
    }

    /// Get connection for read operation
    pub async fn get_read_connection(&self, key: &str, region: Option<&str>) -> String {
        // For sharded data, get the shard first
        let _shard = self.shard_router.get_shard(key);
        // Then get a replica if available
        self.replica_manager.get_read_replica(region).await
    }

    /// Get all shards for scatter-gather
    pub fn get_all_shards(&self) -> Vec<&ShardConfig> {
        self.shard_router.get_all_shards()
    }

    /// Health check
    pub async fn health_check(&self) {
        self.replica_manager.health_check_all().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consistent_hash_ring() {
        let config = ShardingConfig::default();
        let ring = ConsistentHashRing::new(&config);

        // Same key should always return same node
        let node1 = ring.get_node("test_key");
        let node2 = ring.get_node("test_key");
        assert_eq!(node1, node2);
    }

    #[test]
    fn test_shard_router() {
        let config = ShardingConfig {
            strategy: ShardingStrategy::Hash,
            shards: vec![
                ShardConfig {
                    id: "shard1".to_string(),
                    name: "Shard 1".to_string(),
                    connection_string: "sqlite://shard1.db".to_string(),
                    weight: 50,
                    active: true,
                    region: None,
                    range_min: None,
                    range_max: None,
                    tenant_ids: vec![],
                },
                ShardConfig {
                    id: "shard2".to_string(),
                    name: "Shard 2".to_string(),
                    connection_string: "sqlite://shard2.db".to_string(),
                    weight: 50,
                    active: true,
                    region: None,
                    range_min: None,
                    range_max: None,
                    tenant_ids: vec![],
                },
            ],
            default_shard: "shard1".to_string(),
            ..Default::default()
        };

        let router = ShardRouter::new(config);
        let shard = router.get_shard("some_key");
        assert!(shard.id == "shard1" || shard.id == "shard2");
    }
}
