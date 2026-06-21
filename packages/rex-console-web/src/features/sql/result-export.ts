import type { SqlColumn } from '@/api/sql'

/** 下载文件触发浏览器保存 */
function downloadFile(content: string, filename: string, mimeType: string) {
  const blob = new Blob([content], { type: mimeType })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = filename
  a.click()
  URL.revokeObjectURL(url)
}

/** CSV 转义：包裹双引号，内部双引号转义为两个 */
function csvEscape(cell: unknown): string {
  if (cell === null || cell === undefined) return ''
  const str = String(cell)
  if (str.includes(',') || str.includes('"') || str.includes('\n') || str.includes('\r')) {
    return `"${str.replace(/"/g, '""')}"`
  }
  return str
}

/** 导出为 CSV 文件 */
export function exportCsv(columns: SqlColumn[], rows: unknown[][]): void {
  const header = columns.map((c) => c.name).join(',')
  const body = rows.map((row) => row.map(csvEscape).join(',')).join('\n')
  downloadFile(`${header}\n${body}`, 'query-result.csv', 'text/csv;charset=utf-8')
}

/** 导出为 JSON 文件 */
export function exportJson(columns: SqlColumn[], rows: unknown[][]): void {
  const data = rows.map((row) => {
    const obj: Record<string, unknown> = {}
    columns.forEach((c, i) => { obj[c.name] = row[i] })
    return obj
  })
  downloadFile(JSON.stringify(data, null, 2), 'query-result.json', 'application/json')
}
