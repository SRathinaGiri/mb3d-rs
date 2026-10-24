//! The common dialogs: message boxes, file open / save, colour, input
//! query.  They are forms of the toolkit (built from form file text) and
//! report their result as a `DialogResult` event with the caller's tag.

use super::canvas::line_height;
use super::control::{Column, Kind};
use super::font::Font;
use super::form::*;
use super::lang::tr;
use super::input::Raw;
use super::ui::Ui;
use std::cmp::Ordering;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) enum DialogState {
    Message { tag: String },
    File(FileDlg),
    Color { tag: String },
    Input { tag: String },
}

pub(crate) struct FileDlg {
    tag: String,
    save: bool,
    multi: bool,
    dir: PathBuf,
    filters: Vec<(String, Vec<String>)>,
    default_ext: String,
    /// choose a folder instead of a file
    folder: bool,
    /// List entries, with the metadata needed by the file-sort choices.
    entries: Vec<FileEntry>,
    /// a preview column: a clicked file goes to `Ui::file_preview`
    preview: bool,
    /// Column and direction of the report-style file list.
    sort_column: usize,
    sort_desc: bool,
    /// Enter in DirEdit was handled there (Delphi's Key := #0): the default
    /// button's click that follows it is dropped
    eat_ok: bool,
}

#[derive(Clone)]
struct FileEntry {
    is_dir: bool,
    name: String,
    modified: Option<SystemTime>,
    size: u64,
}

fn name_cmp(a: &FileEntry, b: &FileEntry) -> Ordering {
    a.name.to_ascii_lowercase().cmp(&b.name.to_ascii_lowercase())
}

fn type_cmp(a: &FileEntry, b: &FileEntry) -> Ordering {
    let ext = |e: &FileEntry| Path::new(&e.name).extension().map(|s| s.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
    ext(a).cmp(&ext(b)).then_with(|| name_cmp(a, b))
}

/// Sort a single directory's entries.  Directories themselves are kept
/// before files, as in the native Windows picker.
fn entry_cmp(a: &FileEntry, b: &FileEntry, column: usize, desc: bool) -> Ordering {
    let order = match column {
        1 => a.modified.cmp(&b.modified).then_with(|| name_cmp(a, b)),
        2 => type_cmp(a, b),
        3 => a.size.cmp(&b.size).then_with(|| name_cmp(a, b)),
        _ => name_cmp(a, b),
    };
    if desc { order.reverse() } else { order }
}

fn entry_type(e: &FileEntry) -> String {
    if e.is_dir {
        "Folder".into()
    } else {
        Path::new(&e.name).extension().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "File".into())
    }
}

fn entry_size(e: &FileEntry) -> String {
    if e.is_dir {
        return String::new();
    }
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut size = e.size as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit + 1 < UNITS.len() {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 { format!("{} B", e.size) } else { format!("{size:.1} {}", UNITS[unit]) }
}

/// UTC civil date without a platform-specific time library.
fn entry_modified(e: &FileEntry) -> String {
    let Some(seconds) = e.modified.and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_secs() as i64) else { return String::new() };
    let days = seconds.div_euclid(86_400);
    let time = seconds.rem_euclid(86_400);
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    year += if month <= 2 { 1 } else { 0 };
    format!("{year:04}-{month:02}-{day:02} {:02}:{:02}", time / 3_600, time % 3_600 / 60)
}

fn file_columns(c: &mut super::control::Control, sort_column: usize, sort_desc: bool) {
    let labels = [tr("Name"), tr("Date modified"), tr("Type"), tr("Size")];
    c.cols = labels.iter().enumerate().map(|(i, label)| Column {
        caption: if i == sort_column { format!("{label} {}", if sort_desc { "▼" } else { "▲" }) } else { label.clone() },
        width: [50, 145, 90, 85][i],
        auto: i == 0,
    }).collect();
}

/// Options of a file dialog (TOpenDialog / TSaveDialog properties).
#[derive(Clone, Debug, Default)]
pub struct FileOptions {
    pub title: String,
    /// VCL filter: "Parameter files|*.m3p;*.m3i|All files|*.*"
    pub filter: String,
    pub filter_index: usize,
    pub initial_dir: Option<PathBuf>,
    pub file_name: String,
    pub default_ext: String,
    pub multi: bool,
    /// choose a folder (the folder shown when "Select" is pressed)
    pub folder: bool,
    /// a preview column on the right (`PreviewImage`, `PreviewLabel`): a
    /// clicked file is put into `Ui::file_preview` for the application
    pub preview: bool,
}

fn esc(s: &str) -> String {
    let mut o = String::from("'");
    for ch in s.chars() {
        match ch {
            '\'' => o.push_str("''"),
            '\r' => {}
            '\n' => o.push_str("'#13#10'"),
            c => o.push(c),
        }
    }
    o.push('\'');
    o
}

