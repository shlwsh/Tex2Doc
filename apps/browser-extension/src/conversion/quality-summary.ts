import type { QualityGate } from '@/shared/types';

export interface ParsedReport {
  summary: ReportSummary;
  scores: ScoreBreakdown;
  issues: Issue[];
  recommendations: string[];
}

export interface ReportSummary {
  jobId: string;
  mainTex: string;
  profile: string;
  overallScore: number;
  status: 'passed' | 'warning' | 'failed';
  backend?: string;
  docxSize?: number;
}

export interface ScoreBreakdown {
  parse?: number;
  semantic?: number;
  docx?: number;
  visual?: number;
  editable?: number;
}

export interface Issue {
  severity: 'critical' | 'warning' | 'info';
  category: string;
  message: string;
  location?: string;
}

export interface QualitySummaryDisplay {
  overallScore: ScoreDisplay;
  status: StatusDisplay;
  checks: CheckDisplay[];
  warnings: string[];
  details: DetailItem[];
}

export interface ScoreDisplay {
  value: number;
  formatted: string;
  label: string;
  color: string;
}

export interface StatusDisplay {
  value: 'passed' | 'warning' | 'failed';
  label: string;
  color: string;
  description: string;
}

export interface CheckDisplay {
  name: string;
  passed: boolean;
  message?: string;
}

export interface DetailItem {
  label: string;
  value: string;
}

export function parseReport(report: {
  job_id: string;
  main_tex: string;
  profile: string;
  quality_score?: number;
  backend?: string;
  docx_bytes?: number;
  quality_gate?: QualityGate;
  warnings: string[];
  errors: string[];
}): ParsedReport {
  const summary = parseSummary(report);
  const scores = parseScores(report);
  const issues = parseIssues(report);
  const recommendations = generateRecommendations(report);

  return { summary, scores, issues, recommendations };
}

function parseSummary(report: {
  job_id: string;
  main_tex: string;
  profile: string;
  quality_score?: number;
  backend?: string;
  docx_bytes?: number;
  quality_gate?: QualityGate;
}): ReportSummary {
  const status = report.quality_gate?.status ?? 'warning';

  return {
    jobId: report.job_id,
    mainTex: report.main_tex,
    profile: report.profile,
    overallScore: report.quality_score ?? 0,
    status: status as 'passed' | 'warning' | 'failed',
    backend: report.backend ?? undefined,
    docxSize: report.docx_bytes ?? undefined,
  };
}

function parseScores(report: { quality_score?: number }): ScoreBreakdown {
  const scores: ScoreBreakdown = {};

  if (report.quality_gate) {
    scores.parse = Math.round((report.quality_score ?? 0) * 0.3);
    scores.semantic = Math.round((report.quality_score ?? 0) * 0.4);
    scores.docx = Math.round((report.quality_score ?? 0) * 0.3);
  }

  return scores;
}

function parseIssues(report: {
  quality_gate?: QualityGate;
  errors: string[];
}): Issue[] {
  const issues: Issue[] = [];
  const qualityGate = report.quality_gate;

  if (!qualityGate) return issues;

  for (const check of qualityGate.failed_checks ?? []) {
    issues.push({ severity: 'critical', category: 'Quality Gate', message: check });
  }

  for (const warning of qualityGate.warnings ?? []) {
    issues.push({ severity: 'warning', category: 'Warning', message: warning });
  }

  for (const error of report.errors ?? []) {
    issues.push({ severity: 'critical', category: 'Conversion Error', message: error });
  }

  return issues;
}

function generateRecommendations(report: {
  quality_gate?: QualityGate;
  backend?: string;
  warnings: string[];
}): string[] {
  const recommendations: string[] = [];
  const qualityGate = report.quality_gate;

  if (!qualityGate || qualityGate.status === 'passed') {
    recommendations.push('Conversion quality is good. No major issues detected.');
    return recommendations;
  }

  if (qualityGate.failed_checks.length > 0) {
    recommendations.push('Consider simplifying complex LaTeX commands or using standard packages.');
  }

  if (report.warnings.length > 0) {
    recommendations.push('Review the warnings to see if any visual or formatting issues can be addressed.');
  }

  if (report.backend === 'fallback') {
    recommendations.push('The conversion used a fallback engine. Consider using standard LaTeX for better results.');
  }

  return recommendations;
}

export function getStatusColor(status: 'passed' | 'warning' | 'failed'): string {
  switch (status) {
    case 'passed': return 'green';
    case 'warning': return 'yellow';
    case 'failed': return 'red';
    default: return 'gray';
  }
}

