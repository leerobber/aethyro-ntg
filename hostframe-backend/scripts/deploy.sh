#!/bin/bash
set -euo pipefail

# Hostframe backend deployment script
# Usage: ./scripts/deploy.sh <gcp-project> <gcp-region>

if [[ $# -lt 1 ]]; then
    echo "Usage: $0 <gcp-project> [gcp-region]"
    echo "Example: $0 my-gcp-project us-central1"
    exit 1
fi

GCP_PROJECT=$1
GCP_REGION=${2:-us-central1}
IMAGE_TAG=${IMAGE_TAG:-latest}

echo "=========================================="
echo "Hostframe Backend Deployment"
echo "=========================================="
echo "GCP Project:  $GCP_PROJECT"
echo "Region:       $GCP_REGION"
echo "Image Tag:    $IMAGE_TAG"
echo "=========================================="

# Step 1: Build container
echo ""
echo "[1/5] Building Docker image..."
IMAGE="gcr.io/$GCP_PROJECT/hostframe-backend:$IMAGE_TAG"
docker build -t "$IMAGE" .

# Step 2: Push to GCR
echo ""
echo "[2/5] Pushing to Google Container Registry..."
docker push "$IMAGE"

# Step 3: Configure gcloud
echo ""
echo "[3/5] Configuring gcloud..."
gcloud config set project "$GCP_PROJECT"

# Step 4: Enable APIs
echo ""
echo "[4/5] Enabling GCP APIs..."
gcloud services enable \
    run.googleapis.com \
    bigquery.googleapis.com \
    storage.googleapis.com \
    container.googleapis.com

# Step 5: Deploy with Terraform
echo ""
echo "[5/5] Deploying infrastructure with Terraform..."
cd terraform
terraform init
terraform plan -var="gcp_project=$GCP_PROJECT" -var="gcp_region=$GCP_REGION" -var="container_image=$IMAGE"
terraform apply -var="gcp_project=$GCP_PROJECT" -var="gcp_region=$GCP_REGION" -var="container_image=$IMAGE"

echo ""
echo "=========================================="
echo "Deployment Complete!"
echo "=========================================="
echo ""
echo "Cloud Run URL:"
terraform output cloud_run_url
echo ""
echo "BigQuery Dataset:"
terraform output bigquery_dataset
echo ""
echo "Next: Update kernel hostframe_bridge.rs with the Cloud Run URL"
echo "=========================================="
