use crate::distro::Distro;
use crate::Namespace;
use std::os::fd::AsRawFd;
use std::fs::create_dir_all;
use nix::fcntl::{open, OFlag};
use nix::mount::{mount, umount};
use nix::sys::stat::Mode;
use nix::unistd::close;
use serde::de::Error;
use std::fs;
use std::path::Path;
use libc;
use std::os::unix::fs::MetadataExt;
use fs_extra::dir::move_dir;
use fs_extra::dir::CopyOptions;

const LOOP_CTL_GET_FREE: libc::c_ulong = 0x4C82;
const LOOP_SET_FD: libc::c_ulong = 0x4C00;
const LOOP_CLR_FD: libc::c_ulong = 0x4C01;
const BLKPG: libc::c_int = 0x1269;
const BLKPG_ADD_PARTITION: libc::c_int = 1;

#[repr(C)]
struct blkpg_ioctl_arg {
    op: libc::c_int,
    flags: libc::c_int,
    datalen: libc::c_int,
    data: *mut blkpg_partition,
}

#[repr(C)]
struct blkpg_partition {
    start: libc::c_longlong,
    length: libc::c_longlong,
    pno: libc::c_int,
    devname: [libc::c_char; 64],
    volname: [libc::c_char; 64],
}

pub fn prepare_nfs_brick(namespace: &Namespace) -> std::io::Result<()> {
    create_dir_all(&format!(
        "/mnt/glusterfs/vms/{}/brick",
        namespace.inner().to_string()
    ))?;

    Ok(())
}

fn convert_disk_image(source: &str, dest: &str, fmt: &str) -> std::io::Result<()> {

    let dest_path = Path::new(dest).parent().ok_or(
        std::io::Error::new(
            std::io::ErrorKind::Other,
            "invalid path to destination in convert_disk_image"
        )
    )?;
    std::fs::create_dir_all(dest_path)?;
    std::process::Command::new("qemu-img")
        .arg("convert")
        .arg("-O")
        .arg(fmt)
        .arg(source)
        .arg(dest)
        .output()?;

    Ok(())
}

fn copy_disk_image(source: &str, dest: &str) -> std::io::Result<()> {
    let dest_path = Path::new(dest).parent().ok_or(
        std::io::Error::new(
            std::io::ErrorKind::Other,
            "invalid path for dest in copy_disk_image function"
        )
    )?;

    std::fs::create_dir_all(dest_path)?;
    std::fs::copy(source, dest)?;
    Ok(())
}

pub fn prepare_disk_image(
    source: &str,
    raw_tmp_dest: &str,
    used_from: &str,
    namespace: &Namespace
) -> std::io::Result<()> {
    convert_disk_image(source, raw_tmp_dest, "raw")?;
    copy_disk_image(source, used_from)?;
    copy_disk_image_contents_to_nfs_brick(source, namespace)?;

    Ok(())
}


pub fn alternative_prepare_disk_image(
    source: &str,
    raw_tmp_dest: &str,
    used_from: &str,
    namespace: &Namespace
) -> std::io::Result<()> {
    convert_disk_image(source, raw_tmp_dest, "raw").map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Error in convert_disk_image function: {e}")
        )
    })?;
    copy_disk_image(source, used_from).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Error in copy_disk_image function: {e}")
        )
    })?;
    alternative_copy_disk_image_contents_to_nfs_brick(raw_tmp_dest, namespace).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Error in alternative_copy_disk_image_contents_to_nfs_brick function: {e}")
        )
    })?;

    Ok(())
}

