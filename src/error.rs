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

    #[error("SwitchType conversion not allowed")]
    SwitchTypeNotSupported,

    #[error("Sql error {0}")]
    SqlError(#[from] sqlx::error::Error),

    #[error("Migrate error {0}")]
    MigrateError(#[from] sqlx::migrate::MigrateError),
}
