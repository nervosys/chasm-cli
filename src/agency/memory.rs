// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! Memory and RAG (Retrieval-Augmented Generation) System
//!
//! Provides persistent context, knowledge retrieval, and semantic search for agents.
//!
//! ## Features
//!
//! - **Vector Store**: Semantic similarity search using embeddings
//! - **Memory Types**: Short-term, long-term, episodic, and semantic memory
//! - **Knowledge Base**: Structured document storage with chunking
//! - **Context Window**: Smart context management for LLM prompts

#![allow(dead_code)]
//! - **Caching**: Frequently accessed information caching

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

// =============================================================================
// Core Types
// =============================================================================

/// Unique identifier for memory entries
pub type MemoryId = String;

/// Vector embedding (typically 384-1536 dimensions depending on model)
pub type Embedding = Vec<f32>;

/// Memory entry representing a piece of stored knowledge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    /// Unique identifier
    pub id: MemoryId,
    /// The content/text of this memory
    pub content: String,
    /// Vector embedding for similarity search
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding: Option<Embedding>,
    /// Memory type classification
    pub memory_type: MemoryType,
    /// Source of this memory (conversation, document, user input, etc.)
    pub source: MemorySource,
    /// Importance score (0.0 - 1.0)
    pub importance: f32,
    /// Access count for LRU caching
    pub access_count: u64,
    /// Last accessed timestamp
    pub last_accessed: DateTime<Utc>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Optional expiration
    pub expires_at: Option<DateTime<Utc>>,
    /// Associated agent ID
    pub agent_id: Option<String>,
    /// Associated session ID
    pub session_id: Option<String>,
    /// Custom metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// Tags for filtering
    pub tags: Vec<String>,
}

impl MemoryEntry {
    /// Create a new memory entry
    pub fn new(content: impl Into<String>, memory_type: MemoryType, source: MemorySource) -> Self {
        let now = Utc::now();
        Self {
            id: generate_memory_id(),
            content: content.into(),
            embedding: None,
            memory_type,
            source,
            importance: 0.5,
            access_count: 0,
            last_accessed: now,
            created_at: now,
            expires_at: None,
            agent_id: None,
            session_id: None,
            metadata: HashMap::new(),
            tags: Vec::new(),
        }
    }

    /// Set the embedding
    pub fn with_embedding(mut self, embedding: Embedding) -> Self {
        self.embedding = Some(embedding);
        self
    }

    /// Set importance score
    pub fn with_importance(mut self, importance: f32) -> Self {
        self.importance = importance.clamp(0.0, 1.0);
        self
    }

    /// Set agent ID
    pub fn with_agent(mut self, agent_id: impl Into<String>) -> Self {
        self.agent_id = Some(agent_id.into());
        self
    }

    /// Set session ID
    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// Add a tag
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Set expiration
    pub fn expires_in(mut self, duration: chrono::Duration) -> Self {
        self.expires_at = Some(Utc::now() + duration);
        self
    }

    /// Check if expired
    pub fn is_expired(&self) -> bool {
        self.expires_at.map(|exp| Utc::now() > exp).unwrap_or(false)
    }
}

/// Types of memory
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryType {
    /// Short-term working memory (current conversation context)
    ShortTerm,
    /// Long-term persistent memory (facts, preferences, learned info)
    LongTerm,
    /// Episodic memory (specific events and experiences)
    Episodic,
    /// Semantic memory (concepts, relationships, general knowledge)
    Semantic,
    /// Procedural memory (how to do things, workflows)
    Procedural,
    /// User preferences and settings
    Preference,
    /// Cached computation results
    Cache,
}

/// Source of memory entry
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemorySource {
    /// From a conversation message
    Conversation {
        session_id: String,
        message_id: String,
    },
    /// From a document/file
    Document { path: String, chunk_index: u32 },
    /// Direct user input/instruction
    UserInput,
    /// Agent reasoning/reflection
    AgentReasoning { agent_id: String },
    /// External API or tool result
    ToolResult { tool_name: String },
    /// Web page or URL
    WebPage { url: String },
    /// System-generated summary
    Summary { source_ids: Vec<String> },
    /// Custom source
    Custom { source_type: String },
}

// =============================================================================
// Vector Store
// =============================================================================

/// Configuration for the vector store
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorStoreConfig {
    /// Embedding model to use
    pub embedding_model: EmbeddingModel,
    /// Dimension of embeddings
    pub embedding_dim: usize,
    /// Similarity metric
    pub similarity_metric: SimilarityMetric,
    /// Maximum entries before pruning
    pub max_entries: usize,
    /// Database path
    pub db_path: Option<String>,
}

impl Default for VectorStoreConfig {
    fn default() -> Self {
        Self {
            embedding_model: EmbeddingModel::default(),
            embedding_dim: 384,
            similarity_metric: SimilarityMetric::Cosine,
            max_entries: 100_000,
            db_path: None,
        }
    }
}

