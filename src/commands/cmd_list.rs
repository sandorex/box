use crate::util::command_extensions::*;
use crate::{util::Engine, ExitResult};

pub fn print_containers(engine: Engine, dry_run: bool) -> ExitResult {
    let mut cmd = Command::new(&engine.path);
    cmd.args(["container", "ls", "--filter", format!("label={}", crate::BIN_NAME).as_str()]);

    if dry_run {
        cmd.print_escaped_cmd()
    } else {
        cmd.status()
            .expect(crate::ENGINE_ERR_MSG)
            .to_exitcode()
    }
}
