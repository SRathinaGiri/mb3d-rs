//! The "Connect to an AI assistant" window (`AiConnectFrm`, this program's
//! own, the Prefs tab's "AI assistant" button): which Claude apps are on
//! this computer and whether mb3d is connected to them; Claude's desktop
//! app is connected or disconnected with a click (its settings file, see
//! `crate::mcp`), Claude Code's command is shown to copy (the program
//! starts no other programs); a few things to ask, the output folder and
//! the limits.

use super::{Mb3d, Ui};
use crate::vcl::lang::{tr, trf};
use crate::vcl::Event;

const F: &str = "AiConnectFrm";

/// Opens the window (the Prefs tab's button).
pub fn open(app: &mut Mb3d, ui: &mut Ui) {
    refresh(app, ui);
    ui.show(F);
}

fn refresh(app: &mut Mb3d, ui: &mut Ui) {
    let exe = crate::mcp::cli_program();
    // Claude's desktop app
    let (d_inst, d_conn) = crate::mcp::claude_desktop_status();
    let d_text = match (d_inst, d_conn) {
        (true, true) => tr("Connected. Ask it in a new conversation; restart the app if mb3d does not show among its tools."),
        (true, false) => tr("Found on this computer, not connected yet."),
        (false, _) if cfg!(target_os = "linux") => tr("Not available for Linux: use Claude Code below."),
        (false, _) => tr("Not installed. Get it from claude.ai/download, start it once, then come back here."),
    };
    ui.set_caption(F, "Label2", &d_text);
    ui.set_caption(F, "Button1", &if d_conn { tr("Disconnect") } else { tr("Connect") });
    ui.set_enabled(F, "Button1", d_inst && exe.is_ok());
    // Claude Code
    let (c_inst, c_conn) = crate::mcp::claude_code_status();
    let c_text = match (c_inst, c_conn) {
        (_, true) => tr("Connected. Start a new Claude Code session to use it. To take it out again, run the command below."),
        (true, false) => tr("Found on this computer. Run this command once in a terminal, then start a new Claude Code session:"),
        (false, false) => tr("Not found. If you use it, run this command once in a terminal:"),
    };
    ui.set_caption(F, "Label3", &c_text);
    let cmd = exe.as_ref().map_err(|e| e.clone()).and_then(|p| crate::mcp::claude_code_command(c_conn, p)).unwrap_or_else(|e| e);
    ui.set_text(F, "Edit1", &cmd);
    // things to try, where it writes, the limits
    let tries = [
        tr("Show me a preview of the last picture I rendered in mb3d."),
        tr("Make the light of that picture warmer and render it again."),
        tr("Make three variations of it and tell me which one you like best."),
        tr("Render my last picture at 4K and tell me how long it took."),
        tr("What did I do in mb3d today? Were there any errors?"),
    ];
    ui.set_text(F, "Memo1", &tries.iter().map(|t| format!("•  {t}")).collect::<Vec<_>>().join("\n"));
    let o = crate::mcp::Options::default();
    let note = trf(
        "Pictures and files the assistant makes go into {}; nothing is deleted. Pictures larger than 4K are made only after you agreed, and a calculation stops after 10 minutes. Everything it does is in the log.",
        &[&o.output.display()],
    );
    let note = match &exe {
        Ok(_) => note,
        Err(e) => format!("{note}\n{e}"),
    };
    ui.set_caption(F, "Label5", &note);
    let _ = app;
}

pub fn event(app: &mut Mb3d, ui: &mut Ui, e: &Event) {
    match e.handler.as_str() {
        "FormShow" => refresh(app, ui),
        "Button1Click" => {
            let (_, connected) = crate::mcp::claude_desktop_status();
            let r = crate::mcp::cli_program().and_then(|exe| crate::mcp::claude_desktop(connected, &exe));
            match r {
                Ok(t) => app.message(ui, &t),
                Err(e) => app.message(ui, &e),
            }
            refresh(app, ui);
        }
        "Button2Click" => {
            crate::vcl::clipboard::set(&ui.text(F, "Edit1"));
            app.message(ui, &tr("Copied to the clipboard."));
        }
        "Button3Click" => ui.hide(F),
        _ => {}
    }
}
