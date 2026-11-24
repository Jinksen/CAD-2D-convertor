//! Small monochrome tree icons (16-unit grid, stroked in the text colour).

use rune::Icon;

pub static ASSEMBLY: Icon = Icon {
    name: "c2m-assembly",
    view: 16.0,
    body: r#"<rect x="5.5" y="1.5" width="5" height="5" stroke="currentColor"/><rect x="1.5" y="9.5" width="5" height="5" stroke="currentColor"/><rect x="9.5" y="9.5" width="5" height="5" stroke="currentColor"/>"#,
};
pub static BODY: Icon = Icon {
    name: "c2m-body",
    view: 16.0,
    body: r#"<path d="M8 1.5 14 4.75v6.5L8 14.5 2 11.25v-6.5Z M2 4.75 8 8l6-3.25M8 8v6.5" stroke="currentColor" stroke-linejoin="round"/>"#,
};
pub static GROUP: Icon = Icon {
    name: "c2m-group",
    view: 16.0,
    body: r#"<rect x="1.5" y="1.5" width="13" height="13" rx="1.5" stroke="currentColor"/><path d="M5 5.5h6M5 8h6M5 10.5h6" stroke="currentColor" stroke-linecap="round"/>"#,
};
pub static SECTION: Icon = Icon {
    name: "c2m-section",
    view: 16.0,
    body: r#"<path d="M1.5 11.5 4.5 5.5h10l-3 6Z" stroke="currentColor" stroke-linejoin="round"/><path d="M8 1.5v4" stroke="currentColor" stroke-linecap="round"/>"#,
};
pub static CARET_LEFT: Icon = Icon {
    name: "c2m-caret-left",
    view: 16.0,
    body: r#"<path d="M10 4L6 8L10 12V4Z" fill="currentColor"/>"#,
};
