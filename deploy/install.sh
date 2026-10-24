#!/usr/bin/env bash
set -euo pipefail

echo "============================================================"
echo "🚀 VORA WORKER UBUNTU INSTALLER"
echo "============================================================"

# ۱. به‌روزرسانی پکیج‌ها و نصب ابزارهای پایه
sudo apt-get update
sudo apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    git \
    build-essential \
    pkg-config \
    libssl-dev \
    ufw

# ۲. نصب Docker و Docker Compose رسمی
if ! command -v docker &> /dev/null; then
    echo "[*] Installing Docker Engine..."
    curl -fsSL https://get.docker.com | sh
    sudo usermod -aG docker "$USER"
fi

# ۳. ایجاد دایرکتوری داده‌های پایدار ورکر
echo "[*] Setting up persistent storage..."
sudo mkdir -p /data/vora/logs
sudo mkdir -p /data/vora/sandboxes
sudo chown -R "$USER":"$USER" /data/vora

# ۴. تنظیم فایروال اولیه (UFW)
echo "[*] Configuring firewall..."
sudo ufw allow ssh
sudo ufw allow 4040/tcp comment 'Vora Worker REST API'
sudo ufw --force enable

echo "============================================================"
echo "✅ Environment setup complete!"
echo "You can now run: docker compose -f docker-compose.worker.yml up -d --build"
echo "============================================================"
