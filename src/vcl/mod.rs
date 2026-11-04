//! A small VCL-like GUI toolkit: forms are built from Delphi form files
//! (`.dfm`) and drawn by the program itself in the look of MB3D's VCL
//! styles ("Glossy" and "Windows"), so the original windows can be shown
//! unchanged on every platform.
//!
//! * [`dfm`] parses form files, [`bitmap`] decodes their glyphs and pictures
//! * [`form`] holds a form's controls (one [`control::Control`] per VCL
//!   component) and does VCL's alignment / anchor layout
//! * [`paint`] and [`input`] give the controls their look and behaviour
//! * [`ui::Ui`] is the program's view of all forms; events are delivered
//!   with the handler names of the form files (`Button1Click`)
//! * [`run`] opens one window per visible form

pub mod bitmap;
pub mod canvas;
pub mod clipboard;
pub mod control;
pub mod dfm;
pub mod dialogs;
pub mod font;
pub mod form;
pub mod input;
pub mod lang;
pub mod paint;
pub mod run;
pub mod theme;
pub mod ui;

pub use form::{DialogResult, Ev, Event, MouseButton};
pub use lang::{tr, trf};
pub use theme::Style;
pub use ui::{App, Ui};
