#!/bin/bash

# glusterfs-sync.sh

# Load environment variables
if [ -f /etc/default/glusterfs_mount ]; then
    source /etc/default/glusterfs_mount
else
    echo "Environment file not found. Exiting."
    exit 1
fi

# Set up logging
LOG_FILE="/var/log/glusterfs-sync.log"

log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $1" | tee -a "$LOG_FILE"
}

# Function to sync a directory
sync_directory() {
    local src="$1"
    local dest="/mnt/glusterfs$1"
    if [ ! -d "$src" ]; then
        log "Directory $src does not exist. Skipping."
        return
    fi
    log "Syncing $src to $dest"
    if [ ! -d "$dest" ]; then
        log "Creating directory $dest in GlusterFS"
        mkdir -p "$dest"
    fi
    rsync -avz --delete "$src/" "$dest/"
    if [ $? -eq 0 ]; then
        log "Sync successful for $src"
    else
        log "Sync failed for $src"
    fi
}

# Function to bind mount a directory
bind_mount() {
    local src="$1"
    local dest="$2"
    if [ ! -d "$src" ]; then
        log "Directory $src does not exist. Skipping bind mount."
        return
    fi
    if [ ! -d "$dest" ]; then
        log "Creating directory $dest for bind mount"
        mkdir -p "$dest"
    fi
    log "Bind mounting $src to $dest"
    mkdir -p "$dest"
    mount --rbind "$src" "$dest"
    if [ $? -eq 0 ]; then
        log "Bind mount successful from $src to $dest"
    else
        log "Bind mount failed from $src to $dest"
    fi
}

# Main execution starts here
log "Starting glusterfs-sync script"

# Step 1: Mount GlusterFS
log "Mounting GlusterFS"
mkdir -p /mnt/glusterfs
mount -t glusterfs "${GLUSTER_NODE_IP}:/${GLUSTER_VOLUME_NAME}" /mnt/glusterfs
if [ $? -ne 0 ]; then
    log "Failed to mount GlusterFS. Exiting."
    exit 1
fi

# Define directories
USER_DIRS=(/bin /boot /etc /home /lib /lib32 /lib64 /libx32 /lost+found /media /opt /root /sbin /snap /srv /www /tmp /usr /var)
SYSTEM_DIRS=(/dev /proc /sys /run)

# Step 2: Sync user directories
log "Syncing user directories"
for dir in "${USER_DIRS[@]}"; do
    sync_directory "$dir"
done

# Step 3: Bind mount user directories
log "Performing bind mounts for user directories"
for dir in "${USER_DIRS[@]}"; do
    bind_mount "/mnt/glusterfs$dir" "$dir"
done

# Step 4: Bind mount system directories in reverse
log "Performing reverse bind mounts for system directories"
for dir in "${SYSTEM_DIRS[@]}"; do
    bind_mount "$dir" "/mnt/glusterfs$dir"
done

# Step 5: Set up inotifywait to watch for new directories
log "Setting up inotifywait watch"
inotifywait -m -e create -e moved_to --format '%w%f' / | while read path
do
    if [ -d "$path" ]; then
        log "New directory detected: $path"
        if [[ " ${SYSTEM_DIRS[@]} " =~ " ${path} " ]]; then
            # It's a system directory, do reverse bind mount
            bind_mount "$path" "/mnt/glusterfs$path"
        else
            # It's a user directory, sync and bind mount
            sync_directory "$path"
            bind_mount "/mnt/glusterfs$path" "$path"
        fi
    elif [ -f "$path" ]; then
        log "New file detected: $path"
        dirname=$(dirname "$path")
        if [[ ! " ${SYSTEM_DIRS[@]} " =~ " ${dirname} " ]]; then
            # Only sync if it's not in a system directory
            sync_directory "$dirname"
        fi
    fi
done &

log "glusterfs-sync script completed initial setup"

# Keep the script running to maintain the inotifywait process
wait
