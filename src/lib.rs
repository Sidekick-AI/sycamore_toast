#![allow(clippy::type_complexity)]
use std::{marker::PhantomData, fmt::Debug};

use sycamore::{prelude::*, futures::{spawn_local_scoped}};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ToastType {
    Primary,
    Success,
    Warning,
    Danger,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Toast {
    title: String,
    body: String,
    toast_type: ToastType,
    id: uuid::Uuid,
}

impl Default for Toast {
    fn default() -> Self {
        Self { title: Default::default(), body: Default::default(), toast_type: ToastType::Primary, id: uuid::Uuid::new_v4() }
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
pub struct Toasts<T: Clone + Debug + Default> {
    toasts: RcSignal<Vec<(T, u8)>>,
}

impl <T: Clone + Debug + Default> Toasts<T> {
    pub fn clear_toasts(&self) {
        self.toasts.modify().retain(|(_, r)| *r >= 1);
        for (_, rank) in self.toasts.modify().iter_mut() {
            *rank -= 1;
        }
    }

    pub fn add_toast(&self, toast: T) -> &Self {
        self.toasts.modify().push((toast, 0));
        self
    }

    pub fn with_rank(&self, rank: u8) -> &Self {
        if let Some((_, r)) = self.toasts.modify().last_mut() {
            *r = rank;
        }
        self
    }
}

#[derive(Prop, Default)]
pub struct ToastsViewProp<'a, G: GenericNode, F, T: Clone + Debug + Default>
where F: Fn(BoundedScope<'_, 'a>, T) -> View<G> + 'a
 {
    view: F,
    toasts: Toasts<T>,
    #[builder(default)]
    _phantom: PhantomData<&'a ()>,
}

#[component]
pub fn ToastsView<'a, G: Html, T: Clone + Debug + Default + PartialEq + 'static, F: Fn(BoundedScope<'_, 'a>, T) -> View<G> + 'a>(
    cx: Scope<'a>, 
    ToastsViewProp { view, toasts, _phantom }: ToastsViewProp<'a, G, F, T>
) -> View<G> {
    if try_use_context::<Toasts<T>>(cx).is_none() {
        provide_context(cx, toasts.clone());
    }
    let new_toasts = create_memo(cx, move || {
        toasts.toasts.get().iter().filter(|(_, r)| *r == 0).cloned().map(|(t, _)| t).collect()
    });
    view!{cx,
        div (class="-translate-y-[300px]") // To include the right class for fading out toasts
        div (class="fixed top-14 w-full flex flex-col items-center") {
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

    let (bg_color, text_color, image_name) = match toast.toast_type {
        ToastType::Danger => ("#9c4d44", "text-red-600", "x_toast.png"),
        ToastType::Warning => ("#9c9844", "text-yellow-400", "warning_toast.png"),
        ToastType::Primary => ("#445d9c", "text-blue-600", "info_toast.png"),
        ToastType::Success => ("#449c5a", "text-green-500", "check_toast.png"),
    };
    view! {cx,
        div (ref=node_ref, style=(format!("background-image: linear-gradient(to right, {bg_color}, #111827, #111827, #111827, #111827);")), class="w-full max-w-lg p-5 m-2 rounded-lg flex flex-row items-center transition-all z-50") {
            // Icon
            img (src=(format!("/static/images/icons/{image_name}")), width="30px", height="30px", class="object-scale-down")
            
            // Title / text
            (if toast.body.replace(' ', "").is_empty() {
                let title = toast.title.clone();
                view!{cx,
                    p (class=(format!("ml-8 pt-1 font-montserrat font-lg font-bold {text_color}"))) {(title)}
                }
            } else {
                let (title, message) = (toast.title.clone(), toast.body.clone());
                view!{cx, 
                    div (class="ml-8 flex flex-col") {
                        p (class=(format!("font-montserrat font-lg font-bold {text_color}"))) {(title)}
                        p (class="text-gray-400") {(message)}
                    }      
                }
            })

            div (class="flex-grow")

            // Close button
            button (class="bg-[#111827] hover:bg-slate-600 text-2xl text-slate-500 hover:text-slate-900 w-8 h-8 mr-5 transition-all rounded-lg", on:click=remove) {
                "X"
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use sycamore::prelude::*;
    use super::*;

    #[test]
    fn test_toasts() {
        let _ = sycamore::render_to_string(|cx| {
            let toasts = Toasts::default();
            view!{cx,
                ToastsView (
                    toasts=toasts,
                    view=DefaultToastView
                )
            }
        });
    }
}