/// Embedding model options
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmbeddingModel {
    /// OpenAI text-embedding-3-small (1536 dims)
    OpenAISmall,
    /// OpenAI text-embedding-3-large (3072 dims)
    OpenAILarge,
    /// OpenAI text-embedding-ada-002 (1536 dims)
    OpenAIAda,
    /// Sentence Transformers all-MiniLM-L6-v2 (384 dims)
    #[default]
    MiniLM,
    /// Sentence Transformers all-mpnet-base-v2 (768 dims)
    MPNet,
    /// Cohere embed-english-v3.0 (1024 dims)
    Cohere,
    /// Google text-embedding-004 (768 dims)
    GoogleGecko,
    /// Voyage AI voyage-2 (1024 dims)
    Voyage,
    /// Local model via Ollama
    Ollama { model: String },
    /// Custom model
    Custom { name: String, dim: usize },
}

impl EmbeddingModel {
    /// Get the dimension for this model
    pub fn dimension(&self) -> usize {
        match self {
            EmbeddingModel::OpenAISmall => 1536,
            EmbeddingModel::OpenAILarge => 3072,
            EmbeddingModel::OpenAIAda => 1536,
            EmbeddingModel::MiniLM => 384,
            EmbeddingModel::MPNet => 768,
            EmbeddingModel::Cohere => 1024,
            EmbeddingModel::GoogleGecko => 768,
            EmbeddingModel::Voyage => 1024,
            EmbeddingModel::Ollama { .. } => 4096, // Typical for Ollama models
            EmbeddingModel::Custom { dim, .. } => *dim,
        }
    }
}

/// Similarity metrics for vector search
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SimilarityMetric {
    #[default]
    Cosine,
    Euclidean,
    DotProduct,
    Manhattan,
}

impl SimilarityMetric {
    /// Calculate similarity between two vectors
    pub fn calculate(&self, a: &[f32], b: &[f32]) -> f32 {
        assert_eq!(a.len(), b.len(), "Vector dimensions must match");

        match self {
            SimilarityMetric::Cosine => {
                let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
                let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
                let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
                if norm_a == 0.0 || norm_b == 0.0 {
                    0.0
                } else {
                    dot / (norm_a * norm_b)
                }
            }
            SimilarityMetric::Euclidean => {
                let dist: f32 = a
                    .iter()
                    .zip(b.iter())
                    .map(|(x, y)| (x - y).powi(2))
                    .sum::<f32>()
                    .sqrt();
                1.0 / (1.0 + dist) // Convert distance to similarity
            }
            SimilarityMetric::DotProduct => a.iter().zip(b.iter()).map(|(x, y)| x * y).sum(),
            SimilarityMetric::Manhattan => {
                let dist: f32 = a.iter().zip(b.iter()).map(|(x, y)| (x - y).abs()).sum();
                1.0 / (1.0 + dist)
            }
        }
    }
}

/// Search result from vector store
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// The memory entry
    pub entry: MemoryEntry,
    /// Similarity score (0.0 - 1.0)
    pub score: f32,
    /// Rank in results
    pub rank: usize,
}

/// Vector store for semantic search
pub struct VectorStore {
    config: VectorStoreConfig,
    entries: Vec<MemoryEntry>,
    db: Option<rusqlite::Connection>,
}

impl VectorStore {
    /// Create a new in-memory vector store
    pub fn new(config: VectorStoreConfig) -> Self {
        Self {
            config,
            entries: Vec::new(),
            db: None,
        }
    }

    /// Create a vector store with SQLite persistence
    pub fn with_persistence(
        config: VectorStoreConfig,
        db_path: impl AsRef<Path>,
    ) -> Result<Self, MemoryError> {
        let db = rusqlite::Connection::open(db_path.as_ref())
            .map_err(|e| MemoryError::Database(e.to_string()))?;

        // Initialize schema
        db.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS memory_entries (
                id TEXT PRIMARY KEY,
                content TEXT NOT NULL,
                embedding BLOB,
                memory_type TEXT NOT NULL,
                source TEXT NOT NULL,
                importance REAL NOT NULL,
                access_count INTEGER NOT NULL DEFAULT 0,
                last_accessed TEXT NOT NULL,
                created_at TEXT NOT NULL,
                expires_at TEXT,
                agent_id TEXT,
                session_id TEXT,
                metadata TEXT,
                tags TEXT
            );
            
            CREATE INDEX IF NOT EXISTS idx_memory_type ON memory_entries(memory_type);
            CREATE INDEX IF NOT EXISTS idx_agent_id ON memory_entries(agent_id);
            CREATE INDEX IF NOT EXISTS idx_session_id ON memory_entries(session_id);
            CREATE INDEX IF NOT EXISTS idx_created_at ON memory_entries(created_at);
            CREATE INDEX IF NOT EXISTS idx_importance ON memory_entries(importance DESC);
        "#,
        )
        .map_err(|e| MemoryError::Database(e.to_string()))?;

        let mut store = Self {
            config,
            entries: Vec::new(),
            db: Some(db),
        };

