use gloo_console as console;
use gloo_net::http::Request;
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsCast;
use yew::prelude::*;

mod auth;
use auth::{
    AuthState, BACKEND_URL, clear_saved_state, clear_url_params, exchange_code_for_token,
    get_saved_state, initiate_oauth_flow, parse_oauth_callback,
};

#[derive(Deserialize, Serialize, Clone, PartialEq)]
struct Message {
    id: Option<i64>,
    content: String,
    author: String,
    created_at: Option<String>,
    user_id: Option<i64>,
}

#[derive(Clone, PartialEq)]
enum AppState {
    CheckingAuth,
    Unauthenticated,
    Authenticated,
    Loading,
    Error(String),
}

#[function_component(App)]
fn app() -> Html {
    let auth_state = use_state(AuthState::default);
    let app_state = use_state(|| AppState::CheckingAuth);
    let messages = use_state(Vec::<Message>::new);

    // Check for OAuth callback on mount
    {
        let auth_state = auth_state.clone();
        let app_state = app_state.clone();

        use_effect_with((), move |()| {
            if let Some((code, state)) = parse_oauth_callback() {
                // Verify state matches
                if let Some(saved_state) = get_saved_state() {
                    if saved_state == state {
                        clear_saved_state();
                        clear_url_params();

                        wasm_bindgen_futures::spawn_local(async move {
                            app_state.set(AppState::Loading);

                            match exchange_code_for_token(code).await {
                                Ok(auth_response) => {
                                    auth_state.set(AuthState {
                                        token: Some(auth_response.token),
                                        user: Some(auth_response.user),
                                    });
                                    app_state.set(AppState::Authenticated);
                                }
                                Err(e) => {
                                    console::error!(&format!("Auth error: {e}"));
                                    app_state.set(AppState::Error(e));
                                }
                            }
                        });
                    } else {
                        console::error!("State mismatch in OAuth callback");
                        app_state.set(AppState::Unauthenticated);
                    }
                } else {
                    console::error!("No saved state found");
                    app_state.set(AppState::Unauthenticated);
                }
            } else {
                // No callback params, check if we have existing auth
                if auth_state.is_authenticated() {
                    app_state.set(AppState::Authenticated);
                } else {
                    app_state.set(AppState::Unauthenticated);
                }
            }
        });
    }

    // Load messages when authenticated
    {
        let messages = messages.clone();
        let auth_state = auth_state.clone();
        let app_state_val = (*app_state).clone();

        use_effect_with(app_state_val, move |state| {
            if matches!(state, AppState::Authenticated) {
                if let Some(token) = &auth_state.token {
                    let token = token.clone();
                    wasm_bindgen_futures::spawn_local(async move {
                        match Request::get(&format!("{BACKEND_URL}/messages"))
                            .header("Authorization", &format!("Bearer {token}"))
                            .send()
                            .await
                        {
                            Ok(response) => {
                                if let Ok(data) = response.json::<Vec<Message>>().await {
                                    messages.set(data);
                                }
                            }
                            Err(e) => {
                                console::error!(&format!("Failed to fetch messages: {e}"));
                            }
                        }
                    });
                }
            }
        });
    }

    let on_login = {
        Callback::from(move |_| {
            initiate_oauth_flow();
        })
    };

    let on_logout = {
        let auth_state = auth_state.clone();
        let app_state = app_state.clone();
        let messages = messages.clone();

        Callback::from(move |_| {
            auth_state.set(AuthState::default());
            app_state.set(AppState::Unauthenticated);
            messages.set(vec![]);
        })
    };

    let on_send_message = {
        let auth_state = auth_state.clone();
        let messages = messages.clone();

        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();

            if let Some(token) = &auth_state.token {
                let token = token.clone();
                let messages = messages.clone();
                let user = auth_state.user.clone();

                if let Some(user) = user {
                    let target = e.target_dyn_into::<web_sys::HtmlFormElement>().unwrap();
                    let content = target
                        .elements()
                        .named_item("content")
                        .unwrap()
                        .dyn_into::<web_sys::HtmlInputElement>()
                        .unwrap()
                        .value();

                    if !content.is_empty() {
                        let message = Message {
                            id: None,
                            content,
                            author: user.name.clone(),
                            created_at: None,
                            user_id: Some(user.id),
                        };

                        wasm_bindgen_futures::spawn_local(async move {
                            match Request::post(&format!("{BACKEND_URL}/messages"))
                                .header("Authorization", &format!("Bearer {token}"))
                                .json(&message)
                                .unwrap()
                                .send()
                                .await
                            {
                                Ok(response) => {
                                    if let Ok(new_message) = response.json::<Message>().await {
                                        let mut current_messages = (*messages).clone();
                                        current_messages.insert(0, new_message);
                                        messages.set(current_messages);
                                    }
                                }
                                Err(e) => {
                                    console::error!(&format!("Failed to send message: {e}"));
                                }
                            }
                        });

                        target.reset();
                    }
                }
            }
        })
    };

    html! {
        <div style="max-width: 800px; margin: 0 auto; padding: 20px;">
            <h1>{"Personal Assistant"}</h1>

            {match &*app_state {
                AppState::CheckingAuth => html! {
                    <div style="text-align: center; padding: 40px;">
                        <p>{"⏳ Checking authentication..."}</p>
                    </div>
                },
                AppState::Loading => html! {
                    <div style="text-align: center; padding: 40px;">
                        <p>{"⏳ Authenticating..."}</p>
                    </div>
                },
                AppState::Unauthenticated => html! {
                    <div style="text-align: center; padding: 40px;">
                        <h2>{"Welcome!"}</h2>
                        <p>{"Please sign in with your Google account to continue."}</p>
                        <button
                            onclick={on_login}
                            style="background: #4285f4; color: white; border: none; padding: 10px 20px; border-radius: 4px; font-size: 16px; cursor: pointer; margin-top: 20px;"
                        >
                            {"Sign in with Google"}
                        </button>
                    </div>
                },
                AppState::Error(error) => html! {
                    <div style="text-align: center; padding: 40px;">
                        <p style="color: red;">{format!("❌ Error: {}", error)}</p>
                        <button
                            onclick={on_login}
                            style="background: #4285f4; color: white; border: none; padding: 10px 20px; border-radius: 4px; font-size: 16px; cursor: pointer; margin-top: 20px;"
                        >
                            {"Try Again"}
                        </button>
                    </div>
                },
                AppState::Authenticated => html! {
                    <>
                        {if let Some(user) = &auth_state.user {
                            html! {
                                <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px; padding: 10px; background: #f5f5f5; border-radius: 8px;">
                                    <div>
                                        <strong>{"Signed in as: "}</strong>{&user.email}
                                    </div>
                                    <button
                                        onclick={on_logout}
                                        style="background: #dc3545; color: white; border: none; padding: 5px 15px; border-radius: 4px; cursor: pointer;"
                                    >
                                        {"Sign Out"}
                                    </button>
                                </div>
                            }
                        } else {
                            html! {}
                        }}

                        <div style="margin-bottom: 20px;">
                            <h2>{"Messages"}</h2>

                            <form onsubmit={on_send_message} style="margin-bottom: 20px;">
                                <div style="display: flex; gap: 10px;">
                                    <input
                                        type="text"
                                        name="content"
                                        placeholder="Type a message..."
                                        style="flex: 1; padding: 8px; border: 1px solid #ddd; border-radius: 4px;"
                                        required=true
                                    />
                                    <button
                                        type="submit"
                                        style="background: #28a745; color: white; border: none; padding: 8px 20px; border-radius: 4px; cursor: pointer;"
                                    >
                                        {"Send"}
                                    </button>
                                </div>
                            </form>

                            <div style="border: 1px solid #ddd; border-radius: 8px; padding: 15px; min-height: 300px; max-height: 500px; overflow-y: auto;">
                                {if messages.is_empty() {
                                    html! {
                                        <p style="text-align: center; color: #666;">{"No messages yet. Start a conversation!"}</p>
                                    }
                                } else {
                                    html! {
                                        <div>
                                            {for messages.iter().map(|msg| html! {
                                                <div style="margin-bottom: 15px; padding: 10px; background: #f9f9f9; border-radius: 4px;">
                                                    <div style="display: flex; justify-content: space-between; margin-bottom: 5px;">
                                                        <strong>{&msg.author}</strong>
                                                        {if let Some(created_at) = &msg.created_at {
                                                            html! { <small style="color: #666;">{created_at}</small> }
                                                        } else {
                                                            html! {}
                                                        }}
                                                    </div>
                                                    <div>{&msg.content}</div>
                                                </div>
                                            })}
                                        </div>
                                    }
                                }}
                            </div>
                        </div>
                    </>
                }
            }}
        </div>
    }
}

fn main() {
    yew::Renderer::<App>::with_root(gloo::utils::document().get_element_by_id("app").unwrap())
        .render();
}

