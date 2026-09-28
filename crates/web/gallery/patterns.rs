//! Interaction patterns against in-memory demo state: partial updates, validated forms, bulk forms.

use std::sync::Mutex;

use topcoat::{
    Result,
    context::{Cx, app_context, try_request_context},
    router::{
        Body, Method, StatusCode,
        content::Form,
        error::{SeeOther, bad_request, rewrite, see_other},
        href, page, route,
    },
    runtime::{Event, procedure, shard, signal},
    view::{View, attributes, view},
};
use yokoku_web::components::ui::{alert::*, button::*, checkbox::*, field::*, input::*, label::*};

use crate::{story, story_page};

/// The demo's server state, shared by every request.
pub(crate) struct Demo {
    episodes: Mutex<Vec<Episode>>,
    address: Mutex<String>,
}

#[derive(Clone)]
struct Episode {
    id: u64,
    title: &'static str,
    monitored: bool,
}

impl Demo {
    pub(crate) fn new() -> Self {
        let episodes = ["Launch", "Drift", "Burn", "Coast", "Descent"]
            .into_iter()
            .zip(1..)
            .map(|(title, id)| Episode { id, title, monitored: id % 2 == 1 })
            .collect();
        Self { episodes: Mutex::new(episodes), address: Mutex::new("localhost:9091".to_owned()) }
    }

    fn episodes(&self) -> Vec<Episode> {
        self.episodes.lock().unwrap().clone()
    }

    /// Returns `false` when no episode has this id.
    fn set_monitored(&self, id: u64, monitored: bool) -> bool {
        let mut episodes = self.episodes.lock().unwrap();
        let found = episodes.iter_mut().find(|episode| episode.id == id);
        found.map(|episode| episode.monitored = monitored).is_some()
    }
}

/// A rejected address form: what was typed, and why it was refused.
#[derive(Clone)]
struct Rejected {
    address: String,
    error: String,
}

#[derive(serde::Deserialize)]
struct AddressInput {
    address: String,
}

/// Accepts `host:port`.
fn validate_address(address: &str) -> Result<String, String> {
    let address = address.trim();
    let (host, port) = address.rsplit_once(':').ok_or("Use host:port, such as localhost:9091.")?;
    if host.is_empty() {
        return Err("The host is missing.".to_owned());
    }
    port.parse::<u16>().map_err(|_| "The port must be a number from 0 to 65535.")?;
    Ok(address.to_owned())
}

#[page("/patterns/interactivity")]
pub(crate) async fn interactivity_story(cx: &Cx) -> Result<impl View> {
    let rejected = try_request_context::<Rejected>(cx);
    let saved = app_context::<Demo>(cx).address.lock().unwrap().clone();
    let address = rejected.map_or(saved.clone(), |rejected| rejected.address.clone());

    Ok(view! {
        if rejected.is_some() {
            (StatusCode::UNPROCESSABLE_ENTITY)
        }
        story_page(
            name: "Interactivity",
            path: "…",
            summary: "Full page loads between pages; partial updates inside one. Every example here changes server state.",
            story(title: "Procedure, then shard re-render", episode_toggles())
            story(
                title: "POST form, 422 re-render on invalid input",
                <form method="post" class="flex flex-col gap-3">
                    field(
                        attrs: attributes! { data-invalid=(rejected.is_some().then_some("true")) },
                        field_label(
                            attrs: attributes! { for="address" },
                            "Transmission address"
                        )
                        input(
                            attrs: attributes! {
                                id="address"
                                name="address"
                                value=(address)
                                aria-invalid=(rejected.is_some().then_some("true"))
                                aria-describedby=(rejected
                                    .is_some()
                                    .then_some("address-error"))
                            }
                        )
                        if let Some(rejected) = rejected {
                            field_error(
                                attrs: attributes! { id="address-error" },
                                (rejected.error.clone())
                            )
                        }
                    )
                    <p class="text-caption text-muted">
                        "Saved: "
                        <span class="yk-code">(saved)</span>
                    </p>
                    <div>
                        button(
                            variant: ButtonVariant::Primary,
                            attrs: attributes! { type="submit" },
                            "Save"
                        )
                    </div>
                </form>
            )
            story(
                title: "Bulk form read as pairs, then redirect",
                <form
                    id="bulk"
                    method="post"
                    action=(href!(unmonitor_selected).resolve(cx))
                    class="flex flex-col gap-3"
                >
                    for episode in app_context::<Demo>(cx).episodes() {
                        let id = format!("bulk-{}", episode.id);
                        <div class="flex items-center gap-2">
                            checkbox(
                                attrs: attributes! { id=(id.clone()) name="episode" value=(episode.id) }
                            )
                            label(
                                attrs: attributes! { for=(id) },
                                (episode.title)
                                if episode.monitored {
                                    " · monitored"
                                }
                            )
                        </div>
                    }
                    <div>
                        button(
                            attrs: attributes! { type="submit" },
                            "Unmonitor selected"
                        )
                    </div>
                </form>
            )
        )
    })
}

