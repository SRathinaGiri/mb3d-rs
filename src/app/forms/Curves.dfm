object CurvesFrm: TCurvesFrm
  Left = 220
  Top = 160
  Caption = '@MB:AnimationCurves'
  ClientHeight = 380
  ClientWidth = 660
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
  object ParamCB: TComboBox
    Left = 8
    Top = 8
    Width = 170
    Height = 21
    Style = csDropDownList
    TabOrder = 0
    OnChange = ParamCBChange
  end
  object KeyLbl: TLabel
    Left = 196
    Top = 12
    Width = 120
    Height = 13
    Caption = ''
  end
  object KeyValEdit: TEdit
    Left = 330
    Top = 8
    Width = 110
    Height = 21
    Hint = '@MB:TheValueOfTheChosenKeyframe'
    ParentShowHint = False
    ShowHint = True
    TabOrder = 1
    Text = ''
  end
  object SetBtn: TButton
    Left = 446
    Top = 8
    Width = 60
    Height = 21
    Caption = '@MB:Set'
    TabOrder = 2
    OnClick = SetBtnClick
  end
  object MaxLbl: TLabel
    Left = 8
    Top = 38
    Width = 80
    Height = 13
    Caption = ''
  end
  object MinLbl: TLabel
    Left = 8
    Top = 358
    Width = 80
    Height = 13
    Caption = ''
  end
  object Image1: TImage
    Left = 96
    Top = 38
    Width = 556
    Height = 334
    Hint = '@MB:TheChosenValueOverAllFrames'
    ParentShowHint = False
    ShowHint = True
    OnMouseDown = Image1MouseDown
    OnMouseMove = Image1MouseMove
    OnMouseUp = Image1MouseUp
  end
end
