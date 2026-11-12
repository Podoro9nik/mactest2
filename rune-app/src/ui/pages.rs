use super::*;
use rune::text_input;

pub(super) fn history(state: &State) -> El {
    let l = state.lang;
    let mut content = scroll()
        .key("history-page")
        .gap(15.0)
        .child(para(l.text("history_help")));
    if state.history.is_empty() {
        content = content.child(
            card()
                .center()
                .padding(32.0)
                .gap(24.0)
                .child(picture(state, "hachiware", 140.0, 160.0))
                .child(para(l.text("empty_history")).align(Align::Center))
                .child(
                    button("history-create", l.text("create"), true, false)
                        .on_press(|s: &mut State| s.page = Page::Create),
                ),
        );
    }
    for job in &state.history {
        let id = job.id.clone();
        let title = if job.description.is_empty() {
            l.text("result")
        } else {
            &job.description
        };
        content = content.child(
            card()
                .key(format!("history-{}", job.id))
                .gap(10.0)
                .child(
                    row()
                        .w_full()
                        .center_y()
                        .gap(15.0)
                        .child(label(title, 26.0).grow(1.0).min_w(0.0).wrap())
                        .child(
                            button(
                                format!("history-open-{}", job.id),
                                l.text("open"),
                                false,
                                state.busy,
                            )
                            .on_press(move |s: &mut State| s.load_job(id.clone())),
                        ),
                )
                .child(para(format!(
                    "{}  ·  {}",
                    job.created_at,
                    l.text(job.stage.key())
                )))
                .child(
                    label(
                        format!("ID: {}", job.remote_id.as_deref().unwrap_or(&job.id)),
                        18.0,
                    )
                    .w_full()
                    .wrap()
                    .color(muted()),
                ),
        );
    }
    content.child(el().h(20.0))
}
pub(super) fn settings(state: &State) -> El {
    let l = state.lang;
    let connection = if state.quote_pending || !state.booted {
        "checking"
    } else if state.quote.is_some() {
        "connected"
    } else {
        "price_retry"
    };
    let input = if state.busy {
        para(&state.restore_id.text)
    } else {
        text_input(&state.restore_id, TextStyle::new(24.0, 400.0))
            .key("restore-id")
            .w_full()
            .single_line()
            .padding(14.0)
            .radius(14.0)
            .fill(color("#FFFCF8"))
            .border(1.0, line())
            .color(ink())
            .caret(ink(), pink())
            .placeholder("ID", muted())
            .on_edit(|s: &mut State| &mut s.restore_id)
    };
    scroll()
        .key("settings-page")
        .gap(18.0)
        .child(
            card()
                .child(heading(l.text("language")))
                .child(languages(state, "settings-language")),
        )
        .child(
            card()
                .child(heading(l.text("connection")))
                .child(
                    row()
                        .w_full()
                        .center_y()
                        .gap(15.0)
                        .child(para(l.text(connection)).grow(1.0))
                        .child(
                            button(
                                "connection-check",
                                l.text("check"),
                                false,
                                state.busy || state.quote_pending,
                            )
                            .on_press(|s: &mut State| {
                                if !s.busy && !s.quote_pending {
                                    s.request_quote();
                                }
                            }),
                        ),
                )
                .child(para(l.text("all_local"))),
        )
        .child(
            card()
                .key("restore-card")
                .child(heading(l.text("restore")))
                .child(para(l.text("restore_help")))
                .child(input)
                .child(
                    button("restore-button", l.text("restore_button"), true, state.busy).on_press(
                        |s: &mut State| {
                            s.restore();
                            s.page = Page::Create;
                        },
                    ),
                ),
        )
        .child(alerts(state))
        .child(
            card()
                .child(heading(&format!(
                    "{} {}",
                    l.text("version"),
                    env!("CARGO_PKG_VERSION")
                )))
                .child(para(l.text("font_note")))
                .child(para(l.text("credits_note"))),
        )
        .child(el().h(20.0))
}
