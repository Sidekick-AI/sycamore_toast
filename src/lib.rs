use std::fmt::Debug;

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
pub struct Toasts<T: Clone + Debug + Default + Serialize + DeserializeOwned + 'static> {
    toasts: Signal<Vec<(T, u8)>>,
}

pub enum CookieError {
    CookieNotPresent,
    InvalidCookie,
}

impl<T: Clone + Debug + Default + Serialize + DeserializeOwned> Toasts<T> {
    pub fn from_cookies() -> Result<Self, CookieError> {
        if let Some(Ok(c)) = wasm_cookies::get("sycamore_toasts") {
            Ok(Self {
                toasts: create_signal(
                    serde_json::from_str(&c).map_err(|_| CookieError::InvalidCookie)?,
                ),
            })
        } else {
            Err(CookieError::CookieNotPresent)
        }
    }

    pub fn clear_toasts(&self) {
        self.toasts.update(|i| i.retain(|(_, r)| *r >= 1));
        self.toasts
            .update(|i| i.iter_mut().for_each(|(_, i)| *i -= 1));
        self.save_to_cookies();
    }

    pub fn add_toast(&self, toast: T) -> &Self {
        self.toasts.update(|i| i.push((toast, 0)));
        self.save_to_cookies();
        self
    }

    pub fn with_rank(&self, rank: u8) -> &Self {
        self.toasts.update(|i| {
            if let Some((_, r)) = i.last_mut() {
                *r = rank;
            }
        });
        self.save_to_cookies();
        self
    }

    fn save_to_cookies(&self) {
        // Save to cookies
        wasm_cookies::set(
            "sycamore_toasts",
            &serde_json::to_string(&self.toasts.get_clone_untracked()).unwrap(),
            &CookieOptions::default(),
        );
    }
}

#[component(inline_props)]
pub fn ToastsView<
    G: Html,
    T: Clone + Debug + Default + PartialEq + Serialize + DeserializeOwned + 'static,
    F: Fn(T) -> View<G> + 'static,
>(
    view: F,
    toasts: Toasts<T>,
) -> View<G> {
    if try_use_context::<Toasts<T>>().is_none() {
        provide_context(toasts.clone());
    }
    let new_toasts = create_memo(move || {
        toasts
            .toasts
            .get_clone()
            .iter()
            .filter(|(_, r)| *r == 0)
            .cloned()
            .map(|(t, _)| t)
            .collect()
    });
    view! {
        div (class="-translate-y-[300px] z-50") // To include the right class for fading out toasts
        div (class="fixed top-14 flex flex-col items-center z-50", style="left: 50%; width: 500px; max-width: 100vw; transform: translateX(-50%);") {
            Indexed (
                iterable=new_toasts,
                view=view
            )
        }
    }
}

#[component]
pub fn DefaultToastView<G: Html>(toast: Toast) -> View<G> {
    let toast1 = toast.clone();
    let node_ref = create_node_ref();
    let remove = move |_| {
        let toast1 = toast1.clone();
        spawn_local_scoped(async move {
            // Move to top
            node_ref.get::<DomNode>().add_class("-translate-y-[300px]");
            gloo_timers::future::TimeoutFuture::new(200).await;
            // Remove
            let toasts = use_context::<Toasts<Toast>>();
            toasts
                .toasts
                .update(|i| i.retain(|(t, _)| t.id != toast1.id));
        })
    };

    // Spawn process to remove toast after 5 seconds
    let toast1 = toast.clone();
    spawn_local_scoped(async move {
        gloo_timers::future::TimeoutFuture::new(5000).await;
        // Move to top
        node_ref.get::<DomNode>().add_class("-translate-y-[300px]");
        gloo_timers::future::TimeoutFuture::new(200).await;
        // Remove
        let toasts = use_context::<Toasts<Toast>>();
        toasts.toasts.update(|i| i.retain(|(t, _)| *t != toast1));
    });

    let (bg_color, image_name) = match toast.toast_type {
        ToastType::Danger => ("#fc2828", "x_toast.png"),
        ToastType::Warning => ("#fae739", "warning_toast.png"),
        ToastType::Primary => ("#395cfa", "info_toast.png"),
        ToastType::Success => ("#04c55e", "check_toast.png"),
    };
    view! {
        div (ref=node_ref, style=format!("border-color: {}", bg_color), class="w-full bg-white max-w-lg px-5 py-4 m-2 border-[3px] rounded-xl flex flex-row items-center transition-all z-50") {
            // Icon
            img (src=(format!("/static/images/icons/{image_name}")), width="30px", height="30px", class="object-scale-down")

            // Title / text
            (if toast.body.replace(' ', "").is_empty() {
                let title = toast.title.clone();
                view!{
                    p (class=(format!("ml-8 pt-1 font-montserrat text-lg font-bold text-slate-900"))) {(title)}
                }
            } else {
                let (title, message) = (toast.title.clone(), toast.body.clone());
                view!{
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
        let _ = sycamore::render_to_string(|| {
            let toasts = Toasts::default();
            view! {
                ToastsView (
                    toasts=toasts,
                    view=DefaultToastView
                )
            }
        });
    }
}
