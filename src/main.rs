mod api;
mod i18n;
mod model;
mod secrets;
mod service;
mod settings;
mod signals;
mod player;
mod state;
mod router;
mod imgutil;
mod ui;

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
}
