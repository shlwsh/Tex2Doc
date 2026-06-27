/**
 * Side Panel Application - Full Commercial Dashboard
 */

import React, { useState, useEffect } from 'react';
import Button from '@/ui/components/Button';
import Badge from '@/ui/components/Badge';
import Card from '@/ui/components/Card';
import Progress from '@/ui/components/Progress';
import Select from '@/ui/components/Select';
import Toast from '@/ui/components/Toast';
import { Tabs } from '@/ui/components/Tabs';
import { Modal } from '@/ui/components/Modal';
import { Textarea } from '@/ui/components/Textarea';
import { Avatar } from '@/ui/components/Avatar';
import { sendToBackground } from '@/browser/messaging';
import { MESSAGE_TYPES } from '@/shared/constants';
import type { JobRecord, Session, UsageSummary, FeedbackThread, PlanSummary } from '@/shared/types';
import { useI18n } from '@/ui/i18n/useI18n';
import type { Tab } from '@/ui/components/Tabs';

export default function SidePanelApp() {
  const { t } = useI18n();
  const [activeTab, setActiveTab] = useState<string>('jobs');

  // Session & Usage
  const [session, setSession] = useState<Session | null>(null);
  const [usage, setUsage] = useState<UsageSummary | null>(null);

  // Jobs
  const [jobs, setJobs] = useState<JobRecord[]>([]);
  const [selectedJob, setSelectedJob] = useState<JobRecord | null>(null);

  // Billing
  const [plans, setPlans] = useState<PlanSummary[]>([]);
  const [redeemCode, setRedeemCode] = useState('');

  // Feedback
  const [feedbackThreads, setFeedbackThreads] = useState<FeedbackThread[]>([]);
  const [showFeedbackModal, setShowFeedbackModal] = useState(false);
  const [feedbackTitle, setFeedbackTitle] = useState('');
  const [feedbackContent, setFeedbackContent] = useState('');
  const [feedbackType, setFeedbackType] = useState<'issue' | 'requirement' | 'other'>('issue');

  // Toast
  const [toast, setToast] = useState<{ type: 'success' | 'error' | 'info'; message: string } | null>(null);

  // Load data on mount
  useEffect(() => {
    loadData();
  }, []);

  const loadData = async () => {
    await Promise.all([loadSession(), loadJobs(), loadPlans()]);
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

  const handleLogout = async () => {
    await sendToBackground({ type: MESSAGE_TYPES.LOGOUT });
    setSession(null);
    setUsage(null);
  };

  const handleRedeemCode = async () => {
    if (!redeemCode.trim()) return;
    try {
      await sendToBackground({ type: MESSAGE_TYPES.REDEEM_CODE, code: redeemCode.trim() });
      await loadSession();
      setRedeemCode('');
      setToast({ type: 'success', message: t('rechargeSuccess') });
    } catch (error) {
      setToast({ type: 'error', message: t('errors.networkError') });
    }
  };

  const handleCheckout = async (planId: string) => {
    try {
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
        feedbackType: feedbackType,
        content: feedbackContent.trim(),
      });
      setShowFeedbackModal(false);
      setFeedbackTitle('');
      setFeedbackContent('');
      setFeedbackType('issue');
      setToast({ type: 'success', message: t('feedback') + ' ' + t('success').toLowerCase() });
    } catch (error) {
      setToast({ type: 'error', message: t('errors.networkError') });
    }
  };

  const showToast = (type: 'success' | 'error' | 'info', message: string) => {
    setToast({ type, message });
  };

  // Tab icons as SVG
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
      {/* Header */}
      <div className="flex-shrink-0 bg-white dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700 p-4">
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center gap-3">
            <Avatar name={session?.user?.display_name || session?.user?.email || 'T'} size="lg" />
            <div>
              <h1 className="text-lg font-bold text-gray-900 dark:text-white">
                {t('appName')}
              </h1>
              <p className="text-xs text-gray-500">
                {session?.user?.email || t('errors.authError')}
              </p>
            </div>
          </div>

          {usage && (
            <div className="text-right">
              <p className="text-sm font-medium text-gray-900 dark:text-white">
                {Math.max(0, usage.cloud_conversions_limit - usage.cloud_conversions_used)} /{' '}
                {usage.cloud_conversions_limit}
              </p>
              <p className="text-xs text-gray-500">{t('remaining')}</p>
              <Progress
                value={usage.cloud_conversions_used}
                max={usage.cloud_conversions_limit}
                size="sm"
                className="w-24 mt-1"
              />
            </div>
          )}
        </div>

        {/* Tabs */}
        <Tabs tabs={tabs} activeTab={activeTab} onChange={setActiveTab} variant="underline" />
      </div>

      {/* Content */}
      <div className="flex-1 overflow-auto p-4">
        {/* Jobs Tab */}
        {activeTab === 'jobs' && (
          <div className="space-y-4">
            <div className="flex items-center justify-between">
              <h2 className="text-lg font-semibold text-gray-900 dark:text-white">
                {t('jobHistory')}
              </h2>
              <Button size="sm" variant="secondary" onClick={loadJobs} leftIcon={icons.jobs}>
                {t('refresh')}
              </Button>
            </div>

            {jobs.length === 0 ? (
              <Card className="text-center py-8">
                <p className="text-gray-500">{t('noJobs')}</p>
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
                        <p className="font-medium text-gray-900 dark:text-white truncate">
                          {job.file_name}
                        </p>
                        <p className="text-sm text-gray-500">
                          {job.main_tex} · {job.profile} · {job.quality}
                        </p>
                        <p className="text-xs text-gray-400">
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

                    {/* Expanded View */}
                    {selectedJob?.id === job.id && job.report && (
                      <div className="mt-4 pt-4 border-t border-gray-200 dark:border-gray-700">
                        <h4 className="text-sm font-medium mb-2">Quality Report</h4>
                        <div className="grid grid-cols-2 gap-2 text-sm">
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

        {/* Billing Tab */}
        {activeTab === 'billing' && (
          <div className="space-y-4">
            <h2 className="text-lg font-semibold text-gray-900 dark:text-white">
              {t('plans')}
            </h2>

            {plans.map((plan) => (
              <Card key={plan.id}>
                <div className="flex items-center justify-between mb-3">
                  <div>
                    <h3 className="font-semibold text-gray-900 dark:text-white">{plan.name}</h3>
                    <p className="text-sm text-gray-500">
                      {plan.monthly_conversions} conversions/month
                    </p>
                  </div>
                  <div className="text-right">
                    <p className="text-lg font-bold text-gray-900 dark:text-white">
                      ${(plan.price_cents / 100).toFixed(2)}
                    </p>
                    <p className="text-xs text-gray-500">/{plan.currency}</p>
                  </div>
                </div>
                {session && (
                  <Button className="w-full" onClick={() => handleCheckout(plan.id)}>
                    {t('checkout')}
                  </Button>
                )}
              </Card>
            ))}

            {/* Redeem Code */}
            <Card>
              <h3 className="font-semibold text-gray-900 dark:text-white mb-3">
                {t('redeemCode')}
              </h3>
              <div className="flex gap-2">
                <input
                  type="text"
                  value={redeemCode}
                  onChange={(e) => setRedeemCode(e.target.value)}
                  placeholder={t('enterCode')}
                  className="flex-1 px-3 py-2 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 text-gray-900 dark:text-white"
                />
                <Button onClick={handleRedeemCode} disabled={!redeemCode.trim()}>
                  {t('redeemCode')}
                </Button>
              </div>
            </Card>
          </div>
        )}

        {/* Feedback Tab */}
        {activeTab === 'feedback' && (
          <div className="space-y-4">
            <div className="flex items-center justify-between">
              <h2 className="text-lg font-semibold text-gray-900 dark:text-white">
                {t('feedback')}
              </h2>
              {session && (
                <Button size="sm" onClick={() => setShowFeedbackModal(true)}>
                  {t('submitFeedback')}
                </Button>
              )}
            </div>

            {!session ? (
              <Card className="text-center py-8">
                <p className="text-gray-500">{t('errors.authError')}</p>
              </Card>
            ) : feedbackThreads.length === 0 ? (
              <Card className="text-center py-8">
                <p className="text-gray-500">No feedback threads yet</p>
              </Card>
            ) : (
              <div className="space-y-2">
                {feedbackThreads.map((thread) => (
                  <Card key={thread.thread_id}>
                    <div className="flex items-start justify-between">
                      <div>
                        <h4 className="font-medium text-gray-900 dark:text-white">{thread.title}</h4>
                        <p className="text-sm text-gray-500">{thread.feedback_type}</p>
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

        {/* Account Tab */}
        {activeTab === 'account' && (
          <div className="space-y-4">
            {session ? (
              <>
                <Card>
                  <h3 className="font-semibold text-gray-900 dark:text-white mb-2">
                    {t('account')}
                  </h3>
                  <div className="space-y-2">
                    <div className="flex justify-between">
                      <span className="text-gray-500">{t('email')}</span>
                      <span className="text-gray-900 dark:text-white">{session.user.email}</span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-gray-500">{t('plan')}</span>
                      <span className="text-gray-900 dark:text-white">{session.user.plan_id}</span>
                    </div>
                  </div>
                </Card>

                {usage && (
                  <Card>
                    <h3 className="font-semibold text-gray-900 dark:text-white mb-2">
                      {t('usage')}
                    </h3>
                    <div className="space-y-3">
                      <div>
                        <div className="flex justify-between text-sm mb-1">
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
              <Card className="text-center py-8">
                <p className="text-gray-500 mb-4">Sign in to access your account</p>
                <Button onClick={() => browser.action.openPopup()}>Sign In</Button>
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
          <div>
            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
              {t('feedbackTitle')}
            </label>
            <input
              type="text"
              value={feedbackTitle}
              onChange={(e) => setFeedbackTitle(e.target.value)}
              className="w-full px-3 py-2 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 text-gray-900 dark:text-white"
            />
          </div>
          <div>
            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
              {t('feedbackType')}
            </label>
            <Select
              value={feedbackType}
              onChange={(v) => setFeedbackType(v as typeof feedbackType)}
              options={feedbackTypeOptions}
            />
          </div>
          <div>
            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
              {t('feedbackContent')}
            </label>
            <Textarea
              value={feedbackContent}
              onChange={setFeedbackContent}
              placeholder={t('feedbackContent')}
              rows={4}
            />
          </div>
        </div>
      </Modal>

      {/* Toast */}
      {toast && (
        <Toast
          type={toast.type}
          message={toast.message}
          onClose={() => setToast(null)}
        />
      )}
    </div>
  );
}
