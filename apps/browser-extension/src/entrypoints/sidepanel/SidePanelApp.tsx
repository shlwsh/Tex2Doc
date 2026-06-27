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
import { sendToBackground } from '@/browser/messaging';
import { MESSAGE_TYPES } from '@/shared/constants';
import type { JobRecord, Session, UsageSummary, FeedbackThread, PlanSummary } from '@/shared/types';
import { t, type Locale } from '@/ui/i18n';

type Tab = 'jobs' | 'billing' | 'feedback' | 'account';

export default function SidePanelApp() {
  const [locale, setLocale] = useState<Locale>('en');
  const [activeTab, setActiveTab] = useState<Tab>('jobs');

  // Session & Usage
  const [session, setSession] = useState<Session | null>(null);
  const [usage, setUsage] = useState<UsageSummary | null>(null);

  // Jobs
  const [jobs, setJobs] = useState<JobRecord[]>([]);
  const [selectedJob, setSelectedJob] = useState<JobRecord | null>(null);

  // Billing
  const [plans, setPlans] = useState<PlanSummary[]>([]);

  // Feedback
  const [feedbackThreads, setFeedbackThreads] = useState<FeedbackThread[]>([]);
  const [showFeedbackForm, setShowFeedbackForm] = useState(false);

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

  const handleRedeemCode = async (code: string) => {
    try {
      await sendToBackground({ type: MESSAGE_TYPES.REDEEM_CODE, code });
      await loadSession();
      alert('Code redeemed successfully!');
    } catch (error) {
      alert('Failed to redeem code');
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
    } catch (error) {
      console.error('Download failed:', error);
    }
  };

  // ============================================
  // Render
  // ============================================

  const tabs = [
    { id: 'jobs' as Tab, label: t(locale, 'jobs'), icon: '📄' },
    { id: 'billing' as Tab, label: t(locale, 'billing'), icon: '💳' },
    { id: 'feedback' as Tab, label: t(locale, 'feedback'), icon: '💬' },
    { id: 'account' as Tab, label: t(locale, 'account'), icon: '👤' },
  ];

  return (
    <div className="flex flex-col h-screen bg-gray-50 dark:bg-gray-900">
      {/* Header */}
      <div className="flex-shrink-0 bg-white dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700 p-4">
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 bg-primary-600 rounded-xl flex items-center justify-center">
              <span className="text-white font-bold">T2D</span>
            </div>
            <div>
              <h1 className="text-lg font-bold text-gray-900 dark:text-white">
                {t(locale, 'appName')}
              </h1>
              <p className="text-xs text-gray-500">
                {session?.user?.email || 'Not signed in'}
              </p>
            </div>
          </div>

          {usage && (
            <div className="text-right">
              <p className="text-sm font-medium text-gray-900 dark:text-white">
                {Math.max(0, usage.cloud_conversions_limit - usage.cloud_conversions_used)} /{' '}
                {usage.cloud_conversions_limit}
              </p>
              <p className="text-xs text-gray-500">{t(locale, 'remaining')}</p>
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
        <div className="flex gap-1">
          {tabs.map((tab) => (
            <button
              key={tab.id}
              onClick={() => setActiveTab(tab.id)}
              className={`flex-1 py-2 px-3 rounded-lg text-sm font-medium transition-colors ${
                activeTab === tab.id
                  ? 'bg-primary-100 text-primary-700 dark:bg-primary-900/30 dark:text-primary-300'
                  : 'text-gray-600 hover:bg-gray-100 dark:text-gray-400 dark:hover:bg-gray-800'
              }`}
            >
              <span className="mr-1">{tab.icon}</span>
              {tab.label}
            </button>
          ))}
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-auto p-4">
        {/* Jobs Tab */}
        {activeTab === 'jobs' && (
          <div className="space-y-4">
            <div className="flex items-center justify-between">
              <h2 className="text-lg font-semibold text-gray-900 dark:text-white">
                {t(locale, 'jobHistory')}
              </h2>
              <Button size="sm" variant="secondary" onClick={loadJobs}>
                {t(locale, 'refresh')}
              </Button>
            </div>

            {jobs.length === 0 ? (
              <Card className="text-center py-8">
                <p className="text-gray-500">{t(locale, 'noJobs')}</p>
              </Card>
            ) : (
              <div className="space-y-2">
                {jobs.map((job) => (
                  <Card
                    key={job.id}
                    hover
                    onClick={() => setSelectedJob(job)}
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
                          {t(locale, `jobStatus.${job.status}`)}
                        </Badge>
                        {job.status === 'completed' && job.docx_ready && (
                          <Button size="sm" onClick={() => handleDownloadDocx(job)}>
                            {t(locale, 'download')}
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
                            <span className="text-gray-500">Score:</span>{' '}
                            {job.report.quality_score}%
                          </div>
                          <div>
                            <span className="text-gray-500">Profile:</span>{' '}
                            {job.report.profile}
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
              {t(locale, 'plans')}
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
                    {t(locale, 'checkout')}
                  </Button>
                )}
              </Card>
            ))}

            {/* Redeem Code */}
            <Card>
              <h3 className="font-semibold text-gray-900 dark:text-white mb-3">
                {t(locale, 'redeemCode')}
              </h3>
              <div className="flex gap-2">
                <input
                  type="text"
                  placeholder={t(locale, 'enterCode')}
                  className="flex-1 px-3 py-2 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800"
                  id="redeem-input"
                />
                <Button
                  onClick={() => {
                    const input = document.getElementById('redeem-input') as HTMLInputElement;
                    if (input?.value) {
                      handleRedeemCode(input.value);
                      input.value = '';
                    }
                  }}
                >
                  {t(locale, 'redeemCode')}
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
                {t(locale, 'feedback')}
              </h2>
              {session && (
                <Button size="sm" onClick={() => setShowFeedbackForm(true)}>
                  {t(locale, 'submitFeedback')}
                </Button>
              )}
            </div>

            {!session ? (
              <Card className="text-center py-8">
                <p className="text-gray-500">{t(locale, 'errors.authError')}</p>
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

            {/* Feedback Form Modal */}
            {showFeedbackForm && (
              <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
                <Card className="w-full max-w-md mx-4">
                  <h3 className="font-semibold mb-4">{t(locale, 'submitFeedback')}</h3>
                  <div className="space-y-3">
                    <input
                      type="text"
                      placeholder={t(locale, 'feedbackTitle')}
                      className="w-full px-3 py-2 rounded-lg border"
                      id="feedback-title"
                    />
                    <textarea
                      placeholder={t(locale, 'feedbackContent')}
                      rows={4}
                      className="w-full px-3 py-2 rounded-lg border"
                      id="feedback-content"
                    />
                    <div className="flex gap-2 justify-end">
                      <Button variant="secondary" onClick={() => setShowFeedbackForm(false)}>
                        {t(locale, 'cancel')}
                      </Button>
                      <Button
                        onClick={() => {
                          // Submit feedback
                          setShowFeedbackForm(false);
                        }}
                      >
                        {t(locale, 'submitFeedback')}
                      </Button>
                    </div>
                  </div>
                </Card>
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
                    {t(locale, 'account')}
                  </h3>
                  <div className="space-y-2">
                    <div className="flex justify-between">
                      <span className="text-gray-500">{t(locale, 'email')}</span>
                      <span className="text-gray-900 dark:text-white">{session.user.email}</span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-gray-500">{t(locale, 'plan')}</span>
                      <span className="text-gray-900 dark:text-white">{session.user.plan_id}</span>
                    </div>
                  </div>
                </Card>

                {usage && (
                  <Card>
                    <h3 className="font-semibold text-gray-900 dark:text-white mb-2">
                      {t(locale, 'usage')}
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
                  {t(locale, 'portal')}
                </Button>

                <Button variant="ghost" className="w-full" onClick={handleLogout}>
                  {t(locale, 'signOut')}
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
    </div>
  );
}
