/**
 * Simple SQL formatter — keywords uppercase, basic indentation.
 * No external dependency; handles common SELECT/INSERT/UPDATE/DELETE patterns.
 */

const KEYWORDS = [
  'SELECT', 'FROM', 'WHERE', 'AND', 'OR', 'NOT', 'IN', 'ON', 'AS',
  'JOIN', 'LEFT', 'RIGHT', 'INNER', 'OUTER', 'CROSS', 'FULL',
  'GROUP BY', 'ORDER BY', 'HAVING', 'LIMIT', 'OFFSET',
  'INSERT INTO', 'VALUES', 'UPDATE', 'SET', 'DELETE FROM',
  'CREATE TABLE', 'ALTER TABLE', 'DROP TABLE',
  'CREATE INDEX', 'DROP INDEX',
  'UNION', 'UNION ALL', 'EXCEPT', 'INTERSECT',
  'CASE', 'WHEN', 'THEN', 'ELSE', 'END',
  'IS', 'NULL', 'LIKE', 'BETWEEN', 'EXISTS', 'DISTINCT',
  'ASC', 'DESC', 'TOP', 'INTO',
]

export function formatSql(sql: string): string {
  if (!sql || !sql.trim()) return sql

  let result = sql
    // Normalize whitespace
    .replace(/\s+/g, ' ')
    .trim()

  // Uppercase keywords (longest first to avoid partial matches)
  const sorted = [...KEYWORDS].sort((a, b) => b.length - a.length)
  for (const kw of sorted) {
    const pattern = new RegExp(`\\b${kw.replace(/ /g, '\\s+')}\\b`, 'gi')
    result = result.replace(pattern, kw)
  }

  // Add newlines before major keywords
  const majorKeywords = [
    'SELECT', 'FROM', 'WHERE', 'GROUP BY', 'ORDER BY', 'HAVING',
    'LIMIT', 'OFFSET', 'JOIN', 'LEFT JOIN', 'RIGHT JOIN', 'INNER JOIN',
    'CROSS JOIN', 'FULL JOIN', 'INSERT INTO', 'VALUES', 'UPDATE',
    'SET', 'DELETE FROM', 'UNION', 'UNION ALL',
  ]

  for (const kw of majorKeywords) {
    const pattern = new RegExp(`\\b${kw.replace(/ /g, '\\s+')}\\b`, 'gi')
    result = result.replace(pattern, `\n${kw}`)
  }

  // Indent AND/OR/ON after WHERE/GROUP BY/ORDER BY/HAVING
  result = result.replace(/\n(AND|OR|ON)\b/g, '\n  $1')

  // Clean up
  result = result
    .replace(/^\n+/, '') // Remove leading newlines
    .replace(/\n{3,}/g, '\n\n') // Collapse multiple newlines
    .trim()

  return result
}