        store.load_from_db()?;
        Ok(store)
    }

    /// Load entries from database
    fn load_from_db(&mut self) -> Result<(), MemoryError> {
        if let Some(ref db) = self.db {
            let mut stmt = db
                .prepare(
                    "SELECT id, content, embedding, memory_type, source, importance, 
                        access_count, last_accessed, created_at, expires_at, 
                        agent_id, session_id, metadata, tags 
                 FROM memory_entries 
                 ORDER BY importance DESC, created_at DESC",
                )
                .map_err(|e| MemoryError::Database(e.to_string()))?;

            let entries = stmt
                .query_map([], |row| {
                    let embedding_blob: Option<Vec<u8>> = row.get(2)?;
                    let embedding = embedding_blob.map(|blob| {
                        blob.chunks(4)
                            .map(|chunk| f32::from_le_bytes(chunk.try_into().unwrap_or([0; 4])))
                            .collect()
                    });

                    Ok(MemoryEntry {
                        id: row.get(0)?,
                        content: row.get(1)?,
                        embedding,
                        memory_type: serde_json::from_str(&row.get::<_, String>(3)?)
                            .unwrap_or(MemoryType::LongTerm),
                        source: serde_json::from_str(&row.get::<_, String>(4)?)
                            .unwrap_or(MemorySource::UserInput),
                        importance: row.get(5)?,
                        access_count: row.get(6)?,
                        last_accessed: row
                            .get::<_, String>(7)?
                            .parse()
                            .unwrap_or_else(|_| Utc::now()),
                        created_at: row
                            .get::<_, String>(8)?
                            .parse()
                            .unwrap_or_else(|_| Utc::now()),
                        expires_at: row
                            .get::<_, Option<String>>(9)?
                            .and_then(|s| s.parse().ok()),
                        agent_id: row.get(10)?,
                        session_id: row.get(11)?,
                        metadata: row
                            .get::<_, Option<String>>(12)?
                            .and_then(|s| serde_json::from_str(&s).ok())
                            .unwrap_or_default(),
                        tags: row
                            .get::<_, Option<String>>(13)?
                            .and_then(|s| serde_json::from_str(&s).ok())
                            .unwrap_or_default(),
                    })
                })
                .map_err(|e| MemoryError::Database(e.to_string()))?;

            self.entries = entries.filter_map(|e| e.ok()).collect();
        }
        Ok(())
    }

    /// Add a memory entry
    pub fn add(&mut self, entry: MemoryEntry) -> Result<MemoryId, MemoryError> {
        let id = entry.id.clone();

        // Persist to database if available
        if let Some(ref db) = self.db {
            let embedding_blob: Option<Vec<u8>> = entry
                .embedding
                .as_ref()
                .map(|emb| emb.iter().flat_map(|f| f.to_le_bytes()).collect());

            db.execute(
                "INSERT OR REPLACE INTO memory_entries 
                 (id, content, embedding, memory_type, source, importance, 
                  access_count, last_accessed, created_at, expires_at, 
                  agent_id, session_id, metadata, tags)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
                rusqlite::params![
                    entry.id,
                    entry.content,
                    embedding_blob,
                    serde_json::to_string(&entry.memory_type).unwrap_or_default(),
                    serde_json::to_string(&entry.source).unwrap_or_default(),
                    entry.importance,
                    entry.access_count,
                    entry.last_accessed.to_rfc3339(),
                    entry.created_at.to_rfc3339(),
                    entry.expires_at.map(|e| e.to_rfc3339()),
                    entry.agent_id,
                    entry.session_id,
                    serde_json::to_string(&entry.metadata).ok(),
                    serde_json::to_string(&entry.tags).ok(),
                ],
            )
            .map_err(|e| MemoryError::Database(e.to_string()))?;
        }

        self.entries.push(entry);

        // Prune if needed
        if self.entries.len() > self.config.max_entries {
            self.prune()?;
        }

        Ok(id)
    }

    /// Search for similar entries
    pub fn search(&mut self, query_embedding: &Embedding, limit: usize) -> Vec<SearchResult> {
        let mut results: Vec<(usize, f32)> = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| !e.is_expired() && e.embedding.is_some())
            .map(|(i, e)| {
                let score = self
                    .config
                    .similarity_metric
                    .calculate(query_embedding, e.embedding.as_ref().unwrap());
                (i, score)
            })
            .collect();

        // Sort by score descending
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Take top results and update access counts
        results
            .into_iter()
            .take(limit)
            .enumerate()
            .map(|(rank, (idx, score))| {
                self.entries[idx].access_count += 1;
                self.entries[idx].last_accessed = Utc::now();

                SearchResult {
                    entry: self.entries[idx].clone(),
                    score,
                    rank,
                }
            })
            .collect()
    }

    /// Search by memory type
    pub fn search_by_type(&self, memory_type: MemoryType, limit: usize) -> Vec<&MemoryEntry> {
        self.entries
            .iter()
            .filter(|e| e.memory_type == memory_type && !e.is_expired())
            .take(limit)
            .collect()
    }

    /// Search by tags
    pub fn search_by_tags(&self, tags: &[String], limit: usize) -> Vec<&MemoryEntry> {
        self.entries
            .iter()
            .filter(|e| !e.is_expired() && tags.iter().any(|t| e.tags.contains(t)))
            .take(limit)
            .collect()
    }

    /// Get entry by ID
    pub fn get(&self, id: &str) -> Option<&MemoryEntry> {
        self.entries.iter().find(|e| e.id == id)
    }

    /// Delete entry
    pub fn delete(&mut self, id: &str) -> Result<bool, MemoryError> {
        if let Some(pos) = self.entries.iter().position(|e| e.id == id) {
            self.entries.remove(pos);

            if let Some(ref db) = self.db {
                db.execute("DELETE FROM memory_entries WHERE id = ?1", [id])
                    .map_err(|e| MemoryError::Database(e.to_string()))?;
            }

            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Prune old/low-importance entries
    fn prune(&mut self) -> Result<(), MemoryError> {
        // Remove expired entries
        self.entries.retain(|e| !e.is_expired());

        // If still over limit, remove lowest importance entries
        if self.entries.len() > self.config.max_entries {
            self.entries.sort_by(|a, b| {
                b.importance
                    .partial_cmp(&a.importance)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            self.entries.truncate(self.config.max_entries);
        }

        Ok(())
    }

    /// Get statistics
    pub fn stats(&self) -> VectorStoreStats {
        VectorStoreStats {
            total_entries: self.entries.len(),
            entries_by_type: self.entries.iter().fold(HashMap::new(), |mut acc, e| {
                *acc.entry(format!("{:?}", e.memory_type)).or_insert(0) += 1;
                acc
            }),
            total_access_count: self.entries.iter().map(|e| e.access_count).sum(),
            avg_importance: if self.entries.is_empty() {
                0.0
            } else {
                self.entries.iter().map(|e| e.importance).sum::<f32>() / self.entries.len() as f32
            },
        }
    }
}

/// Vector store statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorStoreStats {
    pub total_entries: usize,
    pub entries_by_type: HashMap<String, usize>,
    pub total_access_count: u64,
    pub avg_importance: f32,
}

// =============================================================================
// Knowledge Base / RAG
// =============================================================================

/// Document for the knowledge base
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    /// Unique identifier
    pub id: String,
    /// Document title
    pub title: String,
    /// Full content
    pub content: String,
    /// Document type
    pub doc_type: DocumentType,
    /// Source URL or path
    pub source: String,
    /// Chunked content for embedding
    pub chunks: Vec<DocumentChunk>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last updated
    pub updated_at: DateTime<Utc>,
    /// Metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Document types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentType {
    Text,
    Markdown,
    Code { language: String },
    Html,
    Pdf,
    Json,
    Yaml,
    Csv,
    Custom { mime_type: String },
}

/// A chunk of a document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentChunk {
    /// Chunk index
    pub index: u32,
    /// Chunk content
    pub content: String,
    /// Start position in original document
    pub start_pos: usize,
    /// End position in original document
    pub end_pos: usize,
    /// Vector embedding
    pub embedding: Option<Embedding>,
    /// Token count estimate
    pub token_count: u32,
}

