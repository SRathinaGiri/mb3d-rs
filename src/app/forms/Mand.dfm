object Mand3DForm: TMand3DForm
  Left = 65
  Top = 100
  ClientHeight = 572
  ClientWidth = 771
  Color = clBtnFace
  Constraints.MinHeight = 300
  Constraints.MinWidth = 200
  Font.Charset = DEFAULT_CHARSET
  Font.Color = clWindowText
  Font.Height = -11
  Font.Name = 'MS Sans Serif'
  Font.Style = []
  KeyPreview = True
  OldCreateOrder = False
  Visible = True
  OnClose = FormClose
  OnCloseQuery = FormCloseQuery
  OnCreate = FormCreate
  OnDestroy = FormDestroy
  OnDragOver = FormDragOver
  OnHide = FormHide
  OnKeyPress = FormKeyPress
  OnMouseWheel = FormMouseWheel
  OnResize = FormResize
  OnShow = FormShow
  PixelsPerInch = 96
  TextHeight = 13
  object Panel1: TPanel
    Left = 618
    Top = 0
    Width = 153
    Height = 572
    Align = alRight
    ParentShowHint = False
    ShowHint = False
    TabOrder = 0
    object PositionBtn: TSpeedButton
      Left = 1
      Top = 1
      Width = 151
      Height = 21
      Hint = '@MB:ClickToHideShow'
      Align = alTop
      AllowAllUp = True
      GroupIndex = 1
      Caption = '@MB:Position'
      ParentShowHint = False
      ShowHint = True
      OnClick = PositionBtnClick
      ExplicitLeft = -8
      ExplicitTop = -1
      ExplicitWidth = 255
    end
    object RotationBtn: TSpeedButton
      Left = 1
      Top = 172
      Width = 151
      Height = 21
      Hint = '@MB:ClickToHideShow'
      Align = alTop
      AllowAllUp = True
      GroupIndex = 2
      Caption = '@MB:Rotation'
      ParentShowHint = False
      ShowHint = True
      OnClick = RotationBtnClick
      ExplicitLeft = 6
      ExplicitTop = 211
    end
    object PositionPnl: TPanel
      Left = 1
      Top = 22
      Width = 151
      Height = 150
      Align = alTop
      BevelOuter = bvNone
      UseDockManager = False
      TabOrder = 0
      Visible = False
      object Label9: TLabel
        Left = 5
        Top = 4
        Width = 29
        Height = 13
        Alignment = taRightJustify
        Caption = '@MB:XMid'
        Font.Charset = DEFAULT_CHARSET
        Font.Color = clWindowText
        Font.Height = -12
        Font.Name = 'MS Sans Serif'
        Font.Style = []
        ParentFont = False
      end
      object Label10: TLabel
        Left = 6
        Top = 26
        Width = 29
        Height = 13
        Alignment = taRightJustify
        Caption = '@MB:YMid'
        Font.Charset = DEFAULT_CHARSET
        Font.Color = clWindowText
        Font.Height = -12
        Font.Name = 'MS Sans Serif'
        Font.Style = []
        ParentFont = False
      end
      object SpeedButton32: TSpeedButton
        Tag = 2
        Left = -1
        Top = 45
        Width = 40
        Height = 19
        Hint = '@MB:ClickToRenderThe2dPlane'
        Caption = '@MB:ZMid'
        ParentShowHint = False
        ShowHint = True
        OnClick = Button1Click
      end
      object SpeedButton33: TSpeedButton
        Tag = 1
        Left = -1
        Top = 67
        Width = 40
        Height = 19
        Hint = '@MB:ClickToRenderThe2dStartplane'
        Caption = '@MB:ZStart'
        ParentShowHint = False
        ShowHint = True
        OnClick = Button1Click
      end
      object SpeedButton34: TSpeedButton
        Tag = 3
        Left = -1
        Top = 89
        Width = 40
        Height = 18
        Hint = '@MB:ClickToRenderTheZEndplane'
        Caption = '@MB:ZEnd'
        ParentShowHint = False
        ShowHint = True
        OnClick = Button1Click
      end
      object SpeedButton30: TSpeedButton
        Left = 40
        Top = 109
        Width = 70
        Height = 19
        Hint = '@MB:ClickOnTheObjectInThe'
        Caption = '@MB:GetMidpoint'
        ParentShowHint = False
        ShowHint = True
        OnClick = SpeedButton30Click
      end
      object SpeedButton31: TSpeedButton
        Left = 112
        Top = 109
        Width = 35
        Height = 19
        Hint = '@MB:ResetThePositionAndTheRotation'
        Caption = '@MB:Reset'
        ParentShowHint = False
        ShowHint = True
        OnClick = Button14Click
      end
      object Label5: TLabel
        Left = 5
        Top = 131
        Width = 30
        Height = 13
        Alignment = taRightJustify
        Caption = '@MB:Zoom'
        Font.Charset = DEFAULT_CHARSET
        Font.Color = clWindowText
        Font.Height = -12
        Font.Name = 'MS Sans Serif'
        Font.Style = []
        ParentFont = False
      end
      object Edit9: TEdit
        Left = 38
        Top = 1
        Width = 109
        Height = 21
        Font.Charset = DEFAULT_CHARSET
        Font.Color = clWindowText
        Font.Height = -12
        Font.Name = 'MS Serif'
        Font.Style = []
        ParentFont = False
        TabOrder = 0
        Text = '0.0'
      end
      object Edit10: TEdit
        Left = 38
        Top = 23
        Width = 109
        Height = 21
        Font.Charset = DEFAULT_CHARSET
        Font.Color = clWindowText
        Font.Height = -12
        Font.Name = 'MS Serif'
        Font.Style = []
        ParentFont = False
        TabOrder = 1
        Text = '0.0'
      end
      object Edit17: TEdit
        Left = 38
        Top = 45
        Width = 109
        Height = 21
        Font.Charset = DEFAULT_CHARSET
        Font.Color = clWindowText
        Font.Height = -12
        Font.Name = 'MS Serif'
        Font.Style = []
        ParentFont = False
        TabOrder = 2
        Text = '0.0'
      end
      object Edit1: TEdit
        Left = 38
        Top = 67
        Width = 109
        Height = 21
        Font.Charset = DEFAULT_CHARSET
        Font.Color = clWindowText
        Font.Height = -12
        Font.Name = 'MS Serif'
        Font.Style = []
        ParentFont = False
        TabOrder = 3
        Text = '-2.0'
      end
      object Edit3: TEdit
        Left = 38
        Top = 89
        Width = 109
        Height = 21
        Font.Charset = DEFAULT_CHARSET
        Font.Color = clWindowText
        Font.Height = -12
        Font.Name = 'MS Serif'
        Font.Style = []
        ParentFont = False
        TabOrder = 4
        Text = '30.0'
      end
      object Edit5: TEdit
        Left = 38
        Top = 128
        Width = 109
        Height = 21
        Hint = '@MB:DefinesTheSizeOfTheZstart'
        Font.Charset = DEFAULT_CHARSET
        Font.Color = clWindowText
        Font.Height = -12
        Font.Name = 'MS Serif'
        Font.Style = []
        ParentFont = False
        ParentShowHint = False
        ShowHint = True
        TabOrder = 5
        Text = '1.0'
      end
    end
    object RotationPnl: TPanel
      Left = 1
      Top = 193
      Width = 151
      Height = 158
      Align = alTop
      BevelOuter = bvNone
      UseDockManager = False
      TabOrder = 1
      Visible = False
      object Label53: TLabel
        Left = 10
        Top = 3
        Width = 61
        Height = 13
        Alignment = taCenter
        Caption = '@MB:EulerAngles'
        Font.Charset = DEFAULT_CHARSET
        Font.Color = clWindowText
        Font.Height = -12
        Font.Name = 'MS Sans Serif'
        Font.Style = []
        ParentFont = False
      end
      object Label54: TLabel
        Left = 9
        Top = 24
        Width = 10
        Height = 13
        Alignment = taRightJustify
        Caption = 'X:'
        Font.Charset = DEFAULT_CHARSET
        Font.Color = clWindowText
        Font.Height = -12
        Font.Name = 'MS Sans Serif'
        Font.Style = []
        ParentFont = False
      end
      object Label55: TLabel
        Left = 9
        Top = 48
        Width = 10
        Height = 13
        Alignment = taRightJustify
        Caption = 'Y:'
        Font.Charset = DEFAULT_CHARSET
        Font.Color = clWindowText
        Font.Height = -12
        Font.Name = 'MS Sans Serif'
        Font.Style = []
        ParentFont = False
      end
      object Label56: TLabel
        Left = 9
        Top = 72
        Width = 10
        Height = 13
        Alignment = taRightJustify
        Caption = 'Z:'
        Font.Charset = DEFAULT_CHARSET
        Font.Color = clWindowText
        Font.Height = -12
        Font.Name = 'MS Sans Serif'
        Font.Style = []
        ParentFont = False
      end
      object Label57: TLabel
        Left = 3
        Top = 116
        Width = 139
        Height = 26
        Alignment = taCenter
        Caption = '@MB:ApplyYourChangedValuesBecauseEdits'
        Font.Charset = DEFAULT_CHARSET
        Font.Color = clWindowText
        Font.Height = -12
        Font.Name = 'MS Sans Serif'
        Font.Style = []
        ParentFont = False
        WordWrap = True
      end
      object Button7: TButton
        Left = 3
        Top = 94
        Width = 88
        Height = 22
        Hint = '@MB:TheRotationIsPerformedAroundThe'
        Caption = '@MB:ApplyToImage'
        ParentShowHint = False
        ShowHint = True
        TabOrder = 0
        OnClick = Button7Click
      end
      object ButtonR0: TButton
        Left = 97
        Top = 94
        Width = 49
        Height = 22
        Caption = '@MB:Reset0'
        TabOrder = 1
        OnClick = ButtonR0Click
      end
      object Edit27: TEdit
        Left = 25
        Top = 21
        Width = 120
        Height = 21
        TabOrder = 2
        Text = '0.0'
        OnChange = SpinEdit2Change
      end
      object Edit31: TEdit
        Left = 25
        Top = 45
        Width = 120
        Height = 21
        Hint = '@MB:CaseYAngleIsAt90'
        ParentShowHint = False
        ShowHint = True
        TabOrder = 3
        Text = '0.0'
        OnChange = SpinEdit2Change
      end
      object Edit32: TEdit
        Left = 25
        Top = 69
        Width = 120
        Height = 21
        TabOrder = 4
        Text = '0.0'
        OnChange = SpinEdit2Change
      end
    end
    object Memo1: TMemo
      Left = 1
      Top = 710
      Width = 151
      Height = 80
      Hint = '@MB:ShowsMessages'
      Align = alTop
      BorderStyle = bsNone
      Color = clSilver
      Ctl3D = False
      Font.Charset = DEFAULT_CHARSET
      Font.Color = clTeal
      Font.Height = -12
      Font.Name = 'MS Sans Serif'
      Font.Style = []
      ParentCtl3D = False
      ParentFont = False
      ParentShowHint = False
      ReadOnly = True
      ShowHint = True
      TabOrder = 5
    end
    object PageControl1: TPageControl
      Left = 1
      Top = 446
      Width = 151
      Height = 292
      ActivePage = TabSheet5
      Align = alTop
      BiDiMode = bdLeftToRight
      DoubleBuffered = False
      Font.Charset = DEFAULT_CHARSET
      Font.Color = clWindowText
      Font.Height = -12
      Font.Name = 'MS Sans Serif'
      Font.Style = []
      HotTrack = True
      MultiLine = True
      ParentBiDiMode = False
      ParentDoubleBuffered = False
      ParentFont = False
      TabOrder = 3
      Touch.ParentTabletOptions = False
      Touch.TabletOptions = []
      OnChange = PageControl1Change
      object TabSheet2: TTabSheet
        Caption = '@MB:Calculation'
        Highlighted = True
        ImageIndex = 1
        ExplicitLeft = 0
        ExplicitTop = 0
        ExplicitWidth = 0
        ExplicitHeight = 0
        object Label16: TLabel
          Left = 4
          Top = 125
          Width = 81
          Height = 13
          Alignment = taRightJustify
          Caption = '@MB:SmoothNormals'
          Font.Charset = DEFAULT_CHARSET
          Font.Color = clWindowText
          Font.Height = -12
          Font.Name = 'MS Sans Serif'
          Font.Style = []
          ParentFont = False
        end
        object Label18: TLabel
          Left = 44
          Top = 6
          Width = 41
          Height = 13
          Alignment = taRightJustify
          Caption = '@MB:DEStop'
          Font.Charset = DEFAULT_CHARSET
          Font.Color = clWindowText
          Font.Height = -12
          Font.Name = 'MS Sans Serif'
          Font.Style = []
          ParentFont = False
        end
        object Label22: TLabel
          Left = 0
          Top = 51
          Width = 85
          Height = 13
          Alignment = taRightJustify
          Caption = '@MB:RaystepMultiplier'
          Font.Charset = DEFAULT_CHARSET
          Font.Color = clWindowText
          Font.Height = -12
          Font.Name = 'MS Sans Serif'
          Font.Style = []
          ParentFont = False
        end
        object Label13: TLabel
          Left = 18
          Top = 93
          Width = 67
          Height = 26
          Alignment = taRightJustify
          Caption = '@MB:StepcountForBinarySearch'
          Font.Charset = DEFAULT_CHARSET
          Font.Color = clWindowText
          Font.Height = -12
          Font.Name = 'MS Sans Serif'
          Font.Style = []
          ParentFont = False
        end
        object SpeedButton10: TSpeedButton
          Left = 0
          Top = 0
          Width = 23
          Height = 22
          Hint = '@MB:ClickOnMToSaveThese'
          AllowAllUp = True
          GroupIndex = 5
          Caption = 'M'
          Flat = True
          ParentShowHint = False
          ShowHint = True
          OnClick = SpeedButton10Click
        end
        object Label26: TLabel
          Left = 6
          Top = 74
          Width = 79
          Height = 13
          Alignment = taRightJustify
          Caption = '@MB:StepwidthLimiter'
          Font.Charset = DEFAULT_CHARSET
          Font.Color = clWindowText
          Font.Height = -12
          Font.Name = 'MS Sans Serif'
          Font.Style = []
          ParentFont = False
        end
        object Edit25: TEdit
          Left = 92
          Top = 3
          Width = 49
          Height = 21
          Hint = '@MB:DefinesTheDistanceToTheObjects'
          ParentShowHint = False
          ShowHint = True
          TabOrder = 0
          Text = '1.0'
          OnChange = SpinEdit2Change
        end
        object CheckBox9: TCheckBox
          Left = 12
          Top = 27
          Width = 125
          Height = 17
          Hint = '@MB:TheDistanceToTheObjectsSurface'
          Alignment = taLeftJustify
          Caption = '@MB:VaryDEstopOnFOV'
          Checked = True
          ParentShowHint = False
          ShowHint = True
          State = cbChecked
          TabOrder = 1
        end
        object CheckBox1: TCheckBox
          Left = 39
          Top = 147
          Width = 98
          Height = 17
          Hint = '@MB:UncheckThisIfTheImageLooks'
          Alignment = taLeftJustify
          Caption = '@MB:NormalsOnDE'
          Checked = True
          ParentShowHint = False
          ShowHint = True
          State = cbChecked
          TabOrder = 6
        end
        object Edit6: TEdit
          Left = 92
          Top = 48
          Width = 49
          Height = 21
          Hint = '@MB:TheDistanceEstimatesAreDownscaledBy'
          ParentShowHint = False
          ShowHint = True
          TabOrder = 2
          Text = '0.5'
          OnChange = SpinEdit2Change
        end
        object CheckBox3: TCheckBox
          Left = 30
          Top = 164
          Width = 107
          Height = 17
          Hint = '@MB:MultipliesTheFirstStepwidthWithA'
          Alignment = taLeftJustify
          Caption = '@MB:FirstStepRandom'
          Checked = True
          ParentShowHint = False
          ShowHint = True
          State = cbChecked
          TabOrder = 7
        end
        object CheckBox8: TCheckBox
          Left = 1
          Top = 198
          Width = 136
          Height = 17
          Hint = '@MB:UsefulForSteepHeightmaps'
          Alignment = taLeftJustify
          Caption = '@MB:ShortdistanceCheckDE'
          ParentShowHint = False
          ShowHint = True
          TabOrder = 9
          Visible = False
        end
        object Edit8: TEdit
          Left = 92
          Top = 71
          Width = 49
          Height = 21
          Hint = '@MB:UseValuesLowerThan1To'
          ParentShowHint = False
          ShowHint = True
          TabOrder = 3
          Text = '1.0'
          OnChange = SpinEdit2Change
        end
        object Edit19: TEdit
          Left = 92
          Top = 122
          Width = 30
          Height = 21
          ParentShowHint = False
          ReadOnly = True
          ShowHint = True
          TabOrder = 5
          Text = '0'
        end
        object SpinEdit2: TUpDown
          Left = 122
          Top = 122
          Width = 19
          Height = 21
          Associate = Edit19
          Max = 8
          TabOrder = 10
          OnChangingEx = SpinEdit2ChangingEx
        end
        object Edit20: TEdit
          Left = 92
          Top = 97
          Width = 30
          Height = 21
          ParentShowHint = False
          ReadOnly = True
          ShowHint = True
          TabOrder = 4
          Text = '8'
        end
        object SpinEdit5: TUpDown
          Left = 122
          Top = 97
          Width = 19
          Height = 21
          Associate = Edit20
          Max = 31
          Position = 8
          TabOrder = 11
          OnChangingEx = SpinEdit2ChangingEx
        end
        object CheckBox2: TCheckBox
          Left = 13
          Top = 181
          Width = 124
          Height = 17
          Hint = '@MB:SubtractAnAmountOfDEstopFrom'
          Alignment = taLeftJustify
          Caption = '@MB:RaystepSubDEstop'
          ParentShowHint = False
          ShowHint = True
          TabOrder = 8
        end
      end
      object TabSheet4: TTabSheet
        Caption = '@MB:Internal'
        ImageIndex = 3
        ExplicitLeft = 0
        ExplicitTop = 0
        ExplicitWidth = 0
        ExplicitHeight = 0
        object Label15: TLabel
          Left = 12
          Top = 11
          Width = 68
          Height = 26
          Alignment = taRightJustify
          Caption = '@MB:ThreadcountInCalculations'
          Color = clBtnFace
          Font.Charset = DEFAULT_CHARSET
          Font.Color = clWindowText
          Font.Height = -12
          Font.Name = 'MS Sans Serif'
          Font.Style = []
          ParentColor = False
          ParentFont = False
        end
        object Label3: TLabel
          Left = 20
          Top = 166
          Width = 3
          Height = 13
        end
        object Label17: TLabel
          Left = 20
          Top = 181
          Width = 49
          Height = 13
          Caption = '@MB:ItsForFree'
        end
        object CalcOnLabel: TLabel
          Left = 8
          Top = 145
          Width = 50
          Height = 13
          Caption = '@MB:CalculateOn'
        end
        object CalcOnCombo: TComboBox
          Left = 66
          Top = 141
          Width = 75
          Height = 21
          Hint = '@MB:AutoTheCPUAndTheGraphics'
          Style = csDropDownList
          ItemIndex = 0
          ParentShowHint = False
          ShowHint = True
          TabOrder = 5
          Text = 'Auto'
          OnChange = CalcOnComboChange
          Items.Strings = (
            '@MB:Auto'
            '@MB:CPU'
            '@MB:GraphicsCard')
        end
        object Label33: TLabel
          Left = 13
          Top = 74
          Width = 34
          Height = 26
          Alignment = taRightJustify
          Caption = '@MB:ThreadPriority'
          Color = clBtnFace
          Font.Charset = DEFAULT_CHARSET
          Font.Color = clWindowText
          Font.Height = -12
          Font.Name = 'MS Sans Serif'
          Font.Style = []
          ParentColor = False
          ParentFont = False
        end
        object ComboBox2: TComboBox
          Left = 56
          Top = 76
          Width = 81
          Height = 21
          Style = csDropDownList
          ItemIndex = 2
          TabOrder = 0
          Text = 'Lower'
          OnChange = CheckBox14Click
          Items.Strings = (
            '@MB:Idle'
            '@MB:Lowest'
            '@MB:Lower'
            '@MB:Normal')
        end
        object CheckBox12: TCheckBox
          Left = 10
          Top = 43
          Width = 111
          Height = 17
          Hint = '@MB:DetectsTheAmountOfProcessorCores'
          Alignment = taLeftJustify
          Caption = '@MB:AutodetectOnStart'
          Checked = True
          ParentShowHint = False
          ShowHint = True
          State = cbChecked
          TabOrder = 1
        end
        object CheckBox14: TCheckBox
          Left = 8
          Top = 122
          Width = 113
          Height = 17
          Hint = '@MB:CheckThisToStopSettingThe'
          Alignment = taLeftJustify
          Caption = '@MB:DisableThreadboost'
          Checked = True
          ParentShowHint = False
          ShowHint = True
          State = cbChecked
          TabOrder = 2
          OnClick = CheckBox14Click
        end
        object CheckBox15: TCheckBox
          Left = 24
          Top = 104
          Width = 97
          Height = 17
          Hint = '@MB:PreventTheComputerFromShutdownBy'
          Alignment = taLeftJustify
          Caption = '@MB:KeepThisPriority'
          TabOrder = 3
        end
        object Edit21: TEdit
          Left = 91
          Top = 14
          Width = 30
          Height = 21
          ParentShowHint = False
          ReadOnly = True
          ShowHint = True
          TabOrder = 4
          Text = '1'
        end
        object UpDown3: TUpDown
          Left = 121
          Top = 14
          Width = 19
          Height = 21
          Associate = Edit21
          Min = 1
          Max = 64
          Position = 1
          TabOrder = 5
          OnChangingEx = SpinEdit2ChangingEx
        end
      end
      object TabSheet5: TTabSheet
        Caption = '@MB:Infos'
        ImageIndex = 4
        object Label29: TLabel
          Left = 9
          Top = 67
          Width = 67
          Height = 13
          Alignment = taRightJustify
          Caption = '@MB:AvgRaysteps'
        end
        object Label30: TLabel
          Left = 6
          Top = 88
          Width = 70
          Height = 13
          Caption = '@MB:AvgIterations'
        end
        object Label31: TLabel
          Left = 86
          Top = 67
          Width = 3
          Height = 13
        end
        object Label32: TLabel
          Left = 86
          Top = 88
          Width = 3
          Height = 13
        end
        object Label51: TLabel
          Left = 6
          Top = 127
          Width = 71
          Height = 13
          Alignment = taRightJustify
          Caption = '@MB:MainCalcTime'
        end
        object Label52: TLabel
          Left = 86
          Top = 127
          Width = 3
          Height = 13
        end
        object Label7: TLabel
          Left = 14
          Top = 145
          Width = 63
          Height = 13
          Alignment = taRightJustify
          Caption = '@MB:HSCalcTime'
        end
        object Label8: TLabel
          Left = 86
          Top = 145
          Width = 3
          Height = 13
        end
        object Label34: TLabel
          Left = 6
          Top = 106
          Width = 71
          Height = 13
          Alignment = taRightJustify
          Caption = '@MB:MaxIterations'
        end
        object Label40: TLabel
          Left = 86
          Top = 106
          Width = 3
          Height = 13
        end
        object Label47: TLabel
          Left = 14
          Top = 163
          Width = 63
          Height = 13
          Alignment = taRightJustify
          Caption = '@MB:AOCalcTime'
        end
        object Label48: TLabel
          Left = 86
          Top = 163
          Width = 3
          Height = 13
        end
        object Label49: TLabel
          Left = 13
          Top = 181
          Width = 64
          Height = 13
          Alignment = taRightJustify
          Caption = '@MB:ReflectsTime'
        end
        object Label50: TLabel
          Left = 86
          Top = 181
          Width = 3
          Height = 13
        end
        object Label60: TLabel
          Left = 4
          Top = 25
          Width = 22
          Height = 13
          Alignment = taRightJustify
          Caption = '@MB:Orig'
        end
        object Label59: TLabel
          Left = 2
          Top = 46
          Width = 24
          Height = 13
          Alignment = taRightJustify
          Caption = '@MB:Mod'
        end
        object Edit33: TEdit
          Left = 32
          Top = 23
          Width = 109
          Height = 19
          Ctl3D = False
          MaxLength = 16
          ParentCtl3D = False
          TabOrder = 0
          OnChange = Edit33Change
        end
        object Edit34: TEdit
          Left = 32
          Top = 44
          Width = 109
          Height = 19
          Ctl3D = False
          MaxLength = 16
          ParentCtl3D = False
          TabOrder = 1
          OnChange = Edit34Change
        end
        object ButtonAuthor: TButton
          Left = 32
          Top = 0
          Width = 57
          Height = 22
          Hint = '@MB:ClickToChooseYourNameFor'
          Caption = '@MB:Author'
          ParentShowHint = False
          ShowHint = True
          TabOrder = 2
          OnClick = ButtonAuthorClick
        end
        object ButtonInsertAuthor: TButton
          Left = 95
          Top = 0
          Width = 46
          Height = 22
          Hint = '@MB:ClickToInsertYourNameBelow'
          Caption = '@MB:Insert'
          ParentShowHint = False
          ShowHint = True
          TabOrder = 3
          OnClick = ButtonInsertAuthorClick
        end
      end
      object TabSheet8: TTabSheet
        Caption = '@MB:Cutting'
        ImageIndex = 7
        ExplicitLeft = 0
        ExplicitTop = 0
        ExplicitWidth = 0
        ExplicitHeight = 0
        object Label37: TLabel
          Left = 9
          Top = 66
          Width = 10
          Height = 13
          Alignment = taRightJustify
          Caption = 'Z:'
          Font.Charset = DEFAULT_CHARSET
          Font.Color = clWindowText
          Font.Height = -12
          Font.Name = 'MS Sans Serif'
          Font.Style = []
          ParentFont = False
        end
        object Label38: TLabel
          Left = 9
          Top = 18
          Width = 10
          Height = 13
          Alignment = taRightJustify
          Caption = 'X:'
          Font.Charset = DEFAULT_CHARSET
          Font.Color = clWindowText
          Font.Height = -12
          Font.Name = 'MS Sans Serif'
          Font.Style = []
          ParentFont = False
        end
        object Label39: TLabel
          Left = 9
          Top = 42
          Width = 10
          Height = 13
          Alignment = taRightJustify
          Caption = 'Y:'
          Font.Charset = DEFAULT_CHARSET
          Font.Color = clWindowText
          Font.Height = -12
          Font.Name = 'MS Sans Serif'
          Font.Style = []
          ParentFont = False
        end
        object Edit22: TEdit
          Left = 28
          Top = 63
          Width = 93
          Height = 21
          Font.Charset = DEFAULT_CHARSET
          Font.Color = clWindowText
          Font.Height = -12
          Font.Name = 'MS Serif'
          Font.Style = []
          ParentFont = False
          TabOrder = 0
          Text = '0.00'
        end
        object Edit23: TEdit
          Left = 28
          Top = 15
          Width = 93
          Height = 21
          Font.Charset = DEFAULT_CHARSET
          Font.Color = clWindowText
          Font.Height = -12
          Font.Name = 'MS Serif'
          Font.Style = []
          ParentFont = False
          TabOrder = 1
          Text = '0.00'
        end
        object Edit24: TEdit
          Left = 28
          Top = 39
          Width = 93
          Height = 21
          Font.Charset = DEFAULT_CHARSET
          Font.Color = clWindowText
          Font.Height = -12
          Font.Name = 'MS Serif'
          Font.Style = []
          ParentFont = False
          TabOrder = 2
          Text = '0.00'
        end
        object CheckBox4: TCheckBox
          Left = 126
          Top = 17
          Width = 17
          Height = 17
          TabOrder = 3
          OnClick = CheckBox7Click
        end
        object CheckBox5: TCheckBox
          Left = 126
          Top = 41
          Width = 17
          Height = 17
          TabOrder = 4
          OnClick = CheckBox7Click
        end
        object CheckBox6: TCheckBox
          Left = 126
          Top = 65
          Width = 17
          Height = 17
          TabOrder = 5
          OnClick = CheckBox7Click
        end
        object Button19: TButton
          Left = 28
          Top = 96
          Width = 109
          Height = 21
          Hint = '@MB:InsertsTheCurrentMidPosition'
          Caption = '@MB:InsertMidValues'
          ParentShowHint = False
          ShowHint = True
          TabOrder = 6
          OnClick = Button19Click
        end
        object Button20: TButton
          Tag = 6
          Left = 16
          Top = 124
          Width = 121
          Height = 21
          Hint = '@MB:ClickOnTheButtonAndAfterwards'
          Caption = '@MB:GetValuesFromImage'
          ParentShowHint = False
          ShowHint = True
          TabOrder = 7
          OnClick = Button16Click
        end
      end
      object TabSheet9: TTabSheet
        Caption = '@MB:JuliaOff'
        ImageIndex = 8
        ExplicitLeft = 0
        ExplicitTop = 0
        ExplicitWidth = 0
        ExplicitHeight = 0
        object Label43: TLabel
          Left = 9
          Top = 80
          Width = 10
          Height = 13
          Alignment = taRightJustify
          Caption = 'Z:'
          Font.Charset = DEFAULT_CHARSET
          Font.Color = clWindowText
          Font.Height = -12
          Font.Name = 'MS Sans Serif'
          Font.Style = []
          ParentFont = False
        end
        object Label44: TLabel
          Left = 9
          Top = 32
          Width = 10
          Height = 13
          Alignment = taRightJustify
          Caption = 'X:'
          Font.Charset = DEFAULT_CHARSET
          Font.Color = clWindowText
          Font.Height = -12
          Font.Name = 'MS Sans Serif'
          Font.Style = []
          ParentFont = False
        end
        object Label45: TLabel
          Left = 9
          Top = 56
          Width = 10
          Height = 13
          Alignment = taRightJustify
          Caption = 'Y:'
          Font.Charset = DEFAULT_CHARSET
          Font.Color = clWindowText
          Font.Height = -12
          Font.Name = 'MS Sans Serif'
          Font.Style = []
          ParentFont = False
        end
        object Label14: TLabel
          Left = 5
          Top = 104
          Width = 14
          Height = 13
          Alignment = taRightJustify
          Caption = 'W:'
          Font.Charset = DEFAULT_CHARSET
          Font.Color = clWindowText
          Font.Height = -12
          Font.Name = 'MS Sans Serif'
          Font.Style = []
          ParentFont = False
        end
        object CheckBox7: TCheckBox
          Left = 24
          Top = 8
          Width = 113
          Height = 17
          Alignment = taLeftJustify
          Caption = '@MB:CalculateJulia'
          TabOrder = 0
          OnClick = CheckBox7Click
        end
        object Edit28: TEdit
          Left = 28
          Top = 29
          Width = 109
          Height = 21
          Font.Charset = DEFAULT_CHARSET
          Font.Color = clWindowText
          Font.Height = -12
          Font.Name = 'MS Serif'
          Font.Style = []
          ParentFont = False
          TabOrder = 1
          Text = '1.00'
        end
        object Edit29: TEdit
          Left = 28
          Top = 53
          Width = 109
          Height = 21
          Font.Charset = DEFAULT_CHARSET
          Font.Color = clWindowText
          Font.Height = -12
          Font.Name = 'MS Serif'
          Font.Style = []
          ParentFont = False
          TabOrder = 2
          Text = '0.00'
        end
        object Edit30: TEdit
          Left = 28
          Top = 77
          Width = 109
          Height = 21
          Font.Charset = DEFAULT_CHARSET
          Font.Color = clWindowText
          Font.Height = -12
          Font.Name = 'MS Serif'
          Font.Style = []
          ParentFont = False
          TabOrder = 3
          Text = '0.00'
        end
        object Button13: TButton
          Left = 28
          Top = 128
          Width = 109
          Height = 21
          Hint = '@MB:InsertsTheCurrentPositionAsJulia'
          Caption = '@MB:InsertMidValues'
          ParentShowHint = False
          ShowHint = True
          TabOrder = 4
          OnClick = Button13Click
        end
        object Edit7: TEdit
          Left = 28
          Top = 101
          Width = 109
          Height = 21
          Font.Charset = DEFAULT_CHARSET
          Font.Color = clWindowText
          Font.Height = -12
          Font.Name = 'MS Serif'
          Font.Style = []
          ParentFont = False
          TabOrder = 5
          Text = '0.00'
        end
        object Button16: TButton
          Tag = 3
          Left = 16
          Top = 156
          Width = 121
          Height = 21
          Hint = '@MB:ClickOnTheButtonAndAfterwards2'
          Caption = '@MB:GetValuesFromImage'
          ParentShowHint = False
          ShowHint = True
          TabOrder = 6
          OnClick = Button16Click
        end
      end
      object TabSheet1: TTabSheet
        Caption = '@MB:Camera'
        ImageIndex = 5
        ExplicitLeft = 0
        ExplicitTop = 0
        ExplicitWidth = 0
        ExplicitHeight = 0
        object Label21: TLabel
          Left = 24
          Top = 10
          Width = 29
          Height = 13
          Alignment = taRightJustify
          Caption = '@MB:FOVy'
          Font.Charset = DEFAULT_CHARSET
          Font.Color = clWindowText
          Font.Height = -12
          Font.Name = 'MS Sans Serif'
          Font.Style = []
          ParentFont = False
        end
        object Edit14: TEdit
          Left = 64
          Top = 7
          Width = 73
          Height = 21
          Hint = '@MB:VerticalFieldOfViewInDegrees'
          ParentShowHint = False
          ShowHint = True
          TabOrder = 0
          Text = '30'
        end
        object RadioGroup2: TRadioGroup
          Left = 19
          Top = 41
          Width = 110
          Height = 96
          Caption = '@MB:CameraLense'
          ItemIndex = 0
          Items.Strings = (
            '@MB:Common'
            '@MB:Rectilinear'
            '@MB:N360Panorama')
          ParentShowHint = False
          ShowHint = True
          TabOrder = 1
          TabStop = True
        end
      end
      object TabSheet3: TTabSheet
        Caption = '@MB:Coloring'
        ImageIndex = 6
        ExplicitLeft = 0
        ExplicitTop = 0
        ExplicitWidth = 0
        ExplicitHeight = 0
        object Label2: TLabel
          Left = 5
          Top = 132
          Width = 56
          Height = 13
          Alignment = taRightJustify
          Caption = '@MB:LliMultiplier'
          Font.Charset = DEFAULT_CHARSET
          Font.Color = clWindowText
          Font.Height = -12
          Font.Name = 'MS Sans Serif'
          Font.Style = []
          ParentFont = False
        end
        object Label58: TLabel
          Left = 7
          Top = 156
          Width = 72
          Height = 13
          Alignment = taRightJustify
          Caption = '@MB:ColorOnIterat'
          Font.Charset = DEFAULT_CHARSET
          Font.Color = clWindowText
          Font.Height = -12
          Font.Name = 'MS Sans Serif'
          Font.Style = []
          ParentFont = False
        end
        object Label61: TLabel
          Left = 126
          Top = 180
          Width = 13
          Height = 13
          Alignment = taRightJustify
          AutoSize = False
          Caption = '0'
          Visible = False
        end
        object RadioGroup1: TRadioGroup
          Left = 4
          Top = 2
          Width = 140
          Height = 123
          Hint = '@MB:TheMapOptionsCalculatesXY'
          Caption = '@MB:ModeFor2ColorChoice'
          ItemIndex = 0
          Items.Strings = (
            '@MB:OrbitTrapMindistTo0'
            '@MB:LastLengthIncrease'
            '@MB:RoutAngleOfXY'
            '@MB:RoutAngleOfXZ'
            '@MB:RoutAngleOfYZ'
            '@MB:MapOnOutputVector')
          ParentShowHint = False
          ShowHint = True
          TabOrder = 0
        end
        object Edit2: TEdit
          Left = 71
          Top = 129
          Width = 65
          Height = 21
          Hint = '@MB:MultiplierForTheLastLengthIncrease'
          ParentShowHint = False
          ShowHint = True
          TabOrder = 1
          Text = '1.0'
        end
        object Edit16: TEdit
          Left = 87
          Top = 177
          Width = 49
          Height = 21
          ParentShowHint = False
          ShowHint = True
          TabOrder = 2
          Text = '0'
        end
        object Edit35: TEdit
          Left = 87
          Top = 153
          Width = 49
          Height = 21
          Hint = '@MB:DoesTheColoringAfterThisNumber'
          ParentShowHint = False
          ShowHint = True
          TabOrder = 3
          Text = '-1'
        end
        object ButtonVolLight: TButton
          Left = 1
          Top = 175
          Width = 83
          Height = 25
          Hint = '@MB:PressToSwitchBetweenDynamicFog'
          Caption = '@MB:DynFogOnIt'
          ParentShowHint = False
          ShowHint = True
          TabOrder = 4
          OnClick = ButtonVolLightClick
        end
        object UpDown5: TUpDown
          Left = 107
          Top = 176
          Width = 19
          Height = 22
          Hint = '@MB:TuneTheSizeOfTheVolumetric'
          Min = -2
          Max = 9
          ParentShowHint = False
          ShowHint = True
          TabOrder = 5
          Visible = False
          OnClick = UpDown5Click
        end
      end
      object TabSheet6: TTabSheet
        Caption = '@MB:Stereo'
        ImageIndex = 7
        ExplicitLeft = 0
        ExplicitTop = 0
        ExplicitWidth = 0
        ExplicitHeight = 0
        object Label28: TLabel
          Left = 25
          Top = 62
          Width = 60
          Height = 13
          Alignment = taRightJustify
          Caption = '@MB:ImageWidth'
          Font.Charset = DEFAULT_CHARSET
          Font.Color = clWindowText
          Font.Height = -12
          Font.Name = 'MS Sans Serif'
          Font.Style = []
          ParentFont = False
        end
        object Label35: TLabel
          Left = 5
          Top = 38
          Width = 80
          Height = 13
          Alignment = taRightJustify
          Caption = '@MB:ScreenDistance'
          Font.Charset = DEFAULT_CHARSET
          Font.Color = clWindowText
          Font.Height = -12
          Font.Name = 'MS Sans Serif'
          Font.Style = []
          ParentFont = False
        end
        object Label27: TLabel
          Left = 4
          Top = 86
          Width = 81
          Height = 13
          Alignment = taRightJustify
          Caption = '@MB:MinimalDistance'
          Font.Charset = DEFAULT_CHARSET
          Font.Color = clWindowText
          Font.Height = -12
          Font.Name = 'MS Sans Serif'
          Font.Style = []
          ParentFont = False
        end
        object Label36: TLabel
          Left = 11
          Top = 12
          Width = 124
          Height = 13
          Caption = '@MB:AllRealworldUnitsInMeter'
          Visible = False
        end
        object Edit15: TEdit
          Left = 91
          Top = 59
          Width = 49
          Height = 21
          Hint = '@MB:DefinesTheRealworldImageWidthOn'
          ParentShowHint = False
          ShowHint = True
          TabOrder = 2
          Text = '1.0'
          OnChange = SpinEdit2Change
        end
        object Edit18: TEdit
          Left = 91
          Top = 35
          Width = 49
          Height = 21
          Hint = '@MB:DefinesTheRealworldDistanceToThe'
          ParentShowHint = False
          ShowHint = True
          TabOrder = 1
          Text = '2.0'
          OnChange = SpinEdit2Change
        end
        object Edit13: TEdit
          Left = 91
          Top = 83
          Width = 49
          Height = 21
          Hint = '@MB:DefinesHowNearInMetersThe'
          ParentShowHint = False
          ShowHint = True
          TabOrder = 3
          Text = '1.5'
          OnChange = SpinEdit2Change
        end
        object Button12: TButton
          Tag = 1
          Left = 8
          Top = 170
          Width = 131
          Height = 22
          Hint = '@MB:RenderBothEyeImagesAndShow'
          Caption = '@MB:RenderStereoPair'
          ParentShowHint = False
          ShowHint = True
          TabOrder = 6
          OnClick = StereoPairClick
        end
        object StereoPairSaveButton: TButton
          Left = 8
          Top = 198
          Width = 131
          Height = 22
          Hint = '@MB:ChooseAFilenameThenRenderBoth'
          Caption = '@MB:RenderSaveStereoPair'
          ParentShowHint = False
          ShowHint = True
          TabOrder = 7
          OnClick = StereoPairSaveClick
        end
        object StereoCrossCheck: TCheckBox
          Left = 8
          Top = 8
          Width = 135
          Height = 17
          Hint = '@MB:OffLeftImageIsOnThe'
          Caption = '@MB:CrossEyedPreview'
          ParentShowHint = False
          ShowHint = True
          TabOrder = 0
        end
        object Button17: TButton
          Tag = 4
          Left = 11
          Top = 8
          Width = 131
          Height = 21
          Hint = '@MB:ClickOnTheButtonAndAfterwards3'
          Caption = '@MB:GetMinDistFromImage'
          ParentShowHint = False
          ShowHint = True
          TabOrder = 8
          Visible = False
          OnClick = Button16Click
        end
        object Button1: TButton
          Tag = 4
          Left = 8
          Top = 114
          Width = 131
          Height = 22
          Hint = '@MB:AfterCalculationAllUserLightChanges'
          Caption = '@MB:CalculateLeftEyeImage'
          ParentShowHint = False
          ShowHint = True
          TabOrder = 4
          OnClick = Button12Click
        end
        object Button6: TButton
          Tag = 3
          Left = 8
          Top = 142
          Width = 131
          Height = 22
          Hint = '@MB:AfterCalculationAllUserLightChanges'
          Caption = '@MB:CalculateRightEyeImage'
          ParentShowHint = False
          ShowHint = True
          TabOrder = 5
          OnClick = Button12Click
        end
      end
    end
    object Panel4: TPanel
      Left = 1
      Top = 531
      Width = 151
      Height = 40
      Align = alBottom
      BevelOuter = bvNone
      TabOrder = 4
      object Image2: TImage
        Left = 127
        Top = 16
        Width = 24
        Height = 24
        Cursor = crSizeNWSE
        Align = alCustom
        AutoSize = True
        Picture.Data = {
          07544269746D6170F6060000424DF60600000000000036000000280000001800
          0000180000000100180000000000C0060000C30E0000C30E0000000000000000
          0000D8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9EC
          D8E9ECD8E9ECFFFFFF99A8AC99A8ACD8E9ECFFFFFF99A8AC99A8ACD8E9ECFFFF
          FF99A8AC99A8ACD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8
          E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECFFFFFF99A8AC99A8ACD8E9ECFFFFFF
          99A8AC99A8ACD8E9ECFFFFFF99A8AC99A8ACD8E9ECD8E9ECD8E9ECD8E9ECD8E9
          ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECFFFFFF99
          A8AC99A8ACD8E9ECFFFFFF99A8AC99A8ACD8E9ECFFFFFF99A8ACD8E9ECD8E9EC
          D8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9
          ECD8E9ECD8E9ECFFFFFF99A8AC99A8ACD8E9ECFFFFFF99A8AC99A8ACD8E9ECFF
          FFFFD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9EC
          D8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECFFFFFF99A8AC99A8ACD8E9ECFFFF
          FF99A8AC99A8ACD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8
          E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECFFFFFF
          99A8AC99A8ACD8E9ECFFFFFF99A8AC99A8ACD8E9ECD8E9ECD8E9ECD8E9ECD8E9
          ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8
          E9ECD8E9ECD8E9ECFFFFFF99A8AC99A8ACD8E9ECFFFFFF99A8ACD8E9ECD8E9EC
          D8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9
          ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECFFFFFF99A8AC99A8ACD8E9ECFF
          FFFFD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9EC
          D8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECFFFF
          FF99A8AC99A8ACD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8
          E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9EC
          D8E9ECD8E9ECD8E9ECFFFFFF99A8AC99A8ACD8E9ECD8E9ECD8E9ECD8E9ECD8E9
          ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8
          E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECFFFFFF99A8ACD8E9ECD8E9EC
          D8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9
          ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECFF
          FFFFD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9EC
          D8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9
          ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8
          E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9EC
          D8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9
          ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8
          E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9EC
          D8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9
          ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8
          E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9EC
          D8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9
          ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8
          E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9EC
          D8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9
          ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8
          E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9EC
          D8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9
          ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8
          E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9EC
          D8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9
          ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8
          E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9EC
          D8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9
          ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8
          E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9EC
          D8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9
          ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8E9ECD8
          E9EC}
        Transparent = True
        OnMouseDown = Image2MouseDown
        OnMouseMove = Image2MouseMove
        ExplicitWidth = 19
        ExplicitHeight = 20
      end
      object Label19: TLabel
        Left = 8
        Top = 2
        Width = 3
        Height = 13
        Font.Charset = DEFAULT_CHARSET
        Font.Color = clWindowText
        Font.Height = -12
        Font.Name = 'MS Sans Serif'
        Font.Style = []
        ParentFont = False
      end
      object Label1: TLabel
        Left = 35
        Top = 20
        Width = 3
        Height = 13
      end
    end
    object Panel6: TPanel
      Left = 1
      Top = 351
      Width = 151
      Height = 95
      Align = alTop
      BevelOuter = bvNone
      TabOrder = 2
      object SpeedButton3: TSpeedButton
        Left = 2
        Top = 71
        Width = 47
        Height = 21
        Hint = '@MB:QuickQualitySettings'
        AllowAllUp = True
        GroupIndex = 3
        Down = True
        Caption = '@MB:Preview'
        Flat = True
        ParentShowHint = False
        ShowHint = True
        OnClick = SpeedButton3Click
      end
      object SpeedButton5: TSpeedButton
        Tag = 2
        Left = 84
        Top = 71
        Width = 32
        Height = 21
        AllowAllUp = True
        GroupIndex = 3
        Caption = '@MB:Mid'
        Flat = True
        OnClick = SpeedButton3Click
      end
      object SpeedButton6: TSpeedButton
        Tag = 3
        Left = 116
        Top = 71
        Width = 33
        Height = 21
        AllowAllUp = True
        GroupIndex = 3
        Caption = '@MB:High'
        Flat = True
        OnClick = SpeedButton3Click
      end
      object SpeedButton13: TSpeedButton
        Tag = 1
        Left = 49
        Top = 71
        Width = 35
        Height = 21
        AllowAllUp = True
        GroupIndex = 3
        Caption = '@MB:Video'
        Flat = True
        OnClick = SpeedButton3Click
      end
      object SpeedButton9: TSpeedButton
        Left = 4
        Top = 3
        Width = 27
        Height = 21
        Hint = '@MB:LeftclickUndoSetsTheParameterTo'
        Enabled = False
        Flat = True
        Glyph.Data = {
          F6000000424DF600000000000000760000002800000010000000100000000100
          04000000000080000000C30E0000C30E0000100000001000000000000000C6C6
          C600FFFFFF00FFFFFF00FFFFFF00FFFFFF00FFFFFF00FFFFFF00FFFFFF00FFFF
          FF00FFFFFF00FFFFFF00FFFFFF00FFFFFF00FFFFFF00FFFFFF00111111111111
          1111111111111111111111111111111111111111111111111111111111111111
          0111110000011111011111000011111110111100011111111011110010111111
          1011110111001111011111111111000011111111111111111111111111111111
          1111111111111111111111111111111111111111111111111111}
        ParentShowHint = False
        ShowHint = True
        OnMouseUp = SpeedButton9MouseUp
      end
      object Button2: TButton
        Left = -1
        Top = 25
        Width = 81
        Height = 43
        Hint = '@MB:MainRenderingOfTheImage'
        Caption = '@MB:Calculate3D'
        ParentShowHint = False
        ShowHint = True
        TabOrder = 0
        OnClick = Button2Click
        OnMouseDown = Button2MouseDown
      end
      object Button10: TButton
        Left = 86
        Top = 6
        Width = 63
        Height = 20
        Hint = '@MB:PressToBringUpTheFormula'
        Caption = '@MB:Formulas'
        ParentShowHint = False
        ShowHint = True
        TabOrder = 1
        OnClick = Button10Click
        OnMouseDown = Button10MouseDown
      end
      object Button15: TButton
        Tag = 2
        Left = 86
        Top = 48
        Width = 63
        Height = 20
        Hint = '@MB:PressToBringUpThePostprocessing'
        Caption = '@MB:Postprocess'
        ParentShowHint = False
        ShowHint = True
        TabOrder = 2
        OnClick = Button15Click
        OnMouseDown = Button10MouseDown
      end
      object Button11: TButton
        Tag = 1
        Left = 43
        Top = 6
        Width = 37
        Height = 19
        Hint = '@MB:CalculateAFast8x8BlockyImage'
        Caption = '@MB:Calc'
        ParentShowHint = False
        ShowHint = True
        TabOrder = 3
        OnClick = Button11Click
      end
      object Button18: TButton
        Tag = 1
        Left = 86
        Top = 27
        Width = 63
        Height = 20
        Hint = '@MB:PressToBringUpTheLight'
        Caption = '@MB:Lighting'
        ParentShowHint = False
        ShowHint = True
        TabOrder = 4
        OnClick = Button18Click
        OnMouseDown = Button10MouseDown
      end
    end
  end
  object Panel5: TPanel
    Left = 0
    Top = 0
    Width = 618
    Height = 572
    Align = alClient
    BevelOuter = bvNone
    DragKind = dkDock
    TabOrder = 1
    object Panel3: TPanel
      Left = 0
      Top = 537
      Width = 618
      Height = 35
      Align = alBottom
      TabOrder = 0
      object SpeedButton1: TSpeedButton
        Left = 6
        Top = 3
        Width = 48
        Height = 29
        Hint = '@MB:PushDownTo2DZoomIn'
        AllowAllUp = True
        GroupIndex = 1
        Down = True
        Flat = True
        Glyph.Data = {
          760B0000424D760B000000000000360000002800000028000000180000000100
          180000000000400B0000120B0000120B00000000000000000000FFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF00000000000000000000000000000000
          0000000000000000000000000000000000000000000000000000000000000000
          000000000000000000000000000000000000000000000000FFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFF000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFF000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFF000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF000000FFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF000000
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF0000
          00000000FFFFFFFFFFFFFFFFFFFFFFFF000000FFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF000000FFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF000000000000000000FFFFFFFF
          FFFFFFFFFFFFFFFF000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFF000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFF000000000000000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          000000FFFFFFFFFFFFFFFFFFFFFFFF080504080504080504C0C0C0C0C0C00805
          04080504C0C0C0C0C0C0080504080504FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFF000000FFFFFFFFFFFFFFFFFF000000000000000000000000000000
          000000000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF000000FFFFFFFFFF
          FFFFFFFFFFFFFF080504FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFF080504FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF000000
          FFFFFFFFFFFF000000FFFFFFFFFFFFFFFFFFFFFFFF000000000000FFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF000000FFFFFFFFFFFFFFFFFFFFFFFFC0
          C0C0FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF080504
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF000000FFFFFF000000FFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF000000FFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFF000000FFFFFFFFFFFFFFFFFFFFFFFFC0C0C0FFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFC0C0C0FFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFF000000FFFFFF000000FFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFF000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          000000FFFFFFFFFFFFFFFFFFFFFFFF080504FFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFC0C0C0FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFF000000FFFFFF000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF000000FFFFFFFFFF
          FFFFFFFFFFFFFF080504FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFF080504FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF000000
          FFFFFF000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF000000FFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF000000FFFFFFFFFFFFFFFFFFFFFFFFC0
          C0C0FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF080504
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF000000FFFFFFFFFFFF0000
          00FFFFFFFFFFFFFFFFFFFFFFFF000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFF000000FFFFFFFFFFFFFFFFFFFFFFFFC0C0C0FFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFC0C0C0FFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFF000000FFFFFFFFFFFFFFFFFF00000000000000
          0000000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          000000FFFFFFFFFFFFFFFFFFFFFFFF080504FFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFC0C0C0FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFF000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF000000FFFFFFFFFF
          FFFFFFFFFFFFFF080504FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFF080504FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF000000
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF000000FFFFFFFFFFFFFFFFFFFFFFFF08
          0504080504C0C0C0C0C0C0080504080504C0C0C0C0C0C0080504080504080504
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF000000FFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFF000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFF000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFF000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF0000000000000000
          0000000000000000000000000000000000000000000000000000000000000000
          0000000000000000000000000000000000000000000000000000000000000000
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF}
        ParentShowHint = False
        ShowHint = True
        OnClick = SpeedButton1Click
      end
      object SpeedButton2: TSpeedButton
        Left = 54
        Top = 3
        Width = 41
        Height = 29
        Hint = '@MB:PushDownToShiftTheObject'
        AllowAllUp = True
        GroupIndex = 1
        Flat = True
        Glyph.Data = {
          760B0000424D760B000000000000360000002800000028000000180000000100
          180000000000400B0000120B0000120B00000000000000000000FFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF00
          0000000000000000000000000000000000000000000000000000000000000000
          0000000000000000000000000000000000000000000000000000000000000000
          00000000000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF000000FFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF000000FF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFF000000FFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF000000FFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFF000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFF000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF00
          0000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          5A5A5AFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFF000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF000000FFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF5A5A5A0000005A5A5AFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF000000FF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFF000000FFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFF5A5A5A0000000000000000005A5A5AFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF000000FFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFF000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFF000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFF000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF00
          0000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF5A5A5AFFFFFFFFFFFFFFFFFF
          000000FFFFFFFFFFFFFFFFFF5A5A5AFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFF000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF000000FFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFF5A5A5A000000FFFFFFFFFFFFFFFFFF000000FFFFFFFFFF
          FFFFFFFF0000005A5A5AFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF000000FF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFF000000FFFFFFFFFFFFFFFFFFFFFFFF5A5A
          5A00000000000000000000000000000000000000000000000000000000000000
          00005A5A5AFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF000000FFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFF000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFF5A5A5A000000FF
          FFFFFFFFFFFFFFFF000000FFFFFFFFFFFFFFFFFF0000005A5A5AFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFF000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF00
          0000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF5A5A5AFFFFFFFFFFFFFFFFFF
          000000FFFFFFFFFFFFFFFFFF5A5A5AFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFF000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF000000FFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF000000FFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF000000FF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFF000000FFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFF5A5A5A0000000000000000005A5A5AFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF000000FFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFF000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFF5A5A5A0000005A5A5AFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFF000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF00
          0000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          5A5A5AFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFF000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF000000FFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF000000FF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFF000000FFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF000000FFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFF00000000000000000000000000000000000000000000000000
          0000000000000000000000000000000000000000000000000000000000000000
          000000000000000000000000000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF}
        ParentShowHint = False
        ShowHint = True
        OnClick = SpeedButton1Click
      end
      object SpeedButton4: TSpeedButton
        Left = 95
        Top = 3
        Width = 41
        Height = 29
        Hint = '@MB:PushDownToMoveTheObject'
        AllowAllUp = True
        GroupIndex = 1
        Flat = True
        Glyph.Data = {
          760B0000424D760B000000000000360000002800000028000000180000000100
          180000000000400B0000120B0000120B00000000000000000000FFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF00
          0000000000000000000000000000000000000000000000000000000000000000
          0000000000000000000000000000000000000000000000000000000000000000
          00000000000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF000000FFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF000000FF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFF000000FFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF000000FFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFF000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFC0
          C0C0FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFF000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF00
          0000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF404040000000C0C0C0
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFF000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF000000FFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFC0C0C0000000000000000000C0C0C0FFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF000000FF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFF000000FFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFF404040000000000000000000000000000000FFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF000000FFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFF000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFC0C0C0000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFF000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF00
          0000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF404040
          5A5A5AFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFF000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF000000FFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF000000FFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF000000FF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFF000000FFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFC0C0C05A5A5AFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF000000FFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFF000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFF000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFF000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF00
          0000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFC0C0C0808080FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFF000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF000000FFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF0000
          00FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF000000FF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFF000000FFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF000000000000000000000000C0
          C0C0FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF000000FFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFF000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFF8080800000005A5A5AFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFF000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF00
          0000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFC0C0C0FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFF000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF000000FFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF000000FF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFF000000FFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF000000FFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFF00000000000000000000000000000000000000000000000000
          0000000000000000000000000000000000000000000000000000000000000000
          000000000000000000000000000000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
          FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF}
        ParentShowHint = False
        ShowHint = True
        OnClick = SpeedButton1Click
      end
      object Label20: TLabel
        Left = 138
        Top = 11
        Width = 3
        Height = 13
      end
      object SpeedButton11: TSpeedButton
        Tag = 11
        Left = 205
        Top = 0
        Width = 18
        Height = 17
        Hint = '@MB:LeftclickRotateTheBulbAroundThe'
        AllowAllUp = True
        Flat = True
        Glyph.Data = {
          3E020000424D3E0200000000000036000000280000000D0000000D0000000100
          18000000000008020000110B0000110B0000000000000000000084C4C384C4C3
          84C4C384C4C384C4C384C4C384C4C384C4C384C4C384C4C384C4C384C4C384C4
          C30084C4C384C4C384C4C384C4C300000000000000000000000000000084C4C3
          84C4C384C4C384C4C30084C4C384C4C384C4C384C4C30000003E3E3E3E3E3E3E
          3E3E00000084C4C384C4C384C4C384C4C30084C4C384C4C384C4C384C4C30000
          0045454545454545454500000084C4C384C4C384C4C384C4C30084C4C384C4C3
          84C4C384C4C30000004C4C4C4C4C4C4C4C4C00000084C4C384C4C384C4C384C4
          C30084C4C384C4C384C4C384C4C300000052525252525252525200000084C4C3
          84C4C384C4C384C4C30084C4C384C4C384C4C384C4C300000059595959595959
          595900000084C4C384C4C384C4C384C4C30084C4C384C4C30000000000000000
          0060606060606060606000000000000000000084C4C384C4C30084C4C384C4C3
          00000029292967676767676767676767676767676724242400000084C4C384C4
          C30084C4C384C4C384C4C30000002929296E6E6E6E6E6E6E6E6E292929000000
          84C4C384C4C384C4C30084C4C384C4C384C4C384C4C300000033333380808033
          333300000084C4C384C4C384C4C384C4C30084C4C384C4C384C4C384C4C384C4
          C300000033333300000084C4C384C4C384C4C384C4C384C4C30084C4C384C4C3
          84C4C384C4C384C4C384C4C300000084C4C384C4C384C4C384C4C384C4C384C4
          C300}
        Margin = 1
        ParentShowHint = False
        ShowHint = True
        Spacing = 0
        OnMouseUp = SpeedButton18MouseUp
      end
      object SpeedButton16: TSpeedButton
        Tag = 12
        Left = 205
        Top = 17
        Width = 18
        Height = 17
        Hint = '@MB:LeftclickRotateTheBulbAroundThe'
        AllowAllUp = True
        Flat = True
        Glyph.Data = {
          3E020000424D3E0200000000000036000000280000000D0000000D0000000100
          18000000000008020000110B0000110B0000000000000000000084C4C384C4C3
          84C4C384C4C384C4C384C4C300000084C4C384C4C384C4C384C4C384C4C384C4
          C30084C4C384C4C384C4C384C4C384C4C300000033333300000084C4C384C4C3
          84C4C384C4C384C4C30084C4C384C4C384C4C384C4C300000033333380808033
          333300000084C4C384C4C384C4C384C4C30084C4C384C4C384C4C30000002929
          296E6E6E6E6E6E6E6E6E29292900000084C4C384C4C384C4C30084C4C384C4C3
          00000029292967676767676767676767676767676729292900000084C4C384C4
          C30084C4C384C4C3000000000000000000606060606060606060000000000000
          00000084C4C384C4C30084C4C384C4C384C4C384C4C300000059595959595959
          595900000084C4C384C4C384C4C384C4C30084C4C384C4C384C4C384C4C30000
          0052525252525252525200000084C4C384C4C384C4C384C4C30084C4C384C4C3
          84C4C384C4C30000004C4C4C4C4C4C4C4C4C00000084C4C384C4C384C4C384C4
          C30084C4C384C4C384C4C384C4C300000045454545454545454500000084C4C3
          84C4C384C4C384C4C30084C4C384C4C384C4C384C4C30000003E3E3E3E3E3E3E
          3E3E00000084C4C384C4C384C4C384C4C30084C4C384C4C384C4C384C4C30000
          0000000000000000000000000084C4C384C4C384C4C384C4C30084C4C384C4C3
          84C4C384C4C384C4C384C4C384C4C384C4C384C4C384C4C384C4C384C4C384C4
          C300}
        Margin = 1
        ParentShowHint = False
        ShowHint = True
        Spacing = 0
        OnMouseUp = SpeedButton18MouseUp
      end
      object SpeedButton17: TSpeedButton
        Tag = 13
        Left = 188
        Top = 8
        Width = 17
        Height = 19
        Hint = '@MB:LeftclickRotateTheBulbAroundThe2'
        AllowAllUp = True
        Flat = True
        Glyph.Data = {
          3E020000424D3E0200000000000036000000280000000D0000000D0000000100
          18000000000008020000110B0000110B0000000000000000000084C4C384C4C3
          84C4C384C4C384C4C384C4C384C4C384C4C384C4C384C4C384C4C384C4C384C4
          C30084C4C384C4C384C4C384C4C384C4C384C4C384C4C384C4C384C4C384C4C3
          84C4C384C4C384C4C30084C4C384C4C384C4C384C4C300000000000084C4C384
          C4C384C4C384C4C384C4C384C4C384C4C30084C4C384C4C384C4C30000002424
          2400000084C4C384C4C384C4C384C4C384C4C384C4C384C4C30084C4C384C4C3
          00000024242467676700000000000000000000000000000000000000000084C4
          C30084C4C30000002424246E6E6E6767676060605959595252524C4C4C454545
          3E3E3E00000084C4C3000000002424246E6E6E6E6E6E67676760606059595952
          52524C4C4C4545453E3E3E00000084C4C30084C4C30000002424246E6E6E6767
          676060605959595252524C4C4C4545453E3E3E00000084C4C30084C4C384C4C3
          00000024242467676700000000000000000000000000000000000000000084C4
          C30084C4C384C4C384C4C300000024242400000084C4C384C4C384C4C384C4C3
          84C4C384C4C384C4C30084C4C384C4C384C4C384C4C300000000000084C4C384
          C4C384C4C384C4C384C4C384C4C384C4C30084C4C384C4C384C4C384C4C384C4
          C384C4C384C4C384C4C384C4C384C4C384C4C384C4C384C4C30084C4C384C4C3
          84C4C384C4C384C4C384C4C384C4C384C4C384C4C384C4C384C4C384C4C384C4
          C300}
        ParentShowHint = False
        ShowHint = True
        OnMouseUp = SpeedButton18MouseUp
      end
      object SpeedButton18: TSpeedButton
        Tag = 14
        Left = 224
        Top = 8
        Width = 16
        Height = 19
        Hint = '@MB:LeftclickRotateTheBulbAroundThe2'
        AllowAllUp = True
        Flat = True
        Glyph.Data = {
          3E020000424D3E0200000000000036000000280000000D0000000D0000000100
          18000000000008020000110B0000110B0000000000000000000084C4C384C4C3
          84C4C384C4C384C4C384C4C384C4C384C4C384C4C384C4C384C4C384C4C384C4
          C30084C4C384C4C384C4C384C4C384C4C384C4C384C4C384C4C384C4C384C4C3
          84C4C384C4C384C4C30084C4C384C4C384C4C384C4C384C4C384C4C384C4C300
          000000000084C4C384C4C384C4C384C4C30084C4C384C4C384C4C384C4C384C4
          C384C4C384C4C300000024242400000084C4C384C4C384C4C30084C4C3000000
          00000000000000000000000000000000000067676724242400000084C4C384C4
          C30084C4C30000003E3E3E4545454C4C4C5252525959596060606767676E6E6E
          24242400000084C4C30084C4C30000003E3E3E4545454C4C4C52525259595960
          60606767676E6E6E6E6E6E2424240000000084C4C30000003E3E3E4545454C4C
          4C5252525959596060606767676E6E6E24242400000084C4C30084C4C3000000
          00000000000000000000000000000000000067676724242400000084C4C384C4
          C30084C4C384C4C384C4C384C4C384C4C384C4C384C4C3000000242424000000
          84C4C384C4C384C4C30084C4C384C4C384C4C384C4C384C4C384C4C384C4C300
          000000000084C4C384C4C384C4C384C4C30084C4C384C4C384C4C384C4C384C4
          C384C4C384C4C384C4C384C4C384C4C384C4C384C4C384C4C30084C4C384C4C3
          84C4C384C4C384C4C384C4C384C4C384C4C384C4C384C4C384C4C384C4C384C4
          C300}
        ParentShowHint = False
        ShowHint = True
        OnMouseUp = SpeedButton18MouseUp
      end
      object Label6: TLabel
        Left = 418
        Top = 11
        Width = 3
        Height = 13
      end
      object SpeedButton22: TSpeedButton
        Tag = 15
        Left = 166
        Top = 0
        Width = 20
        Height = 17
        Hint = '@MB:LeftclickRotateTheBulbAroundThe3'
        AllowAllUp = True
        Flat = True
        Glyph.Data = {
          9E020000424D9E0200000000000036000000280000000E0000000E0000000100
          18000000000068020000110B0000110B0000000000000000000084C4C384C4C3
          84C4C384C4C384C4C384C4C384C4C384C4C384C4C384C4C384C4C384C4C384C4
          C384C4C3000084C4C384C4C384C4C384C4C384C4C384C4C384C4C384C4C384C4
          C384C4C384C4C384C4C384C4C384C4C300000000000000000000000000000000
          0000000000000000000084C4C384C4C384C4C384C4C384C4C384C4C300000000
          007D7D7D7373736E6E6E67676742424200000084C4C384C4C384C4C384C4C384
          C4C384C4C384C4C300000000007D7D7D7373736E6E6E5F5F5F00000084C4C384
          C4C384C4C384C4C300000000000084C4C384C4C300000000007D7D7D7373736E
          6E6E57575700000084C4C384C4C384C4C300000026262614141400000084C4C3
          00000000007D7D7D6B6B6B6E6E6E6767674C4C4C000000000000000000303030
          3E3E3E36363620202000000000000000004F4F4F0000004F4F4F676767606060
          5252524747474848484545453E3E3E3636362C2C2C0000000000000000000000
          84C4C30000004F4F4F6060605959595252524C4C4C4646463E3E3E3030300000
          0084C4C3000000000084C4C384C4C384C4C30000000000003E3E3E4E4E4E4545
          4538383800000000000084C4C384C4C3000084C4C384C4C384C4C384C4C384C4
          C384C4C300000000000000000000000084C4C384C4C384C4C384C4C3000084C4
          C384C4C384C4C384C4C384C4C384C4C384C4C384C4C384C4C384C4C384C4C384
          C4C384C4C384C4C3000084C4C384C4C384C4C384C4C384C4C384C4C384C4C384
          C4C384C4C384C4C384C4C384C4C384C4C384C4C3000084C4C384C4C384C4C384
          C4C384C4C384C4C384C4C384C4C384C4C384C4C384C4C384C4C384C4C384C4C3
          0000}
        ParentShowHint = False
        ShowHint = True
        OnMouseUp = SpeedButton18MouseUp
      end
      object SpeedButton23: TSpeedButton
        Tag = 16
        Left = 166
        Top = 17
        Width = 20
        Height = 17
        Hint = '@MB:LeftclickRotateTheBulbAroundThe3'
        AllowAllUp = True
        Flat = True
        Glyph.Data = {
          9E020000424D9E0200000000000036000000280000000E0000000E0000000100
          18000000000068020000110B0000110B0000000000000000000084C4C384C4C3
          84C4C384C4C384C4C384C4C384C4C384C4C384C4C384C4C384C4C384C4C384C4
          C384C4C3000084C4C384C4C384C4C384C4C384C4C384C4C384C4C384C4C384C4
          C384C4C384C4C384C4C384C4C384C4C3000084C4C384C4C384C4C384C4C384C4
          C384C4C384C4C384C4C384C4C384C4C384C4C384C4C384C4C384C4C3000084C4
          C384C4C384C4C384C4C384C4C384C4C300000000000000000000000084C4C384
          C4C384C4C384C4C3000000000084C4C384C4C384C4C30000000000003A3A3A4E
          4E4E45454538383800000000000084C4C384C4C3000000000000000084C4C300
          00002F2F2F4C4C4C5959595252524C4C4C4646463E3E3E30303000000084C4C3
          00000000004F4F4F0000003333335353536060605A5A5A4B4B4B484848454545
          3E3E3E3636362C2C2C00000000000000007373734747475656566767675C5C5C
          0000000000000000003030303E3E3E3636362020200000000000000000737373
          7373736E6E6E57575700000084C4C384C4C384C4C30000002626261414140000
          0084C4C300000000007373737373736E6E6E63636300000084C4C384C4C384C4
          C384C4C300000000000084C4C384C4C300000000007373737373736E6E6E6767
          674E4E4E00000084C4C384C4C384C4C384C4C384C4C384C4C384C4C300000000
          0000000000000000000000000000000000000000000084C4C384C4C384C4C384
          C4C384C4C384C4C3000084C4C384C4C384C4C384C4C384C4C384C4C384C4C384
          C4C384C4C384C4C384C4C384C4C384C4C384C4C3000084C4C384C4C384C4C384
          C4C384C4C384C4C384C4C384C4C384C4C384C4C384C4C384C4C384C4C384C4C3
          0000}
        ParentShowHint = False
        ShowHint = True
        OnMouseUp = SpeedButton18MouseUp
      end
      object ProgressBar1: TProgressBar
        Left = 288
        Top = 8
        Width = 126
        Height = 19
        Smooth = True
        TabOrder = 0
        Visible = False
      end
      object Edit4: TEdit
        Left = 243
        Top = 7
        Width = 30
        Height = 21
        Hint = '@MB:DegreeOfRotation'
        ParentShowHint = False
        ShowHint = True
        TabOrder = 1
        Text = '5'
      end
      object Panel7: TPanel
        Left = 509
        Top = 1
        Width = 108
        Height = 33
        Align = alRight
        BevelOuter = bvNone
        TabOrder = 2
        object Label46: TLabel
          Left = 1
          Top = 9
          Width = 32
          Height = 13
          Caption = '@MB:Frame'
          Font.Charset = DEFAULT_CHARSET
          Font.Color = clWindowText
          Font.Height = -12
          Font.Name = 'MS Sans Serif'
          Font.Style = []
          ParentFont = False
        end
        object FrameEdit: TEdit
          Tag = 1
          Left = 35
          Top = 7
          Width = 49
          Height = 21
          Hint = '@MB:FrameNumberToUseForAnimated'
          ParentShowHint = False
          ShowHint = True
          TabOrder = 0
          Text = '1'
          OnExit = FrameEditExit
        end
        object FrameUpDown: TUpDown
          Left = 86
          Top = 7
          Width = 18
          Height = 21
          Hint = '@MB:IncreaseDecreaseFrameNumber'
          Min = -30000
          Max = 30000
          ParentShowHint = False
          ShowHint = True
          TabOrder = 1
          OnClick = FrameUpDownClick
        end
      end
    end
    object ScrollBox1: TScrollBox
      Left = 0
      Top = 73
      Width = 618
      Height = 464
      HorzScrollBar.Tracking = True
      VertScrollBar.Tracking = True
      Align = alClient
      BevelOuter = bvNone
      BorderStyle = bsNone
      TabOrder = 1
      object Image1: TImage
        Left = 0
        Top = 1
        Width = 480
        Height = 400
        AutoSize = True
        OnMouseDown = Image1MouseDown
        OnMouseMove = Image1MouseMove
        OnMouseUp = Image1MouseUp
      end
      object Shape1: TShape
        Left = 72
        Top = 104
        Width = 65
        Height = 65
        Brush.Style = bsClear
        Pen.Color = clWhite
        Pen.Mode = pmXor
        Pen.Style = psDot
        Visible = False
        OnMouseDown = Shape1MouseDown
      end
      object Shape2: TShape
        Left = 176
        Top = 88
        Width = 9
        Height = 9
        Cursor = crCross
        Brush.Style = bsClear
        Pen.Color = clWhite
        Pen.Mode = pmXor
        Shape = stCircle
        Visible = False
        OnMouseDown = Shape2MouseDown
        OnMouseMove = Shape2MouseMove
      end
    end
    object Panel2: TPanel
      Left = 0
      Top = 0
      Width = 618
      Height = 73
      Align = alTop
      TabOrder = 2
      object SpeedButton12: TSpeedButton
        Left = 2
        Top = 5
        Width = 52
        Height = 30
        Hint = '@MB:OpenTheAnimationMaker'
        Caption = '@MB:Animations'
        ParentShowHint = False
        ShowHint = True
        OnClick = SpeedButton12Click
      end
      object SpeedButton15: TSpeedButton
        Left = 2
        Top = 38
        Width = 52
        Height = 31
        Hint = '@MB:OpenTheNavigator'
        Caption = '@MB:Navigator'
        ParentShowHint = False
        ShowHint = True
        Spacing = 0
        OnClick = SpeedButton15Click
      end
      object MeshExportBtn: TSpeedButton
        Left = 55
        Top = 5
        Width = 52
        Height = 30
        Hint = '@MB:CreateMeshes'
        Caption = '@MB:BTracer2'
        ParentShowHint = False
        ShowHint = True
        OnClick = MeshExportBtnClick
      end
      object MutaGenBtn: TSpeedButton
        Left = 55
        Top = 38
        Width = 52
        Height = 31
        Hint = '@MB:OpenTheMutaGen'
        Caption = '@MB:MutaGen'
        ParentShowHint = False
        ShowHint = True
        Spacing = 0
        OnClick = MutaGenBtnClick
      end
      object ZBufferGenBtn: TSpeedButton
        Left = 108
        Top = 38
        Width = 52
        Height = 30
        Hint = '@MB:AdvancedZBufferGenerator'
        Caption = '@MB:ZBuf16Bit'
        ParentShowHint = False
        ShowHint = True
        OnClick = ZBufferGenBtnClick
      end
      object HeightMapGenBtn: TSpeedButton
        Left = 108
        Top = 5
        Width = 52
        Height = 30
        Hint = '@MB:ScriptEditor'
        Caption = '@MB:HMapGen'
        ParentShowHint = False
        ShowHint = True
        OnClick = HeightMapGenBtnClick
      end
      object GroupBox1: TGroupBox
        Left = 470
        Top = 1
        Width = 205
        Height = 69
        Caption = '@MB:Image'
        TabOrder = 0
        object Label11: TLabel
          Left = 8
          Top = 14
          Width = 31
          Height = 13
          Caption = '@MB:Width'
          Font.Charset = DEFAULT_CHARSET
          Font.Color = clWindowText
          Font.Height = -12
          Font.Name = 'MS Sans Serif'
          Font.Style = []
          ParentFont = False
        end
        object SpeedButton19: TSpeedButton
          Tag = 1
          Left = 161
          Top = 23
          Width = 34
          Height = 14
          Hint = '@MB:ClickToSetTheImageHeight'
          AllowAllUp = True
          Caption = '4:3'
          Flat = True
          ParentShowHint = False
          ShowHint = True
          OnClick = SpeedButton19Click
        end
        object SpeedButton20: TSpeedButton
          Tag = 2
          Left = 161
          Top = 37
          Width = 34
          Height = 14
          Hint = '@MB:ClickToSetTheImageHeight'
          AllowAllUp = True
          Caption = '5:3'
          Flat = True
          ParentShowHint = False
          ShowHint = True
          OnClick = SpeedButton19Click
        end
        object SpeedButton21: TSpeedButton
          Tag = 3
          Left = 161
          Top = 51
          Width = 34
          Height = 14
          Hint = '@MB:UserDefinedApectRatioRightclickTo'
          AllowAllUp = True
          Caption = '@MB:User'
          Flat = True
          ParentShowHint = False
          ShowHint = True
          OnClick = SpeedButton19Click
          OnMouseDown = SpeedButton21MouseDown
        end
        object Label4: TLabel
          Left = 160
          Top = 8
          Width = 36
          Height = 13
          Caption = '@MB:Aspect'
          Font.Charset = DEFAULT_CHARSET
          Font.Color = clWindowText
          Font.Height = -12
          Font.Name = 'MS Sans Serif'
          Font.Style = []
          ParentFont = False
        end
        object Label25: TLabel
          Left = 62
          Top = 14
          Width = 34
          Height = 13
          Caption = '@MB:Height'
        end
        object Label12: TLabel
          Left = 115
          Top = 14
          Width = 30
          Height = 13
          Caption = '@MB:Scale'
        end
        object Edit11: TEdit
          Tag = 1
          Left = 7
          Top = 29
          Width = 49
          Height = 21
          TabOrder = 0
          Text = '480'
          OnChange = Edit11Change
        end
        object Edit12: TEdit
          Tag = 2
          Left = 61
          Top = 29
          Width = 49
          Height = 21
          TabOrder = 1
          Text = '360'
          OnChange = Edit11Change
        end
        object CheckBox10: TCheckBox
          Left = 7
          Top = 51
          Width = 82
          Height = 15
          Hint = '@MB:KeepTheAspectRatioWhenManually'
          Caption = '@MB:KeepAspect'
          Checked = True
          ParentShowHint = False
          ShowHint = True
          State = cbChecked
          TabOrder = 2
          OnClick = CheckBox10Click
        end
        object CheckBox11: TCheckBox
          Left = 93
          Top = 51
          Width = 68
          Height = 15
          Hint = '@MB:ScaleClipRadiusDoFAndTheDE'
          Caption = '@MB:DEstopC'
          Checked = True
          ParentShowHint = False
          ShowHint = True
          State = cbChecked
          TabOrder = 3
        end
        object UpDown2: TUpDown
          Left = 116
          Top = 29
          Width = 30
          Height = 21
          Hint = '@MB:ScaleTheImageSizeBy2'
          Min = -30000
          Max = 30000
          ParentShowHint = False
          ShowHint = True
          TabOrder = 4
          OnClick = UpDown2Click
        end
      end
      object PageControl2: TPageControl
        Left = 164
        Top = 4
        Width = 268
        Height = 64
        ActivePage = TabSheet12
        TabOrder = 1
        object TabSheet7: TTabSheet
          Caption = '@MB:Open'
          ExplicitLeft = 0
          ExplicitTop = 0
          ExplicitWidth = 0
          ExplicitHeight = 0
          object Button9: TSpeedButton
            Left = 2
            Top = 3
            Width = 60
            Height = 31
            Hint = '@MB:OpenFullM3I'
            Caption = '@MB:OpenM3i'
            ParentShowHint = False
            ShowHint = True
            OnClick = Button9Click
          end
          object Button5: TSpeedButton
            Left = 68
            Top = 3
            Width = 60
            Height = 30
            Hint = '@MB:OpenParameter'
            Caption = '@MB:OpenM3p'
            ParentShowHint = False
            ShowHint = True
            OnClick = Button5Click
          end
          object ResumeBtn: TSpeedButton
            Left = 130
            Top = 3
            Width = 40
            Height = 30
            Hint = '@MB:UnfinishedRendersContinueOrDeleteThem'
            Caption = '@MB:Resume'
            ParentShowHint = False
            ShowHint = True
            OnClick = ResumeBtnClick
          end
          object SpeedButton8: TSpeedButton
            Left = 172
            Top = 3
            Width = 83
            Height = 30
            Hint = '@MB:OpenParameterFromClipboardTextSelect'
            Caption = '@MB:FromClipboard'
            ParentShowHint = False
            ShowHint = True
            OnClick = SpeedButton8Click
          end
        end
        object TabSheet10: TTabSheet
          Caption = '@MB:Save'
          ImageIndex = 1
          ExplicitLeft = 0
          ExplicitTop = 0
          ExplicitWidth = 0
          ExplicitHeight = 0
          object Button8: TSpeedButton
            Left = 2
            Top = 3
            Width = 40
            Height = 31
            Hint = '@MB:SaveCompleteM3I'
            Caption = 'm3i'
            ParentShowHint = False
            ShowHint = True
            OnClick = Button8Click
          end
          object Button4: TSpeedButton
            Left = 84
            Top = 3
            Width = 40
            Height = 30
            Hint = '@MB:SaveParametersOnly'
            Caption = 'm3p'
            ParentShowHint = False
            ShowHint = True
            OnClick = Button4Click
          end
          object SpeedButton7: TSpeedButton
            Left = 184
            Top = 3
            Width = 69
            Height = 30
            Hint = '@MB:SaveParametersToClipboardAsText'
            Caption = '@MB:ToClipboard'
            ParentShowHint = False
            ShowHint = True
            OnClick = SpeedButton7Click
          end
          object SpeedButton29: TSpeedButton
            Left = 130
            Top = 3
            Width = 49
            Height = 30
            Hint = '@MB:SaveImageParameters'
            Caption = '@MB:JPEGP'
            ParentShowHint = False
            ShowHint = True
            OnClick = SpeedButton29Click
          end
          object CheckBox16: TCheckBox
            Left = 44
            Top = 9
            Width = 36
            Height = 20
            Hint = '@MB:SaveM3iFilesIncludingTheInternal'
            Caption = '@MB:Img'
            Checked = True
            ParentShowHint = False
            ShowHint = True
            State = cbChecked
            TabOrder = 0
          end
        end
        object TabSheet12: TTabSheet
          Caption = '@MB:SavePic'
          ImageIndex = 2
          object Button3: TSpeedButton
            Left = 2
            Top = 4
            Width = 40
            Height = 30
            Hint = '@MB:SaveTheImageAsBMPOr'
            Caption = '@MB:PNG'
            ParentShowHint = False
            ShowHint = True
            OnClick = Button3Click
          end
          object SBsaveJPEG: TSpeedButton
            Left = 45
            Top = 4
            Width = 40
            Height = 30
            Hint = '@MB:SaveTheImageAsJPEGFile'
            Caption = '@MB:JPEG'
            ParentShowHint = False
            ShowHint = True
            OnClick = SBsaveJPEGClick
          end
          object SpeedButton26: TSpeedButton
            Left = 86
            Top = 4
            Width = 40
            Height = 30
            Hint = '@MB:SaveAZBufferImageAs'
            Caption = '@MB:ZBUF'
            ParentShowHint = False
            ShowHint = True
            OnClick = SpeedButton26Click
          end
          object Label24: TLabel
            Left = 172
            Top = 0
            Width = 29
            Height = 13
            Caption = '@MB:Sharp'
          end
          object Label23: TLabel
            Left = 169
            Top = 17
            Width = 6
            Height = 13
            Alignment = taRightJustify
            Caption = '0'
          end
          object Label41: TLabel
            Left = 132
            Top = 0
            Width = 29
            Height = 13
            Caption = '@MB:JpgQ'
          end
          object Label42: TLabel
            Left = 212
            Top = 0
            Width = 39
            Height = 13
            Caption = '@MB:PngPar'
          end
          object Edit26: TEdit
            Left = 131
            Top = 14
            Width = 29
            Height = 21
            Hint = '@MB:N1100JPEGQuality100Maximal'
            ParentShowHint = False
            ShowHint = True
            TabOrder = 0
            Text = '95'
          end
          object CheckBox13: TCheckBox
            Left = 223
            Top = 16
            Width = 17
            Height = 17
            Hint = '@MB:SaveTextParametersInThePNG'
            ParentShowHint = False
            ShowHint = True
            TabOrder = 1
          end
          object UpDown4: TUpDown
            Left = 181
            Top = 14
            Width = 19
            Height = 21
            Min = -30000
            Max = 30000
            ParentShowHint = False
            ShowHint = True
            TabOrder = 2
            OnClick = UpDown4Click
          end
        end
        object TabSheet11: TTabSheet
          Caption = '@MB:Tools'
          ImageIndex = 3
          ExplicitLeft = 0
          ExplicitTop = 0
          ExplicitWidth = 0
          ExplicitHeight = 0
          object SpeedButton25: TSpeedButton
            Left = 2
            Top = 4
            Width = 50
            Height = 30
            Hint = '@MB:ConvertManyM3PFilesToM3I'
            Caption = 'm3p->m3i'
            ParentShowHint = False
            ShowHint = True
            Spacing = 0
            OnClick = SpeedButton25Click
          end
          object SpeedButton24: TSpeedButton
            Left = 114
            Top = 4
            Width = 60
            Height = 30
            Hint = '@MB:ExportACubeAsAStack'
            Caption = '@MB:Voxelstack'
            ParentShowHint = False
            ShowHint = True
            OnClick = SpeedButton24Click
          end
          object SpeedButton27: TSpeedButton
            Left = 54
            Top = 4
            Width = 58
            Height = 30
            Hint = '@MB:GenerateBigImagesByTiling'
            Caption = '@MB:BigRender'
            ParentShowHint = False
            ShowHint = True
            OnClick = SpeedButton27Click
          end
          object SpeedButton28: TSpeedButton
            Left = 176
            Top = 4
            Width = 34
            Height = 30
            Hint = '@MB:MonteCarloRendering'
            Caption = 'M.C.'
            ParentShowHint = False
            ShowHint = True
            OnClick = SpeedButton28Click
          end
          object SwarmBtn: TSpeedButton
            Left = 212
            Top = 4
            Width = 40
            Height = 30
            Hint = '@MB:ShareRenderingBetweenYourComputersThrough'
            Caption = '@MB:Swarm'
            ParentShowHint = False
            ShowHint = True
            OnClick = SwarmBtnClick
          end
        end
        object TabSheet13: TTabSheet
          Caption = '@MB:Prefs'
          ImageIndex = 4
          ExplicitLeft = 0
          ExplicitTop = 0
          ExplicitWidth = 0
          ExplicitHeight = 0
          object IniDirsBtn: TSpeedButton
            Left = 2
            Top = 3
            Width = 48
            Height = 30
            Hint = '@MB:DefineDefaultDirectoriesForLoadingSaving'
            Caption = '@MB:IniDirs'
            ParentShowHint = False
            ShowHint = True
            OnClick = IniDirsBtnClick
          end
          object MapSequencesBtn: TSpeedButton
            Left = 52
            Top = 3
            Width = 72
            Height = 30
            Hint = '@MB:DefineImageSequencesToAnimateHeight'
            Caption = '@MB:MapSequences'
            ParentShowHint = False
            ShowHint = True
            OnClick = MapSequencesBtnClick
          end
          object VisualThemesBtn: TSpeedButton
            Left = 126
            Top = 3
            Width = 48
            Height = 30
            Hint = '@MB:SetTheVisualThemeOfThe'
            Caption = '@MB:Themes'
            ParentShowHint = False
            ShowHint = True
            OnClick = VisualThemesBtnClick
          end
          object HotkeysBtn: TSpeedButton
            Left = 176
            Top = 3
            Width = 50
            Height = 30
            Hint = '@MB:KeysThatBringAWindowTo'
            Caption = '@MB:Hotkeys'
            ParentShowHint = False
            ShowHint = True
            OnClick = HotkeysBtnClick
          end
          object AiBtn: TSpeedButton
            Left = 228
            Top = 3
            Width = 24
            Height = 30
            Hint = '@MB:ConnectToAnAIAssistantLet'
            Caption = '@MB:AI'
            ParentShowHint = False
            ShowHint = True
            OnClick = AiBtnClick
          end
        end
      end
      object GroupBox2: TGroupBox
        Left = 425
        Top = 1
        Width = 49
        Height = 69
        Caption = '@MB:Viewing'
        TabOrder = 2
        object SpeedButton35: TSpeedButton
          Tag = 1
          Left = 9
          Top = 17
          Width = 31
          Height = 19
          Hint = '@MB:ImageDownscalingForViewingPurposeAnd'
          AllowAllUp = True
          Caption = '1:1'
          Flat = True
          ParentShowHint = False
          ShowHint = True
          OnClick = SpeedButton35Click
        end
        object UpDown1: TUpDown
          Left = 9
          Top = 36
          Width = 31
          Height = 28
          Min = 1
          Max = 10
          Position = 10
          TabOrder = 0
          OnClick = UpDown1Click
        end
      end
    end
  end
  object Timer1: TTimer
    Enabled = False
    Interval = 200
    OnTimer = Timer1Timer
    Left = 304
    Top = 104
  end
  object Timer3: TTimer
    Enabled = False
    Interval = 150
    OnTimer = Timer3Timer
    Left = 304
    Top = 72
  end
  object Timer4: TTimer
    Enabled = False
    Interval = 300
    OnTimer = Timer4Timer
    Left = 336
    Top = 72
  end
  object SaveDialog2: TSaveDialog
    DefaultExt = 'm3p'
    Filter = 'Mandel3D Parameter  (*.m3p)|*.m3p'
    Options = [ofOverwritePrompt, ofHideReadOnly, ofEnableSizing]
    Left = 168
    Top = 72
  end
  object OpenDialog1: TOpenDialog
    DefaultExt = 'm3p'
    Filter = 'M3D parameter (*.m3p)|*.m3p'
    Left = 104
    Top = 72
  end
  object SaveDialog3: TSaveDialog
    DefaultExt = 'm3i'
    Filter = 'M3D Image + Parameter (*.m3i)|*.m3i'
    Options = [ofOverwritePrompt, ofHideReadOnly, ofEnableSizing]
    Left = 232
    Top = 111
  end
  object Timer8: TTimer
    Enabled = False
    Interval = 100
    OnTimer = Timer8Timer
    Left = 408
    Top = 72
  end
  object SaveDialog4: TSaveDialog
    DefaultExt = 'jpg'
    Filter = 'JPEG Image|*.jpg'
    Options = [ofOverwritePrompt, ofHideReadOnly, ofEnableSizing]
    Left = 172
    Top = 76
  end
  object SaveDialog6: TSaveDialog
    DefaultExt = 'bmp'
    Filter = 'Windows Bitmap|*.bmp|Portable Network Graphic 8bit|*.png'
    Options = [ofOverwritePrompt, ofHideReadOnly, ofEnableSizing]
    OnTypeChange = SaveDialog6TypeChange
    Left = 238
    Top = 279
  end
  object PopupMenu1: TPopupMenu
    Left = 496
    Top = 72
    object N111: TMenuItem
      Tag = 1
      Caption = '1:1'
      OnClick = N111Click
    end
    object N12aa1: TMenuItem
      Tag = 2
      Caption = '@MB:N12Aa'
      OnClick = N111Click
    end
    object N13aa1: TMenuItem
      Tag = 3
      Caption = '@MB:N13Aa'
      OnClick = N111Click
    end
    object N141: TMenuItem
      Tag = 4
      Caption = '1:4'
      OnClick = N111Click
    end
    object N151: TMenuItem
      Tag = 5
      Caption = '1:5'
      OnClick = N111Click
    end
    object N161: TMenuItem
      Tag = 6
      Caption = '1:6'
      OnClick = N111Click
    end
    object N171: TMenuItem
      Tag = 7
      Caption = '1:7'
      OnClick = N111Click
    end
    object N181: TMenuItem
      Tag = 8
      Caption = '1:8'
      OnClick = N111Click
    end
    object N191: TMenuItem
      Tag = 9
      Caption = '1:9'
      OnClick = N111Click
    end
    object N1101: TMenuItem
      Tag = 10
      Caption = '1:10'
      OnClick = N111Click
    end
  end
  object SaveDialog1: TSaveDialog
    DefaultExt = 'jpg'
    Filter = 'JPEG|*.jpg|Portable Network Graphic|*.png'
    Options = [ofOverwritePrompt, ofHideReadOnly, ofEnableSizing]
    OnTypeChange = SaveDialog1TypeChange
    Left = 156
    Top = 263
  end
  object PopupMenu2: TPopupMenu
    Left = 576
    Top = 160
    object Stickthiswindowtotherightside1: TMenuItem
      Tag = 1
      Caption = '@MB:StickThisWindowToTheRight'
      Checked = True
      OnClick = Stickthiswindowtotherightside1Click
    end
    object Stickthiswindowtotheleftside1: TMenuItem
      Tag = 2
      Caption = '@MB:StickThisWindowToTheLeft'
      OnClick = Stickthiswindowtotherightside1Click
    end
    object Donotmakethiswindowsticky1: TMenuItem
      Caption = '@MB:DoNotMakeThisWindowSticky'
      OnClick = Stickthiswindowtotherightside1Click
    end
  end
  object PopupMenu3: TPopupMenu
    Left = 576
    Top = 200
    object StartrenderingandsaveafterwardstheM3Ifile1: TMenuItem
      Caption = '@MB:StartRenderingAndSaveAfterwardsThe'
      OnClick = StartrenderingandsaveafterwardstheM3Ifile1Click
    end
  end
  object Timer2: TTimer
    Interval = 60000
    OnTimer = Timer2Timer
    Left = 304
    Top = 136
  end
  object Timer5: TTimer
    Interval = 50
    OnTimer = Timer5Timer
    Left = 360
    Top = 136
  end
  object PopupMenu4: TPopupMenu
    Left = 192
    Top = 168
    object ShapeDisc1: TMenuItem
      Caption = '@MB:Disc'
      OnClick = ShapeDisc1Click
    end
    object ShapeBox1: TMenuItem
      Caption = '@MB:Rect'
      OnClick = ShapeBox1Click
    end
  end
  object Timer6: TTimer
    Enabled = False
    Interval = 50
    OnTimer = Timer6Timer
    Left = 264
    Top = 160
  end
  object SaveDialog5: TSaveDialog
    DefaultExt = 'bmp'
    Filter = 
      'Windows Bitmap|*.bmp|Portable Network Graphic 8bit|*.png|Portabl' +
      'e Network Graphic 16bit|*.png'
    Options = [ofOverwritePrompt, ofHideReadOnly, ofEnableSizing]
    OnTypeChange = SaveDialog5TypeChange
    Left = 362
    Top = 299
  end
end
