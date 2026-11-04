//! The controls of a form: one struct for all VCL component classes, with
//! the published properties the form files use and the run-time state.

use super::bitmap::{self, Bitmap};
use super::canvas::{HAlign, Rect, VAlign};
use super::dfm::{Obj, Value};
use super::font::Font;
use super::theme::Theme;
use std::collections::HashMap;

pub type Id = usize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Form,
    Panel,
    Label,
    StaticText,
    Button,
    SpeedButton,
    Edit,
    Memo,
    CheckBox,
    RadioButton,
    RadioGroup,
    GroupBox,
    ComboBox,
    ListBox,
    TrackBar,
    UpDown,
    PageControl,
    TabSheet,
    TabControl,
    ScrollBox,
    Image,
    Shape,
    Bevel,
    ProgressBar,
    ScrollBar,
    StringGrid,
    ListView,
    CategoryPanelGroup,
    CategoryPanel,
    GridPanel,
    ColorButton,
    PopupMenu,
    MenuItem,
    Timer,
    Dialog,
    ImageList,
    Unknown,
}

impl Kind {
    pub fn from_class(c: &str) -> Kind {
        match c {
            "TPanel" | "TFlowPanel" => Kind::Panel,
            "TLabel" => Kind::Label,
            "TStaticText" => Kind::StaticText,
            "TButton" | "TBitBtn" => Kind::Button,
            "TSpeedButton" | "TSpeedButtonEx" => Kind::SpeedButton,
            "TEdit" | "TSpinEdit" | "TMaskEdit" | "TLabeledEdit" => Kind::Edit,
            "TMemo" | "TRichEdit" | "TSynEdit" => Kind::Memo,
            "TCheckBox" => Kind::CheckBox,
            "TRadioButton" => Kind::RadioButton,
            "TRadioGroup" => Kind::RadioGroup,
            "TGroupBox" | "TJvGroupBox" => Kind::GroupBox,
            "TComboBox" => Kind::ComboBox,
            "TListBox" | "TListBoxEx" | "TCheckListBox" => Kind::ListBox,
            "TTrackBar" | "TTrackBarEx" => Kind::TrackBar,
            "TUpDown" => Kind::UpDown,
            "TPageControl" => Kind::PageControl,
            "TTabSheet" => Kind::TabSheet,
            "TTabControl" => Kind::TabControl,
            "TScrollBox" => Kind::ScrollBox,
            "TImage" | "TPaintBox" => Kind::Image,
            "TShape" => Kind::Shape,
            "TBevel" => Kind::Bevel,
            "TProgressBar" => Kind::ProgressBar,
            "TScrollBar" => Kind::ScrollBar,
            "TStringGrid" => Kind::StringGrid,
            "TListView" => Kind::ListView,
            "TCategoryPanelGroup" => Kind::CategoryPanelGroup,
            "TCategoryPanel" => Kind::CategoryPanel,
            "TGridPanel" => Kind::GridPanel,
            "TJvOfficeColorButton" => Kind::ColorButton,
            "TPopupMenu" | "TMainMenu" => Kind::PopupMenu,
            "TMenuItem" => Kind::MenuItem,
            "TTimer" => Kind::Timer,
            "TOpenDialog" | "TSaveDialog" | "TOpenPictureDialog" | "TSavePictureDialog" | "TColorDialog" | "TFontDialog" => Kind::Dialog,
            "TImageList" => Kind::ImageList,
            _ => Kind::Unknown,
        }
    }

    /// Non-visual components (no position on the form).
    pub fn non_visual(self) -> bool {
        matches!(self, Kind::PopupMenu | Kind::MenuItem | Kind::Timer | Kind::Dialog | Kind::ImageList | Kind::Unknown)
    }

    /// TGraphicControl descendants: drawn on the parent before the
    /// windowed controls, never focused.
    pub fn graphic(self) -> bool {
        matches!(self, Kind::Label | Kind::SpeedButton | Kind::Image | Kind::Shape | Kind::Bevel)
    }

