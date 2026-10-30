object AiConnectFrm: TAiConnectFrm
  Left = 220
  Top = 160
  BorderStyle = bsDialog
  Caption = 'Connect to an AI assistant'
  ClientHeight = 470
  ClientWidth = 640
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
    Width = 616
    Height = 52
    AutoSize = False
    Caption = 
      'An AI assistant on this computer can use Mandelbulb 3D: ask it in' +
      ' plain words, or by voice, to describe, change and render your p' +
      'ictures, to make variations or to render what you worked on last' +
      '. Mandelbulb 3D stays offline: the assistant'#39's app starts it on' +
      ' this computer.'
    WordWrap = True
  end
  object Label7: TLabel
    Left = 12
    Top = 74
    Width = 200
    Height = 13
    Caption = 'Claude (desktop app)'
    Font.Style = [fsBold]
    ParentFont = False
  end
  object Label2: TLabel
    Left = 12
    Top = 92
    Width = 440
    Height = 32
    AutoSize = False
    Caption = ''
    WordWrap = True
  end
  object Button1: TButton
    Left = 488
    Top = 80
    Width = 140
    Height = 30
    Caption = 'Connect'
    TabOrder = 0
    OnClick = Button1Click
  end
  object Label8: TLabel
    Left = 12
    Top = 140
    Width = 200
    Height = 13
    Caption = 'Claude Code (in a terminal)'
    Font.Style = [fsBold]
    ParentFont = False
  end
  object Label3: TLabel
    Left = 12
    Top = 158
    Width = 616
    Height = 32
    AutoSize = False
    Caption = ''
    WordWrap = True
  end
  object Edit1: TEdit
    Left = 12
    Top = 194
    Width = 470
    Height = 21
    ReadOnly = True
    TabOrder = 1
  end
  object Button2: TButton
    Left = 488
    Top = 190
    Width = 140
    Height = 28
    Hint = 'Copy the command, to paste it into a terminal'
    Caption = 'Copy command'
    ParentShowHint = False
    ShowHint = True
    TabOrder = 2
    OnClick = Button2Click
  end
  object Label4: TLabel
    Left = 12
    Top = 234
    Width = 300
    Height = 13
    Caption = 'Things to ask the assistant'
    Font.Style = [fsBold]
    ParentFont = False
  end
  object Memo1: TMemo
    Left = 12
    Top = 252
    Width = 616
    Height = 120
    ReadOnly = True
    ScrollBars = ssVertical
    TabOrder = 3
  end
  object Label5: TLabel
    Left = 12
    Top = 382
    Width = 616
    Height = 44
    AutoSize = False
    Caption = ''
    WordWrap = True
  end
  object Button3: TButton
    Left = 528
    Top = 432
    Width = 100
    Height = 28
    Caption = 'Close'
    TabOrder = 4
    OnClick = Button3Click
  end
end
