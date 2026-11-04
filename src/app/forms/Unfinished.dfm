object UnfinishedFrm: TUnfinishedFrm
  Left = 200
  Top = 160
  BorderStyle = bsToolWindow
  Caption = '@MB:UnfinishedRenders'
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
    Caption = '@MB:AStoppedOrInterruptedRenderKeeps'
  end
  object ListView1: TListView
    Left = 8
    Top = 38
    Width = 624
    Height = 186
    Columns = <
      item
        Caption = '@MB:Name2'
        Width = 190
      end
      item
        Caption = '@MB:Kind'
        Width = 60
      end
      item
        Caption = '@MB:Size'
        Width = 90
      end
      item
        Caption = '@MB:Done'
        Width = 54
      end
      item
        Caption = '@MB:Saved'
        Width = 150
      end
      item
        Caption = '@MB:MB2'
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
    Hint = '@MB:LoadTheParametersOfTheSelected'
    Caption = '@MB:Continue'
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
    Hint = '@MB:LoadTheParametersOfTheSelected2'
    Caption = '@MB:LoadParameters'
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
    Hint = '@MB:DeleteWhatTheSelectedRenderKept'
    Caption = '@MB:Delete'
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
    Hint = '@MB:DeleteWhatAllUnfinishedRendersKept'
    Caption = '@MB:DeleteAll'
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
    Hint = '@MB:WritesThePictureAsItIs'
    Caption = '@MB:SaveM3i'
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
