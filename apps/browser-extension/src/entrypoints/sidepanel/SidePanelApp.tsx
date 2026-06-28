/**
 * Side Panel Application - Commercial Dashboard
 *
 * SaaS-style layout with toolbar (user + balance + language) + tabs
 * (Jobs / Billing / Feedback / Account). All copy goes through useI18n().
 */

import React, { useState, useEffect } from 'react';
import Button from '@/ui/components/Button';
import Badge from '@/ui/components/Badge';
import Card from '@/ui/components/Card';
import Progress from '@/ui/components/Progress';
import Select from '@/ui/components/Select';
import Toast from '@/ui/components/Toast';
import Input from '@/ui/components/Input';
import { Tabs } from '@/ui/components/Tabs';
import { Modal } from '@/ui/components/Modal';
import { Textarea } from '@/ui/components/Textarea';
import { Avatar } from '@/ui/components/Avatar';
import { RenewalHint } from '@/ui/components/RenewalHint';
import { track, rotateSessionId } from '@/analytics/funnel';
import { sendToBackground } from '@/browser/messaging';
import { MESSAGE_TYPES } from '@/shared/constants';
import type { JobRecord, Session, UsageSummary, FeedbackThread, PlanSummary } from '@/shared/types';
import { useI18n } from '@/ui/i18n/useI18n';
import type { Tab } from '@/ui/components/Tabs';

type Panel = 'jobs' | 'billing' | 'feedback' | 'account';

