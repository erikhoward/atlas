# Standalone Deployment Guide

This guide covers deploying Atlas as a standalone binary on Linux, macOS, or Windows servers.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Installation](#installation)
- [Configuration](#configuration)
- [Running Atlas](#running-atlas)
- [Systemd Service (Linux)](#systemd-service-linux)
- [Scheduled Exports](#scheduled-exports)
- [Monitoring and Logging](#monitoring-and-logging)
- [Troubleshooting](#troubleshooting)

## Prerequisites

### System Requirements

- **Operating System**: Linux (Ubuntu 20.04+, RHEL 8+), macOS 11+, or Windows Server 2019+
- **CPU**: 2+ cores recommended
- **Memory**: 2GB+ RAM (4GB+ for large exports)
- **Disk**: 10GB+ free space for logs and temporary files
- **Network**: Outbound HTTPS access to OpenEHR server and Azure

### Dependencies

**Linux**:
```bash
# Ubuntu/Debian
sudo apt-get update
sudo apt-get install -y ca-certificates curl

# RHEL/CentOS
sudo yum install -y ca-certificates curl
```

**macOS**:
```bash
# Install Homebrew if not already installed
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
```

**Windows**:
- No additional dependencies required

## Installation

### Option 1: Download Pre-built Binary

**Linux (x86_64)**:
```bash
# Download latest release
curl -LO https://github.com/erikhoward/atlas/releases/download/v1.0.0/atlas-linux-x86_64.tar.gz

# Extract
tar -xzf atlas-linux-x86_64.tar.gz

# Install to system path
sudo mv atlas /usr/local/bin/
sudo chmod +x /usr/local/bin/atlas

# Verify installation
atlas --version
```

**macOS (x86_64)**:
```bash
# Download latest release
curl -LO https://github.com/erikhoward/atlas/releases/download/v1.0.0/atlas-macos-x86_64.tar.gz

# Extract
tar -xzf atlas-macos-x86_64.tar.gz

# Install to system path
sudo mv atlas /usr/local/bin/
sudo chmod +x /usr/local/bin/atlas

# Verify installation
atlas --version
```

**Windows**:
```powershell
# Download from GitHub releases page
# Extract atlas.exe to C:\Program Files\Atlas\

# Add to PATH
$env:Path += ";C:\Program Files\Atlas"

# Verify installation
atlas --version
```

### Option 2: Build from Source

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Clone repository
git clone https://github.com/erikhoward/atlas.git
cd atlas

# Build release binary
cargo build --release

# Install
sudo cp target/release/atlas /usr/local/bin/

# Verify
atlas --version
```

## Configuration

### Create Configuration Directory

**Linux**:
```bash
sudo mkdir -p /etc/atlas
sudo mkdir -p /var/log/atlas
sudo chown $USER:$USER /etc/atlas /var/log/atlas
```

**macOS**:
```bash
sudo mkdir -p /usr/local/etc/atlas
sudo mkdir -p /usr/local/var/log/atlas
sudo chown $USER:staff /usr/local/etc/atlas /usr/local/var/log/atlas
```

**Windows**:
```powershell
New-Item -ItemType Directory -Path "C:\ProgramData\Atlas\config"
New-Item -ItemType Directory -Path "C:\ProgramData\Atlas\logs"
```

### Generate Configuration File

**Linux/macOS**:
```bash
atlas init --with-examples --output /etc/atlas/atlas.toml
```

**Windows**:
```powershell
atlas init --with-examples --output "C:\ProgramData\Atlas\config\atlas.toml"
```

### Edit Configuration

**Linux/macOS**:
```bash
vi /etc/atlas/atlas.toml
```

**Windows**:
```powershell
notepad "C:\ProgramData\Atlas\config\atlas.toml"
```

Update the following sections:
```toml
[openehr]
base_url = "https://your-ehrbase-server.com/ehrbase/rest/openehr/v1"
username = "${ATLAS_OPENEHR_USERNAME}"
password = "${ATLAS_OPENEHR_PASSWORD}"

[openehr.query]
template_ids = ["Your Template.v1"]

[cosmosdb]
endpoint = "https://your-account.documents.azure.com:443/"
key = "${ATLAS_COSMOSDB_KEY}"
database_name = "openehr_data"

[logging]
local_path = "/var/log/atlas"  # Linux/macOS
# local_path = "C:\\ProgramData\\Atlas\\logs"  # Windows
```

### Set Environment Variables

**Linux/macOS**:
```bash
# Create environment file
sudo tee /etc/atlas/atlas.env > /dev/null << EOF
ATLAS_OPENEHR_USERNAME=your-openehr-username
ATLAS_OPENEHR_PASSWORD=your-openehr-password
ATLAS_COSMOSDB_KEY=your-cosmos-db-key
EOF

# Secure the file
sudo chmod 600 /etc/atlas/atlas.env

# Load environment variables
source /etc/atlas/atlas.env
```

**Windows**:
```powershell
# Set system environment variables
[System.Environment]::SetEnvironmentVariable("ATLAS_OPENEHR_USERNAME", "your-username", "Machine")
[System.Environment]::SetEnvironmentVariable("ATLAS_OPENEHR_PASSWORD", "your-password", "Machine")
[System.Environment]::SetEnvironmentVariable("ATLAS_COSMOSDB_KEY", "your-key", "Machine")
```

### Validate Configuration

```bash
atlas validate-config -c /etc/atlas/atlas.toml
```

## Running Atlas

### Manual Execution

**Linux/macOS**:
```bash
# Load environment variables
source /etc/atlas/atlas.env

# Run export
atlas export -c /etc/atlas/atlas.toml

# Dry run
atlas export -c /etc/atlas/atlas.toml --dry-run

# Check status
atlas status -c /etc/atlas/atlas.toml
```

**Windows**:
```powershell
# Run export
atlas export -c "C:\ProgramData\Atlas\config\atlas.toml"

# Dry run
atlas export -c "C:\ProgramData\Atlas\config\atlas.toml" --dry-run

# Check status
atlas status -c "C:\ProgramData\Atlas\config\atlas.toml"
```

## Systemd Service (Linux)

### Create Service File

```bash
sudo tee /etc/systemd/system/atlas.service > /dev/null << 'EOF'
[Unit]
Description=Atlas OpenEHR to Cosmos DB ETL
After=network-online.target
Wants=network-online.target

[Service]
Type=oneshot
User=atlas
Group=atlas
EnvironmentFile=/etc/atlas/atlas.env
ExecStart=/usr/local/bin/atlas export -c /etc/atlas/atlas.toml
StandardOutput=journal
StandardError=journal
SyslogIdentifier=atlas

# Security hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/log/atlas

[Install]
WantedBy=multi-user.target
EOF
```

### Create Service User

```bash
sudo useradd -r -s /bin/false atlas
sudo chown -R atlas:atlas /etc/atlas /var/log/atlas
```

### Enable and Start Service

```bash
# Reload systemd
sudo systemctl daemon-reload

# Enable service (start on boot)
sudo systemctl enable atlas.service

# Start service manually
sudo systemctl start atlas.service

# Check status
sudo systemctl status atlas.service

# View logs
sudo journalctl -u atlas.service -f
```

### Create Timer for Scheduled Exports

```bash
sudo tee /etc/systemd/system/atlas.timer > /dev/null << 'EOF'
[Unit]
Description=Atlas Export Timer
Requires=atlas.service

[Timer]
# Run daily at 2 AM
OnCalendar=*-*-* 02:00:00
Persistent=true

[Install]
WantedBy=timers.target
EOF
```

Enable the timer:
```bash
sudo systemctl daemon-reload
sudo systemctl enable atlas.timer
sudo systemctl start atlas.timer

# Check timer status
sudo systemctl list-timers atlas.timer
```

## Scheduled Exports

### Cron (Linux/macOS)

Create a cron job for regular exports:

```bash
# Edit crontab
crontab -e

# Add entry (daily at 2 AM)
0 2 * * * source /etc/atlas/atlas.env && /usr/local/bin/atlas export -c /etc/atlas/atlas.toml >> /var/log/atlas/cron.log 2>&1
```

### Task Scheduler (Windows)

```powershell
# Create scheduled task
$action = New-ScheduledTaskAction -Execute "C:\Program Files\Atlas\atlas.exe" -Argument "export -c C:\ProgramData\Atlas\config\atlas.toml"
$trigger = New-ScheduledTaskTrigger -Daily -At 2am
$principal = New-ScheduledTaskPrincipal -UserId "SYSTEM" -LogonType ServiceAccount -RunLevel Highest
$settings = New-ScheduledTaskSettingsSet -StartWhenAvailable -RestartCount 3 -RestartInterval (New-TimeSpan -Minutes 5)

Register-ScheduledTask -TaskName "Atlas Export" -Action $action -Trigger $trigger -Principal $principal -Settings $settings
```

## Monitoring and Logging

### Log Files

**Linux/macOS**:
```bash
# View logs
tail -f /var/log/atlas/atlas.log

# Search logs
grep "ERROR" /var/log/atlas/atlas.log

# Rotate logs (logrotate)
sudo tee /etc/logrotate.d/atlas > /dev/null << 'EOF'
/var/log/atlas/*.log {
    daily
    rotate 30
    compress
    delaycompress
    missingok
    notifempty
    create 0640 atlas atlas
}
EOF
```

**Windows**:
```powershell
# View logs
Get-Content "C:\ProgramData\Atlas\logs\atlas.log" -Tail 50 -Wait

# Search logs
Select-String -Path "C:\ProgramData\Atlas\logs\atlas.log" -Pattern "ERROR"
```

### Monitoring Script

Create a monitoring script to check export status:

```bash
#!/bin/bash
# /usr/local/bin/atlas-monitor.sh

CONFIG="/etc/atlas/atlas.toml"
LOG="/var/log/atlas/monitor.log"

# Run status check
if ! atlas status -c "$CONFIG" >> "$LOG" 2>&1; then
    echo "$(date): Atlas status check failed" >> "$LOG"
    # Send alert (email, Slack, etc.)
    exit 1
fi

echo "$(date): Atlas status check passed" >> "$LOG"
exit 0
```

Make it executable and schedule:
```bash
sudo chmod +x /usr/local/bin/atlas-monitor.sh

# Add to crontab (every hour)
0 * * * * /usr/local/bin/atlas-monitor.sh
```

### Azure Log Analytics Integration

Enable Azure logging in configuration:

```toml
[logging]
azure_enabled = true
azure_tenant_id = "${AZURE_TENANT_ID}"
azure_client_id = "${AZURE_CLIENT_ID}"
azure_client_secret = "${AZURE_CLIENT_SECRET}"
azure_log_analytics_workspace_id = "${AZURE_LOG_ANALYTICS_WORKSPACE_ID}"
azure_dcr_immutable_id = "${AZURE_DCR_IMMUTABLE_ID}"
azure_dce_endpoint = "${AZURE_DCE_ENDPOINT}"
azure_stream_name = "Custom-AtlasExport_CL"
```

Set environment variables:
```bash
export AZURE_TENANT_ID="your-tenant-id"
export AZURE_CLIENT_ID="your-client-id"
export AZURE_CLIENT_SECRET="your-client-secret"
export AZURE_LOG_ANALYTICS_WORKSPACE_ID="your-workspace-id"
export AZURE_DCR_IMMUTABLE_ID="your-dcr-id"
export AZURE_DCE_ENDPOINT="https://your-dce.monitor.azure.com"
```

## Troubleshooting

### Permission Denied

**Problem**: `Permission denied` when accessing log directory

**Solution**:
```bash
sudo chown -R atlas:atlas /var/log/atlas
sudo chmod 755 /var/log/atlas
```

### Service Won't Start

**Problem**: Systemd service fails to start

**Solution**:
```bash
# Check service status
sudo systemctl status atlas.service

# View detailed logs
sudo journalctl -u atlas.service -n 50

# Check configuration
atlas validate-config -c /etc/atlas/atlas.toml

# Test manually
sudo -u atlas /usr/local/bin/atlas export -c /etc/atlas/atlas.toml
```

### Environment Variables Not Loaded

**Problem**: Environment variables not available to service

**Solution**: Ensure `EnvironmentFile` is set in systemd service and file exists:
```bash
ls -l /etc/atlas/atlas.env
sudo systemctl daemon-reload
sudo systemctl restart atlas.service
```

### High Memory Usage

**Problem**: Atlas consuming too much memory

**Solution**: Reduce batch size and parallelism in configuration:
```toml
[openehr.query]
batch_size = 500  # Reduce from 1000
parallel_ehrs = 4  # Reduce from 8
```

---

For more information, see:
- [Docker Deployment](docker.md) - Containerized deployment
- [Kubernetes Deployment](kubernetes.md) - Kubernetes/AKS deployment
- [User Guide](../user-guide.md) - Usage instructions
- [Configuration Guide](../configuration.md) - Configuration reference

