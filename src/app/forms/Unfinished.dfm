object UnfinishedFrm: TUnfinishedFrm
  Left = 200
  Top = 160
  BorderStyle = bsToolWindow
  Caption = 'Unfinished renders'
  ClientHeight = 268
  ClientWidth = 640
  Color = clBtnFace
  Font.Charset = DEFAULT_CHARSET
  Font.Color = clWindowText
  Font.Height = -11
  Font.Name = 'MS Sans Serif'
  Font.Style = []
  OldCreateOrder = False
  Position = poScreenCenter
  OnShow = FormShow
  PixelsPerInch = 96
  TextHeight = 13
  object Label1: TLabel
    Left = 8
    Top = 8
    Width = 624
    Height = 26
    AutoSize = False
    WordWrap = True
    Caption =
      'A stopped or interrupted render keeps what it calculated; Calcul' +
      'ate 3D of the same parameters continues it.'
  end
  object ListView1: TListView
    Left = 8
    Top = 38
    Width = 624
    Height = 186
    Columns = <
      item
        Caption = 'Name'
        Width = 190
      end
      item
        Caption = 'Kind'
        Width = 60
      end
      item
        Caption = 'Size'
        Width = 90
      end
      item
        Caption = 'Done'
        Width = 54
      end
      item
        Caption = 'Saved'
        Width = 150
      end
      item
        Caption = 'MB'
        Width = 60
      end>
    ColumnClick = False
    GridLines = True
    ReadOnly = True
    RowSelect = True
    TabOrder = 0
    ViewStyle = vsReport
  end
  object Button1: TButton
    Left = 8
    Top = 234
    Width = 90
    Height = 25
    Hint = 'Load the parameters of the selected render and continue its calculation'
    Caption = 'Continue'
    ParentShowHint = False
    ShowHint = True
    TabOrder = 1
    OnClick = Button1Click
  end
  object Button2: TButton
    Left = 106
    Top = 234
    Width = 90
    Height = 25
    Hint = 'Load the parameters of the selected render only'
    Caption = 'Load parameters'
    ParentShowHint = False
    ShowHint = True
    TabOrder = 2
    OnClick = Button2Click
  end
  object Button3: TButton
    Left = 204
    Top = 234
    Width = 90
    Height = 25
    Hint = 'Delete what the selected render kept'
    Caption = 'Delete'
    ParentShowHint = False
    ShowHint = True
    TabOrder = 3
    OnClick = Button3Click
  end
  object Button4: TButton
    Left = 302
    Top = 234
    Width = 90
    Height = 25
    Hint = 'Delete what all unfinished renders kept'
    Caption = 'Delete all'
    ParentShowHint = False
    ShowHint = True
    TabOrder = 4
    OnClick = Button4Click
  end
  object Button5: TButton
    Left = 400
    Top = 234
    Width = 90
    Height = 25
    Hint = 'Writes the picture as it is now (the rows not done as background) as a .m3i file'
    Caption = 'Save m3i'
    ParentShowHint = False
    ShowHint = True
    TabOrder = 5
    OnClick = Button5Click
  end
  object Label2: TLabel
    Left = 498
    Top = 240
    Width = 134
    Height = 13
    AutoSize = False
    Caption = ''
  end
end
