use crate::{cluster::Config, garden, garden_page::instance_id, guild::Guild};
use leptos::prelude::window;
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsValue;
use web_sys::UrlSearchParams;

const KEY: &str = "permaculture_rust_garden_handoff";
pub(crate) const APPLIED: &str = "lastClusterHandoff";

#[derive(Deserialize, Serialize)]
pub struct Transfer {
    pub ticket: String,
    pub guild: Guild,
    pub config: Config,
}

pub fn auto_resolve() -> bool {
    let saved = window()
        .local_storage()
        .ok()
        .flatten()
        .and_then(|store| store.get_item(garden::STORAGE_KEY).ok().flatten());
    garden::Garden::saved(saved.as_deref()).auto_resolve_enabled
}

pub fn open(guild: Guild, config: Config, language: &str) -> Result<(), JsValue> {
    let window = window();
    let ticket = instance_id();
    let transfer = Transfer {
        ticket: ticket.clone(),
        guild,
        config,
    };
    let data =
        serde_json::to_string(&transfer).map_err(|error| JsValue::from_str(&error.to_string()))?;
    let store = window
        .session_storage()?
        .ok_or_else(|| JsValue::from_str("Missing session storage"))?;
    store.set_item(KEY, &data)?;
    let url = format!(
        "{}?lang={language}&planting={ticket}",
        crate::routes::garden()
    );
    if let Err(error) = window.location().set_href(&url) {
        let _ = store.remove_item(KEY);
        return Err(error);
    }
    Ok(())
}

pub fn read(params: &UrlSearchParams) -> Option<Transfer> {
    let ticket = params.get("planting")?;
    let store = window().session_storage().ok()??;
    let value = store.get_item(KEY).ok()??;
    let transfer: Transfer = serde_json::from_str(&value).ok()?;
    (ticket == transfer.ticket).then_some(transfer)
}

pub fn consume(params: &UrlSearchParams) {
    if crate::routes::clean_garden_query(params, "planting").is_ok() {
        if let Ok(Some(store)) = window().session_storage() {
            let _ = store.remove_item(KEY);
        }
    }
}