    pub fn container(self) -> bool {
        matches!(
            self,
            Kind::Form
                | Kind::Panel
                | Kind::GroupBox
                | Kind::RadioGroup
                | Kind::PageControl
                | Kind::TabSheet
                | Kind::TabControl
                | Kind::ScrollBox
                | Kind::CategoryPanelGroup
                | Kind::CategoryPanel
                | Kind::GridPanel
        )
    }

    pub fn focusable(self) -> bool {
        matches!(
            self,
            Kind::Edit
                | Kind::Memo
                | Kind::Button
                | Kind::CheckBox
                | Kind::RadioButton
                | Kind::RadioGroup
                | Kind::ComboBox
                | Kind::ListBox
                | Kind::TrackBar
                | Kind::StringGrid
                | Kind::ListView
                | Kind::ScrollBar
                | Kind::PageControl
                | Kind::TabControl
                | Kind::ColorButton
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    None,
    Top,
    Bottom,
    Left,
    Right,
    Client,
}

pub const AK_LEFT: u8 = 1;
pub const AK_TOP: u8 = 2;
pub const AK_RIGHT: u8 = 4;
pub const AK_BOTTOM: u8 = 8;

/// A string grid / list view column.
#[derive(Clone, Debug, Default)]
pub struct Column {
    pub caption: String,
    pub width: i32,
    /// takes the width the other columns leave (`AutoSize`)
    pub auto: bool,
}

#[derive(Clone, Debug)]
pub struct Control {
    pub name: String,
    pub class: String,
    pub kind: Kind,
    pub parent: Option<Id>,
    pub children: Vec<Id>,

    // ---- geometry (logical pixels, relative to the parent's origin)
    pub left: i32,
    pub top: i32,
    pub width: i32,
    pub height: i32,
    pub align: Align,
    pub anchors: u8,
    /// distance to the parent's right / bottom edge at design time
    pub(crate) anchor_r: i32,
    pub(crate) anchor_b: i32,
    pub(crate) anchored: bool,
    pub min_w: i32,
    pub min_h: i32,

    pub visible: bool,
    pub enabled: bool,
    pub caption: String,
    pub hint: String,
    pub show_hint: Option<bool>,
    pub font: Option<Font>,
    /// explicit background colour (None = parent's / class default)
    pub color: Option<u32>,
    pub transparent: bool,
    pub tag: i64,
    pub tab_order: i32,
    pub tab_stop: bool,
    /// event name (OnClick) -> handler method name (Button1Click)
    pub events: HashMap<String, String>,
    pub popup_menu: Option<String>,
    pub cursor: String,
    pub auto_size: bool,
    pub word_wrap: bool,
    pub alignment: HAlign,
    pub layout: VAlign,

    // ---- edits, memos, combo boxes
    pub text: String,
    pub read_only: bool,
    pub max_length: usize,
    pub password_char: Option<char>,
    pub numbers_only: bool,
    pub border: bool,
    pub want_tabs: bool,
    pub want_returns: bool,
    pub scroll_bars: u8,
    /// caret and selection anchor (character indices)
    pub caret: usize,
    pub sel_anchor: usize,
    pub scroll_x: i32,
    pub scroll_y: i32,

    // ---- buttons
    pub checked: bool,
    /// check box state: 0 unchecked, 1 checked, 2 grayed
    pub state: u8,
    pub allow_grayed: bool,
    pub down: bool,
    pub group_index: i32,
    pub allow_all_up: bool,
    pub flat: bool,
    pub glyph: Option<Bitmap>,
    pub num_glyphs: i32,
    pub spacing: i32,
    pub margin: i32,
    pub default: bool,
    pub cancel: bool,
    pub modal_result: i32,

    // ---- lists
    pub items: Vec<String>,
    /// a small picture before each item (a list box; empty: none)
    pub item_images: Vec<bitmap::Bitmap>,
    pub item_index: i32,
    pub item_height: i32,
    pub top_index: i32,
    pub columns: i32,
    pub sorted: bool,
    pub style: String,
    pub drop_down_count: i32,
    /// per item text colours (owner drawn lists in MB3D)
    pub item_colors: Vec<Option<u32>>,
    pub multi_select: bool,
    pub selected: Vec<bool>,

    // ---- ranges (track bar, up-down, progress, scroll bar)
    pub min: i64,
    pub max: i64,
    pub position: i64,
    pub frequency: i64,
    pub page_size: i64,
    pub line_size: i64,
    pub large_change: i64,
    pub increment: i64,
    pub vertical: bool,
    pub tick_marks: String,
    pub tick_style: String,
    pub thumb_length: i32,
    pub slider_visible: bool,
    pub associate: Option<String>,
    pub wrap: bool,
    pub thousands: bool,
    pub sel_start: i64,
    pub sel_end: i64,

    // ---- images, shapes, bevels, panels
    pub picture: Option<Bitmap>,
    pub stretch: bool,
    pub proportional: bool,
    pub center: bool,
    /// draw the picture in device pixels (no scaling on HiDPI)
    pub device_pixels: bool,
    pub shape: String,
    pub brush_color: u32,
    pub brush_clear: bool,
    pub pen_color: u32,
    pub pen_width: i32,
    pub bevel_outer: String,
    pub bevel_inner: String,
    pub bevel_width: i32,
    pub border_width: i32,
    pub border_style: String,
    pub bevel_shape: String,
    pub bevel_style: String,

    // ---- pages, tabs
    pub active_page: Option<Id>,
    pub tab_index: i32,
    pub tabs: Vec<String>,
    pub tab_height: i32,
    pub tab_width: i32,
    pub tab_visible: bool,
    pub multi_line: bool,
    pub collapsed: bool,

    // ---- grids, list views
    pub cols: Vec<Column>,
    pub cells: Vec<Vec<String>>,
    /// a list view whose owner fills `cells` with a window of its rows
    /// only (a list of a million rows): the rows in all (0: `cells` are
    /// all the rows) and the row `cells[0]` is
    pub virtual_rows: usize,
    pub cells_from: usize,
    /// list view with check boxes: the check state per item
    pub checkboxes: bool,
    pub checks: Vec<bool>,
    pub fixed_rows: i32,
    pub fixed_cols: i32,
    pub default_row_height: i32,
    pub row: i32,
    pub col: i32,
    pub editing: bool,

    // ---- menu items, timers, dialogs
    pub shortcut: String,
    pub radio_item: bool,
    pub interval: u32,
    pub filter: String,
    pub default_ext: String,
    pub initial_dir: String,
    pub file_name: String,
    pub title: String,
    pub image_index: i32,
    pub images: Vec<Bitmap>,

    /// other properties of the form file
    pub props: Vec<(String, Value)>,
    pub(crate) design: Rect,
    /// grid panel cell (column, row) of a child
    pub(crate) grid_cell: Option<(i32, i32)>,
}

impl Control {
    /// The widths of the list view columns in `inner_w` pixels: an
    /// auto-size column (or the last one) takes the rest.
    pub fn column_widths(&self, inner_w: i32) -> Vec<i32> {
        let n = self.cols.len();
        let fixed: i32 = self.cols.iter().filter(|k| !k.auto).map(|k| k.width).sum();
        let autos = self.cols.iter().filter(|k| k.auto).count() as i32;
        let mut v: Vec<i32> = self
            .cols
            .iter()
            .map(|k| if k.auto { ((inner_w - fixed) / autos.max(1)).max(20) } else { k.width })
            .collect();
        if autos == 0 && n > 0 {
            let total: i32 = v.iter().sum();
            if total < inner_w {
                v[n - 1] += inner_w - total;
            }
        }
        v
    }

    pub fn new(name: &str, class: &str) -> Control {
        let kind = Kind::from_class(class);
        Control {
            name: name.to_string(),
            class: class.to_string(),
            kind,
            parent: None,
            children: Vec::new(),
            left: 0,
            top: 0,
            width: 0,
            height: 0,
            align: Align::None,
            anchors: AK_LEFT | AK_TOP,
            anchor_r: 0,
            anchor_b: 0,
            anchored: false,
            min_w: 0,
            min_h: 0,
            visible: true,
            enabled: true,
            caption: String::new(),
            hint: String::new(),
            show_hint: None,
            font: None,
            color: None,
            transparent: kind == Kind::SpeedButton,
            tag: 0,
            tab_order: 0,
            tab_stop: kind.focusable(),
            events: HashMap::new(),
            popup_menu: None,
            cursor: String::new(),
            auto_size: matches!(kind, Kind::Label | Kind::Edit | Kind::StaticText),
            // TMemo wraps by default (WordWrap = True)
            word_wrap: kind == Kind::Memo,
            alignment: if kind == Kind::Panel { HAlign::Center } else { HAlign::Left },
            layout: if kind == Kind::SpeedButton || kind == Kind::Panel { VAlign::Center } else { VAlign::Top },
            text: String::new(),
            read_only: false,
            max_length: 0,
            password_char: None,
            numbers_only: false,
            border: true,
            want_tabs: false,
            want_returns: true,
            scroll_bars: 0,
            caret: 0,
            sel_anchor: 0,
            scroll_x: 0,
            scroll_y: 0,
            checked: false,
            state: 0,
            allow_grayed: false,
            down: false,
            group_index: 0,
            allow_all_up: false,
            flat: false,
            glyph: None,
            num_glyphs: 1,
            spacing: 4,
            margin: -1,
            default: false,
            cancel: false,
            modal_result: 0,
            items: Vec::new(),
            item_images: Vec::new(),
            item_index: -1,
            item_height: 13,
            top_index: 0,
            columns: 1,
            sorted: false,
            style: String::new(),
            drop_down_count: 8,
            item_colors: Vec::new(),
            multi_select: false,
            selected: Vec::new(),
            min: 0,
            max: if kind == Kind::UpDown || kind == Kind::ProgressBar || kind == Kind::ScrollBar { 100 } else { 10 },
            position: 0,
            frequency: 1,
            page_size: if kind == Kind::TrackBar { 2 } else { 0 },
            line_size: 1,
            large_change: 1,
            increment: 1,
            vertical: false,
            tick_marks: "tmBottomRight".into(),
            tick_style: "tsAuto".into(),
            thumb_length: 20,
            slider_visible: true,
            associate: None,
            wrap: false,
            thousands: true,
            sel_start: 0,
            sel_end: 0,
            picture: None,
            stretch: false,
            proportional: false,
            center: false,
            device_pixels: false,
            shape: "stRectangle".into(),
            brush_color: 0xFFFF_FFFF,
            brush_clear: false,
            pen_color: 0xFF00_0000,
            pen_width: 1,
            bevel_outer: if kind == Kind::Panel || kind == Kind::CategoryPanelGroup { "bvRaised".into() } else { "bvNone".into() },
            bevel_inner: "bvNone".into(),
            bevel_width: 1,
            border_width: 0,
            border_style: match kind {
                Kind::Panel | Kind::Label | Kind::StaticText => "bsNone".into(),
                _ => "bsSingle".into(),
            },
            bevel_shape: "bsBox".into(),
            bevel_style: "bsLowered".into(),
            active_page: None,
            tab_index: -1,
            tabs: Vec::new(),
            tab_height: 0,
            tab_width: 0,
            tab_visible: true,
            multi_line: false,
            collapsed: false,
            cols: Vec::new(),
            cells: Vec::new(),
            virtual_rows: 0,
            cells_from: 0,
            checkboxes: false,
            checks: Vec::new(),
            fixed_rows: 1,
            fixed_cols: 1,
            default_row_height: 24,
            row: 1,
            col: 1,
            editing: false,
            shortcut: String::new(),
            radio_item: false,
            interval: 1000,
            filter: String::new(),
            default_ext: String::new(),
            initial_dir: String::new(),
            file_name: String::new(),
            title: String::new(),
            image_index: -1,
            images: Vec::new(),
            props: Vec::new(),
            design: Rect::default(),
            grid_cell: None,
        }
    }

    pub fn rect(&self) -> Rect {
        Rect::new(self.left, self.top, self.width, self.height)
    }

    pub fn set_bounds(&mut self, l: i32, t: i32, w: i32, h: i32) {
        self.left = l;
        self.top = t;
        self.width = w;
        self.height = h;
    }

    pub fn handler(&self, ev: &str) -> Option<&str> {
        self.events.get(ev).map(String::as_str)
    }

    pub fn prop(&self, key: &str) -> Option<&Value> {
        self.props.iter().find(|(k, _)| k.eq_ignore_ascii_case(key)).map(|(_, v)| v)
    }

    /// Hint text shown in the tooltip (the part before '|').
    pub fn short_hint(&self) -> &str {
        self.hint.split('|').next().unwrap_or("")
    }

    /// The edit text as characters (caret positions are char indices).
    pub fn chars(&self) -> Vec<char> {
        self.text.chars().collect()
    }

    pub fn selection(&self) -> (usize, usize) {
        (self.caret.min(self.sel_anchor), self.caret.max(self.sel_anchor))
    }

    /// Sets the text and puts the caret at its end (as VCL's `Text :=`).
    pub fn set_text(&mut self, s: &str) {
        if self.text != s {
            self.text = s.to_string();
            let n = self.text.chars().count();
            self.caret = self.caret.min(n);
            self.sel_anchor = self.caret;
            if self.kind != Kind::Memo {
                self.scroll_x = 0;
            }
        }
    }

    /// The selected item's text ("" when none).
    pub fn item_text(&self) -> String {
        if self.item_index >= 0 {
            self.items.get(self.item_index as usize).cloned().unwrap_or_default()
        } else {
            String::new()
        }
    }

    pub fn set_item_index(&mut self, i: i32) {
        self.item_index = if i >= 0 && (i as usize) < self.items.len() { i } else { -1 };
        if self.kind == Kind::ComboBox {
            self.text = self.item_text();
            let n = self.text.chars().count();
            self.caret = n;
            self.sel_anchor = n;
        }
    }

    pub fn set_position(&mut self, p: i64) {
        self.position = p.clamp(self.min.min(self.max), self.max.max(self.min));
    }

    /// Image of a glyph for the button state: 0 up, 1 disabled, 2 down.
    pub fn glyph_image(&self, state: usize) -> Option<Bitmap> {
        let g = self.glyph.as_ref()?;
        let n = self.num_glyphs.max(1) as usize;
        let w = g.w / n;
        let i = if state < n { state } else { 0 };
        Some(g.crop(i * w, 0, w, g.h).with_transparent_corner())
    }
}

fn ident<'a>(o: &'a Obj, k: &str) -> Option<&'a str> {
    match o.get(k) {
        Some(Value::Ident(s)) => Some(s),
        _ => None,
    }
}

