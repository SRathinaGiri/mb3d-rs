object SwarmFrm: TSwarmFrm
  Left = 200
  Top = 160
  BorderStyle = bsToolWindow
  Caption = '@MB:SwarmRenderingSharedBetweenYourComputers'
  ClientHeight = 424
  ClientWidth = 1000
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
    Width = 88
    Height = 13
    Caption = '@MB:SharedFolder'
  end
  object Edit1: TEdit
    Left = 100
    Top = 8
    Width = 440
    Height = 21
    Hint = '@MB:AFolderEveryComputerOfThe'
    ParentShowHint = False
    ShowHint = True
    TabOrder = 0
    OnChange = Edit1Change
  end
  object Button1: TButton
    Left = 548
    Top = 6
    Width = 84
    Height = 25
    Caption = '@MB:Browse'
    TabOrder = 1
    OnClick = Button1Click
  end
  object Label2: TLabel
    Left = 8
    Top = 40
    Width = 88
    Height = 13
    Caption = '@MB:ThisComputer'
  end
  object Edit2: TEdit
    Left = 100
    Top = 36
    Width = 160
    Height = 21
    Hint = '@MB:HowThisComputerIsListedIn'
    ParentShowHint = False
    ShowHint = True
    TabOrder = 2
    OnChange = Edit2Change
  end
  object Label3: TLabel
    Left = 280
    Top = 40
    Width = 150
    Height = 13
    Caption = '@MB:ThreadsForJobs0AllCores'
  end
  object Edit3: TEdit
    Left = 440
    Top = 36
    Width = 50
    Height = 21
    TabOrder = 3
    OnChange = Edit3Change
  end
  object CheckBox1: TCheckBox
    Left = 100
    Top = 64
    Width = 520
    Height = 17
    Hint = '@MB:RenderTheJobsOtherComputersPut'
    Caption = '@MB:ShareThisComputerRenderTheJobs'
    ParentShowHint = False
    ShowHint = True
    TabOrder = 4
    OnClick = CheckBox1Click
  end
  object Label11: TLabel
    Left = 100
    Top = 90
    Width = 330
    Height = 13
    Caption = '@MB:Calculate3DAndTheBatchWindow'
  end
  object Edit5: TEdit
    Left = 432
    Top = 86
    Width = 34
    Height = 21
    Hint = '@MB:ColumnsOfTiles'
    ParentShowHint = False
    ShowHint = True
    TabOrder = 8
    OnChange = Edit5Change
  end
  object Label9: TLabel
    Left = 470
    Top = 90
    Width = 10
    Height = 13
    Caption = 'x'
  end
  object Edit6: TEdit
    Left = 482
    Top = 86
    Width = 34
    Height = 21
    Hint = '@MB:RowsOfTiles'
    ParentShowHint = False
    ShowHint = True
    TabOrder = 9
    OnChange = Edit6Change
  end
  object Label10: TLabel
    Left = 520
    Top = 90
    Width = 40
    Height = 13
    Caption = '@MB:Tiles'
  end
  object Label5: TLabel
    Left = 8
    Top = 120
    Width = 100
    Height = 13
    Caption = '@MB:InTheSwarm'
  end
  object ListView1: TListView
    Left = 8
    Top = 136
    Width = 624
    Height = 170
    Columns = <
      item
        Caption = '@MB:Computer'
        Width = 120
      end
      item
        Caption = '@MB:Version'
        Width = 56
      end
      item
        Caption = '@MB:Cores'
        Width = 44
      end
      item
        Caption = '@MB:GraphicsCard'
        Width = 160
      end
      item
        Caption = '@MB:Doing'
        Width = 236
      end>
    ColumnClick = False
    GridLines = True
    ReadOnly = True
    RowSelect = True
    TabOrder = 11
    ViewStyle = vsReport
  end
  object Label6: TLabel
    Left = 8
    Top = 314
    Width = 100
    Height = 13
    Caption = '@MB:Jobs'
  end
  object ListView2: TListView
    Left = 8
    Top = 330
    Width = 624
    Height = 70
    Columns = <
      item
        Caption = '@MB:Job'
        Width = 220
      end
      item
        Caption = '@MB:Kind'
        Width = 80
      end
      item
        Caption = '@MB:By'
        Width = 140
      end
      item
        Caption = '@MB:Progress'
        Width = 176
      end>
    ColumnClick = False
    GridLines = True
    ReadOnly = True
    RowSelect = True
    TabOrder = 12
    ViewStyle = vsReport
  end
  object Label7: TLabel
    Left = 8
    Top = 404
    Width = 624
    Height = 13
    AutoSize = False
    Caption = ''
  end
  object Image1: TImage
    Left = 648
    Top = 8
    Width = 344
    Height = 380
    Hint = '@MB:TheNewestJobSoFarRendered'
    ParentShowHint = False
    ShowHint = True
  end
  object Label8: TLabel
    Left = 648
    Top = 392
    Width = 344
    Height = 26
    AutoSize = False
    WordWrap = True
    Caption = ''
  end
  object Timer1: TTimer
    Enabled = False
    Interval = 2000
    OnTimer = Timer1Timer
    Left = 592
    Top = 136
  end
end