export default function SidePanelApp() {
  const { t, locale, setLocale } = useI18n();
  const [activeTab, setActiveTab] = useState<Panel>('jobs');

  const [session, setSession] = useState<Session | null>(null);
  const [usage, setUsage] = useState<UsageSummary | null>(null);

  const [jobs, setJobs] = useState<JobRecord[]>([]);
  const [selectedJob, setSelectedJob] = useState<JobRecord | null>(null);

  const [plans, setPlans] = useState<PlanSummary[]>([]);
  const [redeemCode, setRedeemCode] = useState('');
  const [isRedeeming, setIsRedeeming] = useState(false);

  const [feedbackThreads, setFeedbackThreads] = useState<FeedbackThread[]>([]);
  const [showFeedbackModal, setShowFeedbackModal] = useState(false);
  const [feedbackTitle, setFeedbackTitle] = useState('');
  const [feedbackContent, setFeedbackContent] = useState('');
  const [feedbackType, setFeedbackType] = useState<'issue' | 'requirement' | 'other'>('issue');

  const [toast, setToast] = useState<{ type: 'success' | 'error' | 'info'; message: string } | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [isExportingDiagnostics, setIsExportingDiagnostics] = useState(false);

  useEffect(() => {
    loadData();
  }, []);

  const loadData = async () => {
    setIsLoading(true);
    await Promise.all([loadSession(), loadJobs(), loadPlans()]);
    setIsLoading(false);
  };

  const loadSession = async () => {
    try {
      const result = await sendToBackground<{
        signedIn: boolean;
        user?: unknown;
        usage?: UsageSummary;
      }>({ type: MESSAGE_TYPES.REFRESH_SESSION });

      if (result.signedIn) {
        setSession(result as unknown as Session);
        setUsage(result.usage || null);
      }
    } catch (error) {
      console.error('Failed to load session:', error);
    }
  };

  const loadJobs = async () => {
    try {
      const jobList = await sendToBackground<JobRecord[]>({ type: MESSAGE_TYPES.FETCH_JOBS });
      setJobs(jobList);
    } catch (error) {
      console.error('Failed to load jobs:', error);
    }
  };

  const loadPlans = async () => {
    try {
      const planList = await sendToBackground<PlanSummary[]>({ type: MESSAGE_TYPES.FETCH_PLANS });
      setPlans(planList);
    } catch (error) {
      console.error('Failed to load plans:', error);
    }
  };

  const loadFeedback = async () => {
    if (!session) return;
    try {
      const threads = await sendToBackground<FeedbackThread[]>({ type: MESSAGE_TYPES.FETCH_FEEDBACK });
      setFeedbackThreads(threads);
    } catch (error) {
      console.error('Failed to load feedback:', error);
    }
  };

  useEffect(() => {
    if (activeTab === 'feedback' && session) {
      loadFeedback();
    }
  }, [activeTab, session]);

  const handleLogout = async () => {
    await sendToBackground({ type: MESSAGE_TYPES.LOGOUT });
    setSession(null);
    setUsage(null);
    await rotateSessionId();
  };

  const handleRedeemCode = async () => {
    if (!redeemCode.trim()) return;
    setIsRedeeming(true);
    try {
      // If already signed in, plain top-up; if not, attempt auto-account redeem
      const msgType = session ? MESSAGE_TYPES.REDEEM_CODE : MESSAGE_TYPES.REDEEM_CODE_AND_LOGIN;
      await sendToBackground({ type: msgType, code: redeemCode.trim().toUpperCase() });
      await loadSession();
      setRedeemCode('');
      setToast({ type: 'success', message: session ? t('rechargeSuccess') : t('redeemSuccessNewAccount') });
    } catch (error) {
      setToast({ type: 'error', message: error instanceof Error ? error.message : t('redeemFailed') });
    } finally {
      setIsRedeeming(false);
    }
  };

  const handleCheckout = async (planId: string) => {
    try {
      track('checkout_opened', { stage: 'sidepanel', meta: { plan_id: planId } });
      await sendToBackground({ type: MESSAGE_TYPES.CREATE_CHECKOUT, planId });
    } catch (error) {
      console.error('Checkout failed:', error);
    }
  };

  const handleDownloadDocx = async (job: JobRecord) => {
    if (!job.job_id) return;
    try {
      await sendToBackground({
        type: MESSAGE_TYPES.DOWNLOAD_DOCX,
        jobId: job.id,
        cloudJobId: job.job_id,
      });
      setToast({ type: 'success', message: t('download') + ' ' + t('success').toLowerCase() });
    } catch (error) {
      console.error('Download failed:', error);
    }
  };

  const handleSubmitFeedback = async () => {
    if (!feedbackTitle.trim() || !feedbackContent.trim()) return;
    try {
      await sendToBackground({
        type: MESSAGE_TYPES.CREATE_FEEDBACK,
        title: feedbackTitle.trim(),
        feedbackType,
        content: feedbackContent.trim(),
      });
      setShowFeedbackModal(false);
      setFeedbackTitle('');
      setFeedbackContent('');
      setFeedbackType('issue');
      setToast({ type: 'success', message: t('feedback') + ' ' + t('success').toLowerCase() });
      loadFeedback();
    } catch (error) {
      setToast({ type: 'error', message: error instanceof Error ? error.message : t('errors.networkError') });
    }
  };

  /**
   * P1-3 — Build a sanitized diagnostics bundle and download it as a JSON
   * file the user can attach to a feedback ticket.
   */
  const handleExportDiagnostics = async () => {
    setIsExportingDiagnostics(true);
    try {
      const result = await sendToBackground<{ success: boolean; filename?: string; error?: string }>({
        type: MESSAGE_TYPES.EXPORT_DIAGNOSTICS,
        eventLimit: 200,
      });
      if (result?.success) {
        setToast({ type: 'success', message: t('diagnostics.exportSuccess') });
        track('diagnostics_exported', { stage: 'sidepanel' });
      } else {
        setToast({ type: 'error', message: result?.error ?? t('errors.unknown') });
      }
    } catch (error) {
      setToast({ type: 'error', message: error instanceof Error ? error.message : t('errors.unknown') });
    } finally {
      setIsExportingDiagnostics(false);
    }
  };

  const icons = {
    jobs: (
      <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
      </svg>
    ),
    billing: (
      <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M3 10h18M7 15h1m4 0h1m-7 4h12a3 3 0 003-3V8a3 3 0 00-3-3H6a3 3 0 00-3 3v8a3 3 0 003 3z" />
      </svg>
    ),
    feedback: (
      <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z" />
      </svg>
    ),
    account: (
      <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z" />
      </svg>
    ),
  };

  const tabs: Tab[] = [
    { id: 'jobs', label: t('jobs'), icon: icons.jobs },
    { id: 'billing', label: t('billing'), icon: icons.billing },
    { id: 'feedback', label: t('feedback'), icon: icons.feedback },
    { id: 'account', label: t('account'), icon: icons.account },
  ];

  const feedbackTypeOptions = [
    { value: 'issue', label: t('feedbackTypes.issue') },
    { value: 'requirement', label: t('feedbackTypes.requirement') },
    { value: 'other', label: t('feedbackTypes.other') },
  ];

  return (
    <div className="flex flex-col h-screen bg-gray-50 dark:bg-gray-900">
      {/* Toolbar */}
      <div className="flex-shrink-0 bg-white dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700">
        <div className="flex items-center justify-between p-4">
          <div className="flex items-center gap-3">
            <Avatar name={session?.user?.display_name || session?.user?.email || 'T'} size="md" />
            <div className="leading-tight">
              <h1 className="text-sm font-semibold text-gray-900 dark:text-white">
                {t('appName')}
              </h1>
              <p className="text-[11px] text-gray-500 truncate max-w-[160px]">
                {session?.user?.email || t('signInRequired')}
              </p>
            </div>
          </div>

          <div className="flex items-center gap-2">
            {usage && (
              <div className="text-right">
                <p className="text-xs font-medium text-gray-900 dark:text-white">
                  {Math.max(0, usage.cloud_conversions_limit - usage.cloud_conversions_used)} /{' '}
                  {usage.cloud_conversions_limit}
                </p>
                <p className="text-[10px] text-gray-500">{t('remaining')}</p>
              </div>
            )}
            <select
              value={locale}
              onChange={(e) => setLocale(e.target.value as typeof locale)}
              className="text-xs bg-transparent border border-gray-200 dark:border-gray-700 rounded-md px-1.5 py-1 text-gray-600 dark:text-gray-300"
              aria-label={t('language')}
            >
              <option value="en">EN</option>
              <option value="zh">中</option>
            </select>
          </div>
        </div>

        {usage && (
          <div className="px-4 pb-3">
            <Progress value={usage.cloud_conversions_used} max={usage.cloud_conversions_limit} size="sm" />
          </div>
        )}

        <div className="px-4">
          <Tabs tabs={tabs} activeTab={activeTab} onChange={(id) => setActiveTab(id as Panel)} variant="underline" />
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-auto p-4 space-y-4">
        {isLoading && (
          <div className="flex items-center justify-center py-12 text-sm text-gray-500">
            <svg className="animate-spin h-4 w-4 mr-2" fill="none" viewBox="0 0 24 24">
              <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4"></circle>
              <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
            </svg>
            {t('loading')}
          </div>
        )}

        {!isLoading && activeTab === 'jobs' && (
          <div className="space-y-4">
            <div className="flex items-center justify-between">
              <h2 className="text-sm font-semibold text-gray-900 dark:text-white">{t('jobHistory')}</h2>
              <Button size="sm" variant="secondary" onClick={loadJobs} leftIcon={icons.jobs}>
                {t('refresh')}
              </Button>
            </div>

            {jobs.length === 0 ? (
              <Card className="text-center py-8 space-y-1">
                <p className="text-sm font-medium text-gray-700 dark:text-gray-300">{t('empty.noJobs.title')}</p>
                <p className="text-xs text-gray-500">{t('empty.noJobs.description')}</p>
              </Card>
            ) : (
              <div className="space-y-2">
                {jobs.map((job) => (
                  <Card
                    key={job.id}
                    hover
                    onClick={() => setSelectedJob(selectedJob?.id === job.id ? null : job)}
                    className={selectedJob?.id === job.id ? 'ring-2 ring-primary-500' : ''}
                  >
                    <div className="flex items-center justify-between">
                      <div className="flex-1 min-w-0">
                        <p className="text-sm font-medium text-gray-900 dark:text-white truncate">
                          {job.file_name}
                        </p>
                        <p className="text-xs text-gray-500">
                          {job.main_tex} · {job.profile} · {job.quality}
                        </p>
                        <p className="text-[11px] text-gray-400">
                          {new Date(job.created_at).toLocaleString()}
                        </p>
                      </div>
                      <div className="flex flex-col items-end gap-2">
                        <Badge
                          variant={
                            job.status === 'completed'
                              ? 'success'
                              : job.status === 'failed'
                                ? 'error'
                                : job.status === 'processing'
                                  ? 'warning'
                                  : 'default'
                          }
                        >
                          {t(`jobStatus.${job.status}`)}
                        </Badge>
                        {job.status === 'completed' && job.docx_ready && (
                          <Button size="sm" onClick={() => handleDownloadDocx(job)}>
                            {t('download')}
                          </Button>
                        )}
                      </div>
                    </div>

                    {selectedJob?.id === job.id && job.report && (
                      <div className="mt-3 pt-3 border-t border-gray-200 dark:border-gray-700">
                        <h4 className="text-xs font-medium mb-2 text-gray-700 dark:text-gray-300">Quality Report</h4>
                        <div className="grid grid-cols-2 gap-2 text-xs">
                          <div>
                            <span className="text-gray-500">Score:</span> {job.report.quality_score}%
                          </div>
                          <div>
                            <span className="text-gray-500">Profile:</span> {job.report.profile}
                          </div>
                        </div>
                      </div>
                    )}
                  </Card>
                ))}
              </div>
            )}
          </div>
        )}

        {!isLoading && activeTab === 'billing' && (
          <div className="space-y-4">
            <h2 className="text-sm font-semibold text-gray-900 dark:text-white">{t('plans')}</h2>

            {plans.length === 0 ? (
              <Card className="text-center py-8 space-y-1">
                <p className="text-sm font-medium text-gray-700 dark:text-gray-300">{t('empty.noPlans.title')}</p>
                <p className="text-xs text-gray-500">{t('empty.noPlans.description')}</p>
              </Card>
            ) : (
              <div className="space-y-3">
                {plans.map((plan) => (
                  <Card key={plan.id}>
                    <div className="flex items-center justify-between mb-3">
                      <div>
                        <h3 className="text-sm font-semibold text-gray-900 dark:text-white">{plan.name}</h3>
                        <p className="text-xs text-gray-500">
                          {plan.monthly_conversions} conversions/month
                        </p>
                      </div>
                      <div className="text-right">
                        <p className="text-lg font-bold text-gray-900 dark:text-white">
                          ${(plan.price_cents / 100).toFixed(2)}
                        </p>
                        <p className="text-[11px] text-gray-500">/{plan.currency}</p>
                      </div>
                    </div>
                    {session && (
                      <Button className="w-full" size="sm" onClick={() => handleCheckout(plan.id)}>
                        {t('checkout')}
                      </Button>
                    )}
                  </Card>
                ))}
              </div>
            )}

            <Card>
              <h3 className="text-sm font-semibold text-gray-900 dark:text-white mb-2">
                {t('redeemCode')}
              </h3>
              <p className="text-xs text-gray-500 mb-3">{t('redeemDescription')}</p>
              <div className="flex gap-2">
                <Input
                  type="text"
                  value={redeemCode}
                  onChange={(e) => setRedeemCode(e.target.value)}
                  placeholder={t('redeemPlaceholder')}
                />
                <Button onClick={handleRedeemCode} disabled={!redeemCode.trim() || isRedeeming} isLoading={isRedeeming}>
                  {t('redeem')}
                </Button>
              </div>
            </Card>
          </div>
        )}

        {!isLoading && activeTab === 'feedback' && (
          <div className="space-y-4">
            <div className="flex items-center justify-between">
              <h2 className="text-sm font-semibold text-gray-900 dark:text-white">{t('feedback')}</h2>
              {session && (
                <Button size="sm" onClick={() => setShowFeedbackModal(true)}>
                  {t('submitFeedback')}
                </Button>
              )}
            </div>

            {!session ? (
              <Card className="text-center py-8">
                <p className="text-sm text-gray-500">{t('signInRequired')}</p>
              </Card>
            ) : (
              <div className="space-y-3">
                <Card className="space-y-2">
                  <div className="flex items-start justify-between gap-2">
                    <div>
                      <h3 className="text-sm font-semibold text-gray-900 dark:text-white">
                        {t('diagnostics.title')}
                      </h3>
                      <p className="text-xs text-gray-500 mt-1">
                        {t('diagnostics.description')}
                      </p>
                    </div>
                    <Button
                      size="sm"
                      variant="secondary"
                      onClick={handleExportDiagnostics}
                      disabled={isExportingDiagnostics}
                    >
                      {isExportingDiagnostics ? t('loading') : t('diagnostics.export')}
                    </Button>
                  </div>
                  <p className="text-[11px] text-gray-500 leading-snug">
                    {t('diagnostics.privacyNote')}
                  </p>
                </Card>
                {feedbackThreads.length === 0 ? (
                  <Card className="text-center py-8 space-y-1">
                    <p className="text-sm font-medium text-gray-700 dark:text-gray-300">{t('empty.noFeedback.title')}</p>
                    <p className="text-xs text-gray-500">{t('empty.noFeedback.description')}</p>
                  </Card>
                ) : (
                  <div className="space-y-2">
                    {feedbackThreads.map((thread) => (
                      <Card key={thread.thread_id}>
                        <div className="flex items-start justify-between">
                          <div>
                            <h4 className="text-sm font-medium text-gray-900 dark:text-white">{thread.title}</h4>
                            <p className="text-xs text-gray-500">{thread.feedback_type}</p>
                          </div>
                          <Badge variant={thread.status === 'open' ? 'warning' : 'default'}>
                            {thread.status}
                          </Badge>
                        </div>
                      </Card>
                    ))}
                  </div>
                )}
              </div>
            )}
          </div>
        )}

        {!isLoading && activeTab === 'account' && (
          <div className="space-y-4">
            {session ? (
              <>
                <Card>
                  <h3 className="text-sm font-semibold text-gray-900 dark:text-white mb-3">
                    {t('account')}
                  </h3>
                  <div className="space-y-2 text-xs">
                    <div className="flex justify-between">
                      <span className="text-gray-500">{t('email')}</span>
                      <span className="text-gray-900 dark:text-white">{session.user.email}</span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-gray-500">{t('plan')}</span>
                      <span className="text-gray-900 dark:text-white">{session.user.plan_id}</span>
                    </div>
                  </div>
                  {usage && (
                    <RenewalHint dateValidUntil={usage.date_valid_until} variant="banner" className="mt-3" />
                  )}
                </Card>

                {usage && (
                  <Card>
                    <h3 className="text-sm font-semibold text-gray-900 dark:text-white mb-3">
                      {t('usage')}
                    </h3>
                    <div className="space-y-3 text-xs">
                      <div>
                        <div className="flex justify-between mb-1">
                          <span className="text-gray-500">Cloud Conversions</span>
                          <span>
                            {usage.cloud_conversions_used} / {usage.cloud_conversions_limit}
                          </span>
                        </div>
                        <Progress
                          value={usage.cloud_conversions_used}
                          max={usage.cloud_conversions_limit}
                        />
                      </div>
                      <div className="flex justify-between">
                        <span className="text-gray-500">Count Balance</span>
                        <span className="text-gray-900 dark:text-white">
                          {usage.count_balance}
                        </span>
                      </div>
                    </div>
                  </Card>
                )}

                <Button
                  variant="secondary"
                  className="w-full"
                  onClick={() => sendToBackground({ type: MESSAGE_TYPES.CREATE_PORTAL })}
                >
                  {t('portal')}
                </Button>

                <Button variant="ghost" className="w-full" onClick={handleLogout}>
                  {t('signOut')}
                </Button>
              </>
            ) : (
              <Card className="text-center py-8 space-y-2">
                <p className="text-sm font-medium text-gray-700 dark:text-gray-300">{t('signInRequired')}</p>
                <p className="text-xs text-gray-500">{t('signInOrRedeem')}</p>
                <Button onClick={() => browser.action.openPopup()}>{t('signIn')}</Button>
              </Card>
            )}
          </div>
        )}
      </div>

      {/* Feedback Modal */}
      <Modal
        open={showFeedbackModal}
        onClose={() => setShowFeedbackModal(false)}
        title={t('submitFeedback')}
        footer={
          <div className="flex gap-2 justify-end">
            <Button variant="secondary" onClick={() => setShowFeedbackModal(false)}>
              {t('cancel')}
            </Button>
            <Button onClick={handleSubmitFeedback} disabled={!feedbackTitle.trim() || !feedbackContent.trim()}>
              {t('submitFeedback')}
            </Button>
          </div>
        }
      >
        <div className="space-y-4">
          <Input
            type="text"
            label={t('feedbackTitle')}
            value={feedbackTitle}
            onChange={(e) => setFeedbackTitle(e.target.value)}
          />
          <Select
            label={t('feedbackType')}
            value={feedbackType}
            onChange={(v) => setFeedbackType(v as typeof feedbackType)}
            options={feedbackTypeOptions}
          />
          <Textarea
            label={t('feedbackContent')}
            value={feedbackContent}
            onChange={setFeedbackContent}
            placeholder={t('feedbackContent')}
            rows={4}
          />
        </div>
      </Modal>

      {toast && (
        <Toast type={toast.type} title={toast.message} onClose={() => setToast(null)} />
      )}
    </div>
  );
}