fn button(name: &str, caption: &str, x: i32, y: i32, w: i32, mr: i32, default: bool, cancel: bool) -> String {
    format!(
        "  object {name}: TButton\n    Left = {x}\n    Top = {y}\n    Width = {w}\n    Height = 25\n    Caption = {}\n    ModalResult = {mr}\n    Default = {}\n    Cancel = {}\n    TabOrder = 0\n  end\n",
        esc(caption),
        if default { "True" } else { "False" },
        if cancel { "True" } else { "False" }
    )
}

const FORM_HEAD: &str = "  BorderStyle = bsDialog\n  Color = clBtnFace\n  Font.Charset = DEFAULT_CHARSET\n  Font.Color = clWindowText\n  Font.Height = -11\n  Font.Name = 'MS Sans Serif'\n  Font.Style = []\n  Position = poOwnerFormCenter\n";

fn mr_caption(mr: i32) -> String {
    match mr {
        MR_CANCEL => tr("Cancel"),
        MR_ABORT => tr("Abort"),
        MR_RETRY => tr("Retry"),
        MR_IGNORE => tr("Ignore"),
        MR_YES => tr("Yes"),
        MR_NO => tr("No"),
        _ => tr("OK"),
    }
}

fn parse_filter(f: &str) -> Vec<(String, Vec<String>)> {
    let parts: Vec<&str> = f.split('|').collect();
    let mut out = Vec::new();
    for ch in parts.chunks(2) {
        if ch.len() == 2 {
            let pats = ch[1].split(';').map(|p| p.trim().to_ascii_lowercase()).filter(|p| !p.is_empty()).collect();
            out.push((ch[0].to_string(), pats));
        }
    }
    if out.is_empty() {
        out.push((tr("All files (*.*)"), vec!["*.*".into()]));
    }
    out
}

fn matches(pats: &[String], name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    pats.iter().any(|p| {
        if p == "*.*" || p == "*" {
            return true;
        }
        if let Some(ext) = p.strip_prefix("*.") {
            return n.ends_with(&format!(".{ext}"));
        }
        if let Some((a, b)) = p.split_once('*') {
            return n.starts_with(a) && n.ends_with(b);
        }
        n == *p
    })
}

impl Ui {
    fn dialog_form(&mut self, name: &str, dfm: &str) -> usize {
        let mut f = Form::from_dfm(dfm, &self.theme).expect("dialog form");
        // one still open (a confirm not answered yet) is kept: the new one
        // opens over it as "MessageBoxDlg1" ..., both report their results
        for n in 0.. {
            let nm = if n == 0 { name.to_string() } else { format!("{name}{n}") };
            match self.form_index(&nm) {
                Some(i) if self.forms[i].visible => {}
                Some(i) => {
                    f.name = nm;
                    self.forms[i] = f;
                    return i;
                }
                None => {
                    f.name = nm;
                    return self.add_built(f);
                }
            }
        }
        unreachable!()
    }

    fn center_on_main(&mut self, i: usize) {
        let m = &self.forms[self.main_form];
        let (mw, mh) = m.client_size();
        let (ml, mt) = (m.left, m.top);
        let (w, h) = self.forms[i].client_size();
        self.forms[i].left = ml + (mw - w) / 2;
        self.forms[i].top = mt + (mh - h) / 2;
    }

    /// `MessageDlg`: a message with buttons (mrOk, mrYes, ...); the result
    /// comes back as `DialogResult::Button` with `tag` as sender.
    pub fn message_box(&mut self, tag: &str, caption: &str, text: &str, buttons: &[i32]) {
        let font = Font::default();
        let measure = |s: &str| super::font::text_width(font.face(), font.em(), s).ceil() as i32;
        let lines: Vec<&str> = text.lines().collect();
        let tw = lines.iter().map(|l| measure(l)).max().unwrap_or(0).min(560);
        let lh = line_height(&font);
        // wrapped line count estimate
        let n: i32 = lines.iter().map(|l| (measure(l) / 560 + 1).max(1)).sum::<i32>().max(1);
        let bw = 75;
        let btn_total = buttons.len() as i32 * (bw + 8) - 8;
        let w = (tw + 48).max(btn_total + 32).max(220);
        let h = n * lh + 24 + 25 + 22;
        let mut d = format!("object MessageBoxDlg: TForm\n  Caption = {}\n  ClientWidth = {w}\n  ClientHeight = {h}\n{FORM_HEAD}", esc(caption));
        d.push_str(&format!(
            "  object MsgLabel: TLabel\n    Left = 24\n    Top = 16\n    Width = {}\n    Height = {}\n    AutoSize = False\n    WordWrap = True\n    Caption = {}\n  end\n",
            w - 40,
            n * lh,
            esc(text)
        ));
        let mut x = (w - btn_total) / 2;
        for (k, &mr) in buttons.iter().enumerate() {
            let default = k == 0;
            let cancel = mr == MR_CANCEL || mr == MR_NO || buttons.len() == 1;
            d.push_str(&button(&format!("Btn{k}"), &mr_caption(mr), x, h - 25 - 12, bw, mr, default, cancel));
            x += bw + 8;
        }
        d.push_str("end\n");
        let i = self.dialog_form("MessageBoxDlg", &d);
        self.dialogs.insert(i, DialogState::Message { tag: tag.to_string() });
        self.center_on_main(i);
        self.forms[i].modal = true;
        let first = self.forms[i].id("Btn0");
        self.forms[i].focus = first;
        self.show_index(i);
        self.forms[i].modal = true;
    }