/// Configuration for document chunking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkingConfig {
    /// Target chunk size in tokens
    pub chunk_size: usize,
    /// Overlap between chunks in tokens
    pub chunk_overlap: usize,
    /// Chunking strategy
    pub strategy: ChunkingStrategy,
}

impl Default for ChunkingConfig {
    fn default() -> Self {
        Self {
            chunk_size: 512,
            chunk_overlap: 50,
            strategy: ChunkingStrategy::Semantic,
        }
    }
}

/// Chunking strategies
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChunkingStrategy {
    /// Fixed-size character chunks
    FixedSize,
    /// Split on sentences
    Sentence,
    /// Split on paragraphs
    Paragraph,
    /// Semantic chunking (respects structure)
    #[default]
    Semantic,
    /// Code-aware chunking
    Code,
}

/// Knowledge base for RAG
pub struct KnowledgeBase {
    /// Vector store for semantic search
    vector_store: VectorStore,
    /// Documents
    documents: HashMap<String, Document>,
    /// Chunking configuration
    chunking_config: ChunkingConfig,
}

impl KnowledgeBase {
    /// Create a new knowledge base
    pub fn new(vector_config: VectorStoreConfig) -> Self {
        Self {
            vector_store: VectorStore::new(vector_config),
            documents: HashMap::new(),
            chunking_config: ChunkingConfig::default(),
        }
    }

    /// Create with persistence
    pub fn with_persistence(
        vector_config: VectorStoreConfig,
        db_path: impl AsRef<Path>,
    ) -> Result<Self, MemoryError> {
        Ok(Self {
            vector_store: VectorStore::with_persistence(vector_config, db_path)?,
            documents: HashMap::new(),
            chunking_config: ChunkingConfig::default(),
        })
    }

    /// Add a document
    pub fn add_document(&mut self, mut document: Document) -> Result<String, MemoryError> {
        // Chunk the document
        document.chunks = self.chunk_document(&document.content);

        let doc_id = document.id.clone();

        // Add chunks to vector store
        for chunk in &document.chunks {
            if let Some(ref embedding) = chunk.embedding {
                let entry = MemoryEntry::new(
                    &chunk.content,
                    MemoryType::Semantic,
                    MemorySource::Document {
                        path: document.source.clone(),
                        chunk_index: chunk.index,
                    },
                )
                .with_embedding(embedding.clone())
                .with_tag(format!("doc:{}", doc_id));

                self.vector_store.add(entry)?;
            }
        }

        self.documents.insert(doc_id.clone(), document);
        Ok(doc_id)
    }

    /// Chunk a document
    fn chunk_document(&self, content: &str) -> Vec<DocumentChunk> {
        match self.chunking_config.strategy {
            ChunkingStrategy::Semantic => self.semantic_chunk(content),
            ChunkingStrategy::Paragraph => self.paragraph_chunk(content),
            ChunkingStrategy::Sentence => self.sentence_chunk(content),
            ChunkingStrategy::FixedSize => self.fixed_chunk(content),
            ChunkingStrategy::Code => self.code_chunk(content),
        }
    }

