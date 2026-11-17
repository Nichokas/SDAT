//! TIDAL country/region for API requests, over `tonearm_core::country`.

use tonearm_core::{api, country};

#[derive(Debug, Clone, uniffi::Record)]
pub struct Country {
    /// ISO 3166-1 alpha-2, e.g. `ES`.
    pub code: String,
    /// English name.
    pub name: String,
}

impl From<&(&str, &str)> for Country {
    fn from((code, name): &(&str, &str)) -> Self {
        Self { code: (*code).into(), name: (*name).into() }
    }
}

/// Countries where TIDAL is available, sorted by code.
#[uniffi::export]
pub fn countries() -> Vec<Country> {
    country::COUNTRIES.iter().map(Country::from).collect()
}

/// The user's choice, `None` for automatic.
#[uniffi::export]
pub fn preferred_country() -> Option<String> {
    country::preferred()
}

/// Saves the choice (`None` = automatic); pages loaded afterwards use it.
#[uniffi::export]
pub fn set_preferred_country(code: Option<String>) {
    country::set_preferred(code.as_deref().and_then(country::parse).as_deref());
}

/// What automatic resolves to: the account's country, else the IP's; `None`
/// when neither is known or TIDAL isn't listed there.
#[uniffi::export]
pub async fn automatic_country() -> Option<Country> {
    let code = blocking::unblock(api::automatic_country_code).await;
    country::COUNTRIES.iter().find(|(c, _)| *c == code).map(Country::from)
}
