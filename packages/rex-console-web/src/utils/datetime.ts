/** Display an API timestamp in the browser timezone, with an explicit offset.
 * Storage and API timestamps remain unchanged. Naive timestamps are not guessed.
 */
export function formatDateTime(value: string | null | undefined, timeZone?: string): string {
  if (!value) return '—'
  if (!/(?:Z|[+-]\d{2}:?\d{2})$/i.test(value)) return value
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return value
  return new Intl.DateTimeFormat(undefined, {
    year: 'numeric', month: '2-digit', day: '2-digit',
    hour: '2-digit', minute: '2-digit', second: '2-digit',
    hourCycle: 'h23', timeZoneName: 'longOffset',
    ...(timeZone ? { timeZone } : {}),
  }).format(date)
}
