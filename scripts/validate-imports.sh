#!/bin/bash
# validate-imports.sh: Pre-commit import validation

echo "🔍 Validating imports..."

./scripts/audit-modules.sh || exit 1

echo "✅ All imports are valid"
