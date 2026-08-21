import type { ITheme } from '@xterm/xterm'

export interface TerminalTheme {
  name: string
  label: string
  theme: ITheme
}

export const terminalThemes: TerminalTheme[] = [
  {
    name: 'default',
    label: 'REX Default',
    theme: {
      background: '#0D1117',
      foreground: '#E6EDF3',
      cursor: '#E6EDF3',
      cursorAccent: '#0D1117',
      selectionBackground: '#264F78',
      selectionForeground: '#E6EDF3',
      black: '#484F58',
      red: '#FF7B72',
      green: '#3FB950',
      yellow: '#D29922',
      blue: '#58A6FF',
      magenta: '#BC8CFF',
      cyan: '#39C5CF',
      white: '#B1BAC4',
      brightBlack: '#6E7681',
      brightRed: '#FFA198',
      brightGreen: '#56D364',
      brightYellow: '#E3B341',
      brightBlue: '#79C0FF',
      brightMagenta: '#D2A8FF',
      brightCyan: '#56D4DD',
      brightWhite: '#F0F6FC',
    },
  },
  {
    name: 'ubuntu',
    label: 'Ubuntu',
    theme: {
      background: '#300A24',
      foreground: '#FFFFFF',
      cursor: '#FFFFFF',
      cursorAccent: '#300A24',
      selectionBackground: '#5E2750',
      selectionForeground: '#FFFFFF',
      black: '#2E3436',
      red: '#CC0000',
      green: '#4E9A06',
      yellow: '#C4A000',
      blue: '#3465A4',
      magenta: '#75507B',
      cyan: '#06989A',
      white: '#D3D7CF',
      brightBlack: '#555753',
      brightRed: '#EF2929',
      brightGreen: '#8AE234',
      brightYellow: '#FCE94F',
      brightBlue: '#729FCF',
      brightMagenta: '#AD7FA8',
      brightCyan: '#34E2E2',
      brightWhite: '#EEEEEC',
    },
  },
  {
    name: 'solarized-dark',
    label: 'Solarized Dark',
    theme: {
      background: '#002B36',
      foreground: '#839496',
      cursor: '#839496',
      cursorAccent: '#002B36',
      selectionBackground: '#073642',
      selectionForeground: '#839496',
      black: '#073642',
      red: '#DC322F',
      green: '#859900',
      yellow: '#B58900',
      blue: '#268BD2',
      magenta: '#D33682',
      cyan: '#2AA198',
      white: '#EEE8D5',
      brightBlack: '#586E75',
      brightRed: '#CB4B16',
      brightGreen: '#586E75',
      brightYellow: '#657B83',
      brightBlue: '#839496',
      brightMagenta: '#6C71C4',
      brightCyan: '#93A1A1',
      brightWhite: '#FDF6E3',
    },
  },
]

/** 根据名称获取主题 */
export function getTerminalTheme(name: string): ITheme {
  const found = terminalThemes.find((t) => t.name === name)
  if (found) return found.theme
  return terminalThemes[0]!.theme
}
