use rune::tok::{self, space, type_};
use rune::{AppSpec, Cx, El, col, row, text};

struct AppState;

fn panel(key: &'static str, title: &str, detail: &str) -> El {
    let theme = tok::theme(true);
    col()
        .key(key)
        .role("region")
        .label(title)
        .size(250.0, 660.0)
        .padding(space::S3)
        .gap(space::S2)
        .fill(theme.card)
        .child(text(title).font(type_::TITLE).color(theme.text))
        .child(text(detail).font(type_::BODY).color(theme.text_secondary))
}

fn view(_: &AppState, _: &mut Cx<AppState>) -> El {
    let theme = tok::theme(true);
    col()
        .key("root")
        .w_full()
        .h_full()
        .padding(space::S3)
        .gap(space::S2)
        .fill(theme.bg)
        .child(
            row()
                .key("toolbar")
                .w_full()
                .gap(space::S2)
                .child(text("CAD2Maxwell").font(type_::TITLE).color(theme.text))
                .child(text("Rune UI evaluation").color(theme.text_secondary)),
        )
        .child(
            row()
                .key("workspace")
                .w_full()
                .gap(space::S2)
                .child(panel(
                    "model-tree",
                    "Model Builder",
                    "Imported components and visibility",
                ))
                .child(panel(
                    "section-viewport",
                    "Section viewport",
                    "Display-only section rendering spike",
                ))
                .child(panel(
                    "properties",
                    "Properties",
                    "Selection and diagnostics",
                )),
        )
}

fn main() {
    rune::run(AppSpec::new(
        "CAD2Maxwell - Rune UI spike",
        (1280.0, 760.0),
        || AppState,
        view,
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use rune::{Env, Runtime};

    #[test]
    fn engineering_workspace_panels_are_present_in_the_native_view_tree() {
        let mut runtime = Runtime::new(
            AppState,
            view,
            (1280.0, 760.0),
            Env {
                system_dark: true,
                scale: 1.0,
            },
        );
        runtime.frame(0.0);

        assert!(runtime.find("toolbar").is_some());
        assert!(runtime.find("model-tree").is_some());
        assert!(runtime.find("section-viewport").is_some());
        assert!(runtime.find("properties").is_some());
    }
}
