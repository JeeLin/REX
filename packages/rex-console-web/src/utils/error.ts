import type { AxiosError } from 'axios'

type ApiErrorBody = { error?: { message?: string } }

/** Extract user-facing message from an Axios or generic Error. */
export function getErrorMessage(err: unknown, fallback: string): string {
  if (err && typeof err === 'object' && 'response' in err) {
    const axErr = err as AxiosError<ApiErrorBody>
    return axErr.response?.data?.error?.message ?? fallback
  }
  return err instanceof Error ? err.message : fallback
}
