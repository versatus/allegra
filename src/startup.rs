use std::process::{Output, Command};

use crate::account::Namespace;

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