    fn semantic_chunk(&self, content: &str) -> Vec<DocumentChunk> {
        // Split on double newlines (paragraphs) but respect size limits
        let mut chunks = Vec::new();
        let mut current_chunk = String::new();
        let mut start_pos = 0;
        let mut chunk_index = 0;

        for para in content.split("\n\n") {
            let para = para.trim();
            if para.is_empty() {
                continue;
            }

            let para_tokens = estimate_tokens(para);
            let current_tokens = estimate_tokens(&current_chunk);

            if current_tokens + para_tokens > self.chunking_config.chunk_size
                && !current_chunk.is_empty()
            {
                // Save current chunk
                let end_pos = start_pos + current_chunk.len();
                chunks.push(DocumentChunk {
                    index: chunk_index,
                    content: current_chunk.trim().to_string(),
                    start_pos,
                    end_pos,
                    embedding: None,
                    token_count: estimate_tokens(&current_chunk) as u32,
                });
                chunk_index += 1;
                start_pos = end_pos;
                current_chunk = String::new();
            }

            if !current_chunk.is_empty() {
                current_chunk.push_str("\n\n");
            }
            current_chunk.push_str(para);
        }

        // Add remaining content
        if !current_chunk.is_empty() {
            let end_pos = start_pos + current_chunk.len();
            chunks.push(DocumentChunk {
                index: chunk_index,
                content: current_chunk.trim().to_string(),
                start_pos,
                end_pos,
                embedding: None,
                token_count: estimate_tokens(&current_chunk) as u32,
            });
        }

        chunks
    }

    fn paragraph_chunk(&self, content: &str) -> Vec<DocumentChunk> {
        content
            .split("\n\n")
            .filter(|p| !p.trim().is_empty())
            .enumerate()
            .scan(0usize, |pos, (i, para)| {
                let start = *pos;
                *pos += para.len() + 2;
                Some(DocumentChunk {
                    index: i as u32,
                    content: para.trim().to_string(),
                    start_pos: start,
                    end_pos: *pos,
                    embedding: None,
                    token_count: estimate_tokens(para) as u32,
                })
            })
            .collect()
    }

    fn sentence_chunk(&self, content: &str) -> Vec<DocumentChunk> {
        // Simple sentence splitting (could be improved with NLP)
        let sentences: Vec<&str> = content
            .split(['.', '!', '?'])
            .filter(|s| !s.trim().is_empty())
            .collect();

        let mut chunks = Vec::new();
        let mut current = String::new();
        let mut start = 0;
        let mut idx = 0;

        for sentence in sentences {
            let sentence = sentence.trim();
            if estimate_tokens(&current) + estimate_tokens(sentence)
                > self.chunking_config.chunk_size
                && !current.is_empty()
            {
                chunks.push(DocumentChunk {
                    index: idx,
                    content: current.clone(),
                    start_pos: start,
                    end_pos: start + current.len(),
                    embedding: None,
                    token_count: estimate_tokens(&current) as u32,
                });
                idx += 1;
                start += current.len();
                current.clear();
            }
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(sentence);
            current.push('.');
        }

        if !current.is_empty() {
            chunks.push(DocumentChunk {
                index: idx,
                content: current.clone(),
                start_pos: start,
                end_pos: start + current.len(),
                embedding: None,
                token_count: estimate_tokens(&current) as u32,
            });
        }

        chunks
    }

    fn fixed_chunk(&self, content: &str) -> Vec<DocumentChunk> {
        let chars_per_chunk = self.chunking_config.chunk_size * 4; // Rough estimate
        content
            .chars()
            .collect::<Vec<_>>()
            .chunks(chars_per_chunk)
            .enumerate()
            .map(|(i, chars)| {
                let s: String = chars.iter().collect();
                DocumentChunk {
                    index: i as u32,
                    content: s.clone(),
                    start_pos: i * chars_per_chunk,
                    end_pos: (i + 1) * chars_per_chunk,
                    embedding: None,
                    token_count: estimate_tokens(&s) as u32,
                }
            })
            .collect()
    }

    fn code_chunk(&self, content: &str) -> Vec<DocumentChunk> {
        // Split on function/class definitions (simple heuristic)
        let mut chunks = Vec::new();
        let mut current = String::new();
        let mut start = 0;
        let mut idx = 0;

        for line in content.lines() {
            let is_boundary = line.starts_with("fn ")
                || line.starts_with("pub fn ")
                || line.starts_with("async fn ")
                || line.starts_with("impl ")
                || line.starts_with("struct ")
                || line.starts_with("enum ")
                || line.starts_with("trait ")
                || line.starts_with("class ")
                || line.starts_with("def ")
                || line.starts_with("function ")
                || line.starts_with("const ")
                || line.starts_with("export ");

            if is_boundary && !current.is_empty() {
                chunks.push(DocumentChunk {
                    index: idx,
                    content: current.clone(),
                    start_pos: start,
                    end_pos: start + current.len(),
                    embedding: None,
                    token_count: estimate_tokens(&current) as u32,
                });
                idx += 1;
                start += current.len();
                current.clear();
            }

            current.push_str(line);
            current.push('\n');
        }

        if !current.is_empty() {
            chunks.push(DocumentChunk {
                index: idx,
                content: current.clone(),
                start_pos: start,
                end_pos: start + current.len(),
                embedding: None,
                token_count: estimate_tokens(&current) as u32,
            });
        }

        chunks
    }

