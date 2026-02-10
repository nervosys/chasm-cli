// Copyright (c) 2024-2026 Nervosys LLC
// SPDX-License-Identifier: AGPL-3.0-only
//! AI Intelligence Module

use crate::models::ChatSession;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Topic {
    pub name: String,
    pub confidence: f32,
    pub keywords: Vec<String>,
}

pub struct TopicExtractor {
    patterns: HashMap<String, Vec<String>>,
    min_confidence: f32,
}

impl TopicExtractor {
    pub fn new() -> Self {
        let mut patterns = HashMap::new();
        patterns.insert("rust".to_string(), vec!["rust", "cargo", "crate"].into_iter().map(String::from).collect());
        patterns.insert("python".to_string(), vec!["python", "pip", "django"].into_iter().map(String::from).collect());
        Self { patterns, min_confidence: 0.1 }
    }

    pub fn extract(&self, session: &ChatSession) -> Vec<Topic> {
        let content = session.collect_all_text().to_lowercase();
        let word_count = content.split_whitespace().count().max(1) as f32;
        let mut topics = Vec::new();
        for (name, keywords) in &self.patterns {
            let mut matched = Vec::new();
            let mut count = 0;
            for kw in keywords {
                if content.contains(kw) {
                    matched.push(kw.clone());
                    count += content.matches(kw).count();
                }
            }
            if !matched.is_empty() {
                let confidence = (count as f32 / word_count).min(1.0);
                if confidence >= self.min_confidence {
                    topics.push(Topic { name: name.clone(), confidence, keywords: matched });
                }
            }
        }
        topics
    }
}

impl Default for TopicExtractor { fn default() -> Self { Self::new() } }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationInsights {
    pub session_id: String,
    pub generated_at: DateTime<Utc>,
    pub key_points: Vec<KeyPoint>,
    pub questions: Vec<String>,
    pub stats: ConversationStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyPoint {
    pub summary: String,
    pub importance: f32,
    pub category: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationStats {
    pub message_count: usize,
    pub user_messages: usize,
    pub assistant_messages: usize,
    pub turns: usize,
}

pub struct InsightsGenerator;

impl InsightsGenerator {
    pub fn new() -> Self { Self }
    
    pub fn generate(&self, session: &ChatSession) -> ConversationInsights {
        let user_msgs = session.user_messages();
        let asst_msgs = session.assistant_responses();
        ConversationInsights {
            session_id: session.session_id.clone().unwrap_or_default(),
            generated_at: Utc::now(),
            key_points: Vec::new(),
            questions: user_msgs.iter().flat_map(|m| m.split('.').filter(|s| s.contains('?'))).map(String::from).collect(),
            stats: ConversationStats {
                message_count: user_msgs.len() + asst_msgs.len(),
                user_messages: user_msgs.len(),
                assistant_messages: asst_msgs.len(),
                turns: session.requests.len(),
            },
        }
    }
}

impl Default for InsightsGenerator { fn default() -> Self { Self::new() } }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sentiment { pub score: f32, pub label: String, pub confidence: f32 }

pub struct SentimentAnalyzer {
    positive: HashSet<String>,
    negative: HashSet<String>,
}

impl SentimentAnalyzer {
    pub fn new() -> Self {
        Self {
            positive: vec!["good", "great", "thanks", "helpful", "works"].into_iter().map(String::from).collect(),
            negative: vec!["bad", "error", "fail", "wrong", "bug"].into_iter().map(String::from).collect(),
        }
    }
    
    pub fn analyze(&self, session: &ChatSession) -> Sentiment {
        let text = session.collect_all_text().to_lowercase();
        let words: Vec<&str> = text.split_whitespace().collect();
        let pos = words.iter().filter(|w| self.positive.contains(**w)).count();
        let neg = words.iter().filter(|w| self.negative.contains(**w)).count();
        let total = pos + neg;
        let score = if total > 0 { (pos as f32 - neg as f32) / total as f32 } else { 0.0 };
        let label = if score > 0.3 { "positive" } else if score < -0.3 { "negative" } else { "neutral" };
        Sentiment { score, label: label.to_string(), confidence: if total > 0 { 0.8 } else { 0.5 } }
    }
}

impl Default for SentimentAnalyzer { fn default() -> Self { Self::new() } }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityScore { pub overall: u8, pub clarity: u8, pub completeness: u8 }

pub struct QualityScorer;

impl QualityScorer {
    pub fn new() -> Self { Self }
    pub fn score(&self, session: &ChatSession) -> QualityScore {
        let msgs = session.user_messages();
        let clarity = if msgs.is_empty() { 50 } else { 70 };
        let completeness = if session.assistant_responses().is_empty() { 0 } else { 80 };
        QualityScore { overall: (clarity + completeness) / 2, clarity, completeness }
    }
}

impl Default for QualityScorer { fn default() -> Self { Self::new() } }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarityResult { pub session_a_id: String, pub session_b_id: String, pub score: f32 }

pub struct SimilarityDetector;

impl SimilarityDetector {
    pub fn new() -> Self { Self }
    pub fn compare(&self, a: &ChatSession, b: &ChatSession) -> SimilarityResult {
        let ta = a.collect_all_text().to_lowercase();
        let tb = b.collect_all_text().to_lowercase();
        let wa: HashSet<&str> = ta.split_whitespace().collect();
        let wb: HashSet<&str> = tb.split_whitespace().collect();
        let inter = wa.intersection(&wb).count();
        let union = wa.union(&wb).count();
        let score = if union > 0 { inter as f32 / union as f32 } else { 0.0 };
        SimilarityResult {
            session_a_id: a.session_id.clone().unwrap_or_default(),
            session_b_id: b.session_id.clone().unwrap_or_default(),
            score,
        }
    }
}

impl Default for SimilarityDetector { fn default() -> Self { Self::new() } }
