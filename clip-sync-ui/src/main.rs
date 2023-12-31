use std::time::Duration;
use chrono::{Utc, TimeZone};
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
pub fn CopyButton(cx: Scope, text: String) -> Element {
    cx.render(rsx! {
        a {
            onclick : move |_| {
                log::info!("Copying {}", text);
                let _ = web_sys::window().unwrap().navigator().clipboard().unwrap().write_text(text);
            },
            svg {
                width: "16px",
                height: "auto",
                view_box: "0 0 115.77 122.88",
                path {
                    d: "M89.62,13.96v7.73h12.19h0.01v0.02c3.85,0.01,7.34,1.57,9.86,4.1c2.5,2.51,4.06,5.98,4.07,9.82h0.02v0.02 v73.27v0.01h-0.02c-0.01,3.84-1.57,7.33-4.1,9.86c-2.51,2.5-5.98,4.06-9.82,4.07v0.02h-0.02h-61.7H40.1v-0.02 c-3.84-0.01-7.34-1.57-9.86-4.1c-2.5-2.51-4.06-5.98-4.07-9.82h-0.02v-0.02V92.51H13.96h-0.01v-0.02c-3.84-0.01-7.34-1.57-9.86-4.1 c-2.5-2.51-4.06-5.98-4.07-9.82H0v-0.02V13.96v-0.01h0.02c0.01-3.85,1.58-7.34,4.1-9.86c2.51-2.5,5.98-4.06,9.82-4.07V0h0.02h61.7 h0.01v0.02c3.85,0.01,7.34,1.57,9.86,4.1c2.5,2.51,4.06,5.98,4.07,9.82h0.02V13.96L89.62,13.96z M79.04,21.69v-7.73v-0.02h0.02 c0-0.91-0.39-1.75-1.01-2.37c-0.61-0.61-1.46-1-2.37-1v0.02h-0.01h-61.7h-0.02v-0.02c-0.91,0-1.75,0.39-2.37,1.01 c-0.61,0.61-1,1.46-1,2.37h0.02v0.01v64.59v0.02h-0.02c0,0.91,0.39,1.75,1.01,2.37c0.61,0.61,1.46,1,2.37,1v-0.02h0.01h12.19V35.65 v-0.01h0.02c0.01-3.85,1.58-7.34,4.1-9.86c2.51-2.5,5.98-4.06,9.82-4.07v-0.02h0.02H79.04L79.04,21.69z M105.18,108.92V35.65v-0.02 h0.02c0-0.91-0.39-1.75-1.01-2.37c-0.61-0.61-1.46-1-2.37-1v0.02h-0.01h-61.7h-0.02v-0.02c-0.91,0-1.75,0.39-2.37,1.01 c-0.61,0.61-1,1.46-1,2.37h0.02v0.01v73.27v0.02h-0.02c0,0.91,0.39,1.75,1.01,2.37c0.61,0.61,1.46,1,2.37,1v-0.02h0.01h61.7h0.02 v0.02c0.91,0,1.75-0.39,2.37-1.01c0.61-0.61,1-1.46,1-2.37h-0.02V108.92L105.18,108.92z"
                }
            }
        }
    })
}

#[component]
fn Entry(cx: Scope, entry: ClipboardEntry) -> Element {
    let ClipboardEntry {
        source,
        data,
        timestamp,
    } = entry;
    let datetime = Utc.timestamp_opt(*timestamp, 0).unwrap();
    let local_time = datetime.with_timezone(&chrono::Local);
    let time_string = local_time.format("%Y-%m-%d %H:%M:%S %Z").to_string();
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
                        class: "inline-flex items-center px-2 py-1 text-xs font-medium text-gray-600",
                        CopyButton { text: data.clone() }
                    }
                    span {
                        class: "inline-flex items-center rounded-md bg-yellow-50 px-2 py-1 text-xs font-medium text-yellow-800 ring-1 ring-inset ring-yellow-600/20",
                        "{source}"
                    }
                    span {
                        class: "inline-flex items-center rounded-md bg-gray-50 px-2 py-1 text-xs font-medium text-gray-600 ring-1 ring-inset ring-gray-500/10",
                        "{time_string}"
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
            class: "mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 lg:justify-between",
            div {
                class: "mx-grid grid-cols-4 gap-4 justify-items-start max-w-md",
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
                        class: "ml-10 flex-grow items-baseline ",
                        h1 {
                            class: "text-4xl font-bold tracking-tight text-gray-900 sm:text-4xl",
                            "Clip\u{00a0}Sync"
                        }
                    }
                    div {
                        class: "flex-shrink-0",
                        a {
                            class: "text-gray-500 hover:text-gray-700",
                            href: "https://github.com/windoze/clip-sync",
                            target: "_blank",
                            svg {
                                class: "w-5 h-5",
                                path {
                                    d: "M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z"
                                } 
                            }
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
