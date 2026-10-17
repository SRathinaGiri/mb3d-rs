object SwarmFrm: TSwarmFrm
  Left = 200
  Top = 160
  BorderStyle = bsToolWindow
  Caption = 'Swarm: rendering shared between your computers'
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
    Caption = 'Shared folder:'
  end
  object Edit1: TEdit
    Left = 100
    Top = 8
    Width = 440
    Height = 21
    Hint = 'A folder every computer of the swarm can see: a NAS share, a Windows or Samba share, a sync folder'
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
    Caption = 'Browse...'
    TabOrder = 1
    OnClick = Button1Click
  end
  object Label2: TLabel
    Left = 8
    Top = 40
    Width = 88
    Height = 13
    Caption = 'This computer:'
  end
  object Edit2: TEdit
    Left = 100
    Top = 36
    Width = 160
    Height = 21
    Hint = 'How this computer is listed in the swarm'
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
    Caption = 'Threads for jobs (0 = all cores):'
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
    Hint = 'Render the jobs other computers put into the folder, and give this computer''s renders (Calculate 3D, Big renders, Monte Carlo, animations) to the swarm when another computer is in it'
    Caption = 'Share this computer: render the jobs of the folder, and give mine to the swarm'
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
    Caption = 'Calculate 3D and the Batch window with the swarm, in'
  end
  object Edit5: TEdit
    Left = 432
    Top = 86
    Width = 34
    Height = 21
    Hint = 'Columns of tiles'
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
    Hint = 'Rows of tiles'
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
    Caption = 'tiles'
  end
  object Label5: TLabel
    Left = 8
    Top = 120
    Width = 100
    Height = 13
    Caption = 'In the swarm:'
  end
  object ListView1: TListView
    Left = 8
    Top = 136
    Width = 624
    Height = 170
    Columns = <
      item
        Caption = 'Computer'
        Width = 120
      end
      item
        Caption = 'Version'
        Width = 56
      end
      item
        Caption = 'Cores'
        Width = 44
      end
      item
        Caption = 'Graphics card'
        Width = 160
      end
      item
        Caption = 'Doing'
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
    Caption = 'Jobs:'
  end
  object ListView2: TListView
    Left = 8
    Top = 330
    Width = 624
    Height = 70
    Columns = <
      item
        Caption = 'Job'
        Width = 220
      end
      item
        Caption = 'Kind'
        Width = 80
      end
      item
        Caption = 'By'
        Width = 140
      end
      item
        Caption = 'Progress'
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
    Hint = 'The newest job so far: rendered parts, parts under way (with their progress), missing parts'
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
