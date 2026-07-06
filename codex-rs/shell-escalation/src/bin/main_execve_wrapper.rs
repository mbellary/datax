#[cfg(not(unix))]
fn main() {
    eprintln!("datax-execve-wrapper is only implemented for UNIX");
    std::process::exit(1);
}

#[cfg(unix)]
pub use datax_shell_escalation::main_execve_wrapper as main;
