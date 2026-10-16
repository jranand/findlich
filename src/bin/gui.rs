use adw::prelude::*;
use findlich::{parse::parse, search::search};
use gtk::{gio, glib};
use std::path::Path;

const APP_ID: &str = "dev.sinezero.CmdCenter";
const MAX_RESULTS: usize = 50;

const CSS: &str = "
entry.launcher-search {
    font-size: 1.35em;
    min-height: 52px;
    background: none;
    border: none;
    box-shadow: none;
    outline: none;
}
.launcher-list {
    background: none;
}
.launcher-footer {
    font-size: 0.8em;
}
";

fn main() -> glib::ExitCode {
    let app = adw::Application::builder().application_id(APP_ID).build();
    app.connect_startup(|_| load_css());
    app.connect_activate(build_ui);
    app.run()
}

fn load_css() {
    let provider = gtk::CssProvider::new();
    provider.load_from_data(CSS);
    gtk::style_context_add_provider_for_display(
        &gtk::gdk::Display::default().expect("no display"),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

/// "/home/jranand/Downloads/x.pdf" -> "~/Downloads"
fn pretty_folder(path: &Path) -> String {
    let folder = path.parent().unwrap_or(path);
    if let Some(home) = dirs::home_dir() {
        if let Ok(rest) = folder.strip_prefix(&home) {
            return if rest.as_os_str().is_empty() {
                "~".to_string()
            } else {
                format!("~/{}", rest.display())
            };
        }
    }
    folder.display().to_string()
}

fn file_row(path: &Path) -> adw::ActionRow {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    // Ask the system what kind of file this is, and use its standard icon.
    let (content_type, _) = gio::content_type_guess(Some(path), None::<&[u8]>);
    let icon = gtk::Image::builder()
        .gicon(&gio::content_type_get_icon(&content_type))
        .pixel_size(32)
        .build();

    let row = adw::ActionRow::builder()
        .title(glib::markup_escape_text(&name))
        .subtitle(glib::markup_escape_text(&pretty_folder(path)))
        .title_lines(1)
        .subtitle_lines(1)
        .build();
    row.add_prefix(&icon);
    row
}

fn build_ui(app: &adw::Application) {
    // --- Search field ---
    let entry = gtk::SearchEntry::builder()
        .placeholder_text("Search files…")
        .css_classes(["launcher-search"])
        .margin_start(12)
        .margin_end(12)
        .build();

    // --- Results, or a message when there are none ---
    let list = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::Single)
        .css_classes(["navigation-sidebar", "launcher-list"])
        .build();
    let scroller = gtk::ScrolledWindow::builder()
        .child(&list)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .build();

    let empty = adw::StatusPage::builder()
        .icon_name("system-search-symbolic")
        .title("Search your files")
        .description("Try “pdf last week” or “photos yesterday”")
        .build();

    let stack = gtk::Stack::new();
    stack.add_named(&scroller, Some("results"));
    stack.add_named(&empty, Some("empty"));
    stack.set_visible_child_name("empty");

    // --- Footer: count + the real command (Show specs) ---
    let count = gtk::Label::builder()
        .css_classes(["dim-label", "launcher-footer"])
        .build();
    let command = gtk::Label::builder()
        .xalign(0.0)
        .hexpand(true)
        .ellipsize(gtk::pango::EllipsizeMode::Middle)
        .css_classes(["dim-label", "monospace", "launcher-footer"])
        .build();
    let footer = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(12)
        .margin_top(8)
        .margin_bottom(8)
        .margin_start(16)
        .margin_end(16)
        .build();
    footer.append(&count);
    footer.append(&command);

    // --- Layout ---
    let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
    content.append(&entry);
    content.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    content.append(&stack);
    content.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    content.append(&footer);

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("cmdcenter")
        .default_width(680)
        .default_height(480)
        .resizable(false)
        .content(&content)
        .build();

    // --- Enter: search ---
    {
        let list = list.clone();
        let stack = stack.clone();
        let empty = empty.clone();
        let count = count.clone();
        let command = command.clone();

        entry.connect_activate(move |entry| {
            let q = parse(&entry.text());
            let cmd = q.to_find_command();
            command.set_label(&cmd);
            command.set_tooltip_text(Some(&cmd)); // full command on hover

            while let Some(child) = list.first_child() {
                list.remove(&child);
            }

            let results = search(&q);
            for path in results.iter().take(MAX_RESULTS) {
                list.append(&file_row(path));
            }

            count.set_label(&match results.len() {
                0 => "No files".to_string(),
                1 => "1 file".to_string(),
                n if n > MAX_RESULTS => format!("{MAX_RESULTS} of {n} files"),
                n => format!("{n} files"),
            });

            if results.is_empty() {
                empty.set_title("No files found");
                empty.set_description(Some("Try fewer words, or add “everywhere”"));
                stack.set_visible_child_name("empty");
            } else {
                stack.set_visible_child_name("results");
                list.select_row(list.row_at_index(0).as_ref());
            }
        });
    }

    // --- Esc: close ---
    {
        let window = window.clone();
        entry.connect_stop_search(move |_| window.close());
    }

    window.present();
}
