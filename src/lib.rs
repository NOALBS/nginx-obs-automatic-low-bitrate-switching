pub mod broadcasting_software;
pub mod chat;
pub mod config;
pub mod error;
pub mod events;
pub mod noalbs;
pub mod state;
pub mod stream_servers;
pub mod switcher;
pub mod twitch_pubsub;
pub mod user_manager;
pub mod web_server;
pub mod ws;

pub use crate::noalbs::ChatSender;
pub use crate::noalbs::Noalbs;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
rust_i18n::i18n!("locales");

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
