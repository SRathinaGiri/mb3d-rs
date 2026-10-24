object LanguageFrm: TLanguageFrm
  Left = 148
  Top = 170
  BorderStyle = bsToolWindow
  Caption = 'Interface language'
  ClientHeight = 290
  ClientWidth = 460
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
  object Label1: TLabel
    Left = 8
    Top = 8
    Width = 200
    Height = 13
    AutoSize = False
    Caption = 'Interface language:'
  end
  object LanguagesList: TListBox
    Left = 8
    Top = 26
    Width = 200
    Height = 190
    Hint = 'Click a language to switch the windows to it'
    ItemHeight = 16
    ParentShowHint = False
    ShowHint = True
    TabOrder = 0
    OnClick = LanguagesListClick
  end
  object SaveAndExitBtn: TButton
    Left = 8
    Top = 259
    Width = 200
    Height = 25
    Hint = 'Keeps the chosen language for the next start'
    Caption = 'Save + exit'
    ParentShowHint = False
    ShowHint = True
    TabOrder = 1
    OnClick = SaveAndExitBtnClick
  end
  object InfoLabel: TLabel
    Left = 216
    Top = 26
    Width = 236
    Height = 44
    AutoSize = False
    WordWrap = True
    Caption = ''
  end
  object Label3: TLabel
    Left = 216
    Top = 72
    Width = 236
    Height = 13
    AutoSize = False
    Caption = 'Folder of the language files:'
  end
  object FolderEdit: TEdit
    Left = 216
    Top = 88
    Width = 236
    Height = 21
    ReadOnly = True
    TabOrder = 2
  end
  object Label2: TLabel
    Left = 216
    Top = 118
    Width = 236
    Height = 13
    AutoSize = False
    Caption = 'Your own language (its name):'
  end
  object NameEdit: TEdit
    Left = 216
    Top = 134
    Width = 236
    Height = 21
    Hint = 'The name of the new language, in that language (Deutsch, Polski, ...)'
    ParentShowHint = False
    ShowHint = True
    TabOrder = 3
  end
  object CreateBtn: TButton
    Left = 216
    Top = 161
    Width = 236
    Height = 25
    Hint = 'Writes the English texts as a new language file of this name; translate them with a text editor, then press Reload'
    Caption = 'Create language file'
    ParentShowHint = False
    ShowHint = True
    TabOrder = 4
    OnClick = CreateBtnClick
  end
  object OpenFolderBtn: TButton
    Left = 216
    Top = 192
    Width = 115
    Height = 25
    Hint = 'Opens the folder of the language files'
    Caption = 'Open folder'
    ParentShowHint = False
    ShowHint = True
    TabOrder = 5
    OnClick = OpenFolderBtnClick
  end
  object ReloadBtn: TButton
    Left = 337
    Top = 192
    Width = 115
    Height = 25
    Hint = 'Reads the language files again, after you edited one'
    Caption = 'Reload'
    ParentShowHint = False
    ShowHint = True
    TabOrder = 6
    OnClick = ReloadBtnClick
  end
  object NoteLabel: TLabel
    Left = 216
    Top = 224
    Width = 236
    Height = 40
    AutoSize = False
    WordWrap = True
    Caption = 'A language file has one text per line: left of the "=" its place or the English text, right of it your text.'
  end
end
