use std::process::{Output, Command};

use crate::account::Namespace;

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

pub fn run_script(
    namespace: Namespace,
    script: &'static str
) -> std::io::Result<Output> {
    Command::new("lxc")
        .arg("exec")
        .arg(namespace.inner())
        .arg("--")
        .arg("bash")
        .arg("-c")
        .arg(script.replace("\"", "\\\""))
        .output()
}
