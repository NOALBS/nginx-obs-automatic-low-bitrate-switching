#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Can't access stats page")]
    StatsPageNotAvailable,

    #[error("Reqwest error {0}")]
    PageRequest(#[from] reqwest::Error),

    #[error("XML parsing error {0}")]
    XmlParsing(#[from] quick_xml::DeError),

    #[error("OBS error {0}")]
    ObsError(#[from] obws::Error),

    #[error("OBS error {0}")]
    ObsV5Error(#[from] obwsv5::Error),

    #[error("SwitchType conversion not allowed")]
    SwitchTypeNotSupported,

    // #[error("Sql error {0}")]
    // SqlError(#[from] sqlx::error::Error),

    // #[error("Migrate error {0}")]
    // MigrateError(#[from] sqlx::migrate::MigrateError),
    #[error("Unable to connect to OBS")]
    UnableInitialConnection,

    #[error("No software set for user")]
    NoSoftwareSet,

    #[error("Unable to convert enabled to bool")]
    EnabledToBoolConversionError,

    #[error("IO Error")]
    IoError(#[from] std::io::Error),

    #[error("Json error: {0}")]
    Json(#[from] serde_json::error::Error),

    #[error("No chat configured")]
    NoChat,

    #[error("Language not supported")]
    LangNotSupported,

    #[error("Config file error")]
    ConfigFileError(#[source] std::io::Error),

    #[error("No server info available")]
    NoServerInfo,
}
