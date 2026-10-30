object BatchForm1: TBatchForm1
  Left = 197
  Top = 91
  Caption = '@MB:BatchProcessing'
  ClientHeight = 539
  ClientWidth = 565
  Color = clBtnFace
  Constraints.MinHeight = 200
  Constraints.MinWidth = 360
  Font.Charset = DEFAULT_CHARSET
  Font.Color = clWindowText
  Font.Height = -11
  Font.Name = 'MS Sans Serif'
  Font.Style = []
  OldCreateOrder = False
  OnCreate = FormCreate
  PixelsPerInch = 96
  TextHeight = 13
  object ListView1: TListView
    Left = 0
    Top = 0
    Width = 565
    Height = 462
    Align = alClient
    Checkboxes = True
    Columns = <
      item
        AutoSize = True
        Caption = '@MB:M3PFilesThatShouldBeRendered'
      end
      item
        Caption = '@MB:Status'
        Width = 100
      end>
    ColumnClick = False
    GridLines = True
    HotTrack = True
    MultiSelect = True
    ReadOnly = True
    RowSelect = True
    PopupMenu = PopupMenu1
    ShowWorkAreas = True
    TabOrder = 0
    ViewStyle = vsReport
    OnChange = ListView1Change
    OnInsert = ListView1Insert
    OnKeyDown = ListView1KeyDown
    ExplicitHeight = 470
  end
  object Panel1: TPanel
    Left = 0
    Top = 462
    Width = 565
    Height = 77
    Align = alBottom
    ParentShowHint = False
    ShowHint = True
    TabOrder = 1
    ExplicitTop = 468
    object Button1: TButton
      Left = 201
      Top = 43
      Width = 126
      Height = 25
      Hint = '@MB:StartsRenderingAllSelectedM3PFiles'
      Caption = '@MB:StartBatchRendering'
      ParentShowHint = False
      ShowHint = False
      TabOrder = 0
      OnClick = Button1Click
    end
    object CheckBox1: TCheckBox
      Left = 12
      Top = 7
      Width = 153
      Height = 17
      Hint = '@MB:SelectThisIfYouWantTo'
      Caption = '@MB:OverwriteExistingM3IFiles'
      Checked = True
      Enabled = False
      State = cbChecked
      TabOrder = 1
    end
    object Button2: TButton
      Left = 477
      Top = 47
      Width = 81
      Height = 25
      Caption = '@MB:CloseWindow'
      ParentShowHint = False
      ShowHint = False
      TabOrder = 2
      OnClick = Button2Click
    end
    object Button3: TButton
      Left = 201
      Top = 6
      Width = 126
      Height = 25
      Hint = '@MB:ClickToSelectOneAndMore'
      Caption = '@MB:OpenFiles'
      TabOrder = 3
      OnClick = Button3Click
    end
    object CheckBox2: TCheckBox
      Left = 12
      Top = 29
      Width = 161
      Height = 17
      Hint = '@MB:IfSelectedTheM3IFilesWill'
      Caption = '@MB:StoreM3IFileInDefaultFolder'
      TabOrder = 4
    end
    object Button4: TButton
      Left = 344
      Top = 6
      Width = 126
      Height = 25
      Hint = '@MB:ClickToOpenTheExplorerAnd'
      Caption = '@MB:OpenExplorer'
      TabOrder = 5
      OnClick = Button4Click
    end
    object SavePNGPreviewCBx: TCheckBox
      Left = 12
      Top = 51
      Width = 161
      Height = 17
      Hint = '@MB:StoreAnAdditionalCorrespondingPNGFile'
      Caption = '@MB:StorePNGPreview'
      Checked = True
      State = cbChecked
      TabOrder = 6
    end
  end
  object PopupMenu1: TPopupMenu
    Left = 88
    Top = 48
    object Deleteselectedfilesfromlist1: TMenuItem
      Caption = '@MB:DeleteHighlightedFilesFromTheList'
      OnClick = Deleteselectedfilesfromlist1Click
    end
    object Clearthewholelist1: TMenuItem
      Caption = '@MB:ClearTheWholeList'
      OnClick = Clearthewholelist1Click
    end
  end
end
