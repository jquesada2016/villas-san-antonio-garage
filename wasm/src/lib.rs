#[macro_use]
extern crate tracing;

use firebase_wasm::{
  auth::{
    get_auth,
    on_auth_state_changed,
  },
  firestore::{
    doc,
    get_doc,
    get_firestore,
    set_doc,
  },
};
use leptos::*;
use leptos_daisyui_components::{
  Color,
  *,
};
use leptos_declarative::prelude::*;
use leptos_tea::Cmd;
use wasm_bindgen::prelude::*;

leptos_daisyui_components::include_component_classes!();

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen(start)]
pub fn start() {
  tracing_subscriber::fmt()
    .with_writer(tracing_subscriber_wasm::MakeConsoleWriter::default())
    .without_time()
    .pretty()
    .with_max_level(tracing::Level::TRACE)
    .with_ansi(false)
    .init();

  info!("WASM successfully initialized");

  mount_to_body(|cx| view! { cx, <App /> });
}

#[derive(Default, leptos_tea::Model)]
struct Model {
  is_auth_initialized: bool,
  user: Option<firebase_wasm::auth::User>,
  pin: String,
}

#[derive(Default, Debug)]
enum Msg {
  #[default]
  Init,
  SignInWithGoogle,
  SetPin(String),
  PressButton,
}

fn update(model: UpdateModel, msg: &Msg, mut cmd: Cmd<Msg>) {
  debug!("msg:\n{msg:#?}");

  match msg {
    Msg::Init => {
      let closure = Closure::new(move |user| {
        model.is_auth_initialized.set(true);

        model.user.set(user);
      });

      on_auth_state_changed(get_auth(), &closure);

      closure.forget();
    }
    Msg::SignInWithGoogle => todo!(),
    Msg::SetPin(pin) => model.pin.set(pin.clone()),
    Msg::PressButton => {
      let doc_ref = doc(get_firestore(), "garage/status").unwrap();

      cmd.cmd(async move {
        let doc = get_doc(doc_ref.clone()).await.unwrap();

        let data = doc.data();

        js_sys::Reflect::set(&data, &"pressToken".into(), &true.into())
          .unwrap();

        set_doc(doc_ref, data).await.unwrap();

        None
      });
    }
  }
}

#[component]
fn App(cx: Scope) -> impl IntoView {
  let (model, msg) = Model::default().init(cx, update);

  view! { cx,
      <div class="h-screen flex flex-col items-center justify-center">
        <If signal=model.is_auth_initialized>
          <Then>
            <If signal=(move || model.pin.get() == "7894561230").derive_signal(cx)>
              <Then>
                <Button
                  color=Color::Primary
                  on_click=(move |_| msg(Msg::PressButton)).mapped_signal_setter(cx)
                >"Open"</Button>
              </Then>
              <Else>
                <TextInput
                  label="PIN"
                  type_=TextInputType::Number
                  focus=true
                  value=model.pin
                  on_value=(move |pin| msg(Msg::SetPin(pin))).mapped_signal_setter(cx)
                />
              </Else>
            </If>
          </Then>
        </If>
      </div>
  }
}
