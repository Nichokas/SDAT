mod api;
mod i18n;
mod model;
mod secrets;
mod service;
mod settings;

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
}
