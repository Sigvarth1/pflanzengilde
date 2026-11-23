use wasm_bindgen::{JsCast, JsValue};

pub fn text(text: &str, mime: &str, name: &str) -> Result<(), JsValue> {
    let chunks = js_sys::Array::of1(&JsValue::from_str(text));
    let options = web_sys::BlobPropertyBag::new();
    options.set_type(mime);
    let blob = web_sys::Blob::new_with_str_sequence_and_options(&chunks, &options)?;
    save(&blob, name)
}

pub fn bytes(data: &[u8], mime: &str, name: &str) -> Result<(), JsValue> {
    let data = js_sys::Uint8Array::from(data);
    let chunks = js_sys::Array::of1(&data);
    let options = web_sys::BlobPropertyBag::new();
    options.set_type(mime);
    let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(&chunks, &options)?;
    save(&blob, name)
}

fn save(blob: &web_sys::Blob, name: &str) -> Result<(), JsValue> {
    let document = leptos::prelude::window()
        .document()
        .ok_or_else(|| JsValue::from_str("Missing document"))?;
    let link = document
        .create_element("a")?
        .dyn_into::<web_sys::HtmlAnchorElement>()?;
    let url = web_sys::Url::create_object_url_with_blob(blob)?;
    link.set_href(&url);
    link.set_download(name);
    link.click();
    web_sys::Url::revoke_object_url(&url)
}
