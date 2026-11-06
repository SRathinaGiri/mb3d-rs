object AnimTemplatesFrm: TAnimTemplatesFrm
  Left = 260
  Top = 200
  BorderStyle = bsDialog
  Caption = '@MB:AnimationTemplates2'
  ClientHeight = 340
  ClientWidth = 470
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
    Width = 446
    Height = 30
    AutoSize = False
    Caption = '@MB:KeyframesMadeForYouFromThe'
    WordWrap = True
  end
  object TemplateLabel: TLabel
    Left = 12
    Top = 51
    Width = 150
    Height = 13
    Caption = '@MB:Template'
  end
  object TemplateCmb: TComboBox
    Left = 170
    Top = 48
    Width = 288
    Height = 21
    Style = csDropDownList
    TabOrder = 0
    OnChange = TemplateCmbChange
    Items.Strings = (
      '@MB:N360Orbit'
      '@MB:FlyThrough'
      '@MB:FlyToTheView')
  end
  object NoteLabel: TLabel
    Left = 12
    Top = 76
    Width = 446
    Height = 44
    AutoSize = False
    WordWrap = True
  end
  object DurationLabel: TLabel
    Left = 12
    Top = 131
    Width = 150
    Height = 13
    Caption = '@MB:DurationSeconds'
  end
  object DurationEdit: TEdit
    Left = 170
    Top = 128
    Width = 60
    Height = 21
    TabOrder = 1
    Text = '10'
    OnChange = DurationEditChange
  end
  object FramesLabel: TLabel
    Left = 240
    Top = 131
    Width = 218
    Height = 13
    AutoSize = False
  end
  object AxisLabel: TLabel
    Left = 12
    Top = 163
    Width = 150
    Height = 13
    Caption = '@MB:Axis'
  end
  object AxisCmb: TComboBox
    Left = 170
    Top = 160
    Width = 288
    Height = 21
    Style = csDropDownList
    TabOrder = 9
    Items.Strings = (
      '@MB:LeftToRightAroundItYaw'
      '@MB:OverTheTopAndUnderPitch'
      '@MB:SpinningAroundTheLineOfSight'
      '@MB:AllTogetherYawPitchAndRoll'
      '@MB:TheFractalSXAxis'
      '@MB:TheFractalSYAxis'
      '@MB:TheFractalSZAxis')
  end
  object StepsLabel: TLabel
    Left = 12
    Top = 195
    Width = 150
    Height = 13
    Caption = '@MB:Keyframes'
  end
  object StepsEdit: TEdit
    Left = 170
    Top = 192
    Width = 60
    Height = 21
    TabOrder = 2
    Text = '8'
  end
  object PivotLabel: TLabel
    Left = 12
    Top = 227
    Width = 150
    Height = 13
    Caption = '@MB:Around'
  end
  object PivotCmb: TComboBox
    Left = 170
    Top = 224
    Width = 288
    Height = 21
    Style = csDropDownList
    TabOrder = 3
    Items.Strings = (
      '@MB:AutoTheMiddleOrTheSurface'
      '@MB:TheMiddleOfTheView')
  end
  object UntilCheck: TCheckBox
    Left = 12
    Top = 162
    Width = 446
    Height = 17
    Caption = '@MB:UntilTheViewShowsOnlySky'
    TabOrder = 4
    OnClick = UntilCheckClick
  end
  object SpeedLabel: TLabel
    Left = 12
    Top = 195
    Width = 200
    Height = 13
    Caption = '@MB:SpeedOfTheDistanceAFrame'
  end
  object SpeedEdit: TEdit
    Left = 220
    Top = 192
    Width = 60
    Height = 21
    TabOrder = 5
    Text = '4'
  end
  object TurnLabel: TLabel
    Left = 12
    Top = 227
    Width = 200
    Height = 13
    Caption = '@MB:LargestTurnDegreesAFrame'
  end
  object TurnEdit: TEdit
    Left = 220
    Top = 224
    Width = 60
    Height = 21
    TabOrder = 6
    Text = '2'
  end
  object StartLabel: TLabel
    Left = 12
    Top = 259
    Width = 150
    Height = 13
    Caption = '@MB:StartFrom'
  end
  object StartCmb: TComboBox
    Left = 170
    Top = 256
    Width = 288
    Height = 21
    Style = csDropDownList
    TabOrder = 10
    Items.Strings = (
      '@MB:TheMainWindowSView'
      '@MB:TheLastKeyframe')
  end
  object FromCmb: TComboBox
    Left = 170
    Top = 256
    Width = 288
    Height = 21
    Style = csDropDownList
    TabOrder = 11
    Visible = False
    Items.Strings = (
      '@MB:OutsideTheFractal'
      '@MB:TheLastKeyframe')
  end
  object FlightBar: TProgressBar
    Left = 12
    Top = 284
    Width = 446
    Height = 10
    Visible = False
  end
  object FlightLabel: TLabel
    Left = 12
    Top = 306
    Width = 176
    Height = 13
    AutoSize = False
  end
  object AddBtn: TButton
    Left = 196
    Top = 300
    Width = 160
    Height = 25
    Caption = '@MB:AddTheKeyframes'
    Default = True
    TabOrder = 7
    OnClick = AddBtnClick
  end
  object CloseBtn: TButton
    Left = 364
    Top = 300
    Width = 94
    Height = 25
    Cancel = True
    Caption = '@MB:Close'
    TabOrder = 8
    OnClick = CloseBtnClick
  end
end
