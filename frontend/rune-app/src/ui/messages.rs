//! Messages panel (collapsible, resizable, filterable by severity) and the status bar.

use rune::tok::{Theme, space, type_};
use rune::{El, col, icon, row, spacer, text};
use rune_kit::Kit;
use rune_kit::glyphs::{CARET_RIGHT, TRIANGLE_DOWN};

use super::splitter::Splitter;
use super::{button, choice, scroller, severity_color, severity_label};
use crate::api::dto::Severity;
use crate::state::shell::BackendStatus;
use crate::state::{AppState, layout};

const FILTERS: [(&str, Option<Severity>); 4] = [
    ("All", None),
    ("Errors", Some(Severity::Error)),
    ("Warnings", Some(Severity::Warning)),
    ("Info", Some(Severity::Info)),
];

fn header(s: &AppState, t: &Theme, k: &Kit) -> El {
    let sh = &s.shell;
    let open = sh.messages_open;
    let mut r = row()
        .key("messages-header")
        .w_full()
        .gap(space::S2)
        .px(space::S2)
        .py(space::S1)
        .items_center()
        .fill(t.control_track)
        .child(
            icon(if open { &TRIANGLE_DOWN } else { &CARET_RIGHT }, 12.0)
                .key("messages-toggle")
                .role("button")
                .label(if open {
                    "Collapse messages"
                } else {
                    "Expand messages"
                })
                .focusable(true)
                .color(t.text_secondary)
                .on_press(|s: &mut AppState| s.shell.messages_open = !s.shell.messages_open),
        )
        .child(
            text(format!("Messages ({})", sh.messages.len()))
                .font(type_::SECTION)
                .color(t.text_secondary),
        );
    if open {
        let on = FILTERS.iter().position(|(_, f)| *f == sh.severity_filter);
        r = r.child(choice(
            k,
            "message-filter",
            "Severity filter",
            &FILTERS.map(|(l, _)| l),
            on,
            |s, i| s.shell.severity_filter = FILTERS[i].1,
        ));
    }
    r = r.child(spacer());
    if sh.cancellable() {
        r = r.child(
            button(k, "cancel-operation", "Cancel", true).on_press(AppState::cancel_operation),
        );
    }
    r
}

fn list(s: &AppState, t: &Theme, k: &Kit) -> El {
    let filter = s.shell.severity_filter;
    let rows = s
        .shell
        .messages
        .iter()
        .enumerate()
        .rev()
        .filter(|(_, m)| filter.is_none_or(|f| f == m.severity))
        .map(|(index, m)| {
            let focus = m
                .component_id
                .clone()
                .map(|id| (id, m.related_component_id.clone()));
            let line = row()
                .key(format!("message-{index}"))
                .w_full()
                .gap(space::S2)
                .px(space::S2)
                .py(2.0)
                .hover(t.hover)
                .child(
                    text(severity_label(m.severity))
                        .font(type_::LABEL)
                        .color(severity_color(m.severity, t))
                        .w(52.0),
                )
                .child(
                    text(&m.text)
                        .font(type_::BODY)
                        .color(t.text)
                        .wrap()
                        .grow(1.0)
                        .min_w(0.0),
                );
            match focus {
                Some((id, related)) => line
                    .role("button")
                    .label(format!("{} {}", severity_label(m.severity), m.text))
                    .focusable(true)
                    .on_press(move |s: &mut AppState| s.focus_component(&id, related.clone())),
                None => line,
            }
        });
    let mut list = col().w_full();
    if let Some(busy) = &s.shell.busy {
        list = list.child(
            row()
                .key("progress")
                .role("status")
                .label(format!("{busy}…"))
                .w_full()
                .gap(space::S2)
                .px(space::S2)
                .py(2.0)
                .child(
                    text("RUN")
                        .font(type_::LABEL)
                        .color(t.text_secondary)
                        .w(44.0),
                )
                .child(
                    text(format!("{busy}... {} s", s.shell.elapsed_secs()))
                        .font(type_::BODY)
                        .color(t.text),
                ),
        );
    }
    let empty = if s.shell.messages.is_empty() {
        Some("No messages.")
    } else if filter.is_some() && s.shell.messages.iter().all(|m| Some(m.severity) != filter) {
        Some("No messages of this severity.")
    } else {
        None
    };
    if let (Some(msg), None) = (empty, &s.shell.busy) {
        list = list.child(
            text(msg)
                .font(type_::BODY)
                .color(t.text_secondary)
                .px(space::S2),
        );
    }
    scroller(k, "messages-scroll", list.children(rows))
}

