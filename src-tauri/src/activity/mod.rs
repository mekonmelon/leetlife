//! Activity ingestion. Everything external sits behind `ActivityProvider`, so
//! the engine never depends on one fragile integration. Two providers ship:
//! polling LeetCode's public GraphQL endpoint, and a loopback HTTP endpoint a
//! browser extension can POST accepted submissions to.
pub mod ingest;
pub mod leetcode;

pub use leetcode::LeetCodeProvider;

use crate::error::Result;
use crate::models::{AttemptResult, Difficulty};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExternalAttempt {
    /// Stable id from the source (LeetCode submission id, extension uuid...). Dedup key.
    pub external_id: String,
    pub slug: String,
    pub title: String,
    #[serde(default)]
    pub difficulty: Option<Difficulty>,
    #[serde(default)]
    pub topic: Option<String>,
    #[serde(default)]
    pub result: AttemptResult,
    pub attempted_at: i64,
    #[serde(default)]
    pub duration_ms: Option<i64>,
    #[serde(default = "default_source")]
    pub source: String,
}

fn default_source() -> String {
    "external".into()
}

#[async_trait::async_trait]
pub trait ActivityProvider: Send + Sync {
    fn name(&self) -> &str;
    /// `known` is the set of catalogue slugs, so providers only need to look up
    /// metadata (difficulty/topic) for problems we haven't seen before.
    async fn fetch_recent(&self, known: &HashSet<String>) -> Result<Vec<ExternalAttempt>>;
}

/// The no-op provider: the user logs attempts by hand.
pub struct ManualProvider;

#[async_trait::async_trait]
impl ActivityProvider for ManualProvider {
    fn name(&self) -> &str {
        "manual"
    }
    async fn fetch_recent(&self, _known: &HashSet<String>) -> Result<Vec<ExternalAttempt>> {
        Ok(Vec::new())
    }
}

/// Maps LeetCode topic tags onto LeetLife's coarser topic vocabulary.
pub fn normalize_topic(tags: &[String]) -> String {
    for tag in tags {
        let t = tag.as_str();
        let mapped = match t {
            "Dynamic Programming" | "Memoization" => "Dynamic Programming",
            "Graph" | "Breadth-First Search" | "Depth-First Search" | "Topological Sort" | "Union Find" | "Shortest Path" => "Graphs",
            "Tree" | "Binary Tree" | "Binary Search Tree" => "Trees",
            "Heap (Priority Queue)" => "Heap",
            "Trie" => "Tries",
            "Binary Search" => "Binary Search",
            "Two Pointers" => "Two Pointers",
            "Sliding Window" => "Sliding Window",
            "Stack" | "Monotonic Stack" => "Stack",
            "Linked List" => "Linked List",
            "Backtracking" | "Recursion" => "Backtracking",
            "Greedy" => "Greedy",
            "Intervals" | "Line Sweep" => "Intervals",
            "Bit Manipulation" => "Bit Manipulation",
            "Math" | "Matrix" | "Geometry" | "Number Theory" => "Math",
            "String" | "String Matching" => "Strings",
            "Array" | "Hash Table" | "Sorting" | "Prefix Sum" | "Counting" => "Arrays",
            _ => continue,
        };
        return mapped.to_string();
    }
    "General".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn specific_tags_win_over_generic_ones() {
        let tags = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(normalize_topic(&tags(&["Array", "Dynamic Programming"])), "Arrays");
        assert_eq!(normalize_topic(&tags(&["Breadth-First Search", "Matrix"])), "Graphs");
        assert_eq!(normalize_topic(&tags(&["Something New"])), "General");
        assert_eq!(normalize_topic(&[]), "General");
    }

    #[test]
    fn external_attempt_defaults() {
        let a: ExternalAttempt = serde_json::from_str(r#"{"external_id":"x","slug":"two-sum","title":"Two Sum","attempted_at":1}"#).unwrap();
        assert_eq!(a.result, AttemptResult::Solved);
        assert_eq!(a.source, "external");
        assert!(a.difficulty.is_none());
    }
}
