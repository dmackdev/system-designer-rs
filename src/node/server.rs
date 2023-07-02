#[derive(Clone, Debug, Default)]
pub struct Server {
    pub config: String,
}

impl Server {
    pub fn new() -> Self {
        Default::default()
    }
}
