#[derive(Clone, Debug, Default)]
pub struct Client {
    pub config: String,
}

impl Client {
    pub fn new() -> Self {
        Default::default()
    }
}
