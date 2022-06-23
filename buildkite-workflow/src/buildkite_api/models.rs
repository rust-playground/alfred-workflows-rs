use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize)]
pub struct Organization {
    pub id: String,
    pub url: String,
    pub web_url: String,
    pub name: String,
    pub slug: String,
    pub pipelines_url: String,
    pub agents_url: String,
    pub emojis_url: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Pipeline {
    pub id: String,
    pub url: String,
    pub web_url: String,
    pub name: String,
    pub slug: String,
    pub repository: String,
    pub branch_configuration: Option<String>,
    pub default_branch: Option<String>,
    pub provider: Provider,
    pub skip_queued_branch_builds: bool,
    pub skip_queued_branch_builds_filter: Option<String>,
    pub cancel_running_branch_builds: bool,
    pub cancel_running_branch_builds_filter: Option<String>,
    pub builds_url: String,
    pub badge_url: String,
    pub created_at: DateTime<Utc>,
    pub scheduled_builds_count: i32,
    pub running_builds_count: i32,
    pub scheduled_jobs_count: i32,
    pub running_jobs_count: i32,
    pub waiting_jobs_count: i32,
    pub visibility: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Provider {
    pub id: String,
    pub webhook_url: String,
    pub settings: ProviderSettings,
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Serialize, Deserialize)]
pub struct ProviderSettings {
    pub publish_commit_status: bool,
    pub build_pull_requests: bool,
    pub build_pull_request_forks: bool,
    pub build_tags: bool,
    pub publish_commit_status_per_step: bool,
    pub repository: String,
    pub trigger_mode: String,
}
