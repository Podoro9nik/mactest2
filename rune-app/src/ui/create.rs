use super::*;
use crate::backend::{CHARACTER_IDS, Stage};
use rune::{Drag, Phase, text_input};

pub(super) fn view(state: &State) -> El {
    row()
        .key("create-page")
        .w_full()
        .grow(1.0)
        .min_h(0.0)
        .gap(20.0)
        .child(
            scroll()
                .key("form-scroll")
                .grow(1.0)
                .min_w(0.0)
                .gap(18.0)
                .pr(8.0)
                .child(para(state.lang.text("subtitle")))
                .child(story(state))
                .child(characters(state))
                .child(details(state))
                .child(reference(state))
                .child(el().h(12.0)),
        )
        .child(result(state).w(350.0).h_full())
}
fn story(state: &State) -> El {
    let l = state.lang;
    let mut modes = row().w_full().gap(8.0);
    for (mode, key) in [("text", "text_mode"), ("photo", "photo_mode")] {
        modes = modes.child(
            button(
                format!("mode-{mode}"),
                l.text(key),
                state.request.mode == mode,
                state.busy,
            )
            .grow(1.0)
            .min_w(0.0)
            .role("radio")
            .checked(state.request.mode == mode)
            .on_press(move |s: &mut State| {
                if !s.busy {
                    s.request.mode = mode.into();
                    s.request_quote();
                }
            }),
        );
    }
    let mut body = card().key("story-card").child(modes);
    if state.request.mode == "photo" {
        body = body.child(photos(state)).child(transformation(state));
    }
    let input = if state.busy {
        para(&state.description.text)
            .min_h(120.0)
            .padding(14.0)
            .fill(color("#FFFCF8"))
            .radius(14.0)
    } else {
        text_input(&state.description, TextStyle::new(25.0, 400.0))
            .key("description")
            .w_full()
            .min_h(130.0)
            .padding(14.0)
            .color(ink())
            .fill(color("#FFFCF8"))
            .radius(14.0)
            .border(1.0, line())
            .placeholder(l.text("placeholder"), muted())
            .caret(ink(), pink())
            .wrap_anywhere()
            .on_edit(|s: &mut State| &mut s.description)
            .on_submit(|s: &mut State, _| s.description.insert("\n"))
    };
    body = body
        .child(heading(l.text("description")))
        .child(input)
        .child(label(l.text("ideas"), 20.0).color(muted()));
    let mut ideas = row().w_full().gap(7.0);
    for id in ["picnic", "stars", "bakery"] {
        ideas=ideas.child(button(format!("idea-{id}"),l.text(id),false,state.busy).grow(1.0).min_w(0.0)
            .on_press(move|s:&mut State|if !s.busy {
                let idea=match id {
                    "picnic"=>s.lang.pick("Друзья устроили пикник под цветущей сакурой. Клубничный торт, чай и лепестки на траве.","Friends share a picnic beneath cherry blossoms, with strawberry cake, tea and petals on the grass.","友だちが桜の下でピクニック。いちごケーキとお茶、草の上に花びら。"),
                    "stars"=>s.lang.pick("Друзья сидят на холме под звёздами, укрывшись одним мягким пледом, и смотрят на падающую звезду.","Friends sit on a hill under the stars, sharing a soft blanket and watching a shooting star.","友だちが星空の丘に座り、柔らかい毛布を分け合って流れ星を見ています。"),
                    _=>s.lang.pick("Маленькая уютная пекарня. Друзья готовят булочки в форме зверят, вокруг мука и тёплый свет.","A tiny cozy bakery. Friends make animal-shaped buns in warm light, with flour on their aprons.","小さなあたたかいパン屋。友だちが動物のパンをつくり、エプロンに小麦粉がついています。"),
                };
                s.description.replace_text(idea);
            }));
    }
    body.child(ideas)
}
fn photos(state: &State) -> El {
    let l = state.lang;
    let mut body = col()
        .key("photo-drop")
        .w_full()
        .padding(14.0)
        .radius(16.0)
        .fill(color("#EDF6FA"))
        .gap(12.0)
        .accepts_extensions(&["png", "jpg", "jpeg", "webp"])
        .on_drop_files(|s: &mut State, files, _| {
            if !s.busy {
                for path in files {
                    s.add_photo(path.clone());
                }
            }
        })
        .child(
            row()
                .w_full()
                .center_y()
                .gap(12.0)
                .child(heading(l.text("photos")).grow(1.0))
                .child(label(format!("{} / 8", state.photos.len()), 20.0)),
        )
        .child(para(l.text("photo_help")));
    for (start, chunk) in state.photos.chunks(3).enumerate() {
        let mut strip = row().w_full().gap(8.0);
        for (offset, photo) in chunk.iter().enumerate() {
            let index = start * 3 + offset;
            let mut thumb = el().w_full().h(92.0).radius(10.0).fill(Color::WHITE);
            if let Some(image) = &photo.image {
                thumb = thumb.pixels_fit(image.pixels(), SurfaceFit::Fit);
            }
            strip = strip.child(
                col()
                    .key(format!("photo-{index}"))
                    .grow(1.0)
                    .min_w(0.0)
                    .gap(5.0)
                    .child(thumb)
                    .child(label(&photo.name, 18.0).w_full().truncate_fade())
                    .child(
                        button(
                            format!("remove-photo-{index}"),
                            l.text("remove"),
                            false,
                            state.busy,
                        )
                        .py(4.0)
                        .on_press(move |s: &mut State| s.remove_photo(index)),
                    ),
            );
        }
        body = body.child(strip);
    }
    body.child(add_photo_button(state))
}
fn transformation(state: &State) -> El {
    let l = state.lang;
    let mut choices = row().w_full().gap(8.0);
    for (full, key) in [(false, "classic"), (true, "creature")] {
        choices = choices.child(
            button(
                format!("transform-{key}"),
                l.text(key),
                state.request.full_creature == full,
                state.busy,
            )
            .grow(1.0)
            .min_w(0.0)
            .role("radio")
            .checked(state.request.full_creature == full)
            .on_press(move |s: &mut State| {
                if !s.busy {
                    s.request.full_creature = full;
                    s.request_quote();
                }
            }),
        );
    }
    col()
        .w_full()
        .gap(10.0)
        .child(choices)
        .child(para(l.text(if state.request.full_creature {
            "creature_help"
        } else {
            "classic_help"
        })))
        .child(strength(state))
}
fn strength(state: &State) -> El {
    let value = state.request.strength;
    let mut slider = el()
        .key("strength-slider")
        .role("slider")
        .label(state.lang.text("strength"))
        .size(220.0, 30.0)
        .range_value(value as f64, 0.0, 100.0)
        .range_step(5.0)
        .disabled(state.busy)
        .child(
            el().size(220.0, 7.0)
                .absolute(0.0, 11.5)
                .radius(3.5)
                .fill(line())
                .a11y_hidden(),
        )
        .child(
            el().size(10.0 + value * 2.0, 7.0)
                .absolute(0.0, 11.5)
                .radius(3.5)
                .fill(pink())
                .a11y_hidden(),
        )
        .child(
            el().size(20.0, 20.0)
                .absolute(value * 2.0, 5.0)
                .radius(10.0)
                .fill(Color::WHITE)
                .border(2.0, color("#D8A8B2"))
                .a11y_hidden(),
        );
    if !state.busy {
        slider = slider
            .on_step(|s: &mut State, steps, _| {
                s.request.strength = (s.request.strength + steps as f32 * 5.0).clamp(0.0, 100.0)
            })
            .on_step_limit(|s: &mut State, end, _| {
                s.request.strength = if end { 100.0 } else { 0.0 }
            })
            .on_drag(|s: &mut State, d: &mut Drag, tx| match d.phase {
                Phase::Down => {
                    d.data = s.request.strength;
                    s.request.strength = ((d.x - 10.0) / 2.0).clamp(0.0, 100.0).round();
                }
                Phase::Move => {
                    s.request.strength = ((d.x - 10.0) / 2.0).clamp(0.0, 100.0).round();
                    tx.instant();
                }
                Phase::Up => {
                    if d.cancelled() {
                        s.request.strength = d.data;
                    }
                }
            });
    }
    col()
        .w_full()
        .gap(8.0)
        .child(label(state.lang.text("strength"), 22.0))
        .child(
            row()
                .gap(12.0)
                .center_y()
                .child(slider)
                .child(label(format!("{value:.0}%"), 22.0)),
        )
}
fn characters(state: &State) -> El {
    let mut content = card()
        .key("characters")
        .child(heading(state.lang.text("characters")));
    for chunk in CHARACTER_IDS.chunks(4) {
        let mut cards = row().w_full().gap(8.0);
        for &id in chunk {
            let selected = state.request.character == id;
            cards = cards.child(
                col()
                    .key(format!("character-{id}"))
                    .role("radio")
                    .label(state.lang.text(id))
                    .checked(selected)
                    .focusable(!state.busy)
                    .disabled(state.busy)
                    .grow(1.0)
                    .min_w(0.0)
                    .padding(8.0)
                    .gap(6.0)
                    .center()
                    .radius(16.0)
                    .fill(if selected { blue() } else { color("#FFFCF8") })
                    .border(
                        if selected { 2.0 } else { 1.0 },
                        if selected { color("#9BBFD0") } else { line() },
                    )
                    .opacity(if state.busy { 0.55 } else { 1.0 })
                    .hover(color("#EAF5FA"))
                    .child(picture(state, id, 78.0, 88.0))
                    .child(
                        label(state.lang.text(id), 19.0)
                            .w_full()
                            .wrap()
                            .align(Align::Center),
                    )
                    .on_press(move |s: &mut State| {
                        if !s.busy {
                            s.request.character = id.into();
                            s.request_quote();
                        }
                    }),
            );
        }
        for _ in chunk.len()..4 {
            cards = cards.child(el().grow(1.0).min_w(0.0));
        }
        content = content.child(cards);
    }
    content
}
#[derive(Clone, Copy)]
enum OptionKind {
    Background,
    Emotion,
    Ratio,
    Resolution,
}
fn options(
    state: &State,
    key: &'static str,
    kind: OptionKind,
    values: &'static [&'static str],
) -> El {
    let selected = match kind {
        OptionKind::Background => &state.request.background,
        OptionKind::Emotion => &state.request.emotion,
        OptionKind::Ratio => &state.request.aspect_ratio,
        OptionKind::Resolution => &state.request.resolution,
    };
    let mut group = col()
        .w_full()
        .gap(8.0)
        .child(label(state.lang.text(key), 22.0));
    let chunk_size = if matches!(kind, OptionKind::Ratio | OptionKind::Resolution) {
        5
    } else {
        3
    };
    for chunk in values.chunks(chunk_size) {
        let mut choices = row().w_full().gap(6.0);
        for &value in chunk {
            let title = if matches!(kind, OptionKind::Ratio | OptionKind::Resolution) {
                value
            } else {
                state.lang.text(value)
            };
            choices = choices.child(
                button(
                    format!("{key}-{}", value.replace(':', "-")),
                    title,
                    selected == value,
                    state.busy,
                )
                .grow(1.0)
                .min_w(0.0)
                .py(8.0)
                .role("radio")
                .checked(selected == value)
                .on_press(move |s: &mut State| {
                    if !s.busy {
                        match kind {
                            OptionKind::Background => s.request.background = value.into(),
                            OptionKind::Emotion => s.request.emotion = value.into(),
                            OptionKind::Ratio => s.request.aspect_ratio = value.into(),
                            OptionKind::Resolution => s.request.resolution = value.into(),
                        }
                        if matches!(kind, OptionKind::Resolution) {
                            s.request_quote();
                        }
                    }
                }),
            );
        }
        group = group.child(choices);
    }
    group
}
fn details(state: &State) -> El {
    card()
        .key("details")
        .child(heading(state.lang.text("details")))
        .child(options(
            state,
            "background",
            OptionKind::Background,
            &["scene", "white", "meadow", "room", "sticker"],
        ))
        .child(options(
            state,
            "emotion",
            OptionKind::Emotion,
            &["scene", "happy", "shy", "tears", "surprised"],
        ))
        .child(options(
            state,
            "ratio",
            OptionKind::Ratio,
            &["1:1", "3:4", "4:3", "9:16", "16:9"],
        ))
        .child(options(
            state,
            "resolution",
            OptionKind::Resolution,
            &["1K", "2K", "4K"],
        ))
}
fn reference(state: &State) -> El {
    let mut content = card()
        .key("reference")
        .child(heading(state.lang.text("reference")));
    if let Some(photo) = &state.custom_reference {
        if let Some(image) = &photo.image {
            content = content.child(
                el().w_full()
                    .h(140.0)
                    .pixels_fit(image.pixels(), SurfaceFit::Fit)
                    .radius(12.0),
            );
        }
        content = content.child(para(&photo.name)).child(
            button(
                "reference-remove",
                state.lang.text("remove"),
                false,
                state.busy,
            )
            .on_press(|s: &mut State| s.clear_reference()),
        );
    }
    content.child(
        button(
            "reference-add",
            state.lang.text("add_reference"),
            false,
            state.busy,
        )
        .on_press_tx(|s: &mut State, tx| {
            if s.busy {
                return;
            }
            tx.open_file(
                image_dialog(s, true),
                |s: &mut State, result, _| match result {
                    Ok(Some(path)) => s.set_reference(path),
                    Ok(None) => (),
                    Err(_) => s.error = Some("image_invalid".into()),
                },
            );
        }),
    )
}
fn result(state: &State) -> El {
    let l = state.lang;
    let mut content = scroll()
        .key("result-scroll")
        .gap(14.0)
        .child(result_status(state));
    if let Some(image) = &state.result {
        content = content.child(
            el().key("result-image")
                .role("img")
                .label(l.text("result"))
                .w_full()
                .h(330.0)
                .radius(14.0)
                .pixels_fit(image.pixels(), SurfaceFit::Fit),
        );
    } else {
        content = content.child(
            col()
                .key("result-placeholder")
                .w_full()
                .py(12.0)
                .center()
                .gap(12.0)
                .child(picture(state, "trio", 238.0, 150.0))
                .child(
                    label(l.text("ready_hint"), 25.0)
                        .w_full()
                        .wrap_anywhere()
                        .align(Align::Center),
                )
                .child(para(l.text("ready_help")).align(Align::Center)),
        );
    }
    if state.result.is_some() {
        content = content.child(
            button("export", l.text("export"), false, state.busy)
                .w_full()
                .fill(blue())
                .on_press_tx(|s: &mut State, tx| {
                    if s.busy {
                        return;
                    }
                    let options = FileDialogOptions {
                        title: s.lang.text("export").into(),
                        file_name: Some("chiikawa.png".into()),
                        ..Default::default()
                    }
                    .filter("PNG", &["png"]);
                    tx.save_file(options, |s: &mut State, result, _| match result {
                        Ok(Some(path)) => s.export(path),
                        Ok(None) => (),
                        Err(_) => s.error = Some("save_failed".into()),
                    });
                }),
        );
    }
    let price = if state.quote_pending {
        l.text("price_loading").to_string()
    } else {
        state
            .quote
            .map(|cost| format!("{cost:.2} {}", l.text("credits")))
            .unwrap_or_else(|| l.text("price_retry").into())
    };
    let mut body = card()
        .key("result-panel")
        .min_h(0.0)
        .gap(14.0)
        .child(heading(l.text("result")))
        .child(content)
        .child(el().w_full().h(1.0).fill(line()))
        .child(balance(state))
        .child(
            col()
                .w_full()
                .gap(4.0)
                .child(
                    label(l.text("drawing_cost"), 18.0)
                        .w_full()
                        .wrap()
                        .align(Align::Center)
                        .color(muted()),
                )
                .child(
                    label(price, 24.0)
                        .key("price")
                        .w_full()
                        .wrap()
                        .align(Align::Center),
                ),
        );
    if !state.quote_pending && state.quote.is_none() {
        body = body.child(
            button("price-retry", l.text("price_retry"), false, state.busy)
                .w_full()
                .on_press(|s: &mut State| s.request_quote()),
        );
    }
    let disabled = state.busy || state.quote_pending || state.quote.is_none() || !state.booted;
    body.child(
        button("generate", l.text("generate"), true, disabled)
            .w_full()
            .py(17.0)
            .on_press_tx(|s: &mut State, tx| {
                if !s.busy && !s.quote_pending && s.quote.is_some() && s.booted {
                    s.generate();
                    tx.scroll_to_instant("result-scroll", 0.0, 0.0);
                }
            }),
    )
    .child(
        label(l.text("payment_note"), 18.0)
            .w_full()
            .wrap()
            .align(Align::Center)
            .color(muted()),
    )
}