pub fn copy_disk_image_contents_to_nfs_brick(disk_path: &str, namespace: &Namespace) -> std::io::Result<()> {
    let loop_device = prepare_loop_device(disk_path)?;
    let result = std::panic::catch_unwind(|| -> std::io::Result<()> {
        partition_loop_device(&loop_device)?;
        let partition_device = format!("{}p1", loop_device);
        let tmp_mount_point = format!("/mnt/tmp/iso/{}", namespace.inner().to_string());
        std::fs::create_dir_all(&tmp_mount_point)?;

        mount(
            Some(std::path::Path::new(&partition_device)),
            std::path::Path::new(&tmp_mount_point), 
            Some("ext4"),
            nix::mount::MsFlags::empty(),
            None::<&str>
        )?;

        let brick_dir = format!("/mnt/glusterfs/vms/{}/brick", namespace.inner().to_string());
        std::fs::create_dir_all(&brick_dir)?;
        move_fs_to_nfs_brick(&tmp_mount_point, &brick_dir)?;
        unmount_loop_device(&tmp_mount_point)?;
        detach_loop_device(&loop_device)?;

        Ok(())

    });

    result.unwrap_or_else(|_| Err(std::io::Error::new(std::io::ErrorKind::Other, "Panic occurred")))
}


pub fn alternative_copy_disk_image_contents_to_nfs_brick(disk_path: &str, namespace: &Namespace) -> std::io::Result<()> {
    let loop_device = alternative_create_partition_loop_device(disk_path).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Error in alternative_create_partition_loop_device function: {e}")
        )
    })?;
    let partition_device = format!("{}p1", loop_device);
    let base_mount_point = format!("/mnt/glusterfs/vms/{}", namespace.inner().to_string());
    let tmp_mount_point = Path::new(&base_mount_point).join("tmp_iso_mount");
    log::info!("Temporary mount point: {}", tmp_mount_point.display());

    std::fs::create_dir_all(&tmp_mount_point).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Error trying to create tmp_mount_point: {}: {e}", tmp_mount_point.display())
        )
    })?;

    if !Path::new(&partition_device).exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Partition device does not exist: {}", partition_device)
        ));
    }

    if !tmp_mount_point.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Mount point does not exist: {}", tmp_mount_point.display())
        ));
    }

    mount(
        Some(Path::new(&partition_device)),
        &tmp_mount_point,
        Some("ext4"),
        nix::mount::MsFlags::empty(),
        None::<&str>
    ).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Error calling mount function: {e}")
        )
    })?;

    let brick_dir = format!("/mnt/glusterfs/vms/{}/brick", namespace.inner().to_string());
    log::info!("Brick directory: {}", brick_dir);

    std::fs::create_dir_all(&brick_dir).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Error attempting to create brick_dir: {brick_dir}: {e}")
        )
    })?;

    log::info!("Mount point: {}", tmp_mount_point.display());
    log::info!("Brick directory: {}", brick_dir);

    // Ensure both paths are on the same filesystem
    let tmp_mount_point_metadata = fs::metadata(&tmp_mount_point)?;
    let brick_dir_metadata = fs::metadata(&brick_dir)?;

    if tmp_mount_point_metadata.rdev() != brick_dir_metadata.rdev() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Temporary mount point and brick directory are on different filesystems"
        ));
    }

    move_fs_to_nfs_brick(&tmp_mount_point.display().to_string(), &brick_dir).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Error calling move_fs_to_nfs_brick: {e}")
        )
    })?;
    unmount_loop_device(&tmp_mount_point.display().to_string()).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Error calling unmount_loop_device: {e}")
        )
    })?;
    detach_loop_device(&loop_device).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Error calling detach_loop_device: {e}")
        )
    })?;

    Ok(())
}


pub fn prepare_loop_device(disk_path: &str) -> std::io::Result<String> {
    let loop_control_fd = open("/dev/loop-control", OFlag::O_RDWR, Mode::empty())?;
    let free_loop_number = unsafe {
        let mut num: libc::c_int = -1;
        if libc::ioctl(loop_control_fd, LOOP_CTL_GET_FREE, &mut num) < 0 {
            return Err(
                std::io::Error::last_os_error()
            )
        }
        num
    };

    close(loop_control_fd)?;

    if free_loop_number < 0 {
        return Err(
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "No free loop device available"
            )
        )
    }

    let loop_device = format!("/dev/loop{}", free_loop_number);
    let loop_fd = open(loop_device.as_str(), OFlag::O_RDWR, Mode::empty())?;
    let disk_fd = open(disk_path, OFlag::O_RDWR, Mode::empty())?;

    unsafe {
        if libc::ioctl(loop_fd, LOOP_SET_FD, disk_fd) < 0 {
            let err = std::io::Error::last_os_error();
            close(loop_fd).ok();
            close(disk_fd).ok();
            return Err(err);
        }
    } 

    close(loop_fd)?;
    close(disk_fd)?;

    Ok(loop_device)
}

