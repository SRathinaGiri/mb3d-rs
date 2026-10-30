object LogFrm: TLogFrm
  Left = 200
  Top = 150
  Caption = 'Log'
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
    Caption = 'Search'
  end
  object Edit1: TEdit
    Left = 50
    Top = 8
    Width = 220
    Height = 21
    Hint = 'Only the lines that contain these words'
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
    Caption = 'Show'
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
      'Everything'
      'Warnings and errors'
      'Errors')
  end
  object ComboBox3: TComboBox
    Left = 476
    Top = 8
    Width = 160
    Height = 21
    Hint = 'Only one kind of event'
    Style = csDropDownList
    ItemHeight = 13
    ItemIndex = 0
    ParentShowHint = False
    ShowHint = True
    TabOrder = 2
    Text = 'All kinds'
    OnChange = ComboBox3Change
    Items.Strings = (
      'All kinds'
      'What you did'
      'Calculations'
      'Messages'
      'Files'
      'Windows'
      'Program output'
      'Sessions and crashes')
  end
  object Label7: TLabel
    Left = 650
    Top = 12
    Width = 20
    Height = 13
    Caption = 'Day'
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
      'This session (live)')
  end
  object ListView1: TListView
    Left = 8
    Top = 36
    Width = 924
    Height = 400
    Anchors = [akLeft, akTop, akRight, akBottom]
    Columns = <
      item
        Caption = 'When (UTC)'
        Width = 150
      end
      item
        Caption = 'Level'
        Width = 56
      end
      item
        Caption = 'Program'
        Width = 64
      end
      item
        Caption = 'Kind'
        Width = 72
      end
      item
        Caption = 'What happened'
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
    Hint = 'Keep the newest line in sight as new ones come'
    Anchors = [akLeft, akBottom]
    Caption = 'Follow the newest'
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
    Hint = 'Copy the selected line, or all the lines shown when none is selected'
    Anchors = [akRight, akBottom]
    Caption = 'Copy'
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
    Hint = 'Open the folder of the log files (one a day, kept 30 days)'
    Anchors = [akRight, akBottom]
    Caption = 'Open log folder'
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
    Caption = 'Close'
    TabOrder = 9
    OnClick = Button3Click
  end
end
