object HotkeysFrm: THotkeysFrm
  Left = 148
  Top = 170
  BorderStyle = bsToolWindow
  Caption = 'Hotkeys'
  ClientHeight = 318
  ClientWidth = 300
  Color = clBtnFace
  Font.Charset = DEFAULT_CHARSET
  Font.Color = clWindowText
  Font.Height = -11
  Font.Name = 'MS Sans Serif'
  Font.Style = []
  FormStyle = fsStayOnTop
  OldCreateOrder = False
  Position = poScreenCenter
  OnShow = FormShow
  PixelsPerInch = 96
  TextHeight = 13
  object InfoLabel: TLabel
    Left = 8
    Top = 8
    Width = 284
    Height = 42
    AutoSize = False
    WordWrap = True
    Caption = 'Brings the window to the front from any window, or closes the current one. Click a field and press the keys; Backspace takes the hotkey away.'
  end
  object Label1: TLabel
    Left = 8
    Top = 57
    Width = 130
    Height = 13
    AutoSize = False
    Caption = 'Main window:'
  end
  object Edit1: TEdit
    Tag = 0
    Left = 144
    Top = 54
    Width = 148
    Height = 21
    Hint = 'Click and press the keys of the hotkey'
    ParentShowHint = False
    ReadOnly = True
    ShowHint = True
    TabOrder = 0
    OnKeyDown = HotkeyEditKeyDown
  end
  object Label2: TLabel
    Left = 8
    Top = 85
    Width = 130
    Height = 13
    AutoSize = False
    Caption = 'Navigator:'
  end
  object Edit2: TEdit
    Tag = 1
    Left = 144
    Top = 82
    Width = 148
    Height = 21
    Hint = 'Click and press the keys of the hotkey'
    ParentShowHint = False
    ReadOnly = True
    ShowHint = True
    TabOrder = 1
    OnKeyDown = HotkeyEditKeyDown
  end
  object Label3: TLabel
    Left = 8
    Top = 113
    Width = 130
    Height = 13
    AutoSize = False
    Caption = 'Animation:'
  end
  object Edit3: TEdit
    Tag = 2
    Left = 144
    Top = 110
    Width = 148
    Height = 21
    Hint = 'Click and press the keys of the hotkey'
    ParentShowHint = False
    ReadOnly = True
    ShowHint = True
    TabOrder = 2
    OnKeyDown = HotkeyEditKeyDown
  end
  object Label4: TLabel
    Left = 8
    Top = 141
    Width = 130
    Height = 13
    AutoSize = False
    Caption = 'Formulas:'
  end
  object Edit4: TEdit
    Tag = 3
    Left = 144
    Top = 138
    Width = 148
    Height = 21
    Hint = 'Click and press the keys of the hotkey'
    ParentShowHint = False
    ReadOnly = True
    ShowHint = True
    TabOrder = 3
    OnKeyDown = HotkeyEditKeyDown
  end
  object Label5: TLabel
    Left = 8
    Top = 169
    Width = 130
    Height = 13
    AutoSize = False
    Caption = 'Lighting:'
  end
  object Edit5: TEdit
    Tag = 4
    Left = 144
    Top = 166
    Width = 148
    Height = 21
    Hint = 'Click and press the keys of the hotkey'
    ParentShowHint = False
    ReadOnly = True
    ShowHint = True
    TabOrder = 4
    OnKeyDown = HotkeyEditKeyDown
  end
  object Label6: TLabel
    Left = 8
    Top = 197
    Width = 130
    Height = 13
    AutoSize = False
    Caption = 'Post processing:'
  end
  object Edit6: TEdit
    Tag = 5
    Left = 144
    Top = 194
    Width = 148
    Height = 21
    Hint = 'Click and press the keys of the hotkey'
    ParentShowHint = False
    ReadOnly = True
    ShowHint = True
    TabOrder = 5
    OnKeyDown = HotkeyEditKeyDown
  end
  object Label7: TLabel
    Left = 8
    Top = 225
    Width = 130
    Height = 13
    AutoSize = False
    Caption = 'Monte Carlo:'
  end
  object Edit7: TEdit
    Tag = 6
    Left = 144
    Top = 222
    Width = 148
    Height = 21
    Hint = 'Click and press the keys of the hotkey'
    ParentShowHint = False
    ReadOnly = True
    ShowHint = True
    TabOrder = 6
    OnKeyDown = HotkeyEditKeyDown
  end
  object Label8: TLabel
    Left = 8
    Top = 253
    Width = 130
    Height = 13
    AutoSize = False
    Caption = 'Close the current window:'
  end
  object Edit8: TEdit
    Tag = 7
    Left = 144
    Top = 250
    Width = 148
    Height = 21
    Hint = 'Click and press the keys of the hotkey'
    ParentShowHint = False
    ReadOnly = True
    ShowHint = True
    TabOrder = 7
    OnKeyDown = HotkeyEditKeyDown
  end
  object DefaultsBtn: TButton
    Left = 8
    Top = 287
    Width = 137
    Height = 25
    Hint = 'Ctrl+1 to Ctrl+7 in the order of the list, Ctrl+W to close'
    Caption = 'Defaults'
    ParentShowHint = False
    ShowHint = True
    TabOrder = 8
    OnClick = DefaultsBtnClick
  end
  object SaveAndExitBtn: TButton
    Left = 155
    Top = 287
    Width = 137
    Height = 25
    Hint = 'Keeps the hotkeys for the next start'
    Caption = 'Save + exit'
    ParentShowHint = False
    ShowHint = True
    TabOrder = 9
    OnClick = SaveAndExitBtnClick
  end
end
