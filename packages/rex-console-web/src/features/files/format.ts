//! Formatting helpers shared by the files feature views.

export function fmtSize(b: number): string {
  if (!b) return '-'
  const u = ['B', 'KB', 'MB', 'GB']
  let i = 0
  let s = b
  while (s >= 1024 && i < 3) {
    s /= 1024
    i++
  }
  return `${s.toFixed(i ? 1 : 0)} ${u[i]}`
}

export function fmtSpeed(b: number): string {
  return fmtSize(b) + '/s'
}
