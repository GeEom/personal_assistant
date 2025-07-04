use gloo_net::http::Request;
use serde::Deserialize;
use yew::prelude::*;

#[derive(Deserialize, Clone, PartialEq)]
struct Message {
    text: String,
    from: String,
}

#[function_component(App)]
fn app() -> Html {
    let message = use_state(|| None::<Message>);
    let loading = use_state(|| false);
    let error = use_state(|| None::<String>);

    {
        let message = message.clone();
        let loading = loading.clone();
        let error = error.clone();

        use_effect_with((), move |_| {
            wasm_bindgen_futures::spawn_local(async move {
                loading.set(true);
                error.set(None);

                match Request::get("https://personal-assistant-backend.fly.dev/")
                    .send()
                    .await
                {
                    Ok(response) => match response.json::<Message>().await {
                        Ok(data) => {
                            message.set(Some(data));
                        }
                        Err(e) => {
                            error.set(Some(format!("Failed to parse response: {e}")));
                        }
                    },
                    Err(e) => {
                        error.set(Some(format!("Failed to fetch: {e}")));
                    }
                }
                loading.set(false);
            });
        });
    }

    html! {
        <div style="padding: 20px;">
            <h1>{"Mobile Prototype"}</h1>

            <div style="margin: 20px 0;">
                <h2>{"Frontend Status:"}</h2>
                <p>{"✅ Running Rust/WASM in your browser"}</p>
            </div>

            <div style="margin: 20px 0;">
                <h2>{"Backend Status:"}</h2>
                {if *loading {
                    html! { <p>{"⏳ Connecting to backend..."}</p> }
                } else if let Some(err) = (*error).clone() {
                    html! { <p style="color: red;">{format!("❌ Error: {}", err)}</p> }
                } else if let Some(msg) = (*message).clone() {
                    html! {
                        <>
                            <p>{"✅ Connected to Fly.io backend"}</p>
                            <p><strong>{"Message: "}</strong>{msg.text}</p>
                            <p><strong>{"From: "}</strong>{msg.from}</p>
                        </>
                    }
                } else {
                    html! { <p>{"🔄 Waiting for response..."}</p> }
                }}
            </div>
        </div>
    }
}

fn main() {
    yew::Renderer::<App>::with_root(gloo::utils::document().get_element_by_id("app").unwrap())
        .render();
}
