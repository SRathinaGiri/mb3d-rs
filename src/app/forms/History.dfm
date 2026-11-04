object HistoryFrm: THistoryFrm
  Left = 180
  Top = 140
  Caption = '@MB:History'
  ClientHeight = 540
  ClientWidth = 900
  Color = clBtnFace
  Font.Charset = DEFAULT_CHARSET
  Font.Color = clWindowText
  Font.Height = -11
  Font.Name = 'MS Sans Serif'
  Font.Style = []
  OldCreateOrder = False
  Position = poScreenCenter
  OnShow = FormShow
  OnResize = FormResize
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
    Width = 230
    Height = 21
    Hint = '@MB:OnlyTheEntriesWhoseNameFolder'
    ParentShowHint = False
    ShowHint = True
    TabOrder = 0
    OnChange = Edit1Change
  end
  object Label3: TLabel
    Left = 296
    Top = 12
    Width = 28
    Height = 13
    Caption = '@MB:Show'
  end
  object ComboBox1: TComboBox
    Left = 330
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
      '@MB:CalculatedPictures'
      '@MB:Parameters'
      '@MB:Images'
      '@MB:Animations'
      '@MB:Meshes2'
      '@MB:OtherFiles')
  end
  object Label7: TLabel
    Left = 496
    Top = 12
    Width = 24
    Height = 13
    Caption = '@MB:View'
  end
  object ComboBox2: TComboBox
    Left = 526
    Top = 8
    Width = 112
    Height = 21
    Style = csDropDownList
    ItemHeight = 13
    ItemIndex = 0
    TabOrder = 7
    Text = 'List'
    OnChange = ComboBox2Change
    Items.Strings = (
      '@MB:List'
      '@MB:Rows'
      '@MB:Tiles2')
  end
  object CheckBox1: TCheckBox
    Left = 212
    Top = 504
    Width = 92
    Height = 17
    Hint = '@MB:OfEntriesWhoseFilesAreThe'
    Anchors = [akLeft, akBottom]
    Caption = '@MB:ShowUnique'
    ParentShowHint = False
    ShowHint = True
    TabOrder = 9
    OnClick = CheckBox1Click
  end
  object CheckBox2: TCheckBox
    Left = 308
    Top = 504
    Width = 110
    Height = 17
    Hint = '@MB:TheEntriesUnderTheirDaysA'
    Anchors = [akLeft, akBottom]
    Caption = '@MB:GroupByDay'
    Checked = True
    State = cbChecked
    ParentShowHint = False
    ShowHint = True
    TabOrder = 11
    OnClick = CheckBox2Click
  end
  object TrackBar1: TTrackBar
    Left = 430
    Top = 500
    Width = 208
    Height = 24
    Hint = '@MB:TheSizeOfTheTiles'
    Anchors = [akLeft, akRight, akBottom]
    Max = 4
    Position = 2
    TabOrder = 10
    ThumbLength = 14
    TickStyle = tsNone
    Visible = False
    OnChange = TrackBar1Change
  end
  object Label4: TLabel
    Left = 430
    Top = 506
    Width = 208
    Height = 13
    AutoSize = False
    Alignment = taRightJustify
    Anchors = [akLeft, akRight, akBottom]
    Caption = '@MB:ClickAColumnHeaderToSort'
  end
  object Image2: TImage
    Left = 8
    Top = 36
    Width = 612
    Height = 460
    Anchors = [akLeft, akTop, akRight, akBottom]
    PopupMenu = PopupMenu1
    Visible = False
    OnDblClick = Image2DblClick
    OnMouseDown = Image2MouseDown
    OnMouseWheel = Image2MouseWheel
  end
  object ScrollBar1: TScrollBar
    Left = 622
    Top = 36
    Width = 16
    Height = 460
    Anchors = [akTop, akRight, akBottom]
    Kind = sbVertical
    PageSize = 0
    TabOrder = 8
    Visible = False
    OnChange = ScrollBar1Change
    OnScroll = ScrollBar1Scroll
  end
  object ListView1: TListView
    Left = 8
    Top = 36
    Width = 630
    Height = 460
    Anchors = [akLeft, akTop, akRight, akBottom]
    Columns = <
      item
        Caption = '@MB:When'
        Width = 124
      end
      item
        Caption = '@MB:Action'
        Width = 66
      end
      item
        Caption = '@MB:Kind'
        Width = 76
      end
      item
        Caption = '@MB:Name2'
        Width = 190
      end
      item
        Caption = '@MB:Size'
        Width = 62
      end
      item
        Caption = '@MB:Folder'
        Width = 200
      end>
    ColumnClick = True
    GridLines = True
    ReadOnly = True
    RowSelect = True
    PopupMenu = PopupMenu1
    TabOrder = 2
    ViewStyle = vsReport
    OnChange = ListView1Change
    OnClick = ListView1Click
    OnColumnClick = ListView1ColumnClick
    OnDblClick = ListView1DblClick
  end
  object Image1: TImage
    Left = 648
    Top = 36
    Width = 240
    Height = 180
    Anchors = [akTop, akRight]
    Center = True
  end
  object Label6: TLabel
    Left = 648
    Top = 222
    Width = 240
    Height = 40
    AutoSize = False
    WordWrap = True
    Anchors = [akTop, akRight]
    Caption = ''
  end
  object Edit2: TEdit
    Left = 648
    Top = 268
    Width = 240
    Height = 21
    Hint = '@MB:TheFileSelectAndCopyIt'
    Anchors = [akTop, akRight]
    ParentShowHint = False
    ReadOnly = True
    ShowHint = True
    TabOrder = 6
  end
  object Label2: TLabel
    Left = 648
    Top = 296
    Width = 240
    Height = 60
    AutoSize = False
    WordWrap = True
    Anchors = [akTop, akRight]
    Caption = ''
  end
  object Button1: TButton
    Left = 648
    Top = 440
    Width = 240
    Height = 25
    Hint = '@MB:LoadTheParametersIntoTheMain'
    Anchors = [akRight, akBottom]
    Caption = '@MB:Load'
    ParentShowHint = False
    ShowHint = True
    TabOrder = 3
    OnClick = Button1Click
  end
  object Button2: TButton
    Left = 648
    Top = 471
    Width = 117
    Height = 25
    Hint = '@MB:OpenTheFolderOfTheFile'
    Anchors = [akRight, akBottom]
    Caption = '@MB:ShowInFolder'
    ParentShowHint = False
    ShowHint = True
    TabOrder = 4
    OnClick = Button2Click
  end
  object Button3: TButton
    Left = 771
    Top = 471
    Width = 117
    Height = 25
    Hint = '@MB:TakeTheEntryOutOfThe'
    Anchors = [akRight, akBottom]
    Caption = '@MB:Remove'
    ParentShowHint = False
    ShowHint = True
    TabOrder = 5
    OnClick = Button3Click
  end
  object Label5: TLabel
    Left = 8
    Top = 506
    Width = 200
    Height = 13
    AutoSize = False
    Anchors = [akLeft, akBottom]
    Caption = ''
  end
  object PopupMenu1: TPopupMenu
    Left = 560
    Top = 480
    object Load1: TMenuItem
      Caption = '@MB:Load'
      OnClick = Button1Click
    end
    object Showinfolder1: TMenuItem
      Caption = '@MB:ShowInFolder'
      OnClick = Button2Click
    end
    object Remove1: TMenuItem
      Caption = '@MB:Remove'
      OnClick = Button3Click
    end
  end
end
