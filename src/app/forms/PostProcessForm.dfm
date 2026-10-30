object PostProForm: TPostProForm
  Left = 844
  Top = 100
  BorderStyle = bsDialog
  Caption = '@MB:PostProcessing'
  ClientHeight = 1352
  ClientWidth = 255
  Color = clBtnFace
  Font.Charset = DEFAULT_CHARSET
  Font.Color = clWindowText
  Font.Height = -11
  Font.Name = 'MS Sans Serif'
  Font.Style = []
  OldCreateOrder = False
  OnActivate = FormActivate
  OnCreate = FormCreate
  OnHide = FormHide
  OnMouseWheel = FormMouseWheel
  OnShow = FormShow
  PixelsPerInch = 96
  TextHeight = 13
  object NormalsOnZBufferBtn: TSpeedButton
    Left = 0
    Top = 122
    Width = 255
    Height = 21
    Hint = '@MB:ClickToHideShow'
    Align = alTop
    AllowAllUp = True
    GroupIndex = 2
    Caption = '@MB:NormalsOnZBuffer'
    ParentShowHint = False
    ShowHint = True
    OnClick = NormalsOnZBufferBtnClick
    ExplicitTop = 8
  end
  object HardShadowsBtn: TSpeedButton
    Left = 0
    Top = 208
    Width = 255
    Height = 21
    Hint = '@MB:ClickToHideShow'
    Align = alTop
    AllowAllUp = True
    GroupIndex = 3
    Caption = '@MB:HardShadows'
    ParentShowHint = False
    ShowHint = True
    OnClick = HardShadowsBtnClick
    ExplicitLeft = 8
  end
  object AmbientShadowsBtn: TSpeedButton
    Left = 0
    Top = 422
    Width = 255
    Height = 21
    Hint = '@MB:ClickToHideShow'
    Align = alTop
    AllowAllUp = True
    GroupIndex = 4
    Caption = '@MB:AmbientShadows'
    ParentShowHint = False
    ShowHint = True
    OnClick = AmbientShadowsBtnClick
    ExplicitLeft = 21
    ExplicitTop = 431
  end
  object ReflTransparencyBtn: TSpeedButton
    Left = 0
    Top = 658
    Width = 255
    Height = 21
    Hint = '@MB:ClickToHideShow'
    Align = alTop
    AllowAllUp = True
    GroupIndex = 5
    Caption = '@MB:ReflectionsTransparency'
    ParentShowHint = False
    ShowHint = True
    OnClick = ReflTransparencyBtnClick
    ExplicitLeft = 8
    ExplicitTop = 679
  end
  object DepthOfFieldBtn: TSpeedButton
    Left = 0
    Top = 891
    Width = 255
    Height = 21
    Hint = '@MB:ClickToHideShow'
    Align = alTop
    AllowAllUp = True
    GroupIndex = 6
    Caption = '@MB:DepthOfField'
    ParentShowHint = False
    ShowHint = True
    OnClick = DepthOfFieldBtnClick
    ExplicitLeft = 8
    ExplicitTop = 897
  end
  object RecalcSectionBtn: TSpeedButton
    Left = 0
    Top = 0
    Width = 255
    Height = 21
    Hint = '@MB:ClickToHideShow'
    Align = alTop
    AllowAllUp = True
    GroupIndex = 1
    Caption = '@MB:RecalculateASelection'
    ParentShowHint = False
    ShowHint = True
    OnClick = RecalcSectionBtnClick
    ExplicitLeft = -8
    ExplicitTop = -1
  end
  object PostProcessHintPnl: TPanel
    Left = 0
    Top = 613
    Width = 255
    Height = 45
    Align = alTop
    BevelOuter = bvNone
    TabOrder = 3
    object Label3: TLabel
      Left = 0
      Top = 0
      Width = 246
      Height = 39
      Margins.Left = 2
      Margins.Top = 1
      Margins.Right = 0
      Margins.Bottom = 0
      Align = alClient
      Caption = '@MB:CautionTheProcessingsBelowMustBe'
    end
    object Button16: TButton
      Left = 180
      Top = 27
      Width = 67
      Height = 17
      Hint = '@MB:ResetsTheDoFEffectAndThe'
      Caption = '@MB:ResetNow'
      ParentShowHint = False
      ShowHint = True
      TabOrder = 0
      OnClick = Button16Click
    end
  end
  object RecalcSectionPnl: TPanel
    Left = 0
    Top = 21
    Width = 255
    Height = 101
    Align = alTop
    BevelOuter = bvNone
    TabOrder = 4
    Visible = False
    object Label4: TLabel
      Left = 65
      Top = 49
      Width = 75
      Height = 13
      Alignment = taRightJustify
      Caption = '@MB:RaystepDivisor'
    end
    object Button13: TButton
      Left = 83
      Top = 76
      Width = 67
      Height = 21
      Hint = '@MB:MakeAnImageSelectionWithThe'
      Caption = '@MB:Calculate'
      Enabled = False
      ParentShowHint = False
      ShowHint = True
      TabOrder = 0
      OnClick = Button13Click
    end
    object CheckBox19: TCheckBox
      Left = 31
      Top = 7
      Width = 131
      Height = 17
      Hint = '@MB:ToReduceOverstepping'
      Alignment = taLeftJustify
      Caption = '@MB:KeepOnlyNearerParts'
      Color = clBtnFace
      Font.Charset = DEFAULT_CHARSET
      Font.Color = clWindowText
      Font.Height = -12
      Font.Name = 'MS Sans Serif'
      Font.Style = []
      ParentColor = False
      ParentFont = False
      ParentShowHint = False
      ShowHint = False
      TabOrder = 1
    end
    object CheckBox21: TCheckBox
      Tag = 1
      Left = 12
      Top = 78
      Width = 58
      Height = 17
      Alignment = taLeftJustify
      Caption = '@MB:Enable'
      Color = clBtnFace
      Font.Charset = DEFAULT_CHARSET
      Font.Color = clWindowText
      Font.Height = -12
      Font.Name = 'MS Sans Serif'
      Font.Style = [fsBold]
      ParentColor = False
      ParentFont = False
      ParentShowHint = False
      ShowHint = False
      TabOrder = 2
      OnClick = CheckBox21Click
    end
    object CheckBox30: TCheckBox
      Left = 6
      Top = 26
      Width = 156
      Height = 17
      Hint = '@MB:HelpfulIfYouWantToRedo'
      Alignment = taLeftJustify
      Caption = '@MB:DonTChangeAmbShDFog'
      ParentShowHint = False
      ShowHint = True
      TabOrder = 3
    end
    object Edit12: TEdit
      Left = 149
      Top = 46
      Width = 30
      Height = 21
      ParentShowHint = False
      ReadOnly = True
      ShowHint = True
      TabOrder = 4
      Text = '1'
    end
    object UpDown4: TUpDown
      Left = 179
      Top = 46
      Width = 16
      Height = 21
      Associate = Edit12
      Min = 1
      Max = 99
      Position = 1
      TabOrder = 5
    end
  end
  object NormalsOnZBufferPnl: TPanel
    Left = 0
    Top = 143
    Width = 255
    Height = 65
    Align = alTop
    BevelOuter = bvNone
    TabOrder = 5
    Visible = False
    object Label7: TLabel
      Left = 106
      Top = 34
      Width = 133
      Height = 26
      AutoSize = False
      Caption = '@MB:WarningMustRecalculateTheWholeImage'
      WordWrap = True
    end
    object Button14: TButton
      Left = 12
      Top = 37
      Width = 83
      Height = 21
      Hint = '@MB:UseOnlyAsSecondOptionIf'
      Caption = '@MB:CalculateNow'
      ParentShowHint = False
      ShowHint = True
      TabOrder = 0
      OnClick = Button14Click
    end
    object CheckBox23: TCheckBox
      Tag = 2
      Left = 27
      Top = 9
      Width = 191
      Height = 17
      Caption = '@MB:DoThisFunctionAutomatically'
      Color = clBtnFace
      Font.Charset = DEFAULT_CHARSET
      Font.Color = clWindowText
      Font.Height = -12
      Font.Name = 'MS Sans Serif'
      Font.Style = [fsBold]
      ParentColor = False
      ParentFont = False
      ParentShowHint = False
      ShowHint = False
      TabOrder = 1
      OnClick = CheckBox23Click
    end
  end
  object HardShadowsPnl: TPanel
    Left = 0
    Top = 229
    Width = 255
    Height = 193
    Align = alTop
    BevelOuter = bvNone
    TabOrder = 6
    Visible = False
    object Label6: TLabel
      Left = 29
      Top = 117
      Width = 36
      Height = 13
      Caption = '@MB:Radius'
    end
    object Label8: TLabel
      Left = 11
      Top = 37
      Width = 84
      Height = 13
      Caption = '@MB:MaxLengthCalc'
    end
    object Button3: TButton
      Left = 12
      Top = 170
      Width = 83
      Height = 21
      Hint = '@MB:PushTheButtonToCalculateThe'
      Caption = '@MB:CalculateNow'
      ParentShowHint = False
      ShowHint = True
      TabOrder = 0
      OnClick = Button3Click
    end
    object Button4: TButton
      Tag = 1
      Left = 185
      Top = 34
      Width = 41
      Height = 16
      Hint = '@MB:ResetsAPreviousCalculationOfThis'
      Caption = '@MB:Reset'
      ParentShowHint = False
      ShowHint = True
      TabOrder = 1
      OnClick = Button4Click
    end
    object Button5: TButton
      Tag = 2
      Left = 185
      Top = 57
      Width = 41
      Height = 17
      Hint = '@MB:ResetsAPreviousCalculationOfThis'
      Caption = '@MB:Reset'
      ParentShowHint = False
      ShowHint = True
      TabOrder = 2
      OnClick = Button4Click
    end
    object Button6: TButton
      Tag = 3
      Left = 185
      Top = 80
      Width = 41
      Height = 17
      Hint = '@MB:ResetsAPreviousCalculationOfThis'
      Caption = '@MB:Reset'
      ParentShowHint = False
      ShowHint = True
      TabOrder = 3
      OnClick = Button4Click
    end
    object Button7: TButton
      Tag = 4
      Left = 185
      Top = 103
      Width = 41
      Height = 17
      Hint = '@MB:ResetsAPreviousCalculationOfThis'
      Caption = '@MB:Reset'
      ParentShowHint = False
      ShowHint = True
      TabOrder = 4
      OnClick = Button4Click
    end
    object Button8: TButton
      Tag = 5
      Left = 185
      Top = 126
      Width = 41
      Height = 17
      Hint = '@MB:ResetsAPreviousCalculationOfThis'
      Caption = '@MB:Reset'
      ParentShowHint = False
      ShowHint = True
      TabOrder = 5
      OnClick = Button4Click
    end
    object Button9: TButton
      Tag = 6
      Left = 185
      Top = 149
      Width = 41
      Height = 17
      Hint = '@MB:ResetsAPreviousCalculationOfThis'
      Caption = '@MB:Reset'
      ParentShowHint = False
      ShowHint = True
      TabOrder = 6
      OnClick = Button4Click
    end
    object CheckBox10: TCheckBox
      Left = 125
      Top = 174
      Width = 122
      Height = 17
      Hint = '@MB:SetsAutomaticallyTheCorrespondendingLightFunctio'
      Caption = '@MB:SetLightfuncToCosine'
      Checked = True
      ParentShowHint = False
      ShowHint = True
      State = cbChecked
      TabOrder = 7
    end
    object CheckBox2: TCheckBox
      Left = 125
      Top = 34
      Width = 54
      Height = 17
      Caption = '@MB:Light1'
      Checked = True
      State = cbChecked
      TabOrder = 8
      OnClick = CheckBox2Click
    end
    object CheckBox29: TCheckBox
      Left = 12
      Top = 94
      Width = 72
      Height = 17
      Hint = '@MB:ToCalculateABetterHardShadow'
      Caption = '@MB:SofterHS'
      ParentShowHint = False
      ShowHint = True
      TabOrder = 9
    end
    object CheckBox3: TCheckBox
      Left = 125
      Top = 57
      Width = 54
      Height = 17
      Caption = '@MB:Light2'
      TabOrder = 10
      OnClick = CheckBox2Click
    end
    object CheckBox4: TCheckBox
      Left = 125
      Top = 80
      Width = 54
      Height = 17
      Caption = '@MB:Light3'
      TabOrder = 11
      OnClick = CheckBox2Click
    end
    object CheckBox5: TCheckBox
      Left = 125
      Top = 103
      Width = 54
      Height = 17
      Caption = '@MB:Light4'
      TabOrder = 12
      OnClick = CheckBox2Click
    end
    object CheckBox6: TCheckBox
      Left = 125
      Top = 126
      Width = 54
      Height = 17
      Caption = '@MB:Light5'
      TabOrder = 13
      OnClick = CheckBox2Click
    end
    object CheckBox7: TCheckBox
      Left = 125
      Top = 149
      Width = 54
      Height = 17
      Caption = '@MB:Light6'
      TabOrder = 14
      OnClick = CheckBox2Click
    end
    object CheckBox9: TCheckBox
      Tag = 3
      Left = 26
      Top = 6
      Width = 182
      Height = 17
      Hint = '@MB:IfEnabledTheChoosenHardShadows'
      Caption = '@MB:CalculateHSAutomatically'
      Color = clBtnFace
      Font.Charset = DEFAULT_CHARSET
      Font.Color = clWindowText
      Font.Height = -12
      Font.Name = 'MS Sans Serif'
      Font.Style = [fsBold]
      ParentColor = False
      ParentFont = False
      ParentShowHint = False
      ShowHint = True
      TabOrder = 15
      OnClick = CheckBox9Click
    end
    object Edit5: TEdit
      Left = 29
      Top = 56
      Width = 31
      Height = 21
      Hint = '@MB:AMultiplierThatDefinesTheMiximal'
      ParentShowHint = False
      ShowHint = True
      TabOrder = 16
      Text = '1.0'
    end
    object Edit7: TEdit
      Left = 29
      Top = 133
      Width = 31
      Height = 21
      Hint = '@MB:BiggerRadiusMeansSofterShadows'
      ParentShowHint = False
      ShowHint = True
      TabOrder = 17
      Text = '1.0'
    end
  end
  object AmbientShadowsPnl: TPanel
    Left = 0
    Top = 443
    Width = 255
    Height = 170
    Align = alTop
    BevelOuter = bvNone
    TabOrder = 7
    Visible = False
    object Label24: TLabel
      Left = 215
      Top = 15
      Width = 20
      Height = 20
      Hint = '@MB:SSAO15AmbientShadowBasedOnThe'
      Alignment = taCenter
      AutoSize = False
      Caption = '?'
      Font.Charset = DEFAULT_CHARSET
      Font.Color = clWindowText
      Font.Height = -16
      Font.Name = 'MS Sans Serif'
      Font.Style = [fsBold]
      ParentFont = False
      ParentShowHint = False
      ShowHint = True
    end
    object Button10: TButton
      Left = 11
      Top = 142
      Width = 83
      Height = 21
      Caption = '@MB:CalculateNow'
      ParentShowHint = False
      ShowHint = False
      TabOrder = 0
      OnClick = Button10Click
    end
    object CheckBox11: TCheckBox
      Tag = 4
      Left = 27
      Top = 9
      Width = 182
      Height = 17
      Hint = '@MB:IfEnabledTheAmbientShadowsWill'
      Caption = '@MB:CalculateASAutomatically'
      Checked = True
      Color = clBtnFace
      Font.Charset = DEFAULT_CHARSET
      Font.Color = clWindowText
      Font.Height = -12
      Font.Name = 'MS Sans Serif'
      Font.Style = [fsBold]
      ParentColor = False
      ParentFont = False
      ParentShowHint = False
      ShowHint = True
      State = cbChecked
      TabOrder = 1
      OnClick = CheckBox11Click
    end
    object TabControl1: TTabControl
      Left = 1
      Top = 32
      Width = 245
      Height = 106
      ParentShowHint = False
      ShowHint = True
      TabOrder = 2
      Tabs.Strings = (
        '@MB:SSAO15'
        '@MB:SSAO24'
        '@MB:SSAO24r'
        '@MB:DEAO')
      TabIndex = 1
      OnChange = RadioGroup3Click
      object Label14: TLabel
        Left = 10
        Top = 76
        Width = 85
        Height = 13
        Alignment = taRightJustify
        Caption = '@MB:CalculationCount'
        Font.Charset = DEFAULT_CHARSET
        Font.Color = clWindowText
        Font.Height = -12
        Font.Name = 'MS Sans Serif'
        Font.Style = []
        ParentFont = False
        Visible = False
      end
      object Label15: TLabel
        Left = 25
        Top = 74
        Width = 29
        Height = 13
        Caption = '@MB:MaxL'
        Visible = False
      end
      object Label16: TLabel
        Left = 97
        Top = 71
        Width = 27
        Height = 13
        Caption = '@MB:Rays'
        Visible = False
      end
      object Label17: TLabel
        Left = 127
        Top = 71
        Width = 6
        Height = 13
        Alignment = taCenter
        Caption = '7'
        Visible = False
      end
      object Label20: TLabel
        Left = 38
        Top = 31
        Width = 55
        Height = 13
        Alignment = taRightJustify
        Caption = '@MB:BorderSize'
        Font.Charset = DEFAULT_CHARSET
        Font.Color = clWindowText
        Font.Height = -12
        Font.Name = 'MS Sans Serif'
        Font.Style = []
        ParentFont = False
      end
      object Label50: TLabel
        Left = 20
        Top = 53
        Width = 73
        Height = 13
        Alignment = taRightJustify
        Caption = '@MB:ZRThreshold'
        Font.Charset = DEFAULT_CHARSET
        Font.Color = clWindowText
        Font.Height = -12
        Font.Name = 'MS Sans Serif'
        Font.Style = []
        ParentFont = False
      end
      object CheckBox12: TCheckBox
        Left = 142
        Top = 51
        Width = 90
        Height = 17
        Hint = '@MB:LetTheThresholdDecreaseTheAmbient'
        Alignment = taLeftJustify
        Caption = '@MB:ThresholdTo0'
        ParentShowHint = False
        ShowHint = True
        TabOrder = 1
      end
      object CheckBox22: TCheckBox
        Left = 154
        Top = 30
        Width = 68
        Height = 33
        Hint = '@MB:FirstStepRandomToAvoidSpotty'
        Alignment = taLeftJustify
        Caption = '@MB:FirstStepRandomColon'
        Color = clBtnFace
        ParentColor = False
        ParentShowHint = False
        ShowHint = True
        TabOrder = 0
        Visible = False
        WordWrap = True
      end
      object Edit21: TEdit
        Left = 99
        Top = 73
        Width = 36
        Height = 21
        ParentShowHint = False
        ReadOnly = True
        ShowHint = True
        TabOrder = 2
        Text = '1'
        Visible = False
      end
      object Edit34: TEdit
        Left = 99
        Top = 50
        Width = 36
        Height = 21
        Hint = '@MB:DefinesUpToWhichAngleThe'
        ParentShowHint = False
        ShowHint = True
        TabOrder = 3
        Text = '2.00'
      end
      object Edit8: TEdit
        Left = 59
        Top = 67
        Width = 29
        Height = 21
        Hint = '@MB:ScalesTheMaximumLengthOfThe'
        ParentShowHint = False
        ShowHint = True
        TabOrder = 4
        Text = '1.0'
        Visible = False
      end
      object Edit9: TEdit
        Left = 99
        Top = 28
        Width = 36
        Height = 21
        Hint = '@MB:TheCalculationWillBeExtendedTowards'
        ParentShowHint = False
        ShowHint = True
        TabOrder = 5
        Text = '0.0'
      end
      object RadioGroup5: TRadioGroup
        Left = 12
        Top = 27
        Width = 127
        Height = 33
        Hint = '@MB:OptionToDitherTheAngleOf'
        Caption = '@MB:DitheringForScale'
        Color = clBtnFace
        Columns = 3
        ItemIndex = 0
        Items.Strings = (
          '@MB:No'
          '1:2'
          '1:3')
        ParentBackground = False
        ParentColor = False
        ParentShowHint = False
        ShowHint = True
        TabOrder = 6
        Visible = False
      end
      object UpDown1: TUpDown
        Left = 143
        Top = 66
        Width = 18
        Height = 24
        Hint = '@MB:NumberOfRaysThatWillBe'
        Max = 3
        ParentShowHint = False
        Position = 1
        ShowHint = True
        TabOrder = 7
        Visible = False
        OnChangingEx = UpDown1ChangingEx
      end
      object UpDown3: TUpDown
        Left = 135
        Top = 73
        Width = 16
        Height = 21
        Hint = '@MB:CalculateNTimesToReduceNoise'
        Associate = Edit21
        Min = 1
        Max = 9
        ParentShowHint = False
        Position = 1
        ShowHint = True
        TabOrder = 8
        Visible = False
      end
    end
  end
  object ReflTransparencyPnl: TPanel
    Left = 0
    Top = 679
    Width = 255
    Height = 212
    Align = alTop
    BevelOuter = bvNone
    TabOrder = 8
    Visible = False
    object Label11: TLabel
      Left = 134
      Top = 39
      Width = 32
      Height = 13
      Alignment = taRightJustify
      Caption = '@MB:DepthColon'
    end
    object Label12: TLabel
      Left = 29
      Top = 39
      Width = 39
      Height = 13
      Alignment = taRightJustify
      Caption = '@MB:Amount'
    end
    object Label21: TLabel
      Left = 113
      Top = 91
      Width = 53
      Height = 13
      Alignment = taRightJustify
      Caption = '@MB:Absorption'
    end
    object Label22: TLabel
      Left = 86
      Top = 115
      Width = 80
      Height = 13
      Alignment = taRightJustify
      Caption = '@MB:RefractiveIndex'
    end
    object Label23: TLabel
      Left = 91
      Top = 139
      Width = 75
      Height = 13
      Alignment = taRightJustify
      Caption = '@MB:LightScattering'
    end
    object Button15: TButton
      Tag = 1
      Left = 11
      Top = 182
      Width = 83
      Height = 21
      Hint = '@MB:MakeSureThatTheCalcAutomatic'
      Caption = '@MB:CalculateNow'
      ParentShowHint = False
      ShowHint = True
      TabOrder = 0
      OnClick = Button11Click
    end
    object CheckBox24: TCheckBox
      Tag = 5
      Left = 27
      Top = 9
      Width = 204
      Height = 17
      Hint = '@MB:IfEnabledTheReflectionsWillBe'
      Caption = '@MB:CalculateRTAutomatically'
      Font.Charset = DEFAULT_CHARSET
      Font.Color = clWindowText
      Font.Height = -12
      Font.Name = 'MS Sans Serif'
      Font.Style = [fsBold]
      ParentFont = False
      ParentShowHint = False
      ShowHint = True
      TabOrder = 1
      OnClick = CheckBox24Click
    end
    object CheckBox25: TCheckBox
      Left = 12
      Top = 162
      Width = 207
      Height = 17
      Hint = '@MB:OptionToCalculateOnlyAPart'
      Caption = '@MB:CalculateOnlyASelectionInThe'
      ParentShowHint = False
      ShowHint = True
      TabOrder = 2
      OnClick = CheckBox25Click
    end
    object CheckBox27: TCheckBox
      Left = 12
      Top = 66
      Width = 135
      Height = 17
      Hint = '@MB:IfEnabledTheSpecularAmountIs'
      Caption = '@MB:CalculateTransparency'
      ParentShowHint = False
      ShowHint = True
      TabOrder = 3
    end
    object CheckBox28: TCheckBox
      Left = 12
      Top = 89
      Width = 66
      Height = 17
      Hint = '@MB:CalculateTransparencyOnlyOnDIFSFormulas'
      Caption = '@MB:OnlyDIFS'
      ParentShowHint = False
      ShowHint = True
      TabOrder = 4
    end
    object Edit11: TEdit
      Left = 171
      Top = 36
      Width = 30
      Height = 21
      ParentShowHint = False
      ReadOnly = True
      ShowHint = True
      TabOrder = 5
      Text = '2'
    end
    object Edit13: TEdit
      Left = 171
      Top = 88
      Width = 52
      Height = 21
      Hint = '@MB:AbsoluteAbsorptionAmountInsideTheMaterial'
      ParentShowHint = False
      ShowHint = True
      TabOrder = 6
      Text = '0.2'
    end
    object Edit14: TEdit
      Left = 171
      Top = 112
      Width = 52
      Height = 21
      Hint = '@MB:RefractiveIndexOfTheMaterialAir'
      ParentShowHint = False
      ShowHint = True
      TabOrder = 7
      Text = '1.5'
    end
    object Edit17: TEdit
      Left = 171
      Top = 136
      Width = 52
      Height = 21
      Hint = '@MB:LightScatteringInsideTheMaterial'
      ParentShowHint = False
      ShowHint = True
      TabOrder = 8
      Text = '1.0'
    end
    object Edit6: TEdit
      Left = 73
      Top = 36
      Width = 37
      Height = 21
      Hint = '@MB:TotalAmountOfReflectivityTransparencyIt'
      ParentShowHint = False
      ShowHint = True
      TabOrder = 9
      Text = '1.0'
    end
    object UpDown2: TUpDown
      Left = 201
      Top = 36
      Width = 16
      Height = 21
      Associate = Edit11
      Min = 1
      Max = 30
      Position = 2
      TabOrder = 10
    end
  end
  object NoiseFilterBtn: TSpeedButton
    Left = 0
    Top = 1115
    Width = 255
    Height = 21
    Hint = '@MB:ClickToHideShow'
    Align = alTop
    AllowAllUp = True
    GroupIndex = 7
    Caption = '@MB:NoiseFilter'
    ParentShowHint = False
    ShowHint = True
    OnClick = NoiseFilterBtnClick
  end
  object DepthOfFieldPnl: TPanel
    Left = 0
    Top = 912
    Width = 255
    Height = 203
    Align = alTop
    BevelOuter = bvNone
    TabOrder = 9
    Visible = False
    object Label1: TLabel
      Left = 30
      Top = 37
      Width = 45
      Height = 13
      Alignment = taRightJustify
      Caption = '@MB:Z1Sharp'
    end
    object Label18: TLabel
      Left = 106
      Top = 157
      Width = 115
      Height = 39
      Caption = '@MB:TheEffectAccumulatesResetForA'
      WordWrap = True
    end
    object Label19: TLabel
      Left = 29
      Top = 62
      Width = 45
      Height = 13
      Alignment = taRightJustify
      Caption = '@MB:Z2Sharp'
    end
    object Label2: TLabel
      Left = 138
      Top = 91
      Width = 48
      Height = 13
      Alignment = taRightJustify
      Caption = '@MB:ClippingR'
    end
    object Label5: TLabel
      Left = 33
      Top = 91
      Width = 43
      Height = 13
      Alignment = taRightJustify
      Caption = '@MB:Aperture'
    end
    object Button1: TButton
      Left = 11
      Top = 160
      Width = 83
      Height = 21
      Hint = '@MB:ThisEffectIsAccumulatedOnEach'
      Caption = '@MB:CalculateNow'
      ParentShowHint = False
      ShowHint = True
      TabOrder = 0
      OnClick = Button1Click
    end
    object Button18: TButton
      Tag = 10
      Left = 154
      Top = 59
      Width = 85
      Height = 21
      Hint = '@MB:PressButtonAndAfterwardsTheObject'
      Caption = '@MB:GetZ2'
      ParentShowHint = False
      ShowHint = True
      TabOrder = 1
      OnClick = Button2Click
    end
    object Button19: TButton
      Left = 4
      Top = 60
      Width = 18
      Height = 17
      Hint = '@MB:CopyZ1ToZ2ToGet'
      Caption = '->'
      ParentShowHint = False
      ShowHint = True
      TabOrder = 2
      OnClick = Button19Click
    end
    object Button2: TButton
      Tag = 1
      Left = 154
      Top = 34
      Width = 85
      Height = 21
      Hint = '@MB:PressButtonAndAfterwardsTheObject'
      Caption = '@MB:GetZ1'
      ParentShowHint = False
      ShowHint = True
      TabOrder = 3
      OnClick = Button2Click
    end
    object CheckBox1: TCheckBox
      Tag = 6
      Left = 27
      Top = 9
      Width = 182
      Height = 17
      Hint = '@MB:IfEnabledTheDoFEffectWill'
      Caption = '@MB:CalculateDoFAutomatically'
      Color = clBtnFace
      Font.Charset = DEFAULT_CHARSET
      Font.Color = clWindowText
      Font.Height = -12
      Font.Name = 'MS Sans Serif'
      Font.Style = [fsBold]
      ParentColor = False
      ParentFont = False
      ParentShowHint = False
      ShowHint = True
      TabOrder = 4
      OnClick = CheckBox1Click
    end
    object Edit1: TEdit
      Left = 81
      Top = 34
      Width = 62
      Height = 21
      Hint = '@MB:TheFirstFocusPointTheValue'
      Font.Charset = DEFAULT_CHARSET
      Font.Color = clWindowText
      Font.Height = -12
      Font.Name = 'MS Serif'
      Font.Style = []
      ParentFont = False
      ParentShowHint = False
      ShowHint = True
      TabOrder = 5
      Text = '0.0'
    end
    object Edit10: TEdit
      Left = 81
      Top = 59
      Width = 62
      Height = 21
      Hint = '@MB:SecondFocusPointIfDifferentFrom'
      Font.Charset = DEFAULT_CHARSET
      Font.Color = clWindowText
      Font.Height = -12
      Font.Name = 'MS Serif'
      Font.Style = []
      ParentFont = False
      ParentShowHint = False
      ShowHint = True
      TabOrder = 6
      Text = '0.0'
    end
    object Edit2: TEdit
      Left = 190
      Top = 88
      Width = 49
      Height = 21
      Hint = '@MB:TheBlurRadiusWillBeLimited'
      Font.Charset = DEFAULT_CHARSET
      Font.Color = clWindowText
      Font.Height = -12
      Font.Name = 'MS Serif'
      Font.Style = []
      ParentFont = False
      ParentShowHint = False
      ShowHint = True
      TabOrder = 7
      Text = '25.0'
    end
    object Edit3: TEdit
      Left = 81
      Top = 88
      Width = 50
      Height = 21
      Hint = '@MB:TheApertureOnTheZsharpPoint'
      Font.Charset = DEFAULT_CHARSET
      Font.Color = clWindowText
      Font.Height = -12
      Font.Name = 'MS Serif'
      Font.Style = []
      ParentFont = False
      ParentShowHint = False
      ShowHint = True
      TabOrder = 8
      Text = '0.02'
    end
    object RadioGroup1: TRadioGroup
      Left = 141
      Top = 114
      Width = 101
      Height = 37
      Hint = '@MB:N2Or3PassesGivesA'
      Caption = '@MB:Passes'
      Columns = 3
      ItemIndex = 0
      Items.Strings = (
        '1'
        '2'
        '3')
      ParentShowHint = False
      ShowHint = True
      TabOrder = 9
    end
    object RadioGroup2: TRadioGroup
      Left = 7
      Top = 114
      Width = 129
      Height = 37
      Hint = '@MB:SortedCalculationGivesBetterResultsForward'
      Caption = '@MB:Calculation'
      Columns = 2
      ItemIndex = 0
      Items.Strings = (
        '@MB:Sorted'
        '@MB:Forward')
      ParentShowHint = False
      ShowHint = True
      TabOrder = 10
    end
  end
  object NoiseFilterPnl: TPanel
    Left = 0
    Top = 1136
    Width = 255
    Height = 84
    Align = alTop
    BevelOuter = bvNone
    TabOrder = 10
    Visible = False
    object NoiseStrengthLbl: TLabel
      Left = 29
      Top = 37
      Width = 46
      Height = 13
      Caption = '@MB:Strength'
    end
    object NoiseAutoCbx: TCheckBox
      Left = 27
      Top = 9
      Width = 212
      Height = 17
      Hint = '@MB:EveryNewPictureOfTheMain'
      Caption = '@MB:FilterNewPicturesAutomatically'
      ParentShowHint = False
      ShowHint = True
      TabOrder = 0
      OnClick = NoiseAutoCbxClick
    end
    object NoiseSigmaEdit: TEdit
      Left = 81
      Top = 34
      Width = 40
      Height = 21
      Hint = '@MB:HowStrongTheNoiseIsThat'
      ParentShowHint = False
      ShowHint = True
      TabOrder = 1
      Text = '4'
    end
    object NoiseFilterNowBtn: TButton
      Left = 11
      Top = 59
      Width = 83
      Height = 21
      Caption = '@MB:FilterNow'
      TabOrder = 2
      OnClick = NoiseFilterNowBtnClick
    end
    object NoiseInfoLbl: TLabel
      Left = 130
      Top = 37
      Width = 115
      Height = 13
      Caption = '@MB:DCTDenoisingGIMP'
    end
  end
  object GroupBox4: TGroupBox
    Left = 125
    Top = 1139
    Width = 449
    Height = 65
    Caption = '@MB:LIGHTSTROKES'
    Font.Charset = DEFAULT_CHARSET
    Font.Color = clWindowText
    Font.Height = -12
    Font.Name = 'MS Sans Serif'
    Font.Style = []
    ParentFont = False
    TabOrder = 0
    Visible = False
    object CheckBox13: TCheckBox
      Left = 106
      Top = 39
      Width = 49
      Height = 17
      Caption = '@MB:Light12'
      Checked = True
      State = cbChecked
      TabOrder = 0
    end
    object CheckBox14: TCheckBox
      Left = 162
      Top = 39
      Width = 49
      Height = 17
      Caption = '@MB:Light22'
      TabOrder = 1
    end
    object CheckBox15: TCheckBox
      Left = 218
      Top = 39
      Width = 49
      Height = 17
      Caption = '@MB:Light32'
      TabOrder = 2
    end
    object CheckBox16: TCheckBox
      Left = 274
      Top = 39
      Width = 49
      Height = 17
      Caption = '@MB:Light42'
      TabOrder = 3
    end
    object CheckBox17: TCheckBox
      Left = 330
      Top = 39
      Width = 49
      Height = 17
      Caption = '@MB:Light52'
      TabOrder = 4
    end
    object CheckBox18: TCheckBox
      Left = 386
      Top = 39
      Width = 49
      Height = 17
      Caption = '@MB:Light62'
      TabOrder = 5
    end
    object Button17: TButton
      Left = 12
      Top = 16
      Width = 83
      Height = 21
      Caption = '@MB:CalculateNow'
      ParentShowHint = False
      ShowHint = False
      TabOrder = 6
      OnClick = Button17Click
    end
    object CheckBox20: TCheckBox
      Left = 121
      Top = 11
      Width = 272
      Height = 17
      Caption = '@MB:CalculateRandomLightStrokesAutomatically'
      Color = clBtnFace
      Font.Charset = DEFAULT_CHARSET
      Font.Color = clWindowText
      Font.Height = -12
      Font.Name = 'MS Sans Serif'
      Font.Style = [fsBold]
      ParentColor = False
      ParentFont = False
      ParentShowHint = False
      ShowHint = False
      TabOrder = 7
      OnClick = CheckBox9Click
    end
    object Edit4: TEdit
      Left = 8
      Top = 41
      Width = 89
      Height = 21
      TabOrder = 8
      Text = '3765724374'
    end
  end
  object GroupBox10: TGroupBox
    Left = 56
    Top = 1150
    Width = 449
    Height = 42
    Caption = '@MB:AmbientLight'
    Font.Charset = DEFAULT_CHARSET
    Font.Color = clWindowText
    Font.Height = -12
    Font.Name = 'MS Sans Serif'
    Font.Style = []
    ParentFont = False
    TabOrder = 2
    Visible = False
    object Shape6: TShape
      Left = 148
      Top = 11
      Width = 142
      Height = 18
      Brush.Style = bsClear
      Pen.Style = psClear
      Pen.Width = 0
    end
    object Button20: TButton
      Tag = 1
      Left = 12
      Top = 16
      Width = 83
      Height = 21
      Caption = '@MB:CalculateNow'
      TabOrder = 0
      OnClick = Button20Click
    end
    object CheckBox26: TCheckBox
      Tag = 6
      Left = 154
      Top = 11
      Width = 131
      Height = 17
      Hint = '@MB:IfEnabledTheReflectionsWillBe'
      Caption = '@MB:CalcRAutomatic'
      Font.Charset = DEFAULT_CHARSET
      Font.Color = clWindowText
      Font.Height = -12
      Font.Name = 'MS Sans Serif'
      Font.Style = [fsBold]
      ParentFont = False
      ParentShowHint = False
      ShowHint = True
      TabOrder = 1
      OnClick = CheckBox9Click
    end
  end
  object GroupBox6: TGroupBox
    Left = 21
    Top = 1150
    Width = 351
    Height = 42
    Caption = '@MB:DOUBLEIMAGESIZE'
    Font.Charset = DEFAULT_CHARSET
    Font.Color = clWindowText
    Font.Height = -12
    Font.Name = 'MS Sans Serif'
    Font.Style = []
    ParentFont = False
    TabOrder = 1
    Visible = False
    object Button12: TButton
      Left = 12
      Top = 16
      Width = 83
      Height = 21
      Caption = '@MB:CalculateNow'
      ParentShowHint = False
      ShowHint = False
      TabOrder = 0
      OnClick = Button12Click
    end
    object CheckBox8: TCheckBox
      Left = 121
      Top = 15
      Width = 222
      Height = 17
      Caption = '@MB:DoubleTheImagesizeAutomatically'
      Color = clBtnFace
      Font.Charset = DEFAULT_CHARSET
      Font.Color = clWindowText
      Font.Height = -12
      Font.Name = 'MS Sans Serif'
      Font.Style = [fsBold]
      ParentColor = False
      ParentFont = False
      ParentShowHint = False
      ShowHint = False
      TabOrder = 1
      OnClick = CheckBox9Click
    end
  end
  object ImageList1: TImageList
    Masked = False
    Left = 8
    Top = 40
    Bitmap = {
      494C010101002400340010001000FFFFFFFFFF10FFFFFFFFFFFFFFFF424D3600
      0000000000003600000028000000400000001000000001002000000000000010
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000FF0000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      00000000FF000000FF000000FF00000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      00000000FF000000FF000000FF00000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      FF000000FF000000FF000000FF000000FF000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      00000000000000000000000000000000000000000000000000000000FF000000
      FF000000FF000000FF000000FF000000FF000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      00000000000000000000000000000000000000000000808080000000FF000000
      FF0000000000000000000000FF000000FF000000FF0000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      000000000000000000000000000000000000808080000000FF00000000000000
      00000000000000000000000000000000FF000000FF0000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      00000000000000000000000000000000FF000000FF000000FF00000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000FF000000FF00000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      000000000000000000000000000000000000000000000000FF000000FF000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000808080000000
      FF00000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000008080
      80000000FF000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000FF000000FF00000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      0000000000000000000000000000000000000000000000000000000000000000
      000000000000000000000000000000000000424D3E000000000000003E000000
      2800000040000000100000000100010000000000800000000000000000000000
      000000000000000000000000FFFFFF00FFFF000000000000FFFF000000000000
      F9FF000000000000F0FF000000000000F0FF000000000000E07F000000000000
      C07F000000000000843F0000000000001E3F000000000000FE1F000000000000
      FF1F000000000000FF8F000000000000FFC7000000000000FFE3000000000000
      FFF8000000000000FFFF00000000000000000000000000000000000000000000
      000000000000}
  end
end
