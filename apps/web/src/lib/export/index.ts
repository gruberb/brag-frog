/* eslint-disable @typescript-eslint/no-unused-vars -- placeholder until the export milestone */
import type { CycleBundle, Report } from '@/types/api/models';

export type ExportFormat = 'pdf' | 'docx' | 'markdown';

// Replaced by the real implementation in the export milestone.
export async function downloadReport(_format: ExportFormat, _report: Report): Promise<void> {
  throw new Error('Export not available yet');
}
export async function downloadCycleBundle(_format: ExportFormat | 'json', _bundle: CycleBundle): Promise<void> {
  throw new Error('Export not available yet');
}
export async function downloadReview(_format: ExportFormat, _title: string, _sections: { title: string; content: string; content_format: 'markdown' | 'blocknote' }[]): Promise<void> {
  throw new Error('Export not available yet');
}
