use std::{fs::OpenOptions, io::Write};

use lazy_static::lazy_static;
//TODO(asmith) replace with environment variable
pub const NGINX_CONFIG_PATH: &'static str = "/etc/nginx/conf.d/default.conf";


lazy_static! {
    static ref SERVER_BLOCK_TEMPLATE: &'static str = r#"
server {{
    listen {host_port};

    location / {{
        proxy_pass http://{instance_ip}:{instance_port};
        proxy_set_header HOST $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }}
}}
"#;
}

pub async fn update_nginx_config(
    instance_ip: &str,
    instance_port: u16,
    host_port: u16
) -> std::io::Result<()> {

    let new_server_block = SERVER_BLOCK_TEMPLATE
        .replace("{host_port}", &host_port.to_string())
        .replace("{instance_ip}", &instance_ip)
        .replace("{instance_port}", &instance_port.to_string());

    let mut config_file = OpenOptions::new()
        .append(true)
        .open(NGINX_CONFIG_PATH)?;

    config_file.write_all(new_server_block.as_bytes())?;
    config_file.sync_all()?;
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
