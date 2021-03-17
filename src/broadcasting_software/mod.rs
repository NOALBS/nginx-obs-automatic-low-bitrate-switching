pub mod obs;

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
}