    /// `ShowMessage`.
    pub fn show_message(&mut self, text: &str) {
        let cap = self.forms.get(self.main_form).map(|f| f.caption().to_string()).unwrap_or_else(|| "Mandelbulb3D".into());
        self.message_box("", &cap, text, &[MR_OK]);
    }

    /// Asks yes / no; the result comes as `DialogResult::Button(MR_YES / MR_NO)`.
    pub fn confirm(&mut self, tag: &str, text: &str) {
        self.message_box(tag, "Confirm", text, &[MR_YES, MR_NO]);
    }

    /// `InputQuery`: the result comes as `DialogResult::Text`.
    pub fn input_query(&mut self, tag: &str, caption: &str, prompt: &str, value: &str) {
        let d = format!(
            "object InputQueryDlg: TForm\n  Caption = {}\n  ClientWidth = 300\n  ClientHeight = 104\n{FORM_HEAD}  object PromptLabel: TLabel\n    Left = 12\n    Top = 12\n    Width = 276\n    Height = 13\n    AutoSize = False\n    Caption = {}\n  end\n  object ValueEdit: TEdit\n    Left = 12\n    Top = 32\n    Width = 276\n    Height = 21\n    TabOrder = 0\n    Text = {}\n  end\n{}{}end\n",
            esc(caption),
            esc(prompt),
            esc(value),
            button("OkBtn", &tr("OK"), 132, 68, 75, MR_OK, true, false),
            button("CancelBtn", &tr("Cancel"), 213, 68, 75, MR_CANCEL, false, true)
        );
        let i = self.dialog_form("InputQueryDlg", &d);
        self.dialogs.insert(i, DialogState::Input { tag: tag.to_string() });
        self.center_on_main(i);
        let e = self.forms[i].id("ValueEdit");
        self.forms[i].focus = e;
        if let Some(e) = e {
            let c = &mut self.forms[i].ctl[e];
            c.sel_anchor = 0;
            c.caret = c.text.chars().count();
        }
        self.forms[i].modal = true;
        self.show_index(i);
        self.forms[i].modal = true;
    }

    /// TColorDialog: the result comes as `DialogResult::Color`.
    pub fn pick_color(&mut self, tag: &str, color: u32) {
        let mut d = format!("object ColorDlg: TForm\n  Caption = {}\n  ClientWidth = 300\n  ClientHeight = 290\n{FORM_HEAD}", esc(&tr("Color")));
        d.push_str(&format!("  object BasicLabel: TLabel\n    Left = 8\n    Top = 6\n    Width = 70\n    Height = 13\n    Caption = {}\n  end\n", esc(&tr("&Basic colors:"))));
        for (k, c) in BASIC.iter().enumerate() {
            let (x, y) = (8 + (k % 8) as i32 * 36, 24 + (k / 8) as i32 * 24);
            d.push_str(&format!(
                "  object Swatch{k}: TShape\n    Tag = {k}\n    Left = {x}\n    Top = {y}\n    Width = 30\n    Height = 19\n    Brush.Color = ${:06X}\n    Pen.Color = clGray\n    OnMouseDown = SwatchDown\n  end\n",
                (c & 0xFF) << 16 | (c & 0xFF00) | (c >> 16 & 0xFF)
            ));
        }
        for (k, name) in ["Red", "Green", "Blue"].iter().enumerate() {
            let y = 178 + k as i32 * 26;
            d.push_str(&format!(
                "  object L{name}: TLabel\n    Left = 8\n    Top = {}\n    Width = 30\n    Height = 13\n    Caption = '{name}:'\n  end\n  object T{name}: TTrackBar\n    Left = 46\n    Top = {y}\n    Width = 170\n    Height = 24\n    Max = 255\n    Frequency = 32\n    TickStyle = tsNone\n    TabOrder = {k}\n    OnChange = TrackChange\n  end\n  object E{name}: TEdit\n    Left = 222\n    Top = {}\n    Width = 36\n    Height = 21\n    TabOrder = {}\n    Text = '0'\n    OnChange = EditChange\n  end\n",
                y + 4,
                y + 1,
                k + 3
            ));
        }
        d.push_str("  object Preview: TShape\n    Left = 264\n    Top = 178\n    Width = 28\n    Height = 72\n    Pen.Color = clGray\n  end\n");
        d.push_str(&button("OkBtn", &tr("OK"), 136, 258, 75, MR_OK, true, false));
        d.push_str(&button("CancelBtn", &tr("Cancel"), 217, 258, 75, MR_CANCEL, false, true));
        d.push_str("end\n");
        let i = self.dialog_form("ColorDlg", &d);
        self.dialogs.insert(i, DialogState::Color { tag: tag.to_string() });
        self.set_dialog_color(i, color);
        self.center_on_main(i);
        self.forms[i].modal = true;
        self.show_index(i);
        self.forms[i].modal = true;
    }