fn alternative_create_partition_loop_device(disk_path: &str) -> std::io::Result<String> {
    let output = std::process::Command::new("sudo")
        .arg("losetup")
        .arg("-fP")
        .arg("--show")
        .arg(disk_path)
        .output()?;

    if output.status.success() {
        let loop_device = std::str::from_utf8(&output.stdout).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e
            )
        })?;

        let loop_device_trimmed = loop_device.trim().to_string();

        let partprobe_output = std::process::Command::new("sudo")
            .arg("partprobe")
            .arg(&loop_device_trimmed)
            .output()?;

        if !partprobe_output.status.success() {
            let err = std::str::from_utf8(&partprobe_output.stderr).map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e
                )
            })?;

            return Err(std::io::Error::new(std::io::ErrorKind::Other, err));
        }

        return Ok(loop_device_trimmed)
    } else {
        let err = std::str::from_utf8(&output.stderr).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e
            )
        })?;

        return Err(std::io::Error::new(std::io::ErrorKind::Other, err));
    }
}

fn partition_loop_device(loop_device: &str) -> std::io::Result<()> {
    let file = std::fs::File::open(loop_device)?;
    let fd = file.as_raw_fd();

    let mut partition = blkpg_partition {
        start: 0,
        length: 0,
        pno: 1,
        devname: [0; 64],
        volname: [0; 64]
    };

    let mut arg = blkpg_ioctl_arg {
        op: BLKPG_ADD_PARTITION,
        flags: 0,
        datalen: std::mem::size_of::<blkpg_partition>() as libc::c_int,
        data: &mut partition as *mut blkpg_partition
    };

    let result = unsafe {
        libc::ioctl(fd, BLKPG as _, &mut arg as *mut blkpg_ioctl_arg)
    };

    if result < 0 {
        return Err(std::io::Error::last_os_error());
    }

    Ok(())
}

fn move_fs_to_nfs_brick(tmp_mount_point: &str, brick_dir: &str) -> std::io::Result<()> {
    log::info!("Attempting to move contents of {} to {}", tmp_mount_point, brick_dir);

    let tmp_mount_path = Path::new(tmp_mount_point);
    let brick_path = Path::new(brick_dir);

    if !tmp_mount_path.exists() {
        log::info!("Source path does not exist: {}", tmp_mount_path.display());
        return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "Source path does not exist"));
    }

    if !brick_path.exists() {
        log::info!("Destination path does not exist: {}", brick_path.display());
        return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "Destination path does not exist"));
    }

    log::info!("Source contents before move:");
    for entry in fs::read_dir(tmp_mount_path)? {
        let entry = entry?;
        log::info!("{}", entry.path().display());
    }

    log::info!("Destination contents before move:");
    for entry in fs::read_dir(brick_path)? {
        let entry = entry?;
        log::info!("{}", entry.path().display());
    }

    let result = move_recursive(tmp_mount_path, brick_path);

    match result {
        Ok(_) => log::info!("Move operation successful"),
        Err(ref e) => log::info!("Error in move_recursive: {}", e),
    }

    log::info!("Source contents after move:");
    for entry in fs::read_dir(tmp_mount_path).unwrap_or_else(|_| fs::read_dir("/").unwrap()) {
        let entry = entry?;
        log::info!("{}", entry.path().display());
    }

    log::info!("Destination contents after move:");
    for entry in fs::read_dir(brick_path)? {
        let entry = entry?;
        log::info!("{}", entry.path().display());
    }

    result
}

pub fn unmount_loop_device(mount_point: &str) -> std::io::Result<()> {
    umount(mount_point).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Error calling umount function: {e}")
        )
    })?;
    Ok(())
}

