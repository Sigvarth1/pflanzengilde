use crate::catalog::Catalog;
use js_sys::{encode_uri_component, Function, Promise, Reflect};
use leptos::{html, prelude::*};
use std::sync::Arc;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::{spawn_local, JsFuture};

#[derive(Clone, Default)]
pub struct Payload {
    pub url: String,
    pub embed: String,
    pub title: String,
    pub text: String,
}

impl Payload {
    fn iframe(&self) -> String {
        let url = self
            .embed
            .replace('&', "&amp;")
            .replace('"', "&quot;")
            .replace('<', "&lt;");
        format!(
            r#"<iframe src="{url}" width="640" height="360" style="border:0" title="Pflanzengilde.de" loading="lazy"></iframe>"#
        )
    }

    fn share_links(&self) -> [(&'static str, String); 4] {
        let url = encode(&self.url);
        let text = encode(&self.text);
        let combined = encode(&format!("{} {}", self.text, self.url));
        let subject = encode(&self.title);
        let body = encode(&format!("{}\r\n\r\n{}", self.text, self.url));
        [
            (
                "WhatsApp",
                format!("https://api.whatsapp.com/send?text={combined}"),
            ),
            (
                "Telegram",
                format!("https://t.me/share/url?url={url}&text={text}"),
            ),
            (
                "X",
                format!("https://twitter.com/intent/tweet?url={url}&text={text}"),
            ),
            ("Email", format!("mailto:?subject={subject}&body={body}")),
        ]
    }
}

fn encode(value: &str) -> String {
    String::from(encode_uri_component(value))
}

fn property(object: &JsValue, name: &str) -> Option<JsValue> {
    Reflect::get(object, &JsValue::from_str(name))
        .ok()
        .filter(|value| !value.is_null() && !value.is_undefined())
}

fn method(object: &JsValue, name: &str) -> Option<Function> {
    property(object, name)?.dyn_into().ok()
}

fn copy_request(value: &str) -> Option<Promise> {
    let clipboard = property(window().navigator().as_ref(), "clipboard")?;
    method(&clipboard, "writeText")?
        .call1(&clipboard, &JsValue::from_str(value))
        .ok()?
        .dyn_into()
        .ok()
}

#[component]
pub fn ShareDialog(
    catalog: Arc<Catalog>,
    language: &'static str,
    #[prop(default = false)] garden: bool,
    make_payload: Callback<(), Payload>,
    preview: Callback<(), AnyView>,
) -> impl IntoView {
    let dialog = NodeRef::<html::Dialog>::new();
    let link_field = NodeRef::<html::Input>::new();
    let embed_field = NodeRef::<html::Textarea>::new();
    let visible = RwSignal::new(false);
    let show_preview = RwSignal::new(false);
    let embed_tab = RwSignal::new(false);
    let status = RwSignal::new(String::new());
    let revision = RwSignal::new(0_u32);
    let payload = RwSignal::new(Payload::default());
    let title = catalog.text(if garden {
        "shareGardenTitle"
    } else {
        "shareGuildTitle"
    });
    let link_label = title.clone();
    let summary = catalog.text(if garden {
        "shareGardenSubtitle"
    } else {
        "shareGuildDesc"
    });
    let instructions = catalog.text(if garden {
        "shareGardenEmbedHint"
    } else {
        "shareGuildEmbedInstructions"
    });
    let copy_code = match language {
        "en" => "Copy embed code",
        "de" => "Einbettungscode kopieren",
        _ => "Kopier innbyggingskode",
    };
    let copy_fallback = match language {
        "en" => "Automatic copying is unavailable. Copy the selected text.",
        "de" => "Automatisches Kopieren ist nicht verfügbar. Kopiere den markierten Text.",
        _ => "Automatisk kopiering er ikke tilgjengelig. Kopier den markerte teksten.",
    };
    let share_failure = match language {
        "de" => "Teilen fehlgeschlagen. Du kannst den Link kopieren.",
        "en" => "Could not share. You can copy the link.",
        _ => "Deling mislyktes. Du kan kopiere lenken.",
    };
    let mail = match language {
        "de" => "E-Mail",
        "en" => "Email",
        _ => "E-post",
    };
    let native_available = method(window().navigator().as_ref(), "share").is_some();

    let prepare = move |_| {
        if let Some(node) = dialog.get_untracked() {
            payload.set(make_payload.run(()));
            embed_tab.set(false);
            status.set(String::new());
            revision.update(|n| *n = n.wrapping_add(1));
            visible.set(node.show_modal().is_ok());
        }
    };
    let copy_catalog = catalog.clone();
    let copy = Callback::new(move |embed: bool| {
        let response = payload.with_untracked(|data| {
            if embed {
                copy_request(&data.iframe())
            } else {
                copy_request(&data.url)
            }
        });
        let key = if embed {
            "shareEmbedCodeCopied"
        } else {
            "shareLinkCopied"
        };
        let copied = copy_catalog.text(key);
        let started = revision.get_untracked();
        spawn_local(async move {
            let success = if let Some(promise) = response {
                JsFuture::from(promise).await.is_ok()
            } else {
                false
            };
            if revision.get_untracked() != started || !visible.get_untracked() {
                return;
            }
            if success {
                status.set(copied);
            } else {
                if embed {
                    if let Some(field) = embed_field.get_untracked() {
                        let _ = field.focus();
                        field.select();
                    }
                } else if let Some(field) = link_field.get_untracked() {
                    let _ = field.focus();
                    field.select();
                }
                status.set(copy_fallback.into());
            }
        });
    });
    let native = move |_| {
        let navigator = window().navigator();
        let Some(share) = method(navigator.as_ref(), "share") else {
            return;
        };
        let data = web_sys::ShareData::new();
        payload.with_untracked(|payload| {
            data.set_title(&payload.title);
            data.set_text(&payload.text);
            data.set_url(&payload.url);
        });
        let result = share
            .call1(navigator.as_ref(), data.as_ref())
            .and_then(|value| value.dyn_into::<Promise>());
        if let Ok(promise) = result {
            let started = revision.get_untracked();
            spawn_local(async move {
                if let Err(error) = JsFuture::from(promise).await {
                    let cancelled = property(&error, "name")
                        .and_then(|name| name.as_string())
                        .as_deref()
                        == Some("AbortError");
                    if !cancelled && revision.get_untracked() == started && visible.get_untracked()
                    {
                        status.set(share_failure.into());
                    }
                }
            });
        } else {
            status.set(share_failure.into());
        }
    };
    let link_title = if language == "nb" { "Lenke" } else { "Link" };
    let embed_title = catalog.text("shareEmbedCode");
    let select_tab = Callback::new(move |embed| {
        embed_tab.set(embed);
        revision.update(|n| *n = n.wrapping_add(1));
        status.set(String::new());
    });

    view! {
        <button type="button" class="share-trigger" aria-haspopup="dialog" on:click=prepare>{catalog.text("shareGuildBtn")}</button>
        <dialog class="climate-dialog share-dialog" node_ref=dialog aria-labelledby="share-title"
            on:close=move |_| { visible.set(false); revision.update(|n| *n = n.wrapping_add(1)); }
            on:click=move |ev| {
                if let Some(node) = dialog.get_untracked() {
                    if ev.target().is_some_and(|target| web_sys::EventTarget::from(node.clone()) == target) { node.close(); }
                }
            }>
            <div class="climate-dialog-inner">
                <header class="climate-dialog-header">
                    <h2 id="share-title">{title}</h2>
                    <form method="dialog"><button type="submit" aria-label=catalog.text("closeBtn") autofocus>"×"</button></form>
                </header>
                <p>{summary}</p>
                <details class="share-preview">
                    <summary on:click=move |_| show_preview.set(true)>{catalog.text("shareEmbedPreview")}</summary>
                    {move || (visible.get() && show_preview.get()).then(|| preview.run(()))}
                </details>
                <div class="share-tabs">
                    <button type="button" aria-pressed=move || (!embed_tab.get()).to_string()
                        on:click=move |_| select_tab.run(false)>{link_title}</button>
                    <button type="button" aria-pressed=move || embed_tab.get().to_string()
                        on:click=move |_| select_tab.run(true)>{embed_title.clone()}</button>
                </div>
                <div hidden=embed_tab>
                    <label>{link_label}
                        <input type="text" readonly node_ref=link_field prop:value=move || payload.with(|data| data.url.clone())
                            on:focus=move |_| { if let Some(field) = link_field.get_untracked() { field.select(); } } />
                    </label>
                    <button type="button" on:click=move |_| copy.run(false)>{catalog.text("shareCopyLink")}</button>
                    <div class="share-links" role="group" aria-label=catalog.text("shareGuildShareDirectly")>
                        {move || visible.get().then(|| payload.with(|data| data.share_links().into_iter().map(|(service, href)| {
                            let is_mail = service == "Email";
                            view! {
                                <a href=href target=(!is_mail).then_some("_blank") rel=(!is_mail).then_some("noopener noreferrer")>
                                    {if is_mail { mail } else { service }}
                                </a>
                            }
                        }).collect_view()))}
                        {native_available.then(|| view! {
                            <button type="button" on:click=native>{catalog.text("shareGuildShareViaDevice")}</button>
                        })}
                    </div>
                </div>
                <div hidden=move || !embed_tab.get()>
                    <p>{instructions}</p>
                    <label>{catalog.text("shareEmbedCode")}
                        <textarea readonly rows="4" node_ref=embed_field prop:value=move || payload.with(Payload::iframe)
                            on:focus=move |_| { if let Some(field) = embed_field.get_untracked() { field.select(); } } />
                    </label>
                    <button type="button" on:click=move |_| copy.run(true)>{copy_code}</button>
                </div>
                <p class="share-status" role="status" hidden=move || status.with(String::is_empty)>{status}</p>
            </div>
        </dialog>
    }
}