fn height(s: &AppState) -> f32 {
    s.shell.layout.messages_height
}

pub fn messages(s: &AppState, t: &Theme, k: &Kit) -> El {
    let panel = col()
        .key("messages")
        .role("region")
        .label("Messages")
        .w_full()
        .border(1.0, t.separator);
    if !s.shell.messages_open {
        return panel.child(header(s, t, k));
    }
    panel
        .h(height(s))
        .child(
            Splitter {
                key: "split-messages",
                label: "Resize messages",
                vertical: false,
                sign: -1.0,
                get: height,
                set: |s, v| s.shell.layout.set_messages_height(v),
                reset: |s| s.shell.layout.messages_height = layout::MESSAGES_DEFAULT,
            }
            .build(t),
        )
        .child(header(s, t, k))
        .child(list(s, t, k))
}

/// A status-bar count that opens Messages filtered to that severity.
fn count(s: &AppState, t: &Theme, severity: Severity, noun: &str) -> El {
    let n = s.shell.count(severity);
    text(format!(
        "{n} {}",
        if n == 1 {
            &noun[..noun.len() - 1]
        } else {
            noun
        }
    ))
    .key(format!("status-{noun}"))
    .role("button")
    .label(format!("Show {noun} in Messages"))
    .focusable(true)
    .font(type_::LABEL)
    .color(if n > 0 {
        severity_color(severity, t)
    } else {
        t.text_secondary
    })
    .on_press(move |s: &mut AppState| {
        s.shell.messages_open = true;
        s.shell.severity_filter = Some(severity);
    })
}

pub fn status_bar(s: &AppState, t: &Theme) -> El {
    let backend = match &s.shell.backend {
        BackendStatus::Connecting => "Geometry service: connecting".to_string(),
        BackendStatus::Online {
            service,
            api_version,
        } => format!("Geometry service: {service} {api_version}"),
        BackendStatus::Offline(reason) => format!("Geometry service: offline ({reason})"),
    };
    let backend_color = if matches!(s.shell.backend, BackendStatus::Offline(_)) {
        severity_color(Severity::Error, t)
    } else {
        t.text_secondary
    };
    let model = s
        .model
        .imported
        .as_ref()
        .map_or("No model".to_string(), |i| {
            let unit = i.source_unit.as_deref().unwrap_or("unknown units");
            format!("{} bodies, source {unit}", i.component_count)
        });
    row()
        .key("status")
        .role("status")
        .label("Status")
        .w_full()
        .gap(space::S3)
        .px(space::S2)
        .py(space::S1)
        .child(text(backend).font(type_::LABEL).color(backend_color))
        .child(text(model).font(type_::LABEL).color(t.text_secondary))
        .child(
            text(format!(
                "Units: {}",
                s.shell.number_format.length_unit.symbol()
            ))
            .font(type_::LABEL)
            .color(t.text_secondary),
        )
        .child(spacer())
        .child(
            text(s.shell.busy.as_deref().map_or_else(String::new, |b| {
                format!("{b}... {} s", s.shell.elapsed_secs())
            }))
            .font(type_::LABEL)
            .color(t.text),
        )
        .child(
            text(
                s.section
                    .stats
                    .map_or_else(String::new, |st| format!("Regions {}", st.regions)),
            )
            .font(type_::LABEL)
            .color(t.text_secondary),
        )
        .child(count(s, t, Severity::Error, "errors"))
        .child(count(s, t, Severity::Warning, "warnings"))
}
