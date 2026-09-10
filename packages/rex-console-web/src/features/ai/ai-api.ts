/**
 * AI Assistant API - Mock implementation
 * 
 * This module provides a mock AI backend for the SQL console assistant.
 * The actual AI integration will be implemented later.
 */

export interface AiRequest {
  question: string
  context?: string
  mode?: 'ask' | 'generate'
}

export interface AiResponse {
  answer: string
  sql?: string
}

/**
 * Mock AI function - returns placeholder response
 * In production, this will call the actual AI backend API
 */
export async function askAi(question: string, context?: string): Promise<string> {
  // Simulate API delay
  await new Promise(resolve => setTimeout(resolve, 500 + Math.random() * 500))
  
  const contextInfo = context ? `\n\nContext: ${context}` : ''
  
  return `## AI Response

**Question:** ${question}${contextInfo}

### Analysis

This is a placeholder response from the AI assistant. The actual AI integration will be implemented in a future milestone.

### Key Points

1. **Understanding**: I've analyzed your question about SQL operations
2. **Context**: The query context has been considered
3. **Recommendation**: Please refer to the database documentation for specific syntax

### Example

\`\`\`sql
-- Example SQL based on your question
SELECT * FROM table_name WHERE condition = 'value';
\`\`\`

> **Note:** This is a mock response. Real AI capabilities will be added soon.`
}

/**
 * Mock SQL generation function
 * In production, this will call the actual AI backend API
 */
export async function generateSql(description: string, context?: string): Promise<AiResponse> {
  // Simulate API delay
  await new Promise(resolve => setTimeout(resolve, 800 + Math.random() * 700))
  
  const contextInfo = context ? `\nContext: ${context}` : ''
  
  const sql = `-- Generated based on: ${description}
SELECT 
  column1,
  column2,
  column3
FROM table_name
WHERE condition = 'value'
ORDER BY column1 DESC
LIMIT 100;`

  return {
    answer: `## Generated SQL

Based on your description: "${description}"${contextInfo}

I've generated a SQL query that matches your requirements. You can modify the column names and conditions as needed.

### Notes
- Adjust table and column names to match your schema
- Add appropriate WHERE clauses for filtering
- Consider adding indexes for better performance`,
    sql
  }
}
