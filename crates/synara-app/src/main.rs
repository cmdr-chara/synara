mod close;
mod input;
mod shell;
mod ui;
use anyhow::{Context as _, Result, bail};
use gpui::{App, Bounds, WindowBounds, WindowOptions, prelude::*, px, size};
use std::{path::PathBuf, sync::Arc};
use synara_agent::InteractionBroker;
use synara_workspace::{Controller, Selection, WorkspaceService, parse_profiles};

struct Options {
    data: PathBuf,
    workspace: Option<PathBuf>,
    agents: Option<PathBuf>,
}
fn options() -> Result<Option<Options>> {
    let mut args = std::env::args_os().skip(1);
    let mut data = None;
    let mut workspace = None;
    let mut agents = None;
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--help" | "-h") => {
                println!(
                    "Synara\n\nUsage: synara-app [--workspace DIRECTORY] [--agents PROFILES.json] [--data-dir DIRECTORY]\n\nAgent profiles use id, name, command, args and optional inherit_env. Secrets remain in the launch environment."
                );
                return Ok(None);
            }
            Some("--data-dir") => {
                data = Some(PathBuf::from(
                    args.next().context("--data-dir requires a directory")?,
                ))
            }
            Some("--workspace") => {
                workspace = Some(PathBuf::from(
                    args.next().context("--workspace requires a directory")?,
                ))
            }
            Some("--agents") => {
                agents = Some(PathBuf::from(
                    args.next().context("--agents requires a JSON file")?,
                ))
            }
            _ => bail!("unrecognized argument: {}", arg.to_string_lossy()),
        }
    }
    let data = data.map_or_else(default_data_dir, Ok)?;
    if !data.is_absolute() {
        bail!("the data directory must be an absolute path");
    }
    Ok(Some(Options {
        data,
        workspace,
        agents,
    }))
}
fn default_data_dir() -> Result<PathBuf> {
    #[cfg(windows)]
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .context("LOCALAPPDATA is unavailable. Use --data-dir.")?;
    #[cfg(target_os = "macos")]
    let base =
        PathBuf::from(std::env::var_os("HOME").context("HOME is unavailable. Use --data-dir.")?)
            .join("Library/Application Support");
    #[cfg(all(not(windows), not(target_os = "macos")))]
    let base = match std::env::var_os("XDG_DATA_HOME") {
        Some(path) => PathBuf::from(path),
        None => {
            PathBuf::from(std::env::var_os("HOME").context("HOME is unavailable. Use --data-dir.")?)
                .join(".local/share")
        }
    };
    Ok(base.join("synara/native"))
}
fn run() -> Result<()> {
    let Some(options) = options()? else {
        return Ok(());
    };
    std::fs::create_dir_all(&options.data).context("could not create Synara's data directory")?;
    if std::fs::symlink_metadata(&options.data)?.is_symlink() {
        bail!("the data directory must not be a symlink");
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&options.data, std::fs::Permissions::from_mode(0o700))?;
    }
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    let workspace = runtime.block_on(WorkspaceService::open(
        options.data.join("native-workspace.sqlite3"),
    ))?;
    let bootstrap = runtime.block_on(async {
        workspace.recover_interrupted().await?;
        if let Some(path) = options.agents {
            if std::fs::metadata(&path)
                .map_err(synara_runtime::RuntimeError::Io)?
                .len()
                > 1024 * 1024
            {
                return Err(synara_workspace::WorkspaceError::Invalid(
                    "agent profiles file exceeds 1 MiB".into(),
                ));
            }
            let text = std::fs::read_to_string(path).map_err(synara_runtime::RuntimeError::Io)?;
            workspace.save_profiles(parse_profiles(&text)?).await?;
        }
        if let Some(root) = options.workspace {
            let project = workspace.add_local_workspace(root).await?;
            let catalog = workspace.catalog().await?;
            let task = if let Some(task) = catalog.tasks.iter().find(|t| t.project_id == project.id)
            {
                task.clone()
            } else {
                let profiles = workspace.profiles().await?;
                workspace
                    .create_task(project.id, "New task".into(), profiles[0].id.clone())
                    .await?
            };
            workspace
                .save_selection(Selection {
                    project: Some(project.id),
                    task: Some(task.id),
                })
                .await?;
        }
        Ok::<_, synara_workspace::WorkspaceError>(shell::Bootstrap {
            environment: workspace.environment_layout().await?,
            scratch_directory: options
                .data
                .canonicalize()
                .map_err(synara_runtime::RuntimeError::Io)?
                .join("chats"),
            settings: workspace.settings().await?.settings,
            agent_directory: options.data.join("agents"),
            catalog: workspace.catalog().await?,
            profiles: workspace.profiles().await?,
            selection: workspace.selection().await?,
        })
    })?;
    let (broker, interactions) = InteractionBroker::new();
    let controller = Arc::new(Controller::new(
        workspace,
        Arc::new(synara_acp::AcpBackend::default()),
        Arc::new(broker),
    ));
    let app_controller = controller.clone();
    let handle = runtime.handle().clone();
    gpui_platform::application()
        .with_assets(ui::Assets)
        .run(move |cx: &mut App| {
            if let Err(error) =
                cx.text_system()
                    .add_fonts(vec![std::borrow::Cow::Borrowed(include_bytes!(
                        "../assets/fonts/CalSans-Regular.ttf"
                    ))])
            {
                tracing::warn!(%error, "Could not load the bundled Synara wordmark font");
            }
            cx.set_reduce_motion(bootstrap.settings.appearance.reduced_motion);
            let bounds = Bounds::centered(None, size(px(1420.), px(930.)), cx);
            let result = cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    window_decorations: Some(gpui::WindowDecorations::Client),
                    app_owns_titlebar_drag: true,
                    ..Default::default()
                },
                move |window, cx| {
                    window.set_window_title("Synara");
                    let shell = cx.new(|cx| {
                        shell::Shell::new(app_controller, handle, bootstrap, interactions, cx)
                    });
                    let weak = shell.downgrade();
                    window.on_window_should_close(cx, move |window, cx| {
                        weak.update(cx, |shell, cx| shell.request_close(window, cx))
                            .unwrap_or(false)
                    });
                    shell
                },
            );
            if let Err(error) = result {
                tracing::error!(%error,"Could not open the native window");
                cx.quit();
                return;
            }
            cx.activate(true);
        });
    runtime.block_on(controller.shutdown())?;
    runtime.shutdown_timeout(std::time::Duration::from_secs(5));
    Ok(())
}
fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "synara=info".into()),
        )
        .init();
    if let Err(error) = run() {
        eprintln!("Synara could not start: {error:#}");
        std::process::exit(1);
    }
}