/// Saves the address, or re-renders the page with the error.
#[route(POST "/patterns/interactivity")]
async fn save_address(cx: &Cx, Form(submitted): Form<AddressInput>) -> Result<SeeOther> {
    match validate_address(&submitted.address) {
        Ok(address) => {
            *app_context::<Demo>(cx).address.lock().unwrap() = address;
            Ok(see_other(href!(interactivity_story).resolve(cx)))
        },
        Err(error) => Err(rewrite(href!(interactivity_story).resolve(cx), Body::empty())
            .method(Method::GET)
            .with(Rejected { address: submitted.address, error })
            .into()),
    }
}

/// Unmonitors every checked `episode`.
#[route(POST "/patterns/interactivity/unmonitor")]
async fn unmonitor_selected(cx: &Cx, Form(fields): Form<Vec<(String, String)>>) -> Result<SeeOther> {
    for (_, value) in fields.iter().filter(|(key, _)| key == "episode") {
        let id = value.parse::<u64>().map_err(|_| bad_request("invalid episode"))?;
        app_context::<Demo>(cx).set_monitored(id, false);
    }
    Ok(see_other(href!(interactivity_story).fragment("bulk").resolve(cx)))
}

#[procedure]
async fn set_monitored(cx: &Cx, episode: u64, monitored: bool) -> Result<Result<bool, String>> {
    Ok(app_context::<Demo>(cx)
        .set_monitored(episode, monitored)
        .then_some(monitored)
        .ok_or_else(|| format!("Episode {episode} no longer exists.")))
}

/// Episode rows whose toggles save through [`set_monitored`], then re-render this shard.
#[shard]
async fn episode_toggles(cx: &Cx) -> Result<impl View> {
    let version = signal(cx, || 0usize);
    let _ = version.get();
    let failure = signal(cx, String::new);
    let episodes = app_context::<Demo>(cx).episodes();
    let monitored = episodes.iter().filter(|episode| episode.monitored).count();
    let gone = Episode { id: 99, title: "Deleted elsewhere", monitored: true };

    Ok(view! {
        <div class="flex flex-col gap-3">
            <p class="text-caption text-muted" aria-live="polite">
                (monitored)
                " of "
                (episodes.len())
                " monitored, rendered on the server"
            </p>
            #[key(episode.id)]
            for episode in episodes.into_iter().chain([gone]) {
                let id = episode.id;
                let toggle = format!("toggle-{id}");
                <div class="flex items-center gap-2">
                    checkbox(
                        attrs: attributes! {
                            id=(toggle.clone())
                            checked=(episode.monitored)
                            @change=$(async |e: Event| {
                                let saved = set_monitored(id, e.target.checked).await;
                                if saved.is_ok() {
                                    failure.set("".to_owned())
                                } else {
                                    failure.set(saved.unwrap_err())
                                }
                                version.increment()
                            })
                        }
                    )
                    label(attrs: attributes! { for=(toggle) }, (episode.title))
                </div>
            }
            <div :hidden=$(failure.get().is_empty())>
                alert(variant: AlertVariant::Danger, alert_title($(failure.get())))
            </div>
        </div>
    })
}