    /// Retrieve relevant context for a query
    pub fn retrieve(&mut self, query_embedding: &Embedding, limit: usize) -> Vec<SearchResult> {
        self.vector_store.search(query_embedding, limit)
    }

    /// Get document by ID
    pub fn get_document(&self, id: &str) -> Option<&Document> {
        self.documents.get(id)
    }

    /// List all documents
    pub fn list_documents(&self) -> Vec<&Document> {
        self.documents.values().collect()
    }

    /// Delete document
    pub fn delete_document(&mut self, id: &str) -> bool {
        self.documents.remove(id).is_some()
    }
}

// =============================================================================
// Context Window Manager
// =============================================================================

/// Manages context for LLM prompts
#[derive(Debug, Clone)]
pub struct ContextWindow {
    /// Maximum tokens for context
    pub max_tokens: usize,
    /// Reserved tokens for response
    pub reserved_for_response: usize,
    /// Context segments
    segments: Vec<ContextSegment>,
}

/// A segment of context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSegment {
    /// Segment type
    pub segment_type: ContextSegmentType,
    /// Content
    pub content: String,
    /// Token count
    pub tokens: usize,
    /// Priority (higher = more important)
    pub priority: u32,
    /// Whether this segment is required
    pub required: bool,
}

/// Types of context segments
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextSegmentType {
    SystemPrompt,
    UserPreferences,
    ConversationHistory,
    RetrievedContext,
    ToolResults,
    CurrentQuery,
    Custom { name: String },
}

impl ContextWindow {
    /// Create a new context window
    pub fn new(max_tokens: usize) -> Self {
        Self {
            max_tokens,
            reserved_for_response: max_tokens / 4, // Reserve 25% for response
            segments: Vec::new(),
        }
    }

    /// Add a context segment
    pub fn add_segment(&mut self, segment: ContextSegment) {
        self.segments.push(segment);
    }

    /// Build the final context, respecting token limits
    pub fn build(&mut self) -> String {
        let available = self.max_tokens - self.reserved_for_response;

        // Sort by priority (required first, then by priority)
        self.segments
            .sort_by(|a, b| match (a.required, b.required) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => b.priority.cmp(&a.priority),
            });

        let mut total_tokens = 0;
        let mut result = Vec::new();

        for segment in &self.segments {
            if total_tokens + segment.tokens <= available {
                result.push(segment.content.clone());
                total_tokens += segment.tokens;
            } else if segment.required {
                // Truncate if required
                let remaining = available.saturating_sub(total_tokens);
                if remaining > 0 {
                    let truncated = truncate_to_tokens(&segment.content, remaining);
                    result.push(truncated);
                    break;
                }
            }
        }

        result.join("\n\n")
    }

    /// Get current token usage
    pub fn token_usage(&self) -> (usize, usize) {
        let used: usize = self.segments.iter().map(|s| s.tokens).sum();
        (used, self.max_tokens - self.reserved_for_response)
    }
}

// =============================================================================
// Cache
// =============================================================================

/// Cache entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry<T> {
    pub key: String,
    pub value: T,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub access_count: u64,
}

impl<T> CacheEntry<T> {
    pub fn is_expired(&self) -> bool {
        self.expires_at.map(|exp| Utc::now() > exp).unwrap_or(false)
    }
}

/// Simple LRU cache for agent computations
pub struct AgentCache<T> {
    entries: HashMap<String, CacheEntry<T>>,
    max_size: usize,
}

impl<T: Clone> AgentCache<T> {
    /// Create a new cache
    pub fn new(max_size: usize) -> Self {
        Self {
            entries: HashMap::new(),
            max_size,
        }
    }

    /// Get a value from cache
    pub fn get(&mut self, key: &str) -> Option<T> {
        if let Some(entry) = self.entries.get_mut(key) {
            if entry.is_expired() {
                self.entries.remove(key);
                return None;
            }
            entry.access_count += 1;
            Some(entry.value.clone())
        } else {
            None
        }
    }

    /// Set a value in cache
    pub fn set(&mut self, key: impl Into<String>, value: T, ttl: Option<chrono::Duration>) {
        let key = key.into();
        let now = Utc::now();

        self.entries.insert(
            key.clone(),
            CacheEntry {
                key,
                value,
                created_at: now,
                expires_at: ttl.map(|d| now + d),
                access_count: 0,
            },
        );

        // Evict if over size
        if self.entries.len() > self.max_size {
            self.evict_lru();
        }
    }

    /// Remove a value
    pub fn remove(&mut self, key: &str) -> Option<T> {
        self.entries.remove(key).map(|e| e.value)
    }

