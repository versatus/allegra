pub const NGINX_CONFIG_PATH: &'static str = "/etc/nginx/sites-available/default";
pub const LOWEST_PORT: u16 = 2222;
pub const HIGHEST_PORT: u16 = 65535;
pub const ELECTION_BLOCK_INTERVAL: u64 = 1800;
pub const PUBKEY_AUTH_STARTUP_SCRIPT: &'static str = r#"
if grep -q '#PubkeyAuthentication yes' /etc/ssh/sshd_config; then
    sed -i 's/#PubkeyAuthentication yes/PubkeyAuthentication yes/' /etc/ssh/sshd_config
elif ! grep -q 'PubkeyAuthentication yes' /etc/ssh/sshd_config; then
    echo 'PubkeyAuthentication yes' >> /etc/ssh/sshd_config
fi

if grep -q '#AllowUsers ubuntu' /etc/ssh/sshd_config; then
    sed -i 's/#AllowUsers ubuntu/AllowUsers ubuntu/' /etc/ssh/sshd_config
elif ! greq -q 'PubkeyAuthentication yes' /etc/ssh/sshd_config; then
    echo 'PubkeyAuthentication yes' >> /etc/ssh/sshd_config
fi

if grep -q '#PermitRootLogin without-password' /etc/ssh/sshd_config; then
    sed -i 's/#PermitRootLogin without-password/PermitRootLogin without-password/'
elif ! grep -q 'PermitRootLogin without-password' /etc/ssh/sshd_config; then
    echo 'PermitRootLogin without-password' >> /etc/ssh/sshd_config
fi

systemctl restart ssh
"#;

pub const NETWORK_ENABLE_STARTUP_SCRIPT: &'static str = r#"
ufw allow ssh
ufw enable
"#;
