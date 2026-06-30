/**
 * Report Parser
 *
 * Parses and displays conversion quality reports
 */

import type { ConversionReport } from '@/shared/types';

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

/**
 * Parse conversion report
 */
export function parseReport(report: ConversionReport): ParsedReport {
  const summary = parseSummary(report);
  const scores = parseScores(report);
  const issues = parseIssues(report);
  const recommendations = generateRecommendations(report);

  return {
    summary,
    scores,
    issues,
    recommendations,
  };
}

/**
 * Parse summary from report
 */
function parseSummary(report: ConversionReport): ReportSummary {
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

/**
 * Parse score breakdown
 */
function parseScores(report: ConversionReport): ScoreBreakdown {
  // Note: The actual report structure may vary
  // This is a placeholder that extracts what's available
  const scores: ScoreBreakdown = {};

  if (report.quality_gate) {
    scores.parse = Math.round((report.quality_score ?? 0) * 0.3);
    scores.semantic = Math.round((report.quality_score ?? 0) * 0.4);
    scores.docx = Math.round((report.quality_score ?? 0) * 0.3);
  }

  return scores;
}

/**
 * Parse issues from quality gate
 */
function parseIssues(report: ConversionReport): Issue[] {
  const issues: Issue[] = [];
  const qualityGate = report.quality_gate;

  if (!qualityGate) {
    return issues;
  }

  // Add failed checks as critical issues
  for (const check of qualityGate.failed_checks ?? []) {
    issues.push({
      severity: 'critical',
      category: 'Quality Gate',
      message: check,
    });
  }

  // Add warnings as warning issues
  for (const warning of qualityGate.warnings ?? []) {
    issues.push({
      severity: 'warning',
      category: 'Warning',
      message: warning,
    });
  }

  // Add errors from report
  for (const error of report.errors ?? []) {
    issues.push({
      severity: 'critical',
      category: 'Conversion Error',
      message: error,
    });
  }

  return issues;
}

/**
 * Generate recommendations based on issues
 */
function generateRecommendations(report: ConversionReport): string[] {
  const recommendations: string[] = [];
  const qualityGate = report.quality_gate;

  if (!qualityGate || qualityGate.status === 'passed') {
    recommendations.push('Conversion quality is good. No major issues detected.');
    return recommendations;
  }

  if (qualityGate.failed_checks.length > 0) {
    recommendations.push(
      'Consider simplifying complex LaTeX commands or using standard packages.'
    );
  }

  if (report.warnings.length > 0) {
    recommendations.push(
      'Review the warnings to see if any visual or formatting issues can be addressed.'
    );
  }

  if (report.backend === 'fallback') {
    recommendations.push(
      'The conversion used a fallback engine. Consider using standard LaTeX for better results.'
    );
  }

  return recommendations;
}

/**
 * Get status color
 */
export function getStatusColor(status: 'passed' | 'warning' | 'failed'): string {
  switch (status) {
    case 'passed':
      return 'green';
    case 'warning':
      return 'yellow';
    case 'failed':
      return 'red';
    default:
      return 'gray';
  }
}

/**
 * Get score label
 */
export function getScoreLabel(score: number): string {
  if (score >= 90) return 'Excellent';
  if (score >= 75) return 'Good';
  if (score >= 60) return 'Acceptable';
  if (score >= 40) return 'Poor';
  return 'Failed';
}

/**
 * Format score with color class
 */
export function formatScore(score: number): { value: string; label: string; color: string } {
  const label = getScoreLabel(score);
  let color = 'gray';

  if (score >= 90) color = 'green';
  else if (score >= 75) color = 'blue';
  else if (score >= 60) color = 'yellow';
  else color = 'red';

  return {
    value: `${score}%`,
    label,
    color,
  };
}

/**
 * Format report for display
 */
export function formatReportForDisplay(report: ConversionReport): string {
  const parsed = parseReport(report);
  const lines: string[] = [];

  lines.push(`=== Conversion Report ===`);
  lines.push(`Job: ${parsed.summary.jobId}`);
  lines.push(`Main TeX: ${parsed.summary.mainTex}`);
  lines.push(`Profile: ${parsed.summary.profile}`);
  lines.push(`Score: ${parsed.summary.overallScore}% (${getScoreLabel(parsed.summary.overallScore)})`);
  lines.push(`Status: ${parsed.summary.status.toUpperCase()}`);

  if (parsed.summary.backend) {
    lines.push(`Backend: ${parsed.summary.backend}`);
  }

  if (parsed.issues.length > 0) {
    lines.push('');
    lines.push('=== Issues ===');
    for (const issue of parsed.issues) {
      lines.push(`[${issue.severity.toUpperCase()}] ${issue.message}`);
    }
  }

  if (parsed.recommendations.length > 0) {
    lines.push('');
    lines.push('=== Recommendations ===');
    for (const rec of parsed.recommendations) {
      lines.push(`- ${rec}`);
    }
  }

  return lines.join('\n');
}
