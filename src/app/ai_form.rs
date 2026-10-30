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
        (true, true) => tr("@MB:ConnectedAskItInANew"),
        (true, false) => tr("@MB:FoundOnThisComputerNotConnected"),
        (false, _) if cfg!(target_os = "linux") => tr("@MB:NotAvailableForLinuxUseClaude"),
        (false, _) => tr("@MB:NotInstalledGetItFromClaude"),
    };
    ui.set_caption(F, "Label2", &d_text);
    ui.set_caption(F, "Button1", &if d_conn { tr("@MB:Disconnect") } else { tr("@MB:Connect") });
    ui.set_enabled(F, "Button1", d_inst && exe.is_ok());
    // Claude Code
    let (c_inst, c_conn) = crate::mcp::claude_code_status();
    let c_text = match (c_inst, c_conn) {
        (_, true) => tr("@MB:ConnectedStartANewClaudeCode"),
        (true, false) => tr("@MB:FoundOnThisComputerRunThis"),
        (false, false) => tr("@MB:NotFoundIfYouUseIt"),
    };
    ui.set_caption(F, "Label3", &c_text);
    // (the error of a missing mb3d in the window's language)
    let missing = tr("@MB:TheCommandLineProgramMb3dWas");
    let cmd = exe.as_ref().ok().and_then(|p| crate::mcp::claude_code_command(c_conn, p).ok()).unwrap_or_else(|| missing.clone());
    ui.set_text(F, "Edit1", &cmd);
    // things to try, where it writes, the limits
    let tries = [
        tr("@MB:ShowMeAPreviewOfThe"),
        tr("@MB:MakeTheLightOfThatPicture"),
        tr("@MB:MakeThreeVariationsOfItAnd"),
        tr("@MB:RenderMyLastPictureAt4K"),
        tr("@MB:WhatDidIDoInMb3d"),
    ];
    ui.set_text(F, "Memo1", &tries.iter().map(|t| format!("•  {t}")).collect::<Vec<_>>().join("\n"));
    let o = crate::mcp::Options::default();
    let note = trf(
        "@MB:PicturesAndFilesTheAssistantMakes",
        &[&o.output.display()],
    );
    let note = match &exe {
        Ok(_) => note,
        Err(_) => format!("{note}\n{missing}"),
    };
    ui.set_caption(F, "Label5", &note);
    let _ = app;
}

pub fn event(app: &mut Mb3d, ui: &mut Ui, e: &Event) {
    match e.handler.as_str() {
        "FormShow" => refresh(app, ui),
        "Button1Click" => {
            let (_, connected) = crate::mcp::claude_desktop_status();
            let r = crate::mcp::cli_program().map_err(|_| tr("@MB:TheCommandLineProgramMb3dWas")).and_then(|exe| crate::mcp::claude_desktop(connected, &exe));
            match r {
                Ok(t) => app.message(ui, &t),
                Err(e) => app.message(ui, &e),
            }
            refresh(app, ui);
        }
        "Button2Click" => {
            crate::vcl::clipboard::set(&ui.text(F, "Edit1"));
            app.message(ui, &tr("@MB:CopiedToTheClipboard"));
        }
        "Button3Click" => ui.hide(F),
        _ => {}
    }
}
