use clap::Parser;
use datax_arg0::Arg0DispatchPaths;
use datax_arg0::arg0_dispatch_or_else;

#[derive(Debug, Parser)]
#[command(version)]
struct ExecServerArgs {
    /// Transport endpoint URL. Supported values: `ws://IP:PORT` (default), `stdio`, `stdio://`.
    #[arg(long = "listen", value_name = "URL")]
    listen: Option<String>,
}

fn main() -> anyhow::Result<()> {
    arg0_dispatch_or_else(|arg0_paths: Arg0DispatchPaths| async move {
        let args = ExecServerArgs::parse();
        let runtime_paths = datax_exec_server::ExecServerRuntimePaths::from_optional_paths(
            arg0_paths.codex_self_exe,
            arg0_paths.codex_linux_sandbox_exe,
        )?;
        let listen_url = args
            .listen
            .as_deref()
            .unwrap_or(datax_exec_server::DEFAULT_LISTEN_URL);
        datax_exec_server::run_main(listen_url, runtime_paths)
            .await
            .map_err(anyhow::Error::from_boxed)
    })
}
