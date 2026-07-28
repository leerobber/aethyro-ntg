variable "gcp_project" {
  type        = string
  description = "GCP project ID"
}

variable "gcp_region" {
  type        = string
  default     = "us-central1"
  description = "GCP region for Cloud Run and BigQuery"
}

variable "gcs_location" {
  type        = string
  default     = "US"
  description = "GCS location for audit ledger bucket"
}

variable "bq_location" {
  type        = string
  default     = "US"
  description = "BigQuery dataset location"
}

variable "container_image" {
  type        = string
  description = "Container image URI (e.g., gcr.io/project/hostframe-backend:latest)"
}

variable "bq_dataset" {
  type        = string
  default     = "aethyro_telemetry"
  description = "BigQuery dataset ID"
}

variable "bq_table" {
  type        = string
  default     = "telemetry_events"
  description = "BigQuery table ID"
}

variable "bq_data_expiration_days" {
  type        = number
  default     = 90
  description = "Data retention in BigQuery (days)"
}

variable "cloud_run_cpu" {
  type        = string
  default     = "2"
  description = "Cloud Run CPU allocation"
}

variable "cloud_run_memory" {
  type        = string
  default     = "1Gi"
  description = "Cloud Run memory allocation"
}

variable "cloud_run_max_instances" {
  type        = number
  default     = 100
  description = "Maximum concurrent instances"
}

variable "environment" {
  type        = string
  default     = "production"
  description = "Environment name (dev, staging, production)"
}

variable "alert_email" {
  type        = string
  default     = ""
  description = "Email for error alerts (optional)"
}