    fn set_dialog_color(&mut self, i: usize, color: u32) {
        let f = &mut self.forms[i];
        let ch = [(color >> 16) & 255, (color >> 8) & 255, color & 255];
        for (k, name) in ["Red", "Green", "Blue"].iter().enumerate() {
            f.c_mut(&format!("T{name}")).position = ch[k] as i64;
            f.c_mut(&format!("E{name}")).set_text(&ch[k].to_string());
        }
        f.c_mut("Preview").brush_color = 0xFF00_0000 | (color & 0xFF_FFFF);
    }

    /// TOpenDialog: the result comes as `DialogResult::File` (or `Files`
    /// with `multi`).
    pub fn open_dialog(&mut self, tag: &str, o: &FileOptions) {
        self.file_dialog(tag, o, false);
    }

    /// A folder chooser (`SelectDirectory`): the result comes as
    /// `DialogResult::File` with the folder.
    pub fn folder_dialog(&mut self, tag: &str, title: &str, initial: Option<PathBuf>) {
        let o = FileOptions { title: title.into(), filter: "Folders|*.*".into(), initial_dir: initial, folder: true, ..Default::default() };
        self.file_dialog(tag, &o, false);
    }

    /// TSaveDialog: the result comes as `DialogResult::File`.
    pub fn save_dialog(&mut self, tag: &str, o: &FileOptions) {
        self.file_dialog(tag, o, true);
    }

