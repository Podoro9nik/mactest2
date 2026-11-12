use crate::{
    i18n::Lang,
    state::{Page, State},
};
use rune::{
    Align, Color, Cx, El, FileDialogOptions, ScrollSpec, SurfaceFit, TextStyle, col, el, row, text,
};

mod create;
mod pages;
#[cfg(test)]
mod tests;

fn color(value: &str) -> Color {
    rune::color::hex(value).expect("UI color")
}
fn ink() -> Color {
    color("#493D3A")
}
fn muted() -> Color {
    color("#83716B")
}
fn pink() -> Color {
    color("#F6CED5")
}
fn blue() -> Color {
    color("#D8EBF5")
}
fn line() -> Color {
    color("#EBDDD4")
}
fn label(value: impl Into<String>, size: f32) -> El {
    text(value).font(TextStyle::new(size, 400.0)).color(ink())
}
fn para(value: impl Into<String>) -> El {
    label(value, 22.0).w_full().wrap_anywhere().color(muted())
}
fn heading(value: &str) -> El {
    label(value, 28.0).w_full().wrap()
}
fn card() -> El {
    col()
        .w_full()
        .padding(20.0)
        .gap(15.0)
        .fill(Color::WHITE)
        .radius(22.0)
        .border(1.0, line())
}
fn scroll() -> El {
    col().w_full().grow(1.0).min_h(0.0).scroll(ScrollSpec {
        stick_bottom: false,
    })
}
fn button(key: impl Into<String>, title: impl Into<String>, selected: bool, disabled: bool) -> El {
    let title = title.into();
    row()
        .key(key)
        .role("button")
        .label(title.clone())
        .focusable(!disabled)
        .disabled(disabled)
        .center()
        .padding(11.0)
        .radius(14.0)
        .border(1.0, if selected { color("#D7AAB3") } else { line() })
        .fill(if selected { pink() } else { color("#FFFCF8") })
        .hover(color("#FBE4E7"))
        .opacity(if disabled { 0.50 } else { 1.0 })
        .child(label(title, 22.0).shrink().wrap().align(Align::Center))
}
fn picture(state: &State, id: &str, width: f32, height: f32) -> El {
    let node = el().size(width, height).a11y_hidden();
    match state.assets.get(id) {
        Some(image) => node.pixels_fit(image.pixels(), SurfaceFit::Fit),
        None => node,
    }
}
fn languages(state: &State, prefix: &str) -> El {
    let mut choices = row().gap(6.0);
    for (lang, title) in [
        (Lang::Ru, "Русский"),
        (Lang::En, "English"),
        (Lang::Ja, "日本語"),
    ] {
        choices = choices.child(
            button(
                format!("{prefix}-{}", lang.code()),
                title,
                state.lang == lang,
                false,
            )
            .py(7.0)
            .on_press(move |s: &mut State| s.set_language(lang)),
        );
    }
    choices
}
fn sidebar(state: &State) -> El {
    let l = state.lang;
    let mut nav = col()
        .key("sidebar")
        .w(190.0)
        .h_full()
        .padding(17.0)
        .gap(10.0)
        .fill(color("#FBE8E8"))
        .child(
            col()
                .w_full()
                .center()
                .gap(6.0)
                .pb(22.0)
                .child(picture(state, "chiikawa", 92.0, 108.0))
                .child(
                    label(l.text("app"), 26.0)
                        .w_full()
                        .wrap()
                        .align(Align::Center),
                )
                .child(
                    label(l.text("tagline"), 18.0)
                        .w_full()
                        .wrap()
                        .align(Align::Center)
                        .color(muted()),
                ),
        );
    for (page, key) in [
        (Page::Create, "create"),
        (Page::History, "history"),
        (Page::Settings, "settings"),
    ] {
        nav = nav.child(
            button(format!("nav-{key}"), l.text(key), state.page == page, false)
                .w_full()
                .on_press(move |s: &mut State| s.page = page),
        );
    }
    nav.child(el().grow(1.0))
        .child(label(l.text("footer"), 18.0).w_full().wrap().color(muted()))
}
pub fn view(state: &State, cx: &mut Cx<State>) -> El {
    cx.foreground(ink());
    let l = state.lang;
    let title = match state.page {
        Page::Create => "title",
        Page::History => "history",
        Page::Settings => "settings",
    };
    let page = match state.page {
        Page::Create => create::view(state),
        Page::History => pages::history(state),
        Page::Settings => pages::settings(state),
    };
    row()
        .key("chiikawizator")
        .role("window")
        .label(l.text("app"))
        .w_full()
        .h_full()
        .fill(color("#FFF9EF"))
        .child(sidebar(state))
        .child(
            col()
                .key("workspace")
                .grow(1.0)
                .min_w(0.0)
                .h_full()
                .padding(24.0)
                .gap(18.0)
                .child(
                    row()
                        .w_full()
                        .center_y()
                        .gap(12.0)
                        .child(label(l.text(title), 36.0).grow(1.0).min_w(0.0).wrap())
                        .child(languages(state, "language")),
                )
                .child(page),
        )
}
fn image_dialog(state: &State, reference: bool) -> FileDialogOptions {
    FileDialogOptions {
        title: state
            .lang
            .text(if reference {
                "add_reference"
            } else {
                "add_photo"
            })
            .into(),
        ..Default::default()
    }
    .filter("PNG, JPEG, WebP", &["png", "jpg", "jpeg", "webp"])
}
fn add_photo_button(state: &State) -> El {
    button(
        "add-photo",
        state.lang.text("add_photo"),
        false,
        state.busy || state.photos.len() >= 8,
    )
    .on_press_tx(|s: &mut State, tx| {
        if s.busy || s.photos.len() >= 8 {
            return;
        }
        tx.open_file(
            image_dialog(s, false),
            |s: &mut State, result, _| match result {
                Ok(Some(path)) => s.add_photo(path),
                Ok(None) => (),
                Err(_) => s.error = Some("image_invalid".into()),
            },
        );
    })
}
fn alerts(state: &State) -> El {
    let mut content = col().key("alerts").w_full().gap(8.0);
    if let Some(error) = &state.error {
        content = content.child(
            col()
                .key("error")
                .w_full()
                .padding(14.0)
                .radius(14.0)
                .fill(color("#FCE6E7"))
                .child(para(state.lang.error(error)).color(color("#9A4249"))),
        );
    }
    if let Some(notice) = &state.notice {
        content = content.child(
            para(state.lang.text(notice))
                .key("notice")
                .color(color("#527F6B")),
        );
    }
    content
}