fn color_of(theme: &Theme, v: &Value) -> Option<u32> {
    match v {
        Value::Ident(s) => theme.named(s),
        Value::Int(i) => {
            if *i == 0x1FFF_FFFF || *i == 0x2000_0000 {
                None
            } else {
                Some(theme.color(*i))
            }
        }
        _ => None,
    }
}

fn is_system_color(v: &Value) -> bool {
    match v {
        Value::Ident(s) => Theme::is_system(s),
        Value::Int(i) => *i < 0,
        _ => false,
    }
}

/// Reads the font properties if the control has its own font.
fn font_of(o: &Obj, theme: &Theme, parent: Option<&Font>) -> Option<Font> {
    let has = o.props.iter().any(|(k, _)| k.starts_with("Font."));
    if !has {
        return None;
    }
    let mut f = parent.cloned().unwrap_or_default();
    if let Some(n) = o.str("Font.Name") {
        f.name = n.to_string();
    }
    if let Some(h) = o.int("Font.Height") {
        f.height = h as i32;
    }
    if let Some(Value::Set(s)) = o.get("Font.Style") {
        f.bold = s.iter().any(|x| x == "fsBold");
        f.italic = s.iter().any(|x| x == "fsItalic");
        f.underline = s.iter().any(|x| x == "fsUnderline");
    }
    if let Some(v) = o.get("Font.Color") {
        if !is_system_color(v) && !theme.glossy() {
            if let Some(c) = color_of(theme, v) {
                f.color = c;
                f.custom_color = true;
            }
        } else if !is_system_color(v) {
            // the Glossy style keeps bright explicit colours only
            if let Some(c) = color_of(theme, v) {
                let c = theme.fix_text(c);
                if c != theme.text() {
                    f.color = c;
                    f.custom_color = true;
                }
            }
        }
    }
    Some(f)
}