fn balance(state: &State) -> El {
    let l = state.lang;
    let value = if state.balance_pending {
        l.text("balance_loading").to_string()
    } else if state.balance_error.is_some() {
        l.text("balance_error").to_string()
    } else if let Some(amount) = state.balance {
        format!("{amount:.2} {}", l.text("credits"))
    } else {
        l.text("balance_unknown").to_string()
    };
    let disabled = state.busy || state.balance_pending || !state.booted;
    row()
        .key("balance-panel")
        .w_full()
        .gap(8.0)
        .center_y()
        .padding(12.0)
        .radius(14.0)
        .fill(color("#EFF7FB"))
        .child(
            col()
                .grow(1.0)
                .min_w(0.0)
                .gap(4.0)
                .child(
                    label(l.text("balance"), 18.0)
                        .key("balance-label")
                        .w_full()
                        .wrap_anywhere()
                        .color(muted()),
                )
                .child(
                    label(value, 22.0)
                        .key("balance-value")
                        .w_full()
                        .wrap_anywhere(),
                ),
        )
        .child(
            button(
                "balance-refresh",
                l.text("balance_refresh"),
                false,
                disabled,
            )
            .px(8.0)
            .py(5.0)
            .on_press(|s: &mut State| {
                if !s.busy && !s.balance_pending && s.booted {
                    s.refresh_balance();
                }
            }),
        )
}

fn result_status(state: &State) -> El {
    let l = state.lang;
    let mut status = col()
        .key("result-status")
        .w_full()
        .gap(12.0)
        .child(alerts(state));
    if let Some(job) = &state.job {
        status = status.child(para(l.text(job.stage.key())).key("job-stage"));
        if state.busy {
            status = status.child(
                el().key("job-progress")
                    .w_full()
                    .h(7.0)
                    .radius(3.5)
                    .fill(blue()),
            );
        }
        if !state.busy && (job.stage != Stage::Completed || state.result.is_none()) {
            status = status.child(
                button("resume", l.text("resume"), false, false)
                    .w_full()
                    .on_press(|s: &mut State| s.resume()),
            );
        }
    } else if state.busy {
        status = status.child(para(l.text("preparing")).key("job-stage"));
    }
    status
}
