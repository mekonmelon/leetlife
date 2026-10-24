//! Polls LeetCode's public GraphQL endpoint for a user's recent accepted
//! submissions. Only needs a username; no cookies or login.
use super::{normalize_topic, ActivityProvider, ExternalAttempt};
use crate::error::Result;
use crate::models::{AttemptResult, Difficulty};
use serde::Deserialize;
use std::collections::HashSet;
use std::time::Duration;

const ENDPOINT: &str = "https://leetcode.com/graphql";
const RECENT_QUERY: &str = "query recentAc($username: String!, $limit: Int!) { recentAcSubmissionList(username: $username, limit: $limit) { id title titleSlug timestamp } }";
const QUESTION_QUERY: &str = "query q($slug: String!) { question(titleSlug: $slug) { questionFrontendId difficulty topicTags { name } } }";

pub struct LeetCodeProvider {
    username: String,
    client: reqwest::Client,
}

impl LeetCodeProvider {
    pub fn new(username: &str) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .user_agent("LeetLife/0.1 (+local menu-bar app)")
            .build()
            .expect("reqwest client");
        Self { username: username.to_string(), client }
    }

    async fn graphql<T: for<'de> Deserialize<'de>>(&self, query: &str, variables: serde_json::Value) -> Result<T> {
        let body = serde_json::json!({ "query": query, "variables": variables });
        let resp = self.client.post(ENDPOINT).json(&body).send().await?.error_for_status()?;
        let env: Envelope<T> = resp.json().await?;
        match env.data {
            Some(d) => Ok(d),
            None => Err(crate::Error::Invalid(format!("leetcode error: {}", env.errors.first().map(|e| e.message.as_str()).unwrap_or("empty response")))),
        }
    }

    async fn question(&self, slug: &str) -> Result<Option<QuestionInfo>> {
        let d: QuestionData = self.graphql(QUESTION_QUERY, serde_json::json!({ "slug": slug })).await?;
        Ok(d.question)
    }
}

#[derive(Deserialize)]
struct Envelope<T> {
    data: Option<T>,
    #[serde(default)]
    errors: Vec<GqlError>,
}

#[derive(Deserialize)]
struct GqlError {
    message: String,
}

#[derive(Deserialize)]
struct RecentData {
    #[serde(rename = "recentAcSubmissionList", default)]
    list: Vec<RecentSubmission>,
}

#[derive(Deserialize)]
pub struct RecentSubmission {
    pub id: String,
    pub title: String,
    #[serde(rename = "titleSlug")]
    pub title_slug: String,
    pub timestamp: String,
}

#[derive(Deserialize)]
struct QuestionData {
    question: Option<QuestionInfo>,
}

#[derive(Deserialize)]
pub struct QuestionInfo {
    pub difficulty: String,
    #[serde(rename = "topicTags", default)]
    pub topic_tags: Vec<TopicTag>,
}

#[derive(Deserialize)]
pub struct TopicTag {
    pub name: String,
}

/// Pure conversion so it can be tested without the network.
pub fn to_attempt(s: &RecentSubmission, info: Option<&QuestionInfo>) -> Option<ExternalAttempt> {
    let attempted_at: i64 = s.timestamp.parse().ok()?;
    Some(ExternalAttempt {
        external_id: format!("leetcode:{}", s.id),
        slug: s.title_slug.clone(),
        title: s.title.clone(),
        difficulty: info.and_then(|i| Difficulty::parse(&i.difficulty)),
        topic: info.map(|i| normalize_topic(&i.topic_tags.iter().map(|t| t.name.clone()).collect::<Vec<_>>())),
        result: AttemptResult::Solved,
        attempted_at,
        duration_ms: None,
        source: "leetcode".into(),
    })
}

#[async_trait::async_trait]
impl ActivityProvider for LeetCodeProvider {
    fn name(&self) -> &str {
        "leetcode"
    }

    async fn fetch_recent(&self, known: &HashSet<String>) -> Result<Vec<ExternalAttempt>> {
        let data: RecentData = self.graphql(RECENT_QUERY, serde_json::json!({ "username": self.username, "limit": 20 })).await?;
        let mut out = Vec::with_capacity(data.list.len());
        for s in &data.list {
            let info = if known.contains(&s.title_slug) { None } else { self.question(&s.title_slug).await.ok().flatten() };
            if let Some(a) = to_attempt(s, info.as_ref()) {
                out.push(a);
            }
        }
        // Oldest first so day/streak bookkeeping happens in order.
        out.sort_by_key(|a| a.attempted_at);
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_submission_with_metadata() {
        let s = RecentSubmission { id: "42".into(), title: "Two Sum".into(), title_slug: "two-sum".into(), timestamp: "1700000000".into() };
        let info = QuestionInfo { difficulty: "Easy".into(), topic_tags: vec![TopicTag { name: "Array".into() }] };
        let a = to_attempt(&s, Some(&info)).unwrap();
        assert_eq!(a.external_id, "leetcode:42");
        assert_eq!(a.difficulty, Some(Difficulty::Easy));
        assert_eq!(a.topic.as_deref(), Some("Arrays"));
        assert_eq!(a.attempted_at, 1_700_000_000);
    }

    #[test]
    fn bad_timestamp_is_skipped() {
        let s = RecentSubmission { id: "1".into(), title: "x".into(), title_slug: "x".into(), timestamp: "nope".into() };
        assert!(to_attempt(&s, None).is_none());
    }

    #[test]
    fn parses_graphql_envelope() {
        let raw = r#"{"data":{"recentAcSubmissionList":[{"id":"7","title":"LRU Cache","titleSlug":"lru-cache","timestamp":"1700000001"}]}}"#;
        let env: Envelope<RecentData> = serde_json::from_str(raw).unwrap();
        assert_eq!(env.data.unwrap().list[0].title_slug, "lru-cache");
    }
}
