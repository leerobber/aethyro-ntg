terraform {
  required_version = ">= 1.0"
  required_providers {
    google = {
      source  = "hashicorp/google"
      version = "~> 5.0"
    }
  }
}

provider "google" {
  project = var.gcp_project
  region  = var.gcp_region
}

# Cloud Run service for telemetry ingestion
resource "google_cloud_run_service" "hostframe" {
  name     = "hostframe-backend"
  location = var.gcp_region

  template {
    spec {
      containers {
        image = var.container_image

        env {
          name  = "GCP_PROJECT"
          value = var.gcp_project
        }

        env {
          name  = "BQ_DATASET"
          value = var.bq_dataset
        }

        env {
          name  = "BQ_TABLE"
          value = var.bq_table
        }

        resources {
          limits = {
            cpu    = var.cloud_run_cpu
            memory = var.cloud_run_memory
          }
        }
      }

      service_account_name = google_service_account.hostframe.email

      timeout_seconds = 30
    }

    metadata {
      annotations = {
        "autoscaling.knative.dev/maxScale"       = var.cloud_run_max_instances
        "autoscaling.knative.dev/minScale"       = "1"
        "run.googleapis.com/client-name"         = "terraform"
      }
    }
  }

  traffic {
    percent         = 100
    latest_revision = true
  }

  depends_on = [
    google_project_iam_member.hostframe_bq_user,
  ]
}

# Service account for Cloud Run
resource "google_service_account" "hostframe" {
  account_id   = "hostframe-backend"
  display_name = "Hostframe telemetry service account"
}

# IAM: Service account can write to BigQuery
resource "google_project_iam_member" "hostframe_bq_user" {
  project = var.gcp_project
  role    = "roles/bigquery.dataEditor"
  member  = "serviceAccount:${google_service_account.hostframe.email}"
}

# BigQuery dataset for telemetry
resource "google_bigquery_dataset" "telemetry" {
  dataset_id    = var.bq_dataset
  friendly_name = "VITASCALE Telemetry"
  description   = "Genomic telemetry events and metrics from KAIROS agents"
  location      = var.bq_location

  access {
    role          = "OWNER"
    user_by_email = google_service_account.hostframe.email
  }
}

# BigQuery table: telemetry_events
resource "google_bigquery_table" "telemetry_events" {
  dataset_id = google_bigquery_dataset.telemetry.dataset_id
  table_id   = var.bq_table

  schema = file("${path.module}/../sql/schema.sql")

  time_partitioning {
    type          = "DAY"
    field         = "ingested_at"
    expiration_ms = var.bq_data_expiration_days * 86400 * 1000
  }

  clustering = ["batch_id", "tick"]

  labels = {
    environment = var.environment
    project     = "aethyro"
  }
}

# Cloud Storage for audit ledger (long-term)
resource "google_storage_bucket" "audit_ledger" {
  name          = "${var.gcp_project}-hostframe-ledger"
  location      = var.gcs_location
  force_destroy = false

  uniform_bucket_level_access = true

  versioning {
    enabled = true
  }

  lifecycle_rule {
    condition {
      num_newer_versions = 5
    }
    action {
      delete = true
    }
  }

  labels = {
    environment = var.environment
    purpose     = "audit-ledger"
  }
}

# Allow Cloud Run to read/write audit ledger
resource "google_storage_bucket_iam_member" "hostframe_ledger" {
  bucket = google_storage_bucket.audit_ledger.name
  role   = "roles/storage.objectAdmin"
  member = "serviceAccount:${google_service_account.hostframe.email}"
}

# Cloud Run IAM: allow public access (with API key required in headers)
resource "google_cloud_run_service_iam_member" "public_access" {
  service       = google_cloud_run_service.hostframe.name
  location      = google_cloud_run_service.hostframe.location
  role          = "roles/run.invoker"
  member        = "allUsers"
}

# Monitoring: alerting policy for high error rate
resource "google_monitoring_alert_policy" "hostframe_errors" {
  display_name = "Hostframe Backend Error Rate"
  combiner     = "OR"

  conditions {
    display_name = "Error rate > 5%"

    condition_threshold {
      filter          = "resource.type=\"cloud_run_revision\" AND resource.service_name=\"hostframe-backend\" AND metric.type=\"run.googleapis.com/request_count\" AND metric.response_code_class=\"5xx\""
      duration        = "60s"
      comparison      = "COMPARISON_GT"
      threshold_value = 0.05
    }
  }

  notification_channels = var.alert_email != "" ? [
    google_monitoring_notification_channel.email[0].id
  ] : []
}

# Notification channel: email alerts
resource "google_monitoring_notification_channel" "email" {
  count           = var.alert_email != "" ? 1 : 0
  display_name    = "Hostframe Alerts"
  type            = "email"
  labels = {
    email_address = var.alert_email
  }
}

# Outputs
output "cloud_run_url" {
  value       = google_cloud_run_service.hostframe.status[0].url
  description = "URL of the Hostframe Cloud Run service"
}

output "bigquery_dataset" {
  value       = google_bigquery_dataset.telemetry.dataset_id
  description = "BigQuery dataset ID for telemetry"
}

output "audit_ledger_bucket" {
  value       = google_storage_bucket.audit_ledger.name
  description = "GCS bucket for audit ledger"
}
