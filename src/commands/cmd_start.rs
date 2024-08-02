use crate::{VERSION, DATA_VOLUME_NAME};
use crate::util::{self, CommandOutputExt, Engine, EngineKind};
use crate::cli;
use std::process::{Command, ExitCode};

// Finds all terminfo directories on host so they can be mounted in the container so no terminfo
// installing is required
//
// This function is required as afaik only debian has non-standard paths for terminfo
//
fn find_terminfo(args: &mut Vec<String>) {
    let mut existing: Vec<String> = vec![];
    for x in vec!["/usr/share/terminfo", "/usr/lib/terminfo", "/etc/terminfo"] {
        if std::path::Path::new(x).exists() {
            args.extend(vec!["--volume".into(), format!("{0}:/host{0}:ro", x)]);
            existing.push(x.into());
        }
    }

    let mut terminfo_env = "".to_string();

    // add first the host ones as they are preferred
    for x in &existing {
        terminfo_env.push_str(format!("/host{}:", x).as_str());
    }

    // add container ones as fallback
    for x in &existing {
        terminfo_env.push_str(format!("{}:", x).as_str());
    }

    // remove leading ':'
    if terminfo_env.chars().last().unwrap_or(' ') == ':' {
        terminfo_env.pop();
    }

    // generate the env variable to find them all
    args.extend(vec!["--env".into(), format!("TERMINFO_DIRS={}", terminfo_env)]);
}

pub fn start_container(engine: Engine, dry_run: bool, cli_args: &cli::CmdStartArgs) -> ExitCode {
    let cwd = std::env::current_dir().expect("Failed to get current directory");
    let executable_path = std::env::current_exe().expect("Failed to get executable path");

    // generate a name if not provided already
    let container_name = match &cli_args.name {
        Some(x) => x.clone(),
        // generating a name is easier than reading the output and then running inspect again just
        // to get the human-friendly name of a container
        None => util::generate_name(),
    };

    // allow dry-run regardless if the container exists
    if !dry_run {
        // quit pre-emptively if container already exists
        if let Some(_) = util::get_container_status(&engine, &container_name) {
            eprintln!("Container {} already exists", &container_name);
            return ExitCode::FAILURE;
        }
    }

    // TODO set XDG_ env vars just in case
    // TODO add env var with engine used (but only basename in case its a full path)
    let mut args: Vec<String> = vec![
        "run".into(), "-d".into(), "--rm".into(),
        "--security-opt".into(), "label=disable".into(),
        "--name".into(), container_name.clone(),
        "--user".into(), "root".into(),
        "--label=manager=box".into(),
        "--label=box=box".into(),
        "--env".into(), "BOX=BOX".into(),
        "--env".into(), format!("BOX_VERSION={}", VERSION),
        "--env".into(), format!("BOX_USER={}", util::get_user()),
        "--volume".into(), format!("{}:/box:ro", executable_path.display()),
        "--volume".into(), format!("{}:/ws", &cwd.to_string_lossy()),
        "--hostname".into(), util::get_hostname(),
    ];

    match engine.kind {
        // TODO add docker equivalent
        EngineKind::Podman => {
            args.extend(vec![
                "--userns=keep-id".into(),

                // the default ulimit is low
                "--ulimit".into(), "host".into(),

                // TODO should i add --annotation run.oci.keep_original_groups=1
            ]);
        },
        _ => {},
    }

    // add the env vars, TODO should this be checked for syntax?
    for e in &cli_args.env {
        args.extend(vec!["--env".into(), e.into()]);
    }

    // add remove capabilities easily
    for c in &cli_args.capabilities {
        if c.starts_with("!") {
            args.extend(vec!["--cap-drop".into(), c[1..].to_string()])
        } else {
            args.extend(vec!["--cap-add".into(), c.to_string()])
        }
    }

    // find all terminfo dirs, they differ mostly on debian...
    find_terminfo(&mut args);

    // TODO change this to data_volume so its not confusing with the negation
    if ! cli_args.no_data_volume {
        let inspect_cmd = Command::new(&engine.path)
            .args(&["volume", "inspect", DATA_VOLUME_NAME])
            .output()
            .expect("Could not execute engine");

        // if it fails then volume is missing probably
        if ! inspect_cmd.status.success() {
            let create_vol_cmd = Command::new(&engine.path)
                .args(&["volume", "create", DATA_VOLUME_NAME])
                .output()
                .expect("Could not execute engine");

            // TODO maybe i should print stdout/stderr if it fails?
            if ! create_vol_cmd.status.success() {
                eprintln!("Failed to create data volume: {}", create_vol_cmd.status);
                return create_vol_cmd.to_exitcode();
            }
        }

        args.extend(vec![
            "--volume".into(), format!("{}:/data:Z", DATA_VOLUME_NAME),
        ]);
    }

    // disable network if requested
    // TODO make it network and negate in --network/--no-network
    if cli_args.no_network {
        args.push("--network=none".into());
    }

    // mount dotfiles if provided
    if let Some(dotfiles) = &cli_args.dotfiles {
        args.extend(vec!["--volume".into(), format!("{}:/etc/skel:ro", dotfiles.display())]);
    }

    // add the extra args verbatim
    args.extend(cli_args.engine_args.clone());

    args.extend(vec![
        // TODO add this as an option
        // "--env".into(), "RUST_BACKTRACE=1".into(),
        "--entrypoint".into(), "/box".into(),

        // the container image
        cli_args.image.clone(),

        "init".into(),
    ]);

    if dry_run {
        util::print_cmd_dry_run(&engine, args);

        ExitCode::SUCCESS
    } else {
        Command::new(&engine.path)
            .args(args)
            .status()
            .expect("Could not execute engine")
            .to_exitcode()
    }

    // TODO add interactive version where i can see output from the container, maybe podman logs -f
    // TODO print user friendly name
}