    /// Clear the cache
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Evict least recently used entries
    fn evict_lru(&mut self) {
        // Remove expired first
        self.entries.retain(|_, v| !v.is_expired());

        // If still over, remove least accessed
        if self.entries.len() > self.max_size {
            // Collect keys to remove (sorted by access count)
            let mut entries: Vec<_> = self
                .entries
                .iter()
                .map(|(k, v)| (k.clone(), v.access_count))
                .collect();
            entries.sort_by_key(|(_, count)| *count);

            let to_remove = self.entries.len() - self.max_size;
            let keys_to_remove: Vec<String> = entries
                .into_iter()
                .take(to_remove)
                .map(|(k, _)| k)
                .collect();

            for key in keys_to_remove {
                self.entries.remove(&key);
            }
        }
    }
}

// =============================================================================
// Memory Manager (Unified Interface)
// =============================================================================

/// Configuration for the memory manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    /// Vector store configuration
    pub vector_store: VectorStoreConfig,
    /// Chunking configuration
    pub chunking: ChunkingConfig,
    /// Context window size
    pub context_window_tokens: usize,
    /// Cache size
    pub cache_size: usize,
    /// Database path (None for in-memory)
    pub db_path: Option<String>,
    /// Auto-summarize long conversations
    pub auto_summarize: bool,
    /// Summarize after this many messages
    pub summarize_threshold: usize,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            vector_store: VectorStoreConfig::default(),
            chunking: ChunkingConfig::default(),
            context_window_tokens: 8192,
            cache_size: 1000,
            db_path: None,
            auto_summarize: true,
            summarize_threshold: 20,
        }
    }
}

/// Unified memory manager for agents
pub struct MemoryManager {
    config: MemoryConfig,
    vector_store: VectorStore,
    knowledge_base: KnowledgeBase,
    cache: AgentCache<String>,
}

impl MemoryManager {
    /// Create a new memory manager
    pub fn new(config: MemoryConfig) -> Result<Self, MemoryError> {
        let vector_store = if let Some(ref path) = config.db_path {
            VectorStore::with_persistence(config.vector_store.clone(), path)?
        } else {
            VectorStore::new(config.vector_store.clone())
        };

        let knowledge_base = if let Some(ref path) = config.db_path {
            let kb_path = format!("{}_kb", path);
            KnowledgeBase::with_persistence(config.vector_store.clone(), kb_path)?
        } else {
            KnowledgeBase::new(config.vector_store.clone())
        };

        Ok(Self {
            config: config.clone(),
            vector_store,
            knowledge_base,
            cache: AgentCache::new(config.cache_size),
        })
    }

    /// Store a memory
    pub fn remember(
        &mut self,
        content: impl Into<String>,
        memory_type: MemoryType,
        source: MemorySource,
    ) -> Result<MemoryId, MemoryError> {
        let entry = MemoryEntry::new(content, memory_type, source);
        self.vector_store.add(entry)
    }

    /// Store a memory with embedding
    pub fn remember_with_embedding(
        &mut self,
        content: impl Into<String>,
        embedding: Embedding,
        memory_type: MemoryType,
        source: MemorySource,
    ) -> Result<MemoryId, MemoryError> {
        let entry = MemoryEntry::new(content, memory_type, source).with_embedding(embedding);
        self.vector_store.add(entry)
    }

    /// Recall memories similar to a query
    pub fn recall(&mut self, query_embedding: &Embedding, limit: usize) -> Vec<SearchResult> {
        self.vector_store.search(query_embedding, limit)
    }

    /// Recall by type
    pub fn recall_by_type(&self, memory_type: MemoryType, limit: usize) -> Vec<&MemoryEntry> {
        self.vector_store.search_by_type(memory_type, limit)
    }

    /// Add document to knowledge base
    pub fn add_document(&mut self, document: Document) -> Result<String, MemoryError> {
        self.knowledge_base.add_document(document)
    }

    /// Retrieve from knowledge base
    pub fn retrieve(&mut self, query_embedding: &Embedding, limit: usize) -> Vec<SearchResult> {
        self.knowledge_base.retrieve(query_embedding, limit)
    }

    /// Build context for a prompt
    pub fn build_context(
        &mut self,
        query_embedding: &Embedding,
        system_prompt: &str,
        conversation: &[String],
    ) -> String {
        let mut context = ContextWindow::new(self.config.context_window_tokens);

        // System prompt (required)
        context.add_segment(ContextSegment {
            segment_type: ContextSegmentType::SystemPrompt,
            content: system_prompt.to_string(),
            tokens: estimate_tokens(system_prompt),
            priority: 100,
            required: true,
        });

        // Retrieved context
        let retrieved = self.recall(query_embedding, 5);
        if !retrieved.is_empty() {
            let retrieved_text: String = retrieved
                .iter()
                .map(|r| format!("- {}", r.entry.content))
                .collect::<Vec<_>>()
                .join("\n");

            context.add_segment(ContextSegment {
                segment_type: ContextSegmentType::RetrievedContext,
                content: format!("Relevant context:\n{}", retrieved_text),
                tokens: estimate_tokens(&retrieved_text) + 20,
                priority: 80,
                required: false,
            });
        }

        // Conversation history
        let conv_text = conversation.join("\n");
        context.add_segment(ContextSegment {
            segment_type: ContextSegmentType::ConversationHistory,
            content: conv_text.clone(),
            tokens: estimate_tokens(&conv_text),
            priority: 90,
            required: false,
        });

        context.build()
    }

