#!/bin/bash
# Vaultwarden Backup Script
# Run as vaultwarden user to preserve permissions

BACKUP_DIR="/opt/vaultwarden/data"
TIMESTAMP=$(date +%Y%m%d%H%M%S)
BACKUP_FILE="$BACKUP_DIR/db.sqlite3.backup.$TIMESTAMP"

# Create backup using sqlite3 backup command (safe for running database)
sqlite3 "$BACKUP_DIR/db.sqlite3" ".backup $BACKUP_FILE"

# Set correct ownership
chown vaultwarden:vaultwarden "$BACKUP_FILE"

# Keep only last 7 backups
ls -t "$BACKUP_DIR"/db.sqlite3.backup.* 2>/dev/null | tail -n +8 | xargs -r rm

echo "Backup created: $BACKUP_FILE"
