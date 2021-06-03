pub mod api;
pub mod broadcasting_software;
pub mod chat;
pub mod db;
pub mod error;
pub mod noalbs;
pub mod stream_servers;
pub mod switcher;

pub use error::Error;
pub use noalbs::Noalbs;
pub use switcher::Switcher;

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn print_logo() {
    println!(
        "
    ███╗   ██╗ ██████╗  █████╗ ██╗     ██████╗ ███████╗
    ████╗  ██║██╔═══██╗██╔══██╗██║     ██╔══██╗██╔════╝
    ██╔██╗ ██║██║   ██║███████║██║     ██████╔╝███████╗
    ██║╚██╗██║██║   ██║██╔══██║██║     ██╔══██╗╚════██║
    ██║ ╚████║╚██████╔╝██║  ██║███████╗██████╔╝███████║
    ╚═╝  ╚═══╝ ╚═════╝ ╚═╝  ╚═╝╚══════╝╚═════╝ ╚══════╝ v{}\n",
        VERSION
    );
}