pub fn detach_loop_device(loop_device: &str) -> std::io::Result<()> {
    let fd = open(loop_device, OFlag::O_RDWR, Mode::empty())?;

    unsafe {
        if libc::ioctl(fd, LOOP_CLR_FD, 0) < 0 {
            return Err(
                std::io::Error::last_os_error()
            )
        }
    }

    close(fd)?;

    Ok(())
}


pub fn get_image_path(distro: Distro) -> std::path::PathBuf {
    std::path::PathBuf::from(distro)
}

pub async fn get_image_name(distro: Distro) -> std::io::Result<String> {
    get_image_path(distro)
        .file_name()
        .and_then(|os_str| os_str.to_str())
        .map(String::from)
        .ok_or(std::io::Error::new(
            std::io::ErrorKind::Other,
            "unable to extract the image name",
        ))
}

fn move_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    log::info!("Moving from {} to {}", src.display(), dst.display());

    if !src.exists() {
        log::info!("Source path does not exist: {}", src.display());
        return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "Source path does not exist"));
    }

    if !dst.exists() {
        log::info!("Destination path does not exist, creating: {}", dst.display());
        fs::create_dir_all(dst)?;
    }

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let target = dst.join(entry.file_name());

        if path.is_dir() {
            log::info!("Recursively moving directory: {}", path.display());
            if let Err(e) = std::process::Command::new("sudo")
                .arg("mv")
                .arg(&path)
                .arg(&target)
                .output()
            {
                log::info!("Error moving directory {} to {}: {}", path.display(), target.display(), e);
            }
        } else if path.symlink_metadata()?.file_type().is_symlink() {
            log::info!("Moving symlink from {} to {}", path.display(), target.display());
            if let Err(e) = std::process::Command::new("sudo")
                .arg("mv")
                .arg(&path)
                .arg(&target)
                .output()
            {
                log::info!("Error moving symlink {} to {}: {}", path.display(), target.display(), e);
            }
        } else {
            log::info!("Moving file from {} to {}", path.display(), target.display());
            if let Err(e) = std::process::Command::new("sudo")
                .arg("mv")
                .arg(&path)
                .arg(&target)
                .output()
            {
                log::info!("Error moving file {} to {}: {}", path.display(), target.display(), e);
            }
        }
    }

    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prepare_nfs_brick() {
        let namespace = Namespace::new("test_namespace".to_string());
        
        // Mock the actual directories
        prepare_nfs_brick(&namespace).unwrap();

        let base_dir = std::path::Path::new("/mnt/glusterfs");
        let expected_path = base_dir.join("vms").join("test_namespace").join("brick");
        assert!(expected_path.exists());
        assert!(expected_path.is_dir());
    }

    #[test]
    fn test_get_image_path() {
        let ubuntu_path = get_image_path(Distro::Ubuntu);
        assert_eq!(ubuntu_path, std::path::PathBuf::from("/var/lib/libvirt/images/ubuntu/ubuntu-22.04.qcow2"));
        
        let centos_path = get_image_path(Distro::CentOS);
        assert_eq!(centos_path, std::path::PathBuf::from("/var/lib/libvirt/images/centos/centos-8.qcow2"));
    }

    #[tokio::test]
    async fn test_get_image_name() {
        let ubuntu_name = get_image_name(Distro::Ubuntu).await.unwrap();
        assert_eq!(ubuntu_name, "ubuntu-22.04.qcow2");
        
        let centos_name = get_image_name(Distro::CentOS).await.unwrap();
        assert_eq!(centos_name, "centos-8.qcow2");
    }

    #[test]
    #[ignore = "not complete"]
    fn test_prepare_disk_image() {
        // Create a temporary directory for our test
        let temp_dir = tempfile::TempDir::new().unwrap();
        
        // Path to an actual disk image (you'll need to provide this)
        let source = std::path::Path::new("/var/lib/libvirt/images/ubuntu/ubuntu-22.04.qcow2");
        
        let raw_tmp_dest = temp_dir.path().join("raw_tmp.img");
        let used_from = temp_dir.path().join("used_from.qcow2");
        let namespace = Namespace::new("test_namespace".to_string());
        
        // Prepare the disk image
        prepare_disk_image(
            source.to_str().unwrap(),
            raw_tmp_dest.to_str().unwrap(),
            used_from.to_str().unwrap(),
            &namespace
        ).unwrap();
        
        // Assert that the temporary and final disk images were created
        assert!(raw_tmp_dest.exists());
        assert!(used_from.exists());
        
        // Assert the structure of the NFS brick
        let brick_path = std::path::Path::new("/mnt/glusterfs/vms").join(namespace.inner()).join("brick");
        assert!(brick_path.exists(), "Brick path does not exist");
        assert!(brick_path.join("bin").exists(), "/bin directory does not exist");
        assert!(brick_path.join("usr/bin").exists(), "/usr/bin directory does not exist");
        assert!(brick_path.join("usr/bin/busybox").exists(), "/usr/bin/busybox does not exist");
        
        // Clean up (remove the brick directory)
        let vm_path = std::path::Path::new("/mnt/glusterfs/vms").join(namespace.inner());
        std::fs::remove_dir_all(vm_path).unwrap();
    }


    #[test]
    fn test_alternative_prepare_disk_image() {
        // Path to an actual disk image (you'll need to provide this)
        let source = std::path::Path::new("/var/lib/libvirt/images/ubuntu/ubuntu-22.04.qcow2");
        
        let tmp_dir = std::path::Path::new("/mnt/iso/ubuntu-22.04");
        let used_from = std::path::Path::new("/mnt/images/");
        std::fs::create_dir_all(tmp_dir).unwrap();
        std::fs::create_dir_all(used_from).unwrap();
        let used_from_path = used_from.join("ubuntu-22.04.qcow2");
        let raw_tmp_dest = tmp_dir.join("raw_tmp.img"); 
        let namespace = Namespace::new("test_namespace".to_string());
        
        // Prepare the disk image
        alternative_prepare_disk_image(
            source.to_str().unwrap(),
            raw_tmp_dest.to_str().unwrap(),
            used_from_path.to_str().unwrap(),
            &namespace
        ).unwrap();
        
        
        // Assert that the temporary and final disk images were created
        assert!(raw_tmp_dest.exists());
        assert!(used_from.exists());
        
        // Assert the structure of the NFS brick
        let brick_path = std::path::Path::new("/mnt/glusterfs/vms").join(namespace.inner()).join("brick");
        assert!(brick_path.exists(), "Brick path does not exist");
        assert!(brick_path.join("bin").exists(), "/bin directory does not exist");
        assert!(brick_path.join("usr/bin").exists(), "/usr/bin directory does not exist");
        assert!(brick_path.join("usr/bin/busybox").exists(), "/usr/bin/busybox does not exist");
        
        let vm_path = std::path::Path::new("/mnt/glusterfs/vms").join(namespace.inner());
        std::fs::remove_dir_all(vm_path).unwrap();
    }


    #[test]
    #[ignore = "not complete"]
    fn test_generate_cloud_init_files() {
        /*
        let temp_dir = tempdir().unwrap();
        let instance_name = "test_instance";
        let hostname = "test_hostname";
        let user_data = Some(UserData {
            // Fill this with sample user data
        });
        let ip = "192.168.1.100";
        
        // Mock the actual directories
        std::env::set_var("MOCK_CLOUD_INIT_PATH", temp_dir.path().to_str().unwrap());
        
        generate_cloud_init_files(instance_name, hostname, user_data, ip).unwrap();
        
        let meta_data_path = temp_dir.path().join(instance_name).join("meta-data");
        let user_data_path = temp_dir.path().join(instance_name).join("user-data");
        let network_config_path = temp_dir.path().join(instance_name).join("network-config");
        
        assert!(meta_data_path.exists());
        assert!(user_data_path.exists());
        assert!(network_config_path.exists());
        
        // You might want to add more assertions here to check the content of these files
        */
    }
}
