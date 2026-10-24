# Vora Remote Worker Runtime Blueprint

## 1. System Requirements
* Ubuntu 22.04 LTS / 24.04 LTS (x86_64)
* Minimum 2 vCPU, 4GB RAM, 40GB SSD
* Port 4040 open for REST & SSE Streams
* UFW / Firewall configured

## 2. Automated Setup
Execute the automated provisioning script:
\`\`\`bash
chmod +x deploy/install.sh
./deploy/install.sh
\`\`\`

## 3. Environment Flags
* \`WORKER_MODE=remote\`: Enforces mandatory \`WORKER_AUTH_TOKEN\` check.
* \`WORKER_AUTH_TOKEN\`: Bearer token for desktop authentication.
* \`WORKER_PORT=4040\`: Default listening port.
* \`RUST_LOG=info\`: Structured output telemetry.

## 4. Verification Gate
Run Doctor 2.0 to confirm 100% readiness:
\`\`\`bash
vora_worker doctor
\`\`\`
