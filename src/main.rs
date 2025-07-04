use yew::prelude::*;

#[function_component(App)]
fn app() -> Html {
    html! {
        <div>
            <h1>{"Hello from WASM!"}</h1>
            <p>{"This is running Rust in your browser."}</p>
        </div>
    }
}

fn main() {
    yew::Renderer::<App>::with_root(gloo::utils::document().get_element_by_id("app").unwrap()).render();
}
