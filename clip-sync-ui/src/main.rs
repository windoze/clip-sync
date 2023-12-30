use std::time::Duration;
use dioxus::{
    html::GlobalAttributes,
    prelude::*,
};
use log::LevelFilter;
use serde::{Deserialize, Serialize};
use gloo_timers::future::sleep;

pub static BASE_API_URL: &str = "/api";

pub fn base_url() -> String {
    web_sys::window().unwrap().location().origin().unwrap()
}

#[derive(Clone, PartialEq, Eq, Deserialize, Serialize, Debug)]
pub struct ClipboardEntry {
    pub source: String,
    pub data: String,
    pub timestamp: i64,
}

pub async fn get_entries(s: String) -> Result<Vec<ClipboardEntry>, reqwest::Error> {
    log::info!("Fetching for {}", s);
    let url = format!("{}{BASE_API_URL}/query", base_url());
    let params = [("q", s.as_str()), ("size", "100")];
    let url = reqwest::Url::parse_with_params(&url, &params).unwrap();
    let entries = reqwest::get(url)
        .await?
        .json::<Vec<ClipboardEntry>>()
        .await?;
    Ok(entries)
}

#[component]
fn Entry(cx: Scope, entry: ClipboardEntry) -> Element {
    let ClipboardEntry {
        source,
        data,
        timestamp,
    } = entry;
    let timestamp = chrono::NaiveDateTime::from_timestamp_opt(*timestamp, 0)
        .unwrap()
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();
    cx.render(rsx! {
        li {
            class: "flex justify-between gap-x-6 py-5",
            div {
                class: "flex min-w-0 gap-x-4",
                div {
                    class: "min-w-0 flex-auto",
                    pre {
                        class: "my_pre",
                        "{data}"
                    }
                    span {
                        class: "inline-flex items-center rounded-md bg-yellow-50 px-2 py-1 text-xs font-medium text-yellow-800 ring-1 ring-inset ring-yellow-600/20",
                        "{source}"
                    }
                    span {
                        class: "inline-flex items-center rounded-md bg-gray-50 px-2 py-1 text-xs font-medium text-gray-600 ring-1 ring-inset ring-gray-500/10",
                        "{timestamp}"
                    }
                }
            }
        }
    })
}

pub fn app(cx: Scope) -> Element {
    let search_input = use_state(cx, String::new);
    let s = search_input.get().to_owned();
    let entries = use_future(cx, (), |_| async move {
        sleep(Duration::from_millis(300)).await;
        get_entries(s).await
    });
    render! {
        link {
            rel: "stylesheet",
            href: "/tailwind.min.css",
            href: "/style.css"
        }
        div {
            class: "mx-auto max-w-7xl px-4 sm:px-6 lg:px-8",
            div {
                class: "mx-auto max-w-2xl py-16 sm:py-6 lg:py-7",
                div {
                    class: "flex h-16 items-center",
                    div {
                        class: "flex-shrink-0",
                        img {
                            width: 64,
                            height: 64,
                            src: "/logo.png",
                        }
                    }
                    div {
                        class: "ml-10 flex items-baseline",
                        h1 {
                            class: "text-4xl font-bold tracking-tight text-gray-900 sm:text-4xl",
                            "Clip Sync"
                        }
                    }
                }
            }
            section {
                header {
                    class: "bg-white space-y-4 p-4 sm:px-8 sm:py-6 lg:p-4 xl:px-8 xl:py-6",
                    form {
                        onsubmit: move |event| {
                            log::info!("Submitted! {event:?}");
                            event.data.values.get("search").map(|v| {
                                v.iter().next().map(|v| {
                                    search_input.set(v.to_string());
                                    entries.restart();
                                })
                            });
                        },
                        class: "group relative",
                        svg {
                            class: "absolute left-3 top-1/2 -mt-2.5 text-slate-400 pointer-events-none group-focus-within:text-blue-500",
                            width: 20,
                            height: 20,
                            fill: "currentColor",
                            path {
                                fill_rule: "evenodd",
                                clip_rule: "evenodd",
                                d: "M8 4a4 4 0 100 8 4 4 0 000-8zM2 8a6 6 0 1110.89 3.476l4.817 4.817a1 1 0 01-1.414 1.414l-4.816-4.816A6 6 0 012 8z"
                            }
                        }
                        input {
                            name: "search",
                            class: "focus:ring-2 focus:ring-blue-500 focus:outline-none appearance-none w-full text-sm leading-6 text-slate-900 placeholder-slate-400 rounded-md py-2 pl-10 ring-1 ring-slate-200 shadow-sm",
                            "type": "text",
                            value: "{search_input}",
                            placeholder: "Search for text...",
                            oninput: move |evt| { search_input.set(evt.value.clone()); entries.restart(); },
                            // onkeydown: move |evt| { if evt.key() == Key::Enter {} }
                        }
                    }
                    match entries.value() {
                        Some(Ok(list)) => {
                            render! {
                                ul {
                                    role: "list",
                                    class: "divide-y divide-gray-100",
                                    for e in list {
                                        // render every entry with the Entry component
                                        Entry { entry: e.clone() }
                                    }
                                }
                            }
                        }
                        Some(Err(err)) => {
                            render! {"An error occurred while fetching entries {err}"}
                        }
                        None => {
                            render! { div { "Loading..." } }
                        }
                    }
                }
            }
        }
    }
}

pub fn main() {
    dioxus_logger::init(LevelFilter::Info).expect("failed to init logger");
    dioxus_web::launch(app);
}