    fn file_dialog(&mut self, tag: &str, o: &FileOptions, save: bool) {
        let title = if o.title.is_empty() { if save { tr("Save As") } else { tr("Open") } } else { o.title.clone() };
        let filters = parse_filter(&o.filter);
        let cw = if o.preview { 812 } else { 560 };
        let mut d = format!("object FileDlg: TForm\n  Caption = {}\n  ClientWidth = {cw}\n  ClientHeight = 390\n{FORM_HEAD}", esc(&title));
        d = d.replace("bsDialog", "bsSizeable");
        d.push_str(&format!(
            "  object LookLabel: TLabel\n    Left = 8\n    Top = 12\n    Width = 40\n    Height = 13\n    Caption = {}\n  end\n\
             \x20 object DirEdit: TEdit\n    Left = 62\n    Top = 8\n    Width = 410\n    Height = 21\n    Anchors = [akLeft, akTop, akRight]\n    TabOrder = 0\n  end\n\
             \x20 object UpBtn: TButton\n    Left = 478\n    Top = 7\n    Width = 74\n    Height = 23\n    Anchors = [akTop, akRight]\n    Caption = {}\n    TabOrder = 1\n  end\n\
             \x20 object FileList: TListView\n    Left = 8\n    Top = 36\n    Width = 544\n    Height = 268\n    Anchors = [akLeft, akTop, akRight, akBottom]\n    ReadOnly = True\n    RowSelect = True\n    ViewStyle = vsReport\n    TabOrder = 2\n    OnColumnClick = FileListColumnClick\n    Columns = <\n      item\n        Caption = 'Name'\n        AutoSize = True\n      end\n      item\n        Caption = 'Date modified'\n        Width = 145\n      end\n      item\n        Caption = 'Type'\n        Width = 90\n      end\n      item\n        Caption = 'Size'\n        Width = 85\n      end>\n  end\n\
             \x20 object NameLabel: TLabel\n    Left = 8\n    Top = 320\n    Width = 50\n    Height = 13\n    Anchors = [akLeft, akBottom]\n    Caption = {}\n  end\n\
             \x20 object NameEdit: TEdit\n    Left = 96\n    Top = 316\n    Width = 370\n    Height = 21\n    Anchors = [akLeft, akRight, akBottom]\n    TabOrder = 3\n  end\n\
             \x20 object TypeLabel: TLabel\n    Left = 8\n    Top = 352\n    Width = 70\n    Height = 13\n    Anchors = [akLeft, akBottom]\n    Caption = {}\n  end\n\
             \x20 object TypeCombo: TComboBox\n    Left = 96\n    Top = 348\n    Width = 370\n    Height = 21\n    Anchors = [akLeft, akRight, akBottom]\n    Style = csDropDownList\n    TabOrder = 4\n  end\n",
            esc(&tr("Up one level")),
            esc(&tr("File &name:")),
            esc(&tr("Files of &type:"))
        ));
        d.push_str(&button("OkBtn", &if o.folder { tr("&Select") } else if save { tr("&Save") } else { tr("&Open") }, 477, 314, 75, 0, true, false).replace("TabOrder = 0", "TabOrder = 5\n    Anchors = [akRight, akBottom]"));
        d.push_str(&button("CancelBtn", &tr("Cancel"), 477, 346, 75, MR_CANCEL, false, true).replace("TabOrder = 0", "TabOrder = 6\n    Anchors = [akRight, akBottom]"));
        if o.preview {
            d.push_str(&format!(
                "  object PreviewImage: TImage\n    Left = 564\n    Top = 36\n    Width = 240\n    Height = 180\n    Anchors = [akTop, akRight]\n    Center = True\n  end\n\
                 \x20 object PreviewLabel: TLabel\n    Left = 564\n    Top = 224\n    Width = 240\n    Height = 80\n    Anchors = [akTop, akRight]\n    AutoSize = False\n    WordWrap = True\n    Caption = {}\n  end\n",
                esc(&tr("Click a parameter file for its preview."))
            ));
        }
        d.push_str("end\n");
        let i = self.dialog_form("FileDlg", &d);
        let mut dir = o.initial_dir.clone().filter(|p| p.is_dir()).unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
        let mut name = o.file_name.clone();
        let p = Path::new(&o.file_name);
        if p.is_absolute() || p.parent().is_some_and(|q| !q.as_os_str().is_empty()) {
            if let Some(par) = p.parent().filter(|q| q.is_dir()) {
                dir = par.to_path_buf();
            }
            name = p.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        }
        let dlg = FileDlg {
            tag: tag.to_string(),
            save,
            multi: o.multi && !save,
            dir,
            filters: filters.clone(),
            default_ext: o.default_ext.clone(),
            folder: o.folder,
            entries: Vec::new(),
            preview: o.preview,
            sort_column: self.file_sort,
            sort_desc: self.file_sort_desc,
            eat_ok: false,
        };
        {
            let f = &mut self.forms[i];
            let c = f.c_mut("TypeCombo");
            c.items = filters.iter().map(|(n, _)| n.clone()).collect();
            let fi = o.filter_index.saturating_sub(1).min(c.items.len().saturating_sub(1));
            c.set_item_index(fi as i32);
            f.c_mut("NameEdit").set_text(&name);
            f.c_mut("FileList").multi_select = o.multi && !save;
        }
        self.dialogs.insert(i, DialogState::File(dlg));
        self.file_refresh(i);
        self.center_on_main(i);
        let ne = self.forms[i].id("NameEdit");
        self.forms[i].focus = ne;
        self.forms[i].modal = true;
        self.show_index(i);
        self.forms[i].modal = true;
    }

    fn file_refresh(&mut self, i: usize) {
        let sel = self.forms[i].c("TypeCombo").item_index.max(0) as usize;
        let Some(DialogState::File(d)) = self.dialogs.get_mut(&i) else { return };
        let pats = d.filters.get(sel).map(|f| f.1.clone()).unwrap_or_else(|| vec!["*.*".into()]);
        let mut dirs = Vec::new();
        let mut files = Vec::new();
        if let Ok(rd) = std::fs::read_dir(&d.dir) {
            for e in rd.flatten() {
                let n = e.file_name().to_string_lossy().into_owned();
                if n.starts_with('.') {
                    continue;
                }
                let is_dir = e.path().is_dir();
                let metadata = e.metadata().ok();
                let entry = FileEntry {
                    is_dir,
                    name: n,
                    modified: metadata.as_ref().and_then(|m| m.modified().ok()),
                    size: metadata.as_ref().map(|m| m.len()).unwrap_or(0),
                };
                if is_dir {
                    dirs.push(entry);
                } else if !d.folder && matches(&pats, &entry.name) {
                    files.push(entry);
                }
            }
        }
        dirs.sort_by(|a, b| entry_cmp(a, b, d.sort_column, d.sort_desc));
        files.sort_by(|a, b| entry_cmp(a, b, d.sort_column, d.sort_desc));
        d.entries = dirs.into_iter().chain(files).collect();
        let rows: Vec<Vec<String>> = d.entries.iter().map(|e| vec![if e.is_dir { format!("[{}]", e.name) } else { e.name.clone() }, entry_modified(e), entry_type(e), entry_size(e)]).collect();
        let dir_s = d.dir.display().to_string();
        let f = &mut self.forms[i];
        f.c_mut("DirEdit").set_text(&dir_s);
        let l = f.c_mut("FileList");
        file_columns(l, d.sort_column, d.sort_desc);
        l.selected = vec![false; rows.len()];
        l.cells = rows;
        l.item_index = -1;
        l.top_index = 0;
    }

