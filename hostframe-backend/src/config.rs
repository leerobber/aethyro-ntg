use anyhow::anyhow;

#[derive(Clone, Debug)]
pub struct Config {
    pub gcp_project: String,
    pub gcp_dataset: String,
    pub bq_table: String,
    pub gcp_credentials_path: Option<String>,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let gcp_project = std::env::var("GCP_PROJECT")
            .or_else(|_| std::env::var("GOOGLE_CLOUD_PROJECT"))
            .map_err(|_| anyhow!("GCP_PROJECT or GOOGLE_CLOUD_PROJECT required"))?;

        let gcp_dataset =
            std::env::var("BQ_DATASET").unwrap_or_else(|_| "aethyro_telemetry".to_string());

        let bq_table =
            std::env::var("BQ_TABLE").unwrap_or_else(|_| "telemetry_events".to_string());

        let gcp_credentials_path = std::env::var("GOOGLE_APPLICATION_CREDENTIALS").ok();

        Ok(Self {
            gcp_project,
            gcp_dataset,
            bq_table,
            gcp_credentials_path,
        })
    }
}