    /// Cache a computation result
    pub fn cache_result(
        &mut self,
        key: impl Into<String>,
        value: String,
        ttl: Option<chrono::Duration>,
    ) {
        self.cache.set(key, value, ttl);
    }

    /// Get cached result
    pub fn get_cached(&mut self, key: &str) -> Option<String> {
        self.cache.get(key)
    }

    /// Get statistics
    pub fn stats(&self) -> MemoryStats {
        MemoryStats {
            vector_store: self.vector_store.stats(),
            document_count: self.knowledge_base.list_documents().len(),
        }
    }
}

/// Memory statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStats {
    pub vector_store: VectorStoreStats,
    pub document_count: usize,
}

// =============================================================================
// Error Types
// =============================================================================

/// Memory system errors
#[derive(Debug, Clone)]
pub enum MemoryError {
    /// Database error
    Database(String),
    /// Embedding error
    Embedding(String),
    /// Not found
    NotFound(String),
    /// Invalid input
    InvalidInput(String),
    /// IO error
    Io(String),
}

impl std::fmt::Display for MemoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemoryError::Database(e) => write!(f, "Database error: {}", e),
            MemoryError::Embedding(e) => write!(f, "Embedding error: {}", e),
            MemoryError::NotFound(e) => write!(f, "Not found: {}", e),
            MemoryError::InvalidInput(e) => write!(f, "Invalid input: {}", e),
            MemoryError::Io(e) => write!(f, "IO error: {}", e),
        }
    }
}

impl std::error::Error for MemoryError {}

// =============================================================================
// Utility Functions
// =============================================================================

fn generate_memory_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("mem_{:x}", timestamp)
}

/// Estimate token count (rough approximation: 4 chars per token)
fn estimate_tokens(text: &str) -> usize {
    (text.len() as f32 / 4.0).ceil() as usize
}

/// Truncate text to approximate token count
fn truncate_to_tokens(text: &str, max_tokens: usize) -> String {
    let max_chars = max_tokens * 4;
    if text.len() <= max_chars {
        text.to_string()
    } else {
        format!("{}...", &text[..max_chars.min(text.len())])
    }
}

// =============================================================================
// Embedding Provider Trait
// =============================================================================

/// Trait for embedding providers
#[async_trait::async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Generate embedding for text
    async fn embed(&self, text: &str) -> Result<Embedding, MemoryError>;

    /// Generate embeddings for multiple texts
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Embedding>, MemoryError>;

    /// Get the embedding dimension
    fn dimension(&self) -> usize;
}

/// OpenAI embedding provider
pub struct OpenAIEmbedding {
    #[allow(dead_code)]
    api_key: String,
    model: String,
}

impl OpenAIEmbedding {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: "text-embedding-3-small".to_string(),
        }
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }
}

#[async_trait::async_trait]
impl EmbeddingProvider for OpenAIEmbedding {
    async fn embed(&self, _text: &str) -> Result<Embedding, MemoryError> {
        // Implementation would call OpenAI API
        // For now, return a placeholder
        Ok(vec![0.0; 1536])
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Embedding>, MemoryError> {
        let mut results = Vec::new();
        for text in texts {
            results.push(self.embed(text).await?);
        }
        Ok(results)
    }

    fn dimension(&self) -> usize {
        match self.model.as_str() {
            "text-embedding-3-large" => 3072,
            _ => 1536,
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_entry_creation() {
        let entry = MemoryEntry::new(
            "Test content",
            MemoryType::LongTerm,
            MemorySource::UserInput,
        );
        assert!(!entry.id.is_empty());
        assert_eq!(entry.content, "Test content");
        assert_eq!(entry.memory_type, MemoryType::LongTerm);
    }

    #[test]
    fn test_similarity_metrics() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        let c = vec![0.0, 1.0, 0.0];

        assert!((SimilarityMetric::Cosine.calculate(&a, &b) - 1.0).abs() < 0.001);
        assert!((SimilarityMetric::Cosine.calculate(&a, &c) - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_vector_store() {
        let config = VectorStoreConfig::default();
        let mut store = VectorStore::new(config);

        let entry = MemoryEntry::new("Test", MemoryType::ShortTerm, MemorySource::UserInput)
            .with_embedding(vec![1.0, 0.0, 0.0]);

        let id = store.add(entry).unwrap();
        assert!(!id.is_empty());
        assert!(store.get(&id).is_some());
    }

    #[test]
    fn test_context_window() {
        let mut ctx = ContextWindow::new(1000);

        ctx.add_segment(ContextSegment {
            segment_type: ContextSegmentType::SystemPrompt,
            content: "You are helpful".to_string(),
            tokens: 10,
            priority: 100,
            required: true,
        });

        let result = ctx.build();
        assert!(result.contains("You are helpful"));
    }

    #[test]
    fn test_cache() {
        let mut cache: AgentCache<String> = AgentCache::new(10);

        cache.set("key1", "value1".to_string(), None);
        assert_eq!(cache.get("key1"), Some("value1".to_string()));
        assert_eq!(cache.get("key2"), None);
    }

    #[test]
    fn test_estimate_tokens() {
        assert_eq!(estimate_tokens("hello"), 2); // 5 chars / 4 = 1.25 -> 2
        assert_eq!(estimate_tokens("hello world"), 3); // 11 chars / 4 = 2.75 -> 3
    }
}