    /// The extension of the chosen file type (its first `*.ext` pattern).
    fn file_type_ext(&self, i: usize) -> Option<String> {
        let sel = self.forms[i].c("TypeCombo").item_index.max(0) as usize;
        let Some(DialogState::File(d)) = self.dialogs.get(&i) else { return None };
        d.filters.get(sel)?.1.iter().find_map(|p| p.strip_prefix("*.").filter(|e| !e.is_empty() && !e.contains(['*', '?'])).map(str::to_string))
    }

    /// `SaveDialogTypeChange`: another file type in a save dialog gives the
    /// file name its extension.
    fn file_type_change(&mut self, i: usize) {
        let Some(DialogState::File(d)) = self.dialogs.get(&i) else { return };
        if !d.save {
            return;
        }
        let Some(ext) = self.file_type_ext(i) else { return };
        let name = self.forms[i].c("NameEdit").text.trim().to_string();
        if name.is_empty() {
            return;
        }
        let new = Path::new(&name).with_extension(&ext).to_string_lossy().into_owned();
        if new != name {
            self.forms[i].c_mut("NameEdit").set_text(&new);
        }
    }

    /// Accepts the file name of the dialog; false when it navigated instead.
    fn file_accept(&mut self, i: usize) -> bool {
        let name = self.forms[i].c("NameEdit").text.trim().to_string();
        let Some(DialogState::File(d)) = self.dialogs.get_mut(&i) else { return false };
        if d.multi {
            let l = self.forms[i].c("FileList");
            let picked: Vec<PathBuf> = l
                .selected
                .iter()
                .enumerate()
                .filter(|(k, s)| **s && !d.entries.get(*k).map(|e| e.is_dir).unwrap_or(true))
                .map(|(k, _)| d.dir.join(&d.entries[k].name))
                .collect();
            if picked.len() > 1 {
                let tag = d.tag.clone();
                self.forms[i].modal_result = MR_OK;
                self.events.push_back(Event { form: String::new(), sender: tag, handler: String::new(), ev: Ev::DialogResult(DialogResult::Files(picked)) });
                self.dialogs.remove(&i);
                self.hide_index(i);
                return true;
            }
        }
        if d.folder {
            let p = if name.is_empty() { d.dir.clone() } else { d.dir.join(&name) };
            let p = if p.is_dir() { p } else { d.dir.clone() };
            let tag = d.tag.clone();
            self.forms[i].modal_result = MR_OK;
            self.events.push_back(Event { form: String::new(), sender: tag, handler: String::new(), ev: Ev::DialogResult(DialogResult::File(Some(p))) });
            self.dialogs.remove(&i);
            self.hide_index(i);
            return true;
        }
        if name.is_empty() {
            return false;
        }
        let type_ext = self.file_type_ext(i);
        let Some(DialogState::File(d)) = self.dialogs.get_mut(&i) else { return false };
        let mut p = PathBuf::from(&name);
        if !p.is_absolute() {
            p = d.dir.join(&name);
        }
        if p.is_dir() {
            d.dir = p;
            self.forms[i].c_mut("NameEdit").set_text("");
            self.file_refresh(i);
            return false;
        }
        if d.save && p.extension().is_none() {
            // the chosen file type's extension (MB3D's DefaultExt per type)
            let ext = type_ext.unwrap_or_else(|| d.default_ext.trim_start_matches('.').to_string());
            if !ext.is_empty() {
                p.set_extension(ext);
            }
        }
        if !d.save && !p.exists() {
            let msg = format!("{}\nFile not found.\nPlease verify the correct file name was given.", p.display());
            let _ = msg;
            return false;
        }
        let tag = d.tag.clone();
        let multi = d.multi;
        self.filter_index = self.forms[i].c("TypeCombo").item_index.max(0) as usize + 1;
        self.forms[i].modal_result = MR_OK;
        let res = if multi { DialogResult::Files(vec![p]) } else { DialogResult::File(Some(p)) };
        self.events.push_back(Event { form: String::new(), sender: tag, handler: String::new(), ev: Ev::DialogResult(res) });
        self.dialogs.remove(&i);
        self.hide_index(i);
        true
    }

