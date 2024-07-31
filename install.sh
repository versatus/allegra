#!/bin/bash

set -e

BASE_DIR="/mnt/glusterfs"
IMAGES_DIR="/var/lib/libvirt/images"
VMS_DIR="$BASE_DIR/vms"
GROUP_NAME="glusterfs_users"
RUST_VERSION="1.70.0"

#Distro images
declare -A DISTROS=(
    ["ubuntu"]="https://cloud-images.ubuntu.com/jammy/current/jammy-server-cloudimg-amd64-disk-kvm.img|ubuntu-22.04.qcow2"
    ["centos"]="https://cloud.centos.org/centos/8/x86_64/images/CentOS-8-GenericCloud-8.2.2004-20200611.2.x86_64.qcow2|centos-8.qcow2"
    ["fedora"]="https://download.fedoraproject.org/pub/fedora/linux/releases/40/Cloud/x86_64/images/Fedora-Cloud-Base-Generic.x86_64-40-1.14.qcow2|fedora-40.qcow2"
    ["debian"]="https://cloud.debian.org/images/cloud/bullseye/20240717-1811/debian-11-nocloud-amd64-20240717-1811.qcow2|debian-11.qcow2"
    ["arch"]="https://geo.mirror.pkgbuild.com/images/latest/Arch-Linux-x86_64-cloudimg.qcow2|arch-linux-x86_64.qcow2"
    ["alpine"]="https://dl-cdn.alpinelinux.org/alpine/v3.20/releases/cloud/nocloud_alpine-3.20.2-x86_64-bios-tiny-r0.qcow2|alpine-3.20.qcow2"
)

# Check if script is run as root
check_root() {
    if [ "$EUID" -ne 0 ]; then
        echo "Must run as root"
        exit
    fi
}

# install dependencies
install_dependencies() {
    apt-get update
    apt-get install -y curl qemu-utils libvirt-clients libvirt-daemon-system
}

setup_rust() {
    local user_home
    local actual_user

    if [ "$SUDO_USER" ]; then
        actual_user="$SUDO_USER"
        user_home=$(getent passwd "$SUDO_USER" | cut -d: -f6)
    else
        actual_user="$USER"
        user_home="$HOME"
    fi

    if ! sudo -u "$actual_user" command -v rustc &> /dev/null; then
        echo "Installing Rust for user $actual_user"
        sudo -u "$actual_user" bash -c "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain $RUST_VERSION"
        echo "Rust successfully installed"
    else
        echo "Updating Rust for user $actual_user"
        sudo -u "$actual_user" bash -c "source $user_home/.cargo/env && rustup update && rustup default $RUST_VERSION"
    fi
    
    echo "Rust setup complete"
}

# Create necessary directories and set permissions
setup_directories() {
    mkdir -p $IMAGES_DIR $VMS_DIR

    if ! getent group $GROUP_NAME > /dev/null 2>&1; then
        groupadd $GROUP_NAME
    fi

    chown -R root:$GROUP_NAME $BASE_DIR
    chown -R root:$GROUP_NAME $IMAGES_DIR
    chown -R root:$GROUP_NAME $VMS_DIR
    chmod -R 775 $BASE_DIR
    chmod g+s $BASE_DIR $IMAGES_DIR $VMS_DIR

    setfacl -d -m g::rwx $BASE_DIR $IMAGES_DIR $VMS_DIR
}

download_images() {
    echo "Starting image downloads..."
    for distro in "${!DISTROS[@]}"; do 
        echo "Processing $distro..."
        entry="${DISTROS[$distro]}"
        echo "Full entry: $entry"
        
        IFS='|' read -r url target_filename <<< "$entry"
        echo "URL: '$url'"
        echo "Target filename: '$target_filename'"
        
        if [[ -z "$url" || -z "$target_filename" ]]; then
            echo "Error: Invalid URL or target filename for $distro. Skipping."
            continue
        fi
        
        source_filename=$(basename "$url")
        target_file="$IMAGES_DIR/$distro/$target_filename"
        temp_file="$IMAGES_DIR/$distro/$source_filename"

        mkdir -p "$IMAGES_DIR/$distro"

        if [ ! -f "$target_file" ]; then
            echo "Downloading $distro image..."
            if wget -O "$temp_file" "$url"; then
                if [[ $source_filename != *.qcow2 ]]; then
                    qemu-img convert -f qcow2 "$temp_file" "$target_file"
                    rm "$temp_file"
                elif [[ $source_filename != $target_filename ]]; then
                    mv "$temp_file" "$target_file"
                fi
                echo "$distro image prepared successfully as $target_filename."
            else
                echo "Failed to download $distro image. Skipping."
            fi
        else
            echo "$distro image ($target_filename) already exists, skipping."
        fi
    done
    echo "Image downloads completed."
}

print_distros() {
    echo "Contents of DISTROS array:"
    for distro in "${!DISTROS[@]}"; do
        echo "$distro: ${DISTROS[$distro]}"
    done
}


check_root
install_dependencies
setup_rust
setup_directories
print_distros
download_images
