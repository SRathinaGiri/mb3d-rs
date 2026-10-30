object FTextBox: TFTextBox
  Left = 193
  Top = 152
  Caption = '@MB:OpenTextParameters'
  ClientHeight = 439
  ClientWidth = 461
  Color = clBtnFace
  Font.Charset = DEFAULT_CHARSET
  Font.Color = clWindowText
  Font.Height = -11
  Font.Name = 'MS Sans Serif'
  Font.Style = []
  FormStyle = fsStayOnTop
  OldCreateOrder = False
  PixelsPerInch = 96
  TextHeight = 13
  object Panel1: TPanel
    Left = 0
    Top = 0
    Width = 461
    Height = 33
    Align = alTop
    Alignment = taLeftJustify
    Caption = '@MB:NoValidTextFoundPleaseEnter'
    TabOrder = 0
    object Button1: TButton
      Left = 320
      Top = 4
      Width = 71
      Height = 25
      Caption = '@MB:Cancel'
      TabOrder = 0
      OnClick = Button1Click
    end
  end
  object Memo1: TMemo
    Left = 0
    Top = 33
    Width = 461
    Height = 406
    Align = alClient
    Constraints.MinHeight = 100
    Constraints.MinWidth = 80
    ScrollBars = ssBoth
    TabOrder = 1
    OnChange = Memo1Change
  end
end