    /// Events of the dialog forms.
    pub(crate) fn dialog_event(&mut self, i: usize, r: &Raw) {
        let name = self.forms[i].ctl[r.id].name.clone();
        let is_file = matches!(self.dialogs.get(&i), Some(DialogState::File(_)));
        let is_color = matches!(self.dialogs.get(&i), Some(DialogState::Color { .. }));
        if is_file {
            let eat_ok = match self.dialogs.get_mut(&i) {
                Some(DialogState::File(d)) => std::mem::take(&mut d.eat_ok),
                _ => false,
            };
            match (name.as_str(), r.name) {
                ("OkBtn", "OnClick") if eat_ok => {}
                ("OkBtn", "OnClick") => {
                    self.file_accept(i);
                }
                ("UpBtn", "OnClick") => {
                    if let Some(DialogState::File(d)) = self.dialogs.get_mut(&i) {
                        if let Some(p) = d.dir.parent() {
                            d.dir = p.to_path_buf();
                        }
                    }
                    self.file_refresh(i);
                }
                ("TypeCombo", "OnChange") => {
                    self.file_type_change(i);
                    self.file_refresh(i);
                }
                ("FileList", "OnColumnClick") => {
                    let column = self.forms[i].c("FileList").col.max(0) as usize;
                    if let Some(DialogState::File(d)) = self.dialogs.get_mut(&i) {
                        if d.sort_column == column {
                            d.sort_desc = !d.sort_desc;
                        } else {
                            d.sort_column = column;
                            d.sort_desc = matches!(column, 1 | 3);
                        }
                        self.file_sort = d.sort_column;
                        self.file_sort_desc = d.sort_desc;
                    }
                    self.file_refresh(i);
                }
                ("DirEdit", "OnKeyPress") if r.ev == Ev::KeyPress('\r') => {
                    let t = PathBuf::from(self.forms[i].c("DirEdit").text.trim());
                    if let Some(DialogState::File(d)) = self.dialogs.get_mut(&i) {
                        d.eat_ok = true;
                        if t.is_dir() {
                            d.dir = t;
                            self.file_refresh(i);
                        }
                    }
                }
                ("FileList", "OnClick") => {
                    // -1: the list was just refilled (a double-clicked folder)
                    let k = self.forms[i].c("FileList").item_index;
                    if k < 0 {
                        return;
                    }
                    if let Some(DialogState::File(d)) = self.dialogs.get(&i) {
                        if let Some(entry) = d.entries.get(k as usize) {
                            if entry.is_dir {
                                return;
                            }
                            let n = entry.name.clone();
                            if d.preview {
                                self.file_preview = Some(d.dir.join(&n));
                            }
                            self.forms[i].c_mut("NameEdit").set_text(&n);
                        }
                    }
                }
                ("FileList", "OnDblClick") => {
                    let k = self.forms[i].c("FileList").item_index;
                    let entry = match self.dialogs.get(&i) {
                        Some(DialogState::File(d)) => d.entries.get(k.max(0) as usize).cloned(),
                        _ => None,
                    };
                    match entry {
                        Some(entry) if entry.is_dir => {
                            if let Some(DialogState::File(d)) = self.dialogs.get_mut(&i) {
                                d.dir = d.dir.join(entry.name);
                            }
                            self.forms[i].c_mut("NameEdit").set_text("");
                            self.file_refresh(i);
                        }
                        Some(entry) => {
                            self.forms[i].c_mut("NameEdit").set_text(&entry.name);
                            self.file_accept(i);
                        }
                        None => {}
                    }
                }
                _ => {}
            }
            return;
        }
        if is_color {
            match (name.as_str(), r.name) {
                (s, "OnMouseDown") if s.starts_with("Swatch") => {
                    let col = self.forms[i].ctl[r.id].brush_color;
                    self.set_dialog_color(i, col);
                }
                (_, "OnChange") => {
                    let f = &self.forms[i];
                    let from_edit = name.starts_with('E');
                    let v = |n: &str| -> u32 {
                        if from_edit {
                            f.c(&format!("E{n}")).text.trim().parse::<u32>().unwrap_or(0).min(255)
                        } else {
                            f.c(&format!("T{n}")).position.clamp(0, 255) as u32
                        }
                    };
                    let col = 0xFF00_0000 | v("Red") << 16 | v("Green") << 8 | v("Blue");
                    let f = &mut self.forms[i];
                    f.c_mut("Preview").brush_color = col;
                    for (k, n) in ["Red", "Green", "Blue"].iter().enumerate() {
                        let ch = (col >> (16 - 8 * k)) & 255;
                        if from_edit {
                            f.c_mut(&format!("T{n}")).position = ch as i64;
                        } else if format!("T{n}") == name {
                            f.c_mut(&format!("E{n}")).set_text(&ch.to_string());
                        }
                    }
                }
                _ => {}
            }
        }
        // message box / input query: everything goes through ModalResult
        let _ = Kind::Form;
    }

