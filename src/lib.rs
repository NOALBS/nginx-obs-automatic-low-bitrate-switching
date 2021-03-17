pub mod broadcasting_software;
pub mod chat;
pub mod error;
pub mod stream_servers;
pub mod switcher;

pub use error::Error;
pub use switcher::Switcher;

// #[async_trait]
// pub trait SomethingAllTheServersNeed {
//     async fn get_stats(&self) -> Result<Option<NginxRtmpStream>, error::Error>;
//     fn get_bitrate() {}
// }

// pub trait SomethingAllTheSrtServersNeed {
//     fn get_rtt();
// }
//
// pub trait ChatCommands {
//     fn get_command_by_string(command: &str);
// }

// pub struct Config {}
//
// impl Config {
//     fn new() {
//         todo!();
//     }
//
//     fn load() {
//         todo!();
//     }
//
//     fn config_directory_exists() {
//         todo!();
//     }
// }

pub fn print_logo() {
    println!(
        "
    ███╗   ██╗ ██████╗  █████╗ ██╗     ██████╗ ███████╗
    ████╗  ██║██╔═══██╗██╔══██╗██║     ██╔══██╗██╔════╝
    ██╔██╗ ██║██║   ██║███████║██║     ██████╔╝███████╗
    ██║╚██╗██║██║   ██║██╔══██║██║     ██╔══██╗╚════██║
    ██║ ╚████║╚██████╔╝██║  ██║███████╗██████╔╝███████║
    ╚═╝  ╚═══╝ ╚═════╝ ╚═╝  ╚═╝╚══════╝╚═════╝ ╚══════╝ v2.0.0"
    );
}
