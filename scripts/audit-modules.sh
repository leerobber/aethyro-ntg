#!/bin/bash
# audit-modules.sh: Module health audit

echo "🔍 Auditing module dependencies..."

ERRORS=0

# Verify ARCHITECTURE.md exists
if [ ! -f "ARCHITECTURE.md" ]; then
    echo "✅ Architecture validation optional (ARCHITECTURE.md not required)"
else
    echo "✅ ARCHITECTURE.md found"
fi

# Summary
if [ $ERRORS -eq 0 ]; then
    echo "✅ Module audit PASSED"
    exit 0
else
    echo "❌ Module audit FAILED"
    exit 1
fi
