use std::process::{Output, Command};

use crate::account::Namespace;

pub fn run_script(
    namespace: Namespace,
    script: &'static str
) -> std::io::Result<Output> {
    todo!()
}
