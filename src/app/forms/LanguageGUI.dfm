object LanguageFrm: TLanguageFrm
  Left = 148
  Top = 170
  BorderStyle = bsToolWindow
  Caption = '@MB:InterfaceLanguage'
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
    Caption = '@MB:InterfaceLanguageColon'
  end
  object LanguagesList: TListBox
    Left = 8
    Top = 26
    Width = 200
    Height = 190
    Hint = '@MB:ClickALanguageToSwitchThe'
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
    Hint = '@MB:KeepsTheChosenLanguageForThe'
    Caption = '@MB:SaveExit'
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
    Caption = '@MB:FolderOfTheLanguageFiles'
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
    Caption = '@MB:YourOwnLanguageItsName'
  end
  object NameEdit: TEdit
    Left = 216
    Top = 134
    Width = 236
    Height = 21
    Hint = '@MB:TheNameOfTheNewLanguage'
    ParentShowHint = False
    ShowHint = True
    TabOrder = 3
  end
  object CreateBtn: TButton
    Left = 216
    Top = 161
    Width = 236
    Height = 25
    Hint = '@MB:WritesTheEnglishTextsAsA'
    Caption = '@MB:CreateLanguageFile'
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
    Hint = '@MB:OpensTheFolderOfTheLanguage'
    Caption = '@MB:OpenFolder'
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
    Hint = '@MB:ReadsTheLanguageFilesAgainAfter'
    Caption = '@MB:Reload'
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
    Caption = '@MB:ALanguageFileHasOneLabel'
  end
end
