use crate::statics::SERVER_BLOCK_TEMPLATE;

pub async fn update_nginx_config(
    instance_ip: &str,
    instance_port: u16,
    host_port: u16
) -> std::io::Result<()> {

    log::info!("attempting to update NGINX config file");
    let new_server_block = SERVER_BLOCK_TEMPLATE
        .replace("{host_port}", &host_port.to_string())
        .replace("{instance_ip}", &instance_ip)
        .replace("{instance_port}", &instance_port.to_string());

    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(format!(
                "echo '{}' | sudo tee -a /etc/nginx/sites-available/default",
                new_server_block
            )
        ).output()?;

    if !output.status.success() {
        return Err(
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "error writing new server block to nginx config"
            )
        )
    }

    log::info!("wrote to nginx config file");
    let output = std::process::Command::new("sudo")
        .args(["nginx", "-t"])
        .output()?;

    if !output.status.success() {
        return Err(
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "nginx config has a syntax error"
            )
        )
    }
    log::info!("confirmed nginx config file free of syntax errors...");

    let output = std::process::Command::new("sudo")
        .args(["systemctl", "reload", "nginx"])
        .output()?;

    if !output.status.success() {
        return Err(
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "could not reload nginx after updating config"
            )
        )
    }

    log::info!("reloaded nginx...");
    Ok(())
}

pub async fn reload_nginx() -> Result<(), std::io::Error> {
    let output = tokio::process::Command::new("sudo")
        .arg("nginx")
        .arg("-s")
        .arg("reload")
        .output()
        .await?;

    if output.status.success() {
        return Ok(())
    } else {
        Err(
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to reload nginx"
            )
        )
    }
}
