use anyhow::{Context, Result, bail};
use std::{env, net::IpAddr, path::PathBuf};
use synara_server::{
    DEFAULT_BIND, DEFAULT_PORT, RunningServer, ServerConfig, default_database_path,
    load_bearer_token,
};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(false)
        .try_init()
        .ok();

    let options = parse_args()?;
    if options.help {
        print_usage();
        return Ok(());
    }
    let token = load_bearer_token(options.token_file.as_deref())?;
    let private_database_directory =
        options.database_path.is_none() && env::var_os("SYNARA_SERVER_DB").is_none();
    if !private_database_directory {
        tracing::warn!(
            "an explicit headless database path was selected; startup fails while another process owns it rather than waiting. Once that process closes, startup recovery can update interrupted task states. Set restrictive permissions on the parent directory"
        );
    }
    let database_path = match options.database_path {
        Some(path) => path,
        None => default_database_path()?,
    };
    let config = ServerConfig::new(options.bind, options.port, database_path, token);
    let config = if private_database_directory {
        config.with_private_database_directory()
    } else {
        config
    };
    let config = if options.automations {
        config.with_automations()
    } else {
        config
    };
    let mut server = RunningServer::start(config).await?;

    tracing::info!(
        address = %server.address(),
        automations = options.automations,
        "Synara native headless server listening on loopback"
    );
    let ready = tokio::select! {
        result = server.wait_ready() => Some(result),
        signal = tokio::signal::ctrl_c() => {
            signal.context("could not listen for shutdown signal")?;
            None
        }
    };
    match ready {
        Some(Ok(())) => {}
        Some(Err(error)) => {
            server
                .shutdown()
                .await
                .context("shutdown after startup failure failed")?;
            return Err(error);
        }
        None => {
            server.shutdown().await?;
            return Ok(());
        }
    }
    tracing::info!("Synara workspace recovery completed; read-only task browser is ready");
    tokio::signal::ctrl_c()
        .await
        .context("could not listen for shutdown signal")?;
    server.shutdown().await
}

struct Options {
    bind: IpAddr,
    port: u16,
    database_path: Option<PathBuf>,
    token_file: Option<PathBuf>,
    automations: bool,
    help: bool,
}

fn parse_args() -> Result<Options> {
    parse_options(env::args().skip(1))
}

fn parse_options<I, S>(args: I) -> Result<Options>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut options = Options {
        bind: DEFAULT_BIND,
        port: DEFAULT_PORT,
        database_path: None,
        token_file: None,
        automations: false,
        help: false,
    };
    let mut args = args.into_iter().map(Into::into);
    while let Some(arg) = args.next() {
        if arg == "--help" || arg == "-h" {
            options.help = true;
            continue;
        }
        if arg == "--automations" {
            options.automations = true;
            continue;
        }
        let (flag, inline_value) = arg
            .split_once('=')
            .map(|(flag, value)| (flag.to_owned(), Some(value.to_owned())))
            .unwrap_or_else(|| (arg, None));
        if !matches!(flag.as_str(), "--bind" | "--port" | "--db" | "--token-file") {
            bail!("unknown argument: {flag}");
        }
        let value = match inline_value {
            Some(value) => value,
            None => args
                .next()
                .with_context(|| format!("missing value for {flag}"))?,
        };
        match flag.as_str() {
            "--bind" => options.bind = parse_bind(&value)?,
            "--port" => options.port = parse_port(&value)?,
            "--db" => options.database_path = Some(PathBuf::from(value)),
            "--token-file" => options.token_file = Some(PathBuf::from(value)),
            _ => bail!("unknown argument: {flag}"),
        }
    }
    Ok(options)
}

fn parse_bind(value: &str) -> Result<IpAddr> {
    let value = value
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .unwrap_or(value);
    value
        .parse()
        .with_context(|| format!("invalid IP bind address: {value}"))
}

fn parse_port(value: &str) -> Result<u16> {
    value
        .parse()
        .with_context(|| format!("invalid port number: {value}"))
}

fn print_usage() {
    println!(
        "Synara native headless workspace server\n\
         Usage: synara-server [--bind 127.0.0.1] [--port 17341] [--db PATH] [--token-file PATH] [--automations]\n\
         The server only binds loopback addresses. Set SYNARA_SERVER_TOKEN or provide a\n\
         private token file with 32 or more printable ASCII characters. The token is never\n\
         printed or logged. A token file must not be accessible by group or other users.\n\
         The default database is a separate headless-server.sqlite3 file in a private\n\
         directory under the user data directory. An explicit --db path must not be open\n\
         in the desktop app; startup fails while another process owns it. Its parent\n\
         directory permissions are your responsibility.\n\
         SYNARA_SERVER_DB can set the database path when --db is omitted.\n\
         --automations explicitly arms enabled saved automations in this server process.\n\
         Without it, saved schedules remain disarmed. The workspace owner lock prevents\n\
         desktop and headless processes from scheduling the same database concurrently.\n\
         Open http://127.0.0.1:17341/ for the task browser."
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_loopback_ip_and_options() {
        let options = parse_options([
            "--bind=::1",
            "--port",
            "17342",
            "--db",
            "./workspace.sqlite3",
            "--token-file=/tmp/synara.token",
            "--automations",
        ])
        .unwrap();
        assert_eq!(options.bind, "::1".parse::<IpAddr>().unwrap());
        assert_eq!(options.port, 17342);
        assert_eq!(
            options.database_path,
            Some(PathBuf::from("./workspace.sqlite3"))
        );
        assert_eq!(options.token_file, Some(PathBuf::from("/tmp/synara.token")));
        assert!(options.automations);
    }
}
