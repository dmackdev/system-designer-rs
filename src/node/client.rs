use bevy::utils::HashMap;
use strum::{Display, EnumIter};

#[derive(Clone, Debug, Default)]
pub struct Client {
    pub request_configs: Vec<RequestConfig>,
}

impl Client {
    pub fn new() -> Self {
        Default::default()
    }
}

#[derive(Clone, Debug, Default)]
pub struct RequestConfig {
    pub url: String,
    pub path: String,
    pub method: HttpMethod,
    pub body: String,
    pub params: Vec<(String, String)>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, EnumIter, Display)]
pub enum HttpMethod {
    #[default]
    Get,
    Post,
}
