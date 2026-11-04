object ExperimentalFrm: TExperimentalFrm
  Left = 240
  Top = 180
  BorderStyle = bsDialog
  Caption = '@MB:ExperimentalSpeedUps'
  ClientHeight = 478
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
  object Label1: TLabel
    Left = 12
    Top = 10
    Width = 496
    Height = 58
    AutoSize = False
    Caption = '@MB:FasterWaysOfCalculatingThatChange'
    WordWrap = True
  end
  object OnBox: TCheckBox
    Left = 12
    Top = 76
    Width = 496
    Height = 17
    Caption = '@MB:UseTheExperimentalSpeedUps'
    Font.Style = [fsBold]
    ParentFont = False
    TabOrder = 0
    OnClick = OnBoxClick
  end
  object RelaxLabel: TLabel
    Left = 30
    Top = 111
    Width = 100
    Height = 13
    Caption = '@MB:StepRelax'
  end
  object RelaxEdit: TEdit
    Left = 140
    Top = 108
    Width = 50
    Height = 21
    TabOrder = 1
    Text = '1'
    OnChange = RelaxEditChange
  end
  object RelaxNote: TLabel
    Left = 200
    Top = 106
    Width = 308
    Height = 44
    AutoSize = False
    Caption = '@MB:LongerMarchSteps1Off1'
    WordWrap = True
  end
  object SecantBox: TCheckBox
    Left = 30
    Top = 160
    Width = 478
    Height = 17
    Caption = '@MB:SecantSearchForTheSurface'
    TabOrder = 2
    OnClick = OnBoxClick
  end
  object SecantNote: TLabel
    Left = 48
    Top = 180
    Width = 460
    Height = 38
    AutoSize = False
    Caption = '@MB:FindsTheSurfaceWithFewerDistance'
    WordWrap = True
  end
  object FreeBox: TCheckBox
    Left = 30
    Top = 226
    Width = 478
    Height = 17
    Caption = '@MB:NoStepLimitWithoutDynamicFog'
    TabOrder = 3
    OnClick = OnBoxClick
  end
  object FreeNote: TLabel
    Left = 48
    Top = 246
    Width = 460
    Height = 38
    AutoSize = False
    Caption = '@MB:LongerStepsThroughOpenSpaceWhen'
    WordWrap = True
  end
  object HalfBox: TCheckBox
    Left = 30
    Top = 292
    Width = 478
    Height = 17
    Caption = '@MB:DEOcclusionAtHalfResolution'
    TabOrder = 4
    OnClick = OnBoxClick
  end
  object HalfNote: TLabel
    Left = 48
    Top = 312
    Width = 460
    Height = 38
    AutoSize = False
    Caption = '@MB:CalculatesTheDEAmbientOcclusionOn'
    WordWrap = True
  end
  object ConeBox: TCheckBox
    Left = 30
    Top = 358
    Width = 478
    Height = 17
    Caption = '@MB:ConeMarchThePixelsOfA'
    TabOrder = 5
    OnClick = OnBoxClick
  end
  object ConeNote: TLabel
    Left = 48
    Top = 378
    Width = 460
    Height = 52
    AutoSize = False
    Caption = '@MB:ACoarseRayPerBlockOf'
    WordWrap = True
  end
  object CloseBtn: TButton
    Left = 418
    Top = 440
    Width = 90
    Height = 26
    Caption = '@MB:Close'
    TabOrder = 6
    OnClick = CloseBtnClick
  end
end