/// Builds a control from its form file object (children are added by the
/// caller).
pub fn from_obj(o: &Obj, theme: &Theme, parent_font: Option<&Font>) -> Control {
    let mut c = Control::new(&o.name, &o.class);
    let k = c.kind;
    for (key, v) in &o.props {
        let key = key.as_str();
        if let Some(ev) = key.strip_prefix("On") {
            if let Some(h) = v.as_str() {
                c.events.insert(format!("On{ev}"), h.to_string());
            }
            continue;
        }
        let int = v.as_int().unwrap_or(0);
        let b = v.as_bool().unwrap_or(false);
        let s = v.as_str().unwrap_or("");
        match key {
            "Left" => c.left = int as i32,
            "Top" => c.top = int as i32,
            "Width" | "ClientWidth" => c.width = int as i32,
            "Height" | "ClientHeight" => c.height = int as i32,
            "Constraints.MinWidth" => c.min_w = int as i32,
            "Constraints.MinHeight" => c.min_h = int as i32,
            "Align" => {
                c.align = match s {
                    "alTop" => Align::Top,
                    "alBottom" => Align::Bottom,
                    "alLeft" => Align::Left,
                    "alRight" => Align::Right,
                    "alClient" => Align::Client,
                    _ => Align::None,
                }
            }
            "Anchors" => {
                if let Value::Set(set) = v {
                    c.anchors = 0;
                    for a in set {
                        c.anchors |= match a.as_str() {
                            "akLeft" => AK_LEFT,
                            "akTop" => AK_TOP,
                            "akRight" => AK_RIGHT,
                            "akBottom" => AK_BOTTOM,
                            _ => 0,
                        };
                    }
                }
            }
            "Visible" => c.visible = b,
            "Enabled" => c.enabled = b,
            "Caption" => c.caption = s.to_string(),
            "Hint" => c.hint = s.to_string(),
            "ShowHint" => c.show_hint = Some(b),
            "Color" => {
                // the VCL styles paint system colours (and most explicit
                // panel colours) in the style's colours
                if !is_system_color(v) && !(theme.glossy() && matches!(k, Kind::Form | Kind::Panel | Kind::GroupBox | Kind::CheckBox | Kind::TabSheet | Kind::Memo | Kind::Edit | Kind::ListBox | Kind::RadioGroup | Kind::RadioButton | Kind::Label)) {
                    c.color = color_of(theme, v);
                }
            }
            "Transparent" => c.transparent = b,
            "ParentBackground" if !b && k == Kind::Panel => {}
            "Tag" => c.tag = int,
            "TabOrder" => c.tab_order = int as i32,
            "TabStop" => c.tab_stop = b,
            "PopupMenu" => c.popup_menu = Some(s.to_string()),
            "Cursor" => c.cursor = s.to_string(),
            "AutoSize" => c.auto_size = b,
            "WordWrap" => c.word_wrap = b,
            "Alignment" => {
                c.alignment = match s {
                    "taRightJustify" => HAlign::Right,
                    "taCenter" => HAlign::Center,
                    _ => HAlign::Left,
                }
            }
            "Layout" => {
                c.layout = match s {
                    "tlCenter" | "blGlyphLeft" => VAlign::Center,
                    "tlBottom" => VAlign::Bottom,
                    "tlTop" => VAlign::Top,
                    _ => c.layout,
                };
                if k == Kind::SpeedButton || k == Kind::Button {
                    c.style = s.to_string();
                    c.layout = VAlign::Center;
                }
            }
            "VerticalAlignment" => {
                c.layout = match s {
                    "taAlignTop" => VAlign::Top,
                    "taAlignBottom" => VAlign::Bottom,
                    _ => VAlign::Center,
                }
            }
            "Text" => c.text = s.to_string(),
            "Lines.Strings" => c.text = v.as_strings().join("\n"),
            "ReadOnly" => c.read_only = b,
            "MaxLength" => c.max_length = int.max(0) as usize,
            "PasswordChar" => c.password_char = s.chars().next().filter(|&ch| ch != '\0'),
            "NumbersOnly" => c.numbers_only = b,
            "BorderStyle" => {
                c.border_style = s.to_string();
                if matches!(k, Kind::Edit | Kind::Memo | Kind::ListBox | Kind::ScrollBox) {
                    c.border = s != "bsNone";
                }
            }
            "BevelKind" if s == "bkFlat" || s == "bkTile" => {}
            "WantTabs" => c.want_tabs = b,
            "WantReturns" => c.want_returns = b,
            "ScrollBars" => {
                c.scroll_bars = match s {
                    "ssHorizontal" => 1,
                    "ssVertical" => 2,
                    "ssBoth" => 3,
                    _ => 0,
                }
            }
            "Checked" => {
                c.checked = b;
                c.state = b as u8;
            }
            "State" => {
                c.state = match s {
                    "cbChecked" => 1,
                    "cbGrayed" => 2,
                    _ => 0,
                };
                c.checked = c.state == 1;
            }
            "AllowGrayed" => c.allow_grayed = b,
            "Down" => c.down = b,
            "GroupIndex" => c.group_index = int as i32,
            "AllowAllUp" => c.allow_all_up = b,
            "Flat" => c.flat = b,
            "Glyph.Data" => {
                if let Value::Bin(d) = v {
                    c.glyph = bitmap::decode_tbitmap(d);
                }
            }
            "NumGlyphs" => c.num_glyphs = int.max(1) as i32,
            "Spacing" => c.spacing = int as i32,
            "Margin" => c.margin = int as i32,
            "Default" => c.default = b,
            "Cancel" => c.cancel = b,
            "ModalResult" => c.modal_result = int as i32,
            "Items.Strings" => c.items = v.as_strings(),
            "Tabs.Strings" => c.tabs = v.as_strings(),
            "ItemIndex" => c.item_index = int as i32,
            "ItemHeight" => c.item_height = int as i32,
            "Columns" => match v {
                Value::Int(n) => c.columns = *n as i32,
                Value::Items(items) => {
                    c.cols = items
                        .iter()
                        .map(|it| {
                            let get = |n: &str| it.iter().find(|(k, _)| k == n).map(|(_, v)| v);
                            Column {
                                caption: get("Caption").and_then(Value::as_str).unwrap_or("").to_string(),
                                width: get("Width").and_then(Value::as_int).unwrap_or(50) as i32,
                                auto: get("AutoSize").and_then(Value::as_bool).unwrap_or(false),
                            }
                        })
                        .collect()
                }
                _ => {}
            },
            "Checkboxes" => c.checkboxes = b,
            "Sorted" => c.sorted = b,
            "Style" => c.style = s.to_string(),
            "DropDownCount" => c.drop_down_count = int as i32,
            "MultiSelect" => c.multi_select = b,
            "Min" => c.min = int,
            "Max" => c.max = int,
            "Position" => {
                if k == Kind::Form {
                    c.style = s.to_string();
                } else {
                    c.position = int
                }
            }
            "Frequency" => c.frequency = int,
            "PageSize" => c.page_size = int,
            "LineSize" | "SmallChange" => c.line_size = int,
            "LargeChange" => c.large_change = int,
            "Increment" => c.increment = int,
            "Orientation" => c.vertical = s == "trVertical" || s == "udVertical" || s == "pbVertical",
            "Kind" => c.vertical = s == "sbVertical",
            "TickMarks" => c.tick_marks = s.to_string(),
            "TickStyle" => c.tick_style = s.to_string(),
            "ThumbLength" => c.thumb_length = int as i32,
            "SliderVisible" => c.slider_visible = b,
            "Associate" => c.associate = Some(s.to_string()),
            "Wrap" => c.wrap = b,
            "Thousands" => c.thousands = b,
            "SelStart" => c.sel_start = int,
            "SelEnd" => c.sel_end = int,
            "Picture.Data" => {
                if let Value::Bin(d) = v {
                    c.picture = bitmap::decode_picture(d);
                }
            }
            "Bitmap" if k == Kind::ImageList => {
                if let Value::Bin(d) = v {
                    c.images = bitmap::decode_imagelist(d);
                }
            }
            "Stretch" => c.stretch = b,
            "Proportional" => c.proportional = b,
            "Center" => c.center = b,
            "Shape" => {
                if k == Kind::Bevel {
                    c.bevel_shape = s.to_string()
                } else {
                    c.shape = s.to_string()
                }
            }
            "Brush.Color" => c.brush_color = color_of(theme, v).unwrap_or(0xFFFF_FFFF),
            "Brush.Style" => c.brush_clear = s == "bsClear",
            "Pen.Color" => c.pen_color = color_of(theme, v).unwrap_or(0xFF00_0000),
            "Pen.Width" => c.pen_width = int as i32,
            "Pen.Style" if s == "psClear" => c.pen_width = 0,
            "BevelOuter" => c.bevel_outer = s.to_string(),
            "BevelInner" => c.bevel_inner = s.to_string(),
            "BevelWidth" => c.bevel_width = int as i32,
            "BorderWidth" => c.border_width = int as i32,
            "TabIndex" => c.tab_index = int as i32,
            "TabHeight" => c.tab_height = int as i32,
            "TabWidth" => c.tab_width = int as i32,
            "TabVisible" => c.tab_visible = b,
            "MultiLine" => c.multi_line = b,
            "Collapsed" => c.collapsed = b,
            "ColCount" => c.cols.resize(int.max(0) as usize, Column { caption: String::new(), width: 64, auto: false }),
            "RowCount" => c.cells.resize(int.max(0) as usize, Vec::new()),
            "FixedRows" => c.fixed_rows = int as i32,
            "FixedCols" => c.fixed_cols = int as i32,
            "DefaultRowHeight" => c.default_row_height = int as i32,
            "DefaultColWidth" => {
                for col in c.cols.iter_mut() {
                    col.width = int as i32;
                }
            }
            "ColWidths" => {
                if let Value::List(l) = v {
                    for (i, w) in l.iter().enumerate() {
                        if let (Some(col), Some(w)) = (c.cols.get_mut(i), w.as_int()) {
                            col.width = w as i32;
                        }
                    }
                }
            }
            "ShortCut" => c.shortcut = format!("{int}"),
            "RadioItem" => c.radio_item = b,
            "Interval" => c.interval = int.max(1) as u32,
            "Filter" => c.filter = s.to_string(),
            "DefaultExt" => c.default_ext = s.to_string(),
            "InitialDir" => c.initial_dir = s.to_string(),
            "FileName" => c.file_name = s.to_string(),
            "Title" => c.title = s.to_string(),
            "ImageIndex" => c.image_index = int as i32,
            "SelectedColor" => c.brush_color = color_of(theme, v).unwrap_or(0xFF00_0000),
            _ => {}
        }
        c.props.push((key.to_string(), v.clone()));
    }
    c.font = font_of(o, theme, parent_font);
    if k == Kind::ComboBox && c.item_index >= 0 && c.text.is_empty() {
        c.text = c.item_text();
    }
    if k == Kind::CheckBox || k == Kind::RadioButton {
        c.state = c.checked as u8 | if c.state == 2 { 2 } else { 0 };
        if c.state == 3 {
            c.state = 1;
        }
    }
    if k == Kind::Timer && ident(o, "Enabled").is_none() {
        c.enabled = true;
    }
    if k == Kind::UpDown {
        c.width = c.width.max(1);
    }
    if k == Kind::StringGrid {
        if c.cols.is_empty() {
            c.cols = vec![Column { caption: String::new(), width: 64, auto: false }; 5];
        }
        if c.cells.is_empty() {
            c.cells = vec![Vec::new(); 5];
        }
        let n = c.cols.len();
        for r in c.cells.iter_mut() {
            r.resize(n, String::new());
        }
    }
    if k == Kind::Edit && c.class == "TSpinEdit" {
        if let Some(Value::Int(v)) = o.get("Value") {
            c.text = v.to_string();
        }
    }
    c.selected = vec![false; c.items.len()];
    c.design = c.rect();
    c
}

impl Control {
    /// A list view's rows (see `virtual_rows`).
    pub fn row_count(&self) -> usize {
        if self.virtual_rows > 0 {
            self.virtual_rows
        } else {
            self.cells.len()
        }
    }

    /// The cells of row `ri`, when its owner filled them.
    pub fn cell_row(&self, ri: usize) -> Option<&Vec<String>> {
        if self.virtual_rows > 0 {
            ri.checked_sub(self.cells_from).and_then(|i| self.cells.get(i))
        } else {
            self.cells.get(ri)
        }
    }
}
