#![allow(clippy::type_complexity)]
use std::{fmt::Debug, marker::PhantomData};

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sycamore::{futures::spawn_local_scoped, prelude::*};
use wasm_cookies::CookieOptions;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum ToastType {
    Primary,
    Success,
    Warning,
    Danger,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Toast {
    title: String,
    body: String,
    toast_type: ToastType,
    id: uuid::Uuid,
}

impl Default for Toast {
    fn default() -> Self {
        Self {
            title: Default::default(),
            body: Default::default(),
            toast_type: ToastType::Primary,
            id: uuid::Uuid::new_v4(),
        }
    }
}

impl Toast {
    pub fn primary<T: ToString>(text: T) -> Self {
        Self {
            title: text.to_string(),
            body: String::new(),
            toast_type: ToastType::Primary,
            id: uuid::Uuid::new_v4(),
        }
    }

    pub fn success<T: ToString>(text: T) -> Self {
        Self {
            title: text.to_string(),
            body: String::new(),
            toast_type: ToastType::Success,
            id: uuid::Uuid::new_v4(),
        }
    }

    pub fn warning<T: ToString>(text: T) -> Self {
        Self {
            title: text.to_string(),
            body: String::new(),
            toast_type: ToastType::Warning,
            id: uuid::Uuid::new_v4(),
        }
    }

    pub fn danger<T: ToString>(text: T) -> Self {
        Self {
            title: text.to_string(),
            body: String::new(),
            toast_type: ToastType::Danger,
            id: uuid::Uuid::new_v4(),
        }
    }

    pub fn body<T: ToString>(mut self, body: T) -> Self {
        self.body = body.to_string();
        self
    }
}

#[derive(Default, Debug, Clone)]
pub struct Toasts<T: Clone + Debug + Default + Serialize + DeserializeOwned> {
    toasts: RcSignal<Vec<(T, u8)>>,
}

pub enum CookieError {
    CookieNotPresent,
    InvalidCookie,
}

impl<T: Clone + Debug + Default + Serialize + DeserializeOwned> Toasts<T> {
    pub fn from_cookies() -> Result<Self, CookieError> {
        if let Some(Ok(c)) = wasm_cookies::get("sycamore_toasts") {
            Ok(Self {
                toasts: create_rc_signal(
                    serde_json::from_str(&c).map_err(|_| CookieError::InvalidCookie)?,
                ),
            })
        } else {
            Err(CookieError::CookieNotPresent)
        }
    }

    pub fn clear_toasts(&self) {
        self.toasts.modify().retain(|(_, r)| *r >= 1);
        for (_, rank) in self.toasts.modify().iter_mut() {
            *rank -= 1;
        }
        self.save_to_cookies();
    }

    pub fn add_toast(&self, toast: T) -> &Self {
        self.toasts.modify().push((toast, 0));
        self.save_to_cookies();
        self
    }

    pub fn with_rank(&self, rank: u8) -> &Self {
        if let Some((_, r)) = self.toasts.modify().last_mut() {
            *r = rank;
        }
        self.save_to_cookies();
        self
    }

    fn save_to_cookies(&self) {
        // Save to cookies
        wasm_cookies::set(
            "sycamore_toasts",
            &serde_json::to_string(self.toasts.get_untracked().as_ref()).unwrap(),
            &CookieOptions::default(),
        );
    }
}

#[derive(Prop, Default)]
pub struct ToastsViewProp<
    'a,
    G: GenericNode,
    F,
    T: Clone + Debug + Default + Serialize + DeserializeOwned,
> where
    F: Fn(BoundedScope<'_, 'a>, T) -> View<G> + 'a,
{
    view: F,
    toasts: Toasts<T>,
    #[builder(default)]
    _phantom: PhantomData<&'a ()>,
}

#[component]
pub fn ToastsView<
    'a,
    G: Html,
    T: Clone + Debug + Default + PartialEq + Serialize + DeserializeOwned + 'static,
    F: Fn(BoundedScope<'_, 'a>, T) -> View<G> + 'a,
>(
    cx: Scope<'a>,
    ToastsViewProp {
        view,
        toasts,
        _phantom,
    }: ToastsViewProp<'a, G, F, T>,
) -> View<G> {
    if try_use_context::<Toasts<T>>(cx).is_none() {
        provide_context(cx, toasts.clone());
    }
    let new_toasts = create_memo(cx, move || {
        toasts
            .toasts
            .get()
            .iter()
            .filter(|(_, r)| *r == 0)
            .cloned()
            .map(|(t, _)| t)
            .collect()
    });
    view! {cx,
        div (class="-translate-y-[300px] z-50") // To include the right class for fading out toasts
        div (class="fixed top-14 flex flex-col items-center z-50", style="width: 500px; max-width: 100vw") {
            Indexed (
                iterable=new_toasts,
                view=move |cx, toast| (view)(cx, toast)
            )
        }
    }
}

#[component]
pub fn DefaultToastView<G: Html>(cx: BoundedScope, toast: Toast) -> View<G> {
    let toast1 = toast.clone();
    let node_ref = create_node_ref(cx);
    let remove = move |_| {
        let toast1 = toast1.clone();
        spawn_local_scoped(cx, async move {
            // Move to top
            node_ref.get::<DomNode>().add_class("-translate-y-[300px]");
            gloo_timers::future::TimeoutFuture::new(200).await;
            // Remove
            let toasts = use_context::<Toasts<Toast>>(cx);
            toasts.toasts.modify().retain(|(t, _)| t.id != toast1.id);
        })
    };

    // Spawn process to remove toast after 5 seconds
    let toast1 = toast.clone();
    spawn_local_scoped(cx, async move {
        gloo_timers::future::TimeoutFuture::new(5000).await;
        // Move to top
        node_ref.get::<DomNode>().add_class("-translate-y-[300px]");
        gloo_timers::future::TimeoutFuture::new(200).await;
        // Remove
        let toasts = use_context::<Toasts<Toast>>(cx);
        toasts.toasts.modify().retain(|(t, _)| *t != toast1);
    });

    let (bg_color, image_name) = match toast.toast_type {
        ToastType::Danger => ("#fc2828", "x_toast.png"),
        ToastType::Warning => ("#fae739", "warning_toast.png"),
        ToastType::Primary => ("#395cfa", "info_toast.png"),
        ToastType::Success => ("#04c55e", "check_toast.png"),
    };
    view! {cx,
        div (ref=node_ref, style=format!("border-color: {}", bg_color), class="w-full bg-white max-w-lg px-5 py-4 m-2 border-[3px] rounded-xl flex flex-row items-center transition-all z-50") {
            // Icon
            img (src=(format!("/static/images/icons/{image_name}")), width="30px", height="30px", class="object-scale-down")

            // Title / text
            (if toast.body.replace(' ', "").is_empty() {
                let title = toast.title.clone();
                view!{cx,
                    p (class=(format!("ml-8 pt-1 font-montserrat text-lg font-bold text-slate-900"))) {(title)}
                }
            } else {
                let (title, message) = (toast.title.clone(), toast.body.clone());
                view!{cx,
                    div (class="ml-8 flex flex-col") {
                        p (class=(format!("pt-1 font-montserrat text-lg font-bold text-slate-900"))) {(title)}
                        p (class="text-slate-700") {(message)}
                    }
                }
            })

            div (class="flex-grow")

            // Close button
            button (class="font-comfortaa font-bold hover:bg-slate-200 text-2xl text-slate-500 hover:text-slate-900 w-10 h-10 mr-5 transition-all rounded-lg p-2", on:click=remove) {
                "X"
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toasts() {
        let _ = sycamore::render_to_string(|cx| {
            let toasts = Toasts::default();
            view! {cx,
                ToastsView (
                    toasts=toasts,
                    view=DefaultToastView
                )
            }
        });
    }
}