export function getScoreLabel(score: number): string {
  if (score >= 90) return 'Excellent';
  if (score >= 75) return 'Good';
  if (score >= 60) return 'Acceptable';
  if (score >= 40) return 'Poor';
  return 'Failed';
}

export function formatScore(score: number): ScoreDisplay {
  const label = getScoreLabel(score);
  let color = 'gray';

  if (score >= 90) color = 'green';
  else if (score >= 75) color = 'blue';
  else if (score >= 60) color = 'yellow';
  else color = 'red';

  return {
    value: score,
    formatted: `${score}%`,
    label,
    color,
  };
}

export function createQualitySummary(report: {
  job_id: string;
  main_tex: string;
  profile: string;
  quality_score?: number;
  backend?: string;
  docx_bytes?: number;
  quality_gate?: QualityGate;
  warnings: string[];
  errors: string[];
}): QualitySummaryDisplay {
  const parsed = parseReport(report);

  return {
    overallScore: formatScore(parsed.summary.overallScore),
    status: createStatusDisplay(parsed.summary.status),
    checks: createCheckDisplays(parsed),
    warnings: report.warnings,
    details: createDetailItems(report),
  };
}

function createStatusDisplay(status: 'passed' | 'warning' | 'failed'): StatusDisplay {
  const displays: Record<string, StatusDisplay> = {
    passed: { value: 'passed', label: 'Passed', color: 'green', description: 'Conversion completed successfully with good quality.' },
    warning: { value: 'warning', label: 'Warning', color: 'yellow', description: 'Conversion completed with some issues. Review warnings for details.' },
    failed: { value: 'failed', label: 'Failed', color: 'red', description: 'Conversion failed or produced poor quality output.' },
  };

  return displays[status] ?? displays.warning;
}

function createCheckDisplays(parsed: ParsedReport): CheckDisplay[] {
  const checks: CheckDisplay[] = [];

  for (const issue of parsed.issues) {
    if (issue.severity === 'critical') {
      checks.push({ name: issue.category, passed: false, message: issue.message });
    }
  }

  if (parsed.summary.status !== 'failed' && checks.length === 0) {
    checks.push({ name: 'Quality Gate', passed: true, message: 'All quality checks passed' });
  }

  return checks;
}

function createDetailItems(report: {
  main_tex: string;
  profile: string;
  backend?: string;
  docx_bytes?: number;
  quality_gate?: QualityGate;
}): DetailItem[] {
  const details: DetailItem[] = [];

  details.push({ label: 'Main TeX File', value: report.main_tex });
  details.push({ label: 'Profile', value: report.profile });

  if (report.backend) {
    details.push({ label: 'Backend', value: report.backend });
  }

  if (report.docx_bytes) {
    details.push({ label: 'DOCX Size', value: formatBytes(report.docx_bytes) });
  }

  if (report.quality_gate) {
    const gate = report.quality_gate;

    if (gate.passed_checks.length > 0) {
      details.push({ label: 'Passed Checks', value: gate.passed_checks.length.toString() });
    }

    if (gate.failed_checks.length > 0) {
      details.push({ label: 'Failed Checks', value: gate.failed_checks.length.toString() });
    }
  }

  return details;
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

export function getStatusBadgeVariant(status: 'passed' | 'warning' | 'failed'): 'success' | 'warning' | 'error' | 'info' {
  switch (status) {
    case 'passed': return 'success';
    case 'warning': return 'warning';
    case 'failed': return 'error';
    default: return 'info';
  }
}

export function getScoreIcon(score: number): string {
  if (score >= 90) return 'check-circle';
  if (score >= 75) return 'check';
  if (score >= 60) return 'alert-circle';
  return 'x-circle';
}

export function formatSummaryForNotification(report: {
  job_id: string;
  main_tex: string;
  profile: string;
  quality_score?: number;
  backend?: string;
  docx_bytes?: number;
  quality_gate?: QualityGate;
  warnings: string[];
  errors: string[];
}): string {
  const summary = createQualitySummary(report);

  return `Quality Score: ${summary.overallScore.value}% (${summary.overallScore.label})
Status: ${summary.status.label}`;
}

export function hasCriticalIssues(report: {
  quality_gate?: QualityGate;
  errors: string[];
}): boolean {
  return (
    (report.quality_gate?.failed_checks?.length ?? 0) > 0 ||
    (report.errors?.length ?? 0) > 0
  );
}

export function hasWarnings(report: {
  quality_gate?: QualityGate;
  warnings: string[];
}): boolean {
  return (
    (report.quality_gate?.warnings?.length ?? 0) > 0 ||
    (report.warnings?.length ?? 0) > 0
  );
}
