object SplatFrm: TSplatFrm
  Left = 260
  Top = 160
  BorderStyle = bsDialog
  Caption = '@MB:GaussianSplats'
  ClientHeight = 378
  ClientWidth = 520
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
  object IntroLabel: TLabel
    Left = 12
    Top = 10
    Width = 496
    Height = 58
    AutoSize = False
    Caption = '@MB:TheFractalOfTheMainWindow'
    WordWrap = True
  end
  object AroundLabel: TLabel
    Left = 12
    Top = 80
    Width = 150
    Height = 13
    Caption = '@MB:CalculateFrom'
  end
  object AroundCmb: TComboBox
    Left = 170
    Top = 76
    Width = 338
    Height = 21
    Style = csDropDownList
    TabOrder = 0
    Items.Strings = (
      '@MB:Auto'
      '@MB:AroundTheMiddleAnObject'
      '@MB:FromTheCameraAllAroundA')
  end
  object ViewsLabel: TLabel
    Left = 12
    Top = 110
    Width = 150
    Height = 13
    Caption = '@MB:Directions'
  end
  object ViewsEdit: TEdit
    Left = 170
    Top = 106
    Width = 60
    Height = 21
    TabOrder = 1
    Text = '16'
  end
  object SizeLabel: TLabel
    Left = 12
    Top = 140
    Width = 150
    Height = 13
    Caption = '@MB:SizeOfEachView'
  end
  object SizeEdit: TEdit
    Left = 170
    Top = 136
    Width = 60
    Height = 21
    TabOrder = 2
    Text = '640'
  end
  object SizeNote: TLabel
    Left = 240
    Top = 134
    Width = 268
    Height = 34
    AutoSize = False
    Caption = '@MB:PixelsLargerKeepsMoreDetailAnd'
    WordWrap = True
  end
  object ShLabel: TLabel
    Left = 12
    Top = 180
    Width = 150
    Height = 13
    Caption = '@MB:ColoursByDirection'
  end
  object ShCmb: TComboBox
    Left = 170
    Top = 176
    Width = 150
    Height = 21
    Style = csDropDownList
    TabOrder = 3
    Items.Strings = (
      '@MB:OffSmallestFile'
      '1'
      '2'
      '3')
  end
  object ShNote: TLabel
    Left = 12
    Top = 204
    Width = 496
    Height = 32
    AutoSize = False
    Caption = '@MB:HighlightsMoveAsInThePictures'
    WordWrap = True
  end
  object FileLabel: TLabel
    Left = 12
    Top = 248
    Width = 150
    Height = 13
    Caption = '@MB:OutputFile'
  end
  object FileEdit: TEdit
    Left = 170
    Top = 244
    Width = 300
    Height = 21
    TabOrder = 4
  end
  object FileBtn: TButton
    Left = 474
    Top = 243
    Width = 34
    Height = 23
    Hint = '@MB:ClickToChooseTheOutputFile'
    Caption = '...'
    ParentShowHint = False
    ShowHint = True
    TabOrder = 5
    OnClick = FileBtnClick
  end
  object ProgressBar: TProgressBar
    Left = 12
    Top = 280
    Width = 496
    Height = 17
    Max = 1000
    Smooth = True
    TabOrder = 6
  end
  object StatusLabel: TLabel
    Left = 12
    Top = 302
    Width = 496
    Height = 30
    AutoSize = False
    WordWrap = True
  end
  object CalcBtn: TButton
    Left = 12
    Top = 340
    Width = 110
    Height = 26
    Caption = '@MB:Calculate'
    TabOrder = 7
    OnClick = CalcBtnClick
  end
  object StopBtn: TButton
    Left = 128
    Top = 340
    Width = 90
    Height = 26
    Caption = '@MB:Stop'
    Enabled = False
    TabOrder = 8
    OnClick = StopBtnClick
  end
  object CloseBtn: TButton
    Left = 418
    Top = 340
    Width = 90
    Height = 26
    Caption = '@MB:Close'
    TabOrder = 9
    OnClick = CloseBtnClick
  end
end
