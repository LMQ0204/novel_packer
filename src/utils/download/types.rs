use crate::utils::config::DynamicConfig;


pub struct Task {
    url: url::Url,
    options: Option<DynamicConfig>
}

pub struct Process {
    tasks: Vec<Task>,
    options: Option<DynamicConfig>
}
