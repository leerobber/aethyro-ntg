#!/bin/bash
set -euo pipefail

# ============================================================================
# Aethyro NTG Full Deployment Automation (Windows/MINGW64)
# ============================================================================
# This script installs Terraform, verifies prerequisites, and deploys
# the entire VITASCALE Hostframe infrastructure to GCP.
# ============================================================================

GCP_PROJECT="aethyro-ntg"
GCP_REGION="us-central1"
IMAGE_TAG="latest"

echo "=========================================="
echo "Aethyro NTG — Full Deployment Automation"
echo "=========================================="
echo ""

# ============================================================================
# Step 0: Verify Prerequisites
# ============================================================================
echo "[0/6] Verifying prerequisites..."

if ! command -v docker &> /dev/null; then
    echo "ERROR: Docker not found. Install Docker Desktop for Windows."
    exit 1
fi
echo "  ✓ Docker: $(docker --version)"

if ! command -v gcloud &> /dev/null; then
    echo "ERROR: gcloud not found. Please install Google Cloud SDK."
    exit 1
fi
echo "  ✓ gcloud: $(gcloud --version | head -1)"

if ! command -v git &> /dev/null; then
    echo "ERROR: git not found."
    exit 1
fi
echo "  ✓ git: $(git --version)"

# ============================================================================
# Step 1: Install Terraform (if not present)
# ============================================================================
echo ""
echo "[1/6] Installing Terraform..."

if command -v terraform &> /dev/null; then
    echo "  ✓ Terraform already installed: $(terraform --version | head -1)"
else
    echo "  Installing Terraform..."

    # Detect OS and architecture
    OS=$(uname -s)
    ARCH=$(uname -m)

    if [[ "$OS" == "MINGW64"* ]] || [[ "$OS" == "MSYS"* ]]; then
        TFVER="1.7.5"
        TFURL="https://releases.hashicorp.com/terraform/${TFVER}/terraform_${TFVER}_windows_amd64.zip"
        TFDIR="${USERPROFILE}/.terraform"
        mkdir -p "$TFDIR"
        cd "$TFDIR"

        echo "  Downloading Terraform for Windows..."
        curl -fsSL -o terraform.zip "$TFURL"
        unzip -o terraform.zip
        rm terraform.zip

        # Add to PATH
        export PATH="$TFDIR:$PATH"
        echo "  ✓ Terraform installed to: $TFDIR"
        echo "  NOTE: Add $TFDIR to your PATH permanently in Environment Variables"
    else
        echo "  ERROR: Unsupported OS for automated Terraform install: $OS"
        echo "  Please download from: https://www.terraform.io/downloads"
        exit 1
    fi
fi

# ============================================================================
# Step 2: Verify gcloud Project Configuration
# ============================================================================
echo ""
echo "[2/6] Verifying gcloud configuration..."

CURRENT_PROJECT=$(gcloud config get-value project 2>/dev/null || echo "")
if [[ "$CURRENT_PROJECT" != "$GCP_PROJECT" ]]; then
    echo "  Setting active project to: $GCP_PROJECT"
    gcloud config set project "$GCP_PROJECT"
else
    echo "  ✓ Active project: $GCP_PROJECT"
fi

CURRENT_ACCOUNT=$(gcloud config get-value account 2>/dev/null || echo "")
echo "  ✓ Authenticated as: $CURRENT_ACCOUNT"

# ============================================================================
# Step 3: Build and Push Docker Image
# ============================================================================
echo ""
echo "[3/6] Building Docker image..."

cd "$(pwd)/aethyro-ntg/hostframe-backend"

IMAGE_REPO="${GCP_REGION}-docker.pkg.dev/${GCP_PROJECT}/ntg-kernel"
IMAGE="${IMAGE_REPO}/ntg-kernel:${IMAGE_TAG}"

echo "  Building: $IMAGE"
docker build -t "$IMAGE" .

echo ""
echo "[3b/6] Pushing to Artifact Registry..."
gcloud auth configure-docker "${GCP_REGION}-docker.pkg.dev"
docker push "$IMAGE"
echo "  ✓ Pushed to: $IMAGE"

# ============================================================================
# Step 4: Enable Required GCP APIs
# ============================================================================
echo ""
echo "[4/6] Enabling GCP APIs..."

gcloud services enable \
    run.googleapis.com \
    container.googleapis.com \
    artifactregistry.googleapis.com \
    secretmanager.googleapis.com \
    bigquery.googleapis.com \
    storage.googleapis.com \
    cloudkms.googleapis.com

echo "  ✓ All APIs enabled"

# ============================================================================
# Step 5: Initialize and Plan Terraform
# ============================================================================
echo ""
echo "[5/6] Initializing Terraform..."

cd deploy/terraform

# Create terraform.tfvars for this deployment
cat > terraform.tfvars <<EOF
project_id = "$GCP_PROJECT"
region     = "$GCP_REGION"
environment = "dev"
image_tag  = "$IMAGE_TAG"
keymaster_max_instances = 10
EOF

echo "  Created terraform.tfvars"

echo ""
echo "  Running terraform init..."
terraform init

echo ""
echo "  Running terraform plan..."
terraform plan \
    -var="project_id=$GCP_PROJECT" \
    -var="region=$GCP_REGION" \
    -var="image_tag=$IMAGE_TAG" \
    -out=tfplan

# ============================================================================
# Step 6: Apply Terraform Configuration
# ============================================================================
echo ""
echo "[6/6] Deploying infrastructure..."

terraform apply tfplan

# ============================================================================
# Completion
# ============================================================================
echo ""
echo "=========================================="
echo "✓ Deployment Complete!"
echo "=========================================="
echo ""
echo "GCP Resources Created:"
echo "  • GKE Autopilot: kairos-dev"
echo "  • Cloud Run: keymaster-dev"
echo "  • Artifact Registry: ntg-kernel"
echo ""
echo "Outputs:"
terraform output

echo ""
echo "Next Steps:"
echo "  1. Update hostframe_bridge.rs with the Cloud Run URL"
echo "  2. Deploy KAIROS pods to the GKE cluster"
echo "  3. Configure secrets (KEYMASTER_BACKEND_URL)"
echo ""
echo "Connection test:"
echo "  gcloud run services describe keymaster-dev --region=$GCP_REGION"
echo ""
