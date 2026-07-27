# Aethyro NTG — GCP infrastructure (VITASCALE Hostframe, ADR 0010).
#
# Provisions:
#   - GKE Autopilot cluster  (KAIROS agent hierarchy)
#   - Cloud Run service      (NanoKeymaster HTTP backend)
#   - Artifact Registry      (container images)
#   - Secret Manager secrets (KEYMASTER_BACKEND_URL, etc.)
#   - IAM service accounts   (least-privilege)
#
# Variables in variables.tf; defaults target a dev/staging environment.
# For production, override via terraform.tfvars or -var flags.

terraform {
  required_version = ">= 1.7"
  required_providers {
    google = {
      source  = "hashicorp/google"
      version = "~> 5.30"
    }
  }

  backend "gcs" {
    # Configure before first apply:
    #   bucket = "<your-tfstate-bucket>"
    #   prefix = "aethyro/ntg"
  }
}

provider "google" {
  project = var.project_id
  region  = var.region
}

# ---------------------------------------------------------------------------
# Artifact Registry — container image repository
# ---------------------------------------------------------------------------

resource "google_artifact_registry_repository" "ntg" {
  repository_id = "ntg-kernel"
  location      = var.region
  format        = "DOCKER"
  description   = "Aethyro NTG kernel images"
}

# ---------------------------------------------------------------------------
# Service accounts
# ---------------------------------------------------------------------------

resource "google_service_account" "kairos_gke" {
  account_id   = "kairos-gke"
  display_name = "KAIROS GKE node service account"
}

resource "google_service_account" "keymaster_run" {
  account_id   = "keymaster-run"
  display_name = "NanoKeymaster Cloud Run service account"
}

# Artifact Registry reader — GKE nodes pull images
resource "google_artifact_registry_repository_iam_member" "gke_reader" {
  location   = google_artifact_registry_repository.ntg.location
  repository = google_artifact_registry_repository.ntg.name
  role       = "roles/artifactregistry.reader"
  member     = "serviceAccount:${google_service_account.kairos_gke.email}"
}

# Cloud Run invoker — GKE workloads call the Keymaster backend
resource "google_cloud_run_v2_service_iam_member" "gke_invoker" {
  location = var.region
  name     = google_cloud_run_v2_service.keymaster.name
  role     = "roles/run.invoker"
  member   = "serviceAccount:${google_service_account.kairos_gke.email}"
}

# Secret accessor — Cloud Run reads secrets
resource "google_project_iam_member" "run_secret_accessor" {
  project = var.project_id
  role    = "roles/secretmanager.secretAccessor"
  member  = "serviceAccount:${google_service_account.keymaster_run.email}"
}

# ---------------------------------------------------------------------------
# Secret Manager
# ---------------------------------------------------------------------------

resource "google_secret_manager_secret" "keymaster_backend_url" {
  secret_id = "keymaster-backend-url"
  replication {
    auto {}
  }
}

resource "google_secret_manager_secret" "ntg_ledger_key" {
  secret_id = "ntg-ledger-hmac-key"
  replication {
    auto {}
  }
}

# ---------------------------------------------------------------------------
# GKE Autopilot cluster — KAIROS agent hierarchy
# ---------------------------------------------------------------------------

resource "google_container_cluster" "kairos" {
  name     = "kairos-${var.environment}"
  location = var.region

  enable_autopilot = true

  # Private cluster — nodes have no public IPs
  private_cluster_config {
    enable_private_nodes    = true
    enable_private_endpoint = false
    master_ipv4_cidr_block  = "172.16.0.0/28"
  }

  # Workload Identity — pods authenticate as GCP service accounts
  workload_identity_config {
    workload_pool = "${var.project_id}.svc.id.goog"
  }

  release_channel {
    channel = "REGULAR"
  }

  # Deletion protection — disable explicitly to tear down dev clusters
  deletion_protection = var.environment == "prod"
}

# Workload Identity binding — KSA → GSA for GKE pods
resource "google_service_account_iam_member" "workload_identity" {
  service_account_id = google_service_account.kairos_gke.name
  role               = "roles/iam.workloadIdentityUser"
  member             = "serviceAccount:${var.project_id}.svc.id.goog[kairos/kairos-sa]"
}

# ---------------------------------------------------------------------------
# Cloud Run — NanoKeymaster HTTP backend
# ---------------------------------------------------------------------------

resource "google_cloud_run_v2_service" "keymaster" {
  name     = "keymaster-${var.environment}"
  location = var.region
  ingress  = "INGRESS_TRAFFIC_INTERNAL_ONLY"

  template {
    service_account = google_service_account.keymaster_run.email

    scaling {
      min_instance_count = 0
      max_instance_count = var.keymaster_max_instances
    }

    containers {
      image = "${var.region}-docker.pkg.dev/${var.project_id}/${google_artifact_registry_repository.ntg.repository_id}/ntg-kernel:${var.image_tag}"

      command = ["./kernel_host"]

      resources {
        limits = {
          cpu    = "1"
          memory = "512Mi"
        }
        cpu_idle = true
      }

      env {
        name  = "RUST_LOG"
        value = "info"
      }

      env {
        name = "KEYMASTER_BACKEND_URL"
        value_source {
          secret_key_ref {
            secret  = google_secret_manager_secret.keymaster_backend_url.secret_id
            version = "latest"
          }
        }
      }

      ports {
        container_port = 8080
      }

      liveness_probe {
        http_get {
          path = "/health"
          port = 8080
        }
        initial_delay_seconds = 5
        period_seconds        = 30
      }
    }
  }
}

# ---------------------------------------------------------------------------
# Outputs
# ---------------------------------------------------------------------------

output "gke_cluster_name" {
  value = google_container_cluster.kairos.name
}

output "gke_cluster_endpoint" {
  value     = google_container_cluster.kairos.endpoint
  sensitive = true
}

output "keymaster_url" {
  description = "Internal Cloud Run URL for NanoKeymaster backend"
  value       = google_cloud_run_v2_service.keymaster.uri
}

output "artifact_registry" {
  value = "${var.region}-docker.pkg.dev/${var.project_id}/${google_artifact_registry_repository.ntg.repository_id}"
}
