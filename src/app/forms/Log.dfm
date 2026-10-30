object LogFrm: TLogFrm
  Left = 200
  Top = 150
  Caption = '@MB:Log'
  ClientHeight = 560
  ClientWidth = 940
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
    Top = 12
    Width = 36
    Height = 13
    Caption = '@MB:Search'
  end
  object Edit1: TEdit
    Left = 50
    Top = 8
    Width = 220
    Height = 21
    Hint = '@MB:OnlyTheLinesThatContainThese'
    ParentShowHint = False
    ShowHint = True
    TabOrder = 0
    OnChange = Edit1Change
  end
  object Label3: TLabel
    Left = 284
    Top = 12
    Width = 28
    Height = 13
    Caption = '@MB:Show'
  end
  object ComboBox1: TComboBox
    Left = 318
    Top = 8
    Width = 150
    Height = 21
    Style = csDropDownList
    ItemHeight = 13
    ItemIndex = 0
    TabOrder = 1
    Text = 'Everything'
    OnChange = ComboBox1Change
    Items.Strings = (
      '@MB:Everything'
      '@MB:WarningsAndErrors'
      '@MB:Errors')
  end
  object ComboBox3: TComboBox
    Left = 476
    Top = 8
    Width = 160
    Height = 21
    Hint = '@MB:OnlyOneKindOfEvent'
    Style = csDropDownList
    ItemHeight = 13
    ItemIndex = 0
    ParentShowHint = False
    ShowHint = True
    TabOrder = 2
    Text = 'All kinds'
    OnChange = ComboBox3Change
    Items.Strings = (
      '@MB:AllKinds'
      '@MB:WhatYouDid'
      '@MB:Calculations'
      '@MB:Messages'
      '@MB:Files'
      '@MB:Windows'
      '@MB:ProgramOutput'
      '@MB:SessionsAndCrashes')
  end
  object Label7: TLabel
    Left = 650
    Top = 12
    Width = 20
    Height = 13
    Caption = '@MB:Day'
  end
  object ComboBox2: TComboBox
    Left = 676
    Top = 8
    Width = 256
    Height = 21
    Anchors = [akLeft, akTop, akRight]
    Style = csDropDownList
    ItemHeight = 13
    ItemIndex = 0
    TabOrder = 3
    Text = 'This session (live)'
    OnChange = ComboBox2Change
    Items.Strings = (
      '@MB:ThisSessionLive')
  end
  object ListView1: TListView
    Left = 8
    Top = 36
    Width = 924
    Height = 400
    Anchors = [akLeft, akTop, akRight, akBottom]
    Columns = <
      item
        Caption = '@MB:WhenUTC'
        Width = 150
      end
      item
        Caption = '@MB:Level'
        Width = 56
      end
      item
        Caption = '@MB:Program'
        Width = 64
      end
      item
        Caption = '@MB:Kind'
        Width = 72
      end
      item
        Caption = '@MB:WhatHappened'
        Width = 560
      end>
    GridLines = True
    ReadOnly = True
    RowSelect = True
    TabOrder = 4
    ViewStyle = vsReport
    OnChange = ListView1Change
  end
  object Memo1: TMemo
    Left = 8
    Top = 442
    Width = 924
    Height = 70
    Anchors = [akLeft, akRight, akBottom]
    ReadOnly = True
    ScrollBars = ssVertical
    TabOrder = 5
  end
  object Label5: TLabel
    Left = 8
    Top = 528
    Width = 260
    Height = 13
    AutoSize = False
    Anchors = [akLeft, akBottom]
    Caption = ''
  end
  object CheckBox1: TCheckBox
    Left = 276
    Top = 526
    Width = 150
    Height = 17
    Hint = '@MB:KeepTheNewestLineInSight'
    Anchors = [akLeft, akBottom]
    Caption = '@MB:FollowTheNewest'
    Checked = True
    State = cbChecked
    ParentShowHint = False
    ShowHint = True
    TabOrder = 6
    OnClick = CheckBox1Click
  end
  object Button1: TButton
    Left = 560
    Top = 520
    Width = 110
    Height = 28
    Hint = '@MB:CopyTheSelectedLineOrAll'
    Anchors = [akRight, akBottom]
    Caption = '@MB:Copy'
    ParentShowHint = False
    ShowHint = True
    TabOrder = 7
    OnClick = Button1Click
  end
  object Button2: TButton
    Left = 676
    Top = 520
    Width = 140
    Height = 28
    Hint = '@MB:OpenTheFolderOfTheLog'
    Anchors = [akRight, akBottom]
    Caption = '@MB:OpenLogFolder'
    ParentShowHint = False
    ShowHint = True
    TabOrder = 8
    OnClick = Button2Click
  end
  object Button3: TButton
    Left = 822
    Top = 520
    Width = 110
    Height = 28
    Anchors = [akRight, akBottom]
    Caption = '@MB:Close'
    TabOrder = 9
    OnClick = Button3Click
  end
end