    /// A dialog form was closed: report its result.
    pub(crate) fn dialog_closed(&mut self, i: usize) {
        let Some(st) = self.dialogs.remove(&i) else { return };
        let mr = self.forms[i].modal_result;
        let (tag, res) = match st {
            DialogState::Message { tag } => (tag, DialogResult::Button(if mr == MR_NONE { MR_CANCEL } else { mr })),
            DialogState::Input { tag } => {
                let t = self.forms[i].c("ValueEdit").text.clone();
                (tag, DialogResult::Text(if mr == MR_OK { Some(t) } else { None }))
            }
            DialogState::Color { tag } => {
                let c = self.forms[i].c("Preview").brush_color;
                (tag, DialogResult::Color(if mr == MR_OK { Some(c) } else { None }))
            }
            DialogState::File(d) => (d.tag, if d.multi { DialogResult::Files(Vec::new()) } else { DialogResult::File(None) }),
        };
        if tag.is_empty() {
            return;
        }
        self.events.push_back(Event { form: String::new(), sender: tag, handler: String::new(), ev: Ev::DialogResult(res) });
    }
}

/// The 48 basic colours of the Windows colour dialog.
const BASIC: [u32; 48] = [
    0xFF8080, 0xFFFF80, 0x80FF80, 0x00FF80, 0x80FFFF, 0x0080FF, 0xFF80C0, 0xFF80FF, 0xFF0000, 0xFFFF00, 0x80FF00, 0x00FF40, 0x00FFFF, 0x0080C0,
    0x8080C0, 0xFF00FF, 0x804040, 0xFF8040, 0x00FF00, 0x008080, 0x004080, 0x8080FF, 0x800040, 0xFF0080, 0x800000, 0xFF8000, 0x008000, 0x008040,
    0x0000FF, 0x0000A0, 0x800080, 0x8000FF, 0x400000, 0x804000, 0x004000, 0x004040, 0x000080, 0x000040, 0x400040, 0x400080, 0x000000, 0x808000,
    0x808040, 0x808080, 0x408080, 0xC0C0C0, 0x400040, 0xFFFFFF,
];

#[cfg(test)]
mod tests {
    use super::*;

    /// Another file type in a save dialog changes the name's extension; a
    /// name without one gets the type's when accepted.
    #[test]
    fn save_type_sets_extension() {
        let mut ui = Ui::default();
        let dir = std::env::temp_dir();
        let o = FileOptions { filter: "JPEG (*.jpg)|*.jpg|PNG (*.png)|*.png|All|*.*".into(), default_ext: "jpg".into(), initial_dir: Some(dir.clone()), file_name: "pic.jpg".into(), ..Default::default() };
        ui.save_dialog("t", &o);
        let i = ui.form_index("FileDlg").unwrap();
        let combo = ui.forms[i].id("TypeCombo").unwrap();
        let change = |ui: &mut Ui, k: i32| {
            ui.forms[i].c_mut("TypeCombo").set_item_index(k);
            ui.dialog_event(i, &Raw { id: combo, name: "OnChange", ev: Ev::Click });
        };
        change(&mut ui, 1);
        assert_eq!(ui.forms[i].c("NameEdit").text, "pic.png");
        // "All" has no extension: the name stays
        change(&mut ui, 2);
        assert_eq!(ui.forms[i].c("NameEdit").text, "pic.png");
        change(&mut ui, 1);
        ui.forms[i].c_mut("NameEdit").set_text("noext");
        assert!(ui.file_accept(i));
        let got = ui.events.iter().find_map(|e| match &e.ev {
            Ev::DialogResult(DialogResult::File(Some(p))) => Some(p.clone()),
            _ => None,
        });
        assert_eq!(got, Some(dir.join("noext.png")));
    }

    /// A message while a confirm is open opens over it; the confirm still
    /// gets its answer.
    #[test]
    fn dialog_over_open_confirm() {
        let mut ui = Ui::default();
        ui.confirm("ask", "Sure?");
        let a = ui.form_index("MessageBoxDlg").unwrap();
        ui.show_message("Something else");
        let b = ui.form_index("MessageBoxDlg1").unwrap();
        assert_ne!(a, b);
        assert_eq!(ui.modal_top(), Some(b));
        ui.close_index(b);
        assert_eq!(ui.modal_top(), Some(a));
        ui.forms[a].modal_result = MR_YES;
        ui.close_index(a);
        let got = ui.events.iter().find_map(|e| match &e.ev {
            Ev::DialogResult(DialogResult::Button(mr)) if e.sender == "ask" => Some(*mr),
            _ => None,
        });
        assert_eq!(got, Some(MR_YES));
        // closed, the first is used again
        ui.show_message("Again");
        assert_eq!(ui.modal_top(), Some(a));
    }
}
