pub mod broadcasting_software;
pub mod chat;
pub mod config;
pub mod error;
pub mod noalbs;
pub mod state;
pub mod stream_servers;
pub mod switcher;
pub mod user_manager;
pub mod web_server;
pub mod ws;

pub use crate::noalbs::ChatSender;
pub use crate::noalbs::Noalbs;

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
