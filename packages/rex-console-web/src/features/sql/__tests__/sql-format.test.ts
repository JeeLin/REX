import { describe, it, expect } from 'vitest'
import { formatSql } from '../sql-format'

describe('formatSql', () => {
  it('returns empty/whitespace input unchanged', () => {
    expect(formatSql('')).toBe('')
    expect(formatSql('   ')).toBe('   ')
  })

  it('uppercases keywords', () => {
    expect(formatSql('select * from users')).toBe('SELECT * \nFROM users')
  })

  it('puts newlines before major clauses', () => {
    const out = formatSql('select id from users where active = 1')
    expect(out).toBe('SELECT id \nFROM users \nWHERE active = 1')
  })

  it('uppercases AND/OR but keeps them inline (no leading newline)', () => {
    // The indent rule only applies when AND/OR/ON follow an existing "\n",
    // so a freshly inserted WHERE keeps AND on the same line.
    const out = formatSql('select id from users where a = 1 and b = 2')
    expect(out).toBe('SELECT id \nFROM users \nWHERE a = 1 AND b = 2')
  })

  it('normalizes internal whitespace', () => {
    expect(formatSql('select   id   from    users')).toBe('SELECT id \nFROM users')
  })

  it('handles multi-word keywords like GROUP BY / ORDER BY', () => {
    const out = formatSql('select id from users group by name order by id')
    expect(out).toBe('SELECT id \nFROM users \nGROUP BY name \nORDER BY id')
  })

  it('uppercases keywords even inside string literals', () => {
    // formatSql applies the keyword regex globally; words inside quotes are
    // not protected, so a quoted "select" becomes "SELECT".
    const out = formatSql("select 'select me' as label from t")
    expect(out).toBe("SELECT '\nSELECT me' AS label \nFROM t")
  })
})
