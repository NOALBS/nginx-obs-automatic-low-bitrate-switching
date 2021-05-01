use crate::{error, stream_servers::SwitchType};

pub mod obs;

#[derive(Debug, sqlx::FromRow)]
pub struct SwitchingScenes {
    pub normal: String,
    pub low: String,
    pub offline: String,
}

impl SwitchingScenes {
    pub fn new<N, L, O>(normal: N, low: L, offline: O) -> Self
    where
        N: Into<String>,
        L: Into<String>,
        O: Into<String>,
    {
        SwitchingScenes {
            normal: normal.into(),
            low: low.into(),
            offline: offline.into(),
        }
    }

    fn type_to_scene(&self, s_type: &SwitchType) -> Result<String, error::Error> {
        let str = match s_type {
            SwitchType::Normal => &self.normal,
            SwitchType::Low => &self.low,
            SwitchType::Offline => &self.offline,
            _ => return Err(error::Error::SwitchTypeNotSupported),
        };

        Ok(str.to_string())
    }
}
