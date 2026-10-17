//! The form files of the original Mandelbulb3D (Delphi), unchanged.

pub const ALL: &[(&str, &str)] = &[
    ("Animation", include_str!("forms/Animation.dfm")),
    ("AniPreviewWindow", include_str!("forms/AniPreviewWindow.dfm")),
    ("AniProcess", include_str!("forms/AniProcess.dfm")),
    ("BatchForm", include_str!("forms/BatchForm.dfm")),
    ("BRInfoWindow", include_str!("forms/BRInfoWindow.dfm")),
    ("BulbTracer2UI", include_str!("forms/BulbTracer2UI.dfm")),
    ("ColorOptionForm", include_str!("forms/ColorOptionForm.dfm")),
    ("ColorPick", include_str!("forms/ColorPick.dfm")),
    ("FormulaGUI", include_str!("forms/FormulaGUI.dfm")),
    ("FormulaParser", include_str!("forms/FormulaParser.dfm")),
    ("HeightMapGenUI", include_str!("forms/HeightMapGenUI.dfm")),
    ("IniDirsForm", include_str!("forms/IniDirsForm.dfm")),
    ("JITFormulaEditGUI", include_str!("forms/JITFormulaEditGUI.dfm")),
    ("LightAdjust", include_str!("forms/LightAdjust.dfm")),
    ("Mand", include_str!("forms/Mand.dfm")),
    ("MapSequencesGUI", include_str!("forms/MapSequencesGUI.dfm")),
    ("MeshPreviewUI", include_str!("forms/MeshPreviewUI.dfm")),
    ("MonteCarloForm", include_str!("forms/MonteCarloForm.dfm")),
    ("MutaGenGUI", include_str!("forms/MutaGenGUI.dfm")),
    ("Navigator", include_str!("forms/Navigator.dfm")),
    ("ParamValueEditGUI", include_str!("forms/ParamValueEditGUI.dfm")),
    ("PostProcessForm", include_str!("forms/PostProcessForm.dfm")),
    ("ScriptUI", include_str!("forms/ScriptUI.dfm")),
    ("TextBox", include_str!("forms/TextBox.dfm")),
    ("Tiling", include_str!("forms/Tiling.dfm")),
    ("uMapCalcWindow", include_str!("forms/uMapCalcWindow.dfm")),
    ("VisualStylesGUI", include_str!("forms/VisualStylesGUI.dfm")),
    ("Swarm", include_str!("forms/Swarm.dfm")),
    ("VisualThemesGUI", include_str!("forms/VisualThemesGUI.dfm")),
    ("VoxelExport", include_str!("forms/VoxelExport.dfm")),
    ("ZBuf16BitGenUI", include_str!("forms/ZBuf16BitGenUI.dfm")),
];

pub fn get(name: &str) -> &'static str {
    ALL.iter().find(|(n, _)| n.eq_ignore_ascii_case(name)).map(|(_, t)| *t).unwrap_or_else(|| panic!("no form file {name}"))
}
