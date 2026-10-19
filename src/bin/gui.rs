use adw::prelude::*;
use findlich::{index::Index, parse::parse};
use gtk::{gio, glib};
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

const APP_ID: &str = "io.github.jranand.Findlich";
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
    if let Some(home) = dirs::home_dir()
        && let Ok(rest) = folder.strip_prefix(&home)
    {
        return if rest.as_os_str().is_empty() {
            "~".to_string()
        } else {
            format!("~/{}", rest.display())
        };
    }
    folder.display().to_string()
}

fn file_row(path: &Path) -> adw::ActionRow {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

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
        .activatable(true)
        .build();
    row.add_prefix(&icon);
    row
}

/// The widgets that change while searching, bundled so closures need one clone.
#[derive(Clone)]
struct Ui {
    list: gtk::ListBox,
    stack: gtk::Stack,
    status: adw::StatusPage,
    count: gtk::Label,
    command: gtk::Label,
    paths: Rc<RefCell<Vec<PathBuf>>>,
}

fn show_status(ui: &Ui, title: &str, description: &str) {
    ui.status.set_title(title);
    ui.status.set_description(Some(description));
    ui.stack.set_visible_child_name("status");
}

fn run_search(ui: &Ui, index: &Option<Index>, text: &str) {
    while let Some(child) = ui.list.first_child() {
        ui.list.remove(&child);
    }
    ui.paths.borrow_mut().clear();
    ui.count.set_label("");
    ui.command.set_label("");

    let Some(index) = index else {
        show_status(ui, "Indexing your files…", "This only takes a moment");
        return;
    };
    if text.trim().is_empty() {
        show_status(
            ui,
            "Search your files",
            "Try “pdf last week” or “photos yesterday”",
        );
        return;
    }

    let q = parse(text);
    let plan = q.resolve();
    let cmd = q.to_find_command(&plan);
    ui.command.set_label(&cmd);
    ui.command.set_tooltip_text(Some(&cmd));

    let result = index.search(&q, &plan, MAX_RESULTS);
    let mut paths = ui.paths.borrow_mut();
    for f in &result.matches {
        ui.list.append(&file_row(&f.path));
        paths.push(f.path.clone());
    }

    ui.count.set_label(&match result.total {
        0 => "No files".to_string(),
        1 => "1 file".to_string(),
        n if n > MAX_RESULTS => format!("{MAX_RESULTS} of {n} files"),
        n => format!("{n} files"),
    });

    if result.total == 0 {
        show_status(ui, "No files found", "Try fewer words, or add “everywhere”");
    } else {
        ui.stack.set_visible_child_name("results");
        ui.list.select_row(ui.list.row_at_index(0).as_ref());
    }
}

/// Open a file with the desktop's default app. Never executes it.
fn open_path(path: &Path, ctx: &gtk::gdk::AppLaunchContext) {
    let uri = gio::File::for_path(path).uri();
    if let Err(err) = gio::AppInfo::launch_default_for_uri(&uri, Some(ctx)) {
        eprintln!("findlich: could not open {}: {err}", path.display());
    }
}

fn build_ui(app: &adw::Application) {
    // --- Widgets ---
    let entry = gtk::SearchEntry::builder()
        .placeholder_text("Search files…")
        .css_classes(["launcher-search"])
        .margin_start(12)
        .margin_end(12)
        .build();

    let list = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::Single)
        .css_classes(["navigation-sidebar", "launcher-list"])
        .build();
    let scroller = gtk::ScrolledWindow::builder()
        .child(&list)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .build();

    let status = adw::StatusPage::builder()
        .icon_name("system-search-symbolic")
        .build();

    let stack = gtk::Stack::new();
    stack.add_named(&scroller, Some("results"));
    stack.add_named(&status, Some("status"));

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
        .title("Findlich")
        .default_width(680)
        .default_height(480)
        .resizable(false)
        .content(&content)
        .build();

    // --- State ---
    let ui = Ui {
        list,
        stack,
        status,
        count,
        command,
        paths: Rc::new(RefCell::new(Vec::new())),
    };
    let index: Rc<RefCell<Option<Index>>> = Rc::new(RefCell::new(None));
    run_search(&ui, &index.borrow(), ""); // shows "Indexing your files…"

    // --- Build the index in the background ---
    {
        let ui = ui.clone();
        let index = index.clone();
        let entry = entry.clone();
        glib::spawn_future_local(async move {
            let built = gio::spawn_blocking(Index::build)
                .await
                .expect("indexing thread panicked");
            *index.borrow_mut() = Some(built);
            // Whatever the user typed while we were indexing, search it now.
            run_search(&ui, &index.borrow(), &entry.text());
        });
    }

    // --- Search as you type ---
    {
        let ui = ui.clone();
        let index = index.clone();
        entry.connect_search_changed(move |entry| {
            run_search(&ui, &index.borrow(), &entry.text());
        });
    }

    {
        let paths = ui.paths.clone();
        ui.list.connect_row_activated(move |_, row| {
            let Ok(i) = usize::try_from(row.index()) else {
                return;
            };
            if let Some(path) = paths.borrow().get(i) {
                open_path(path, &row.display().app_launch_context());
            }
        });
    }
    // --- Esc: close ---
    {
        let window = window.clone();
        entry.connect_stop_search(glib::clone!(
            #[weak]
            window,
            move |_| window.close()
        ));
    }

    window.present();
}
