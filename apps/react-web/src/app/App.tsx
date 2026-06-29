import { FormEvent, useEffect, useMemo, useState } from 'react';
import { Link, Navigate, Route, Routes, useLocation, useNavigate } from 'react-router-dom';
import {
  Activity,
  ArrowDownToLine,
  Bot,
  Check,
  ClipboardList,
  CloudUpload,
  FileArchive,
  FileText,
  History,
  Home,
  KeyRound,
  LogOut,
  MessageSquare,
  PackagePlus,
  RefreshCw,
  Rocket,
  Search,
  Send,
  Settings,
  Shield,
  ShoppingCart,
  User,
  Zap,
} from 'lucide-react';
import { unzipSync } from 'fflate';
import { Tex2DocApi } from '../api/client';
import { ApiError, downloadBlob, defaultApiBaseUrl } from '../api/http';
import type {
  AdminDashboardSummary,
  AdminRedeemCode,
  AutomationAgent,
  AutomationEvent,
  AutomationRequest,
  ConversionJob,
  FeedbackThread,
  FeedbackThreadDetail,
  RechargeRecord,
  RedeemCodeBatch,
  RedeemCodeRecord,
  ReleaseManifest,
  UsageSummary,
} from '../api/types';
import { convertZipToDocx } from '../wasm/doc-engine';
import {
  AuthSession,
  initialApiBaseUrl,
  storedQuickCode,
  useSessionStore,
} from '../stores/session';

type LoadState = 'idle' | 'loading' | 'ready' | 'error';
type NavItem = { id: string; label: string; icon: typeof Home };

const userNav: NavItem[] = [
  { id: 'account', label: '账号', icon: User },
  { id: 'recharge', label: '充值', icon: ShoppingCart },
  { id: 'conversion', label: '转换', icon: CloudUpload },
  { id: 'conversion-records', label: '转换记录', icon: ClipboardList },
  { id: 'recharge-records', label: '充值记录', icon: History },
  { id: 'feedback', label: '反馈', icon: MessageSquare },
  { id: 'about', label: '关于', icon: FileText },
];

const adminNav: NavItem[] = [
  { id: 'dashboard', label: '管理端仪表盘', icon: Activity },
  { id: 'account', label: '账号', icon: User },
  { id: 'redeem-create', label: '兑换码生成', icon: PackagePlus },
  { id: 'redeem-batches', label: '兑换码批次', icon: FileArchive },
  { id: 'redeem-stock', label: '兑换码库存', icon: KeyRound },
  { id: 'feedback', label: 'Feedback management', icon: MessageSquare },
  { id: 'release', label: '发布管理', icon: Rocket },
  { id: 'audit', label: '审计中心', icon: ClipboardList },
  { id: 'automation', label: '自动化', icon: Bot },
  { id: 'about', label: '关于', icon: FileText },
];

export function App() {
  const theme = useSessionStore((s) => s.theme);

  useEffect(() => {
    document.documentElement.dataset.theme = theme;
  }, [theme]);

  return (
    <Routes>
      <Route path="/" element={<HomePage />} />
      <Route path="/react" element={<HomePage />} />
      <Route path="/app-react/*" element={<UserApp />} />
      <Route path="/admin-react/*" element={<AdminApp />} />
      <Route path="*" element={<Navigate to="/react" replace />} />
    </Routes>
  );
}

function HomePage() {
  return (
    <main className="home">
      <section className="home__hero">
        <div>
          <p className="eyebrow">React Web</p>
          <h1>Tex2Doc</h1>
          <p className="lead">LaTeX ZIP 项目转换、兑换码运营、反馈与自动化研发管理的一体化 Web 工作台。</p>
          <div className="actions">
            <Link className="button button--primary" to="/app-react">
              <Zap size={18} /> 用户端
            </Link>
            <Link className="button" to="/admin-react">
              <Shield size={18} /> 管理端
            </Link>
          </div>
        </div>
        <div className="hero-panel" aria-hidden>
          <div className="hero-panel__row"><span>Local WASM</span><strong>lazy</strong></div>
          <div className="hero-panel__row"><span>Cloud Queue</span><strong>120 polls</strong></div>
          <div className="hero-panel__row"><span>Admin Gate</span><strong>role check</strong></div>
        </div>
      </section>
    </main>
  );
}

function UserApp() {
  const [mode, setMode] = useState<'quick' | 'member'>('quick');

  return (
    <WorkspaceFrame
      title="Tex2Doc 用户端"
      subtitle="快捷助手与会员中心"
      modeSwitcher={
        <Segmented
          value={mode}
          onChange={(next) => setMode(next as 'quick' | 'member')}
          items={[
            ['quick', '快捷助手'],
            ['member', '会员中心'],
          ]}
        />
      }
    >
      {mode === 'quick' ? <QuickAssistant /> : <MemberCenter />}
    </WorkspaceFrame>
  );
}

function AdminApp() {
  const adminSession = useSessionStore((s) => s.adminSession);
  const setAdminSession = useSessionStore((s) => s.setAdminSession);
  const [active, setActive] = useState('dashboard');
  const [gate, setGate] = useState<LoadState>('idle');
  const [message, setMessage] = useState('');

  useEffect(() => {
    if (!adminSession) {
      setGate('idle');
      return;
    }
    let cancelled = false;
    setGate('loading');
    new Tex2DocApi(adminSession.apiBaseUrl, adminSession.accessToken)
      .adminMe()
      .then((profile) => {
        if (cancelled) return;
        const role = profile.user?.role ?? adminSession.user.role;
        if (role !== 'admin') {
          setAdminSession(undefined);
          setGate('error');
          setMessage('Admin role required.');
          return;
        }
        setGate('ready');
      })
      .catch((error) => {
        if (cancelled) return;
        setAdminSession(undefined);
        setGate('error');
        setMessage(errorMessage(error));
      });
    return () => {
      cancelled = true;
    };
  }, [adminSession, setAdminSession]);

  if (!adminSession) {
    return (
      <WorkspaceFrame title="Tex2Doc 管理端" subtitle="运营、发布、反馈与自动化研发">
        <AuthPanel kind="admin" error={message} />
      </WorkspaceFrame>
    );
  }

  return (
    <WorkspaceFrame title="Tex2Doc 管理端" subtitle="运营、发布、反馈与自动化研发">
      <ShellLayout nav={adminNav} active={active} onActive={setActive}>
        {gate === 'loading' ? <StateBox label="正在校验管理员权限..." /> : <AdminSection active={active} />}
      </ShellLayout>
    </WorkspaceFrame>
  );
}

function WorkspaceFrame({
  title,
  subtitle,
  modeSwitcher,
  children,
}: {
  title: string;
  subtitle: string;
  modeSwitcher?: React.ReactNode;
  children: React.ReactNode;
}) {
  const navigate = useNavigate();
  const location = useLocation();
  const { theme, locale, setPreferences } = useSessionStore();

  return (
    <main className="workspace">
      <header className="topbar">
        <button className="icon-button" title="首页" onClick={() => navigate('/react')}>
          <Home size={18} />
        </button>
        <div className="topbar__title">
          <strong>{title}</strong>
          <span>{subtitle}</span>
        </div>
        {modeSwitcher}
        <div className="topbar__spacer" />
        <select value={theme} onChange={(e) => setPreferences(e.target.value as 'light' | 'dark', locale)}>
          <option value="light">默认</option>
          <option value="dark">深色</option>
        </select>
        <select value={locale} onChange={(e) => setPreferences(theme, e.target.value as 'zh-CN' | 'en-US')}>
          <option value="zh-CN">zh-CN</option>
          <option value="en-US">en-US</option>
        </select>
        <span className="path-pill">{location.pathname}</span>
      </header>
      {children}
    </main>
  );
}

function ShellLayout({
  nav,
  active,
  onActive,
  children,
}: {
  nav: NavItem[];
  active: string;
  onActive: (id: string) => void;
  children: React.ReactNode;
}) {
  return (
    <div className="shell">
      <aside className="sidebar">
        {nav.map((item) => (
          <button key={item.id} className={item.id === active ? 'nav-item is-active' : 'nav-item'} onClick={() => onActive(item.id)}>
            <item.icon size={17} />
            <span>{item.label}</span>
          </button>
        ))}
      </aside>
      <div className="mobile-tabs">
        <Segmented value={active} onChange={onActive} items={nav.map((item) => [item.id, item.label])} />
      </div>
      <section className="content">{children}</section>
    </div>
  );
}

function MemberCenter() {
  const session = useSessionStore((s) => s.userSession);
  const [active, setActive] = useState('account');

  if (!session) {
    return <AuthPanel kind="user" />;
  }

  return (
    <ShellLayout nav={userNav} active={active} onActive={setActive}>
      <UserSection active={active} session={session} />
    </ShellLayout>
  );
}

function AuthPanel({ kind, error }: { kind: 'user' | 'admin'; error?: string }) {
  const setUserSession = useSessionStore((s) => s.setUserSession);
  const setAdminSession = useSessionStore((s) => s.setAdminSession);
  const [tab, setTab] = useState<'login' | 'register'>('login');
  const [apiBaseUrl, setApiBaseUrl] = useState(initialApiBaseUrl());
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [displayName, setDisplayName] = useState('');
  const [message, setMessage] = useState(error ?? '');
  const [loading, setLoading] = useState(false);

  async function submit(event: FormEvent) {
    event.preventDefault();
    setLoading(true);
    setMessage('');
    try {
      const api = new Tex2DocApi(apiBaseUrl);
      const auth = tab === 'login' ? await api.login(email, password) : await api.register(email, password, displayName);
      const session = { apiBaseUrl: api.baseUrl, accessToken: auth.access_token, refreshToken: auth.refresh_token, user: auth.user };
      if (kind === 'admin') {
        const adminApi = new Tex2DocApi(api.baseUrl, auth.access_token);
        const profile = await adminApi.adminMe();
        if ((profile.user?.role ?? auth.user.role) !== 'admin') {
          throw new Error('Admin role required.');
        }
        setAdminSession({ ...session, user: { ...auth.user, role: 'admin' } });
      } else {
        setUserSession(session);
      }
    } catch (err) {
      setMessage(errorMessage(err));
    } finally {
      setLoading(false);
    }
  }

  return (
    <section className="panel panel--narrow">
      <div className="panel__header">
        <div>
          <p className="eyebrow">{kind === 'admin' ? 'Admin Gate' : 'Member Center'}</p>
          <h2>{kind === 'admin' ? '管理员登录' : '登录或注册'}</h2>
        </div>
        <Segmented value={tab} onChange={(value) => setTab(value as 'login' | 'register')} items={[['login', '登录'], ['register', '注册']]} />
      </div>
      <form className="form" onSubmit={submit}>
        <label>API Base URL<input value={apiBaseUrl} onChange={(e) => setApiBaseUrl(e.target.value)} /></label>
        <label>邮箱<input type="email" required value={email} onChange={(e) => setEmail(e.target.value)} /></label>
        <label>密码<input type="password" required minLength={tab === 'register' ? 6 : undefined} value={password} onChange={(e) => setPassword(e.target.value)} /></label>
        {tab === 'register' && <label>显示名<input value={displayName} onChange={(e) => setDisplayName(e.target.value)} /></label>}
        {message && <div className="alert alert--error">{message}</div>}
        <button className="button button--primary" disabled={loading}>
          {loading ? <RefreshCw size={17} className="spin" /> : <KeyRound size={17} />}
          {tab === 'login' ? '登录' : '注册'}
        </button>
      </form>
    </section>
  );
}

function QuickAssistant() {
  const quickSession = useSessionStore((s) => s.quickSession);
  const setQuickSession = useSessionStore((s) => s.setQuickSession);
  const [apiBaseUrl, setApiBaseUrl] = useState(defaultApiBaseUrl());
  const [code, setCode] = useState(storedQuickCode());
  const [mode, setMode] = useState<'local' | 'cloud'>('local');
  const [file, setFile] = useState<File>();
  const [mainTex, setMainTex] = useState('main.tex');
  const [profile, setProfile] = useState('jos');
  const [quality, setQuality] = useState('high');
  const [logs, setLogs] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);
  const [lastDocx, setLastDocx] = useState<Blob>();

  const api = useMemo(
    () => (quickSession ? new Tex2DocApi(quickSession.apiBaseUrl, quickSession.accessToken) : undefined),
    [quickSession],
  );

  function log(line: string) {
    setLogs((old) => [`${new Date().toLocaleTimeString()} ${line}`, ...old].slice(0, 100));
  }

  async function activate(candidate = code) {
    if (!candidate.trim()) return;
    setLoading(true);
    try {
      const anonymous = new Tex2DocApi(apiBaseUrl);
      log('开始使用兑换码登录影子账号。');
      let auth;
      try {
        auth = await anonymous.login(candidate.trim(), candidate.trim());
      } catch (err) {
        if (err instanceof ApiError && [401, 404].includes(err.status)) {
          log('影子账号不存在，尝试注册。');
          auth = await anonymous.register(candidate.trim(), candidate.trim());
        } else {
          throw err;
        }
      }
      const authed = new Tex2DocApi(anonymous.baseUrl, auth.access_token);
      try {
        await authed.redeemCode(candidate.trim());
        log('兑换码兑换成功。');
      } catch (err) {
        if (err instanceof ApiError && err.status === 409) {
          log('兑换码已兑换，按恢复会话处理。');
        } else if (auth.user.email === candidate.trim()) {
          log(`兑换校验未通过，已登录影子账号，按恢复会话处理：${errorMessage(err)}`);
        } else {
          throw err;
        }
      }
      const usage = await authed.usage();
      setQuickSession({
        apiBaseUrl: anonymous.baseUrl,
        accessToken: auth.access_token,
        refreshToken: auth.refresh_token,
        user: auth.user,
        usage,
        redeemCode: candidate.trim(),
      });
      log('快捷助手已激活。');
    } catch (err) {
      log(`激活失败：${errorMessage(err)}`);
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    if (!quickSession && code) {
      void activate(code);
    }
  }, []);

  async function runConvert() {
    if (!api || !quickSession || !file) return;
    if (file.size >= 10 * 1024 * 1024) {
      log('ZIP 文件需小于 10 MB。');
      return;
    }
    setLoading(true);
    try {
      if (mode === 'local') {
        log('检查本地转换额度。');
        const quota = await api.checkLocalConversion();
        if (!quota.allowed) {
          log(`额度不足：${quota.reason ?? 'not allowed'}`);
          return;
        }
        if (!quota.valid_until_active && (quota.count_balance ?? 0) <= 0) {
          log('本地转换需要可用按次余额或日期权益，请先兑换有效卡密。');
          return;
        }
        log('开始懒加载 WASM 并转换。');
        const docx = await convertZipToDocx(file, { mainTex, profile, quality });
        log('转换成功，开始扣减本地额度。');
        const consumed = await api.consumeLocalConversion();
        if (!consumed.consumed) {
          log('扣费失败，已阻止 DOCX 交付。');
          return;
        }
        setLastDocx(docx);
        downloadBlob(docx, file.name.replace(/\.zip$/i, '.docx') || 'tex2doc.docx');
        log('DOCX 已下载。');
      } else {
        log('上传 ZIP 到云端。');
        const upload = await api.uploadProjectZip(file);
        const created = await api.createConversion(upload.upload_id, mainTex, profile, quality);
        const jobId = jobIdOf(created);
        log(`云端任务已创建：${jobId}`);
        const job = await pollConversion(api, jobId, log);
        const docx = await api.downloadConversionDocx(jobIdOf(job));
        downloadBlob(docx, `${jobIdOf(job)}.docx`);
        log('云端 DOCX 已下载。');
      }
      const usage = await api.usage();
      setQuickSession({ ...quickSession, usage });
    } catch (err) {
      log(`转换失败：${errorMessage(err)}`);
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="quick-grid">
      <section className="panel">
        <div className="panel__header">
          <div>
            <p className="eyebrow">Quick Assistant</p>
            <h2>快捷助手</h2>
          </div>
          <StatusBadge value={quickSession ? 'activated' : 'idle'} />
        </div>
        <div className="form">
          <label>API Base URL<input value={apiBaseUrl} onChange={(e) => setApiBaseUrl(e.target.value)} disabled={!!quickSession} /></label>
          <label>兑换码<input value={code} onChange={(e) => setCode(e.target.value)} disabled={!!quickSession} /></label>
          <div className="actions">
            <button className="button button--primary" onClick={() => activate()} disabled={loading || !!quickSession}><KeyRound size={17} /> 激活当前模式</button>
            {quickSession && <button className="button" onClick={() => setQuickSession(undefined)}><LogOut size={17} /> 清除激活</button>}
          </div>
        </div>
        {quickSession && <UsageCards usage={quickSession.usage} />}
      </section>

      <section className="panel">
        <div className="panel__header">
          <div>
            <p className="eyebrow">Conversion</p>
            <h2>ZIP 转 DOCX</h2>
          </div>
          <Segmented value={mode} onChange={(value) => setMode(value as 'local' | 'cloud')} items={[['local', '快捷版'], ['cloud', '专业版']]} />
        </div>
        <div className="form">
          <label>项目 ZIP<input type="file" accept=".zip" disabled={!quickSession} onChange={(e) => void handleQuickZipSelection(e.target.files?.[0], setFile, setMainTex, log)} /></label>
          <label>主 TeX<input value={mainTex} onChange={(e) => setMainTex(e.target.value)} /></label>
          <div className="form-row">
            <label>Profile<select value={profile} onChange={(e) => setProfile(e.target.value)}><option value="jos">JOS</option><option value="standard">Standard</option></select></label>
            <label>Quality<select value={quality} onChange={(e) => setQuality(e.target.value)}><option value="high">High</option><option value="medium">Medium</option></select></label>
          </div>
          <div className="actions">
            <button className="button button--primary" onClick={runConvert} disabled={!quickSession || !file || loading}>
              {loading ? <RefreshCw size={17} className="spin" /> : <Zap size={17} />} 转换
            </button>
            {lastDocx && <button className="button" onClick={() => downloadBlob(lastDocx, 'tex2doc.docx')}><ArrowDownToLine size={17} /> 再次下载</button>}
          </div>
        </div>
      </section>

      <section className="panel quick-grid__logs">
        <div className="panel__header">
          <h2>日志</h2>
          <button className="icon-button" title="清空日志" onClick={() => setLogs([])}><RefreshCw size={16} /></button>
        </div>
        <pre className="logbox">{logs.join('\n') || '等待操作...'}</pre>
      </section>
    </div>
  );
}

function UserSection({ active, session }: { active: string; session: AuthSession }) {
  const api = useMemo(() => new Tex2DocApi(session.apiBaseUrl, session.accessToken), [session]);
  if (active === 'account') return <AccountPanel session={session} />;
  if (active === 'recharge') return <RechargePanel api={api} />;
  if (active === 'conversion') return <CloudConversionPanel api={api} />;
  if (active === 'conversion-records') return <ConversionRecordsPanel api={api} />;
  if (active === 'recharge-records') return <RechargeRecordsPanel api={api} />;
  if (active === 'feedback') return <FeedbackPanel api={api} />;
  return <AboutPanel />;
}

function AdminSection({ active }: { active: string }) {
  const session = useSessionStore((s) => s.adminSession)!;
  const api = useMemo(() => new Tex2DocApi(session.apiBaseUrl, session.accessToken), [session]);
  if (active === 'dashboard') return <AdminDashboardPanel api={api} />;
  if (active === 'account') return <AccountPanel session={session} admin />;
  if (active === 'redeem-create') return <AdminRedeemCreatePanel api={api} />;
  if (active === 'redeem-batches') return <AdminRedeemBatchesPanel api={api} />;
  if (active === 'redeem-stock') return <AdminRedeemStockPanel api={api} />;
  if (active === 'feedback') return <AdminFeedbackPanel api={api} />;
  if (active === 'release') return <AdminReleasePanel api={api} />;
  if (active === 'audit') return <AdminAuditPanel api={api} />;
  if (active === 'automation') return <AdminAutomationPanel api={api} />;
  return <AboutPanel admin />;
}

function AccountPanel({ session, admin = false }: { session: AuthSession; admin?: boolean }) {
  const setUserSession = useSessionStore((s) => s.setUserSession);
  const setAdminSession = useSessionStore((s) => s.setAdminSession);
  const [usage, setUsage] = useState<UsageSummary | undefined>(session.usage);
  const [message, setMessage] = useState('');

  async function refresh() {
    try {
      setUsage(await new Tex2DocApi(session.apiBaseUrl, session.accessToken).usage());
    } catch (err) {
      setMessage(errorMessage(err));
    }
  }

  useEffect(() => {
    void refresh();
  }, [session.accessToken]);

  return (
    <section className="panel">
      <div className="panel__header">
        <div>
          <p className="eyebrow">{admin ? 'Admin Account' : 'Account'}</p>
          <h2>{session.user.email}</h2>
        </div>
        <button className="button" onClick={() => (admin ? setAdminSession(undefined) : setUserSession(undefined))}><LogOut size={17} /> 退出登录</button>
      </div>
      {message && <div className="alert alert--error">{message}</div>}
      <UsageCards usage={usage} />
    </section>
  );
}

function UsageCards({ usage }: { usage?: UsageSummary }) {
  const cards = [
    ['套餐', usage?.plan_id ?? '-'],
    ['云端转换', `${usage?.cloud_conversions_used ?? 0} / ${usage?.cloud_conversions_limit ?? '-'}`],
    ['按次余额', usage?.count_balance ?? 0],
    ['有效期', usage?.date_valid_until ?? '-'],
  ];
  return (
    <div className="metric-grid">
      {cards.map(([label, value]) => (
        <div className="metric" key={label}>
          <span>{label}</span>
          <strong>{String(value)}</strong>
        </div>
      ))}
    </div>
  );
}

function RechargePanel({ api }: { api: Tex2DocApi }) {
  const [code, setCode] = useState('');
  const [records, setRecords] = useState<RedeemCodeRecord[]>([]);
  const [message, setMessage] = useState('');

  async function load() {
    try {
      setRecords(await api.redeemCodeRecords());
    } catch (err) {
      setMessage(errorMessage(err));
    }
  }

  async function redeem() {
    try {
      const result = await api.redeemCode(code);
      setMessage(`兑换成功：${result.package_id ?? result.plan_id ?? '套餐'}，余额 ${result.count_balance ?? '-'}`);
      setCode('');
      await load();
    } catch (err) {
      setMessage(errorMessage(err));
    }
  }

  useEffect(() => {
    void load();
  }, []);

  return (
    <section className="panel">
      <div className="panel__header"><h2>充值</h2><a className="button" href="https://pay.ldxp.cn/item/ns8i2g" target="_blank" rel="noreferrer"><ShoppingCart size={17} /> 购买卡片</a></div>
      <div className="inline-form"><input placeholder="输入兑换码" value={code} onChange={(e) => setCode(e.target.value)} /><button className="button button--primary" onClick={redeem}><Check size={17} /> 提交兑换码</button></div>
      {message && <div className="alert">{message}</div>}
      <DataTable rows={records} columns={['id', 'code_preview', 'package_id', 'quantity', 'redeemed_at', 'created_at']} />
    </section>
  );
}

function CloudConversionPanel({ api }: { api: Tex2DocApi }) {
  const [file, setFile] = useState<File>();
  const [mainTex, setMainTex] = useState('main-jos.tex');
  const [log, setLog] = useState('');
  const [loading, setLoading] = useState(false);

  async function convert() {
    if (!file) return;
    setLoading(true);
    try {
      setLog('上传 ZIP...');
      const upload = await api.uploadProjectZip(file);
      setLog('创建转换任务...');
      const created = await api.createConversion(upload.upload_id, mainTex, 'jos', 'high');
      const job = await pollConversion(api, jobIdOf(created), (line) => setLog(line));
      const docx = await api.downloadConversionDocx(jobIdOf(job));
      downloadBlob(docx, `${jobIdOf(job)}.docx`);
      setLog('转换完成。');
    } catch (err) {
      setLog(errorMessage(err));
    } finally {
      setLoading(false);
    }
  }

  return (
    <section className="panel">
      <div className="panel__header"><h2>云端转换</h2><StatusBadge value={loading ? 'running' : 'ready'} /></div>
      <div className="form">
        <label>ZIP 项目<input type="file" accept=".zip" onChange={(e) => void handleCloudZipSelection(e.target.files?.[0], setFile, setMainTex)} /></label>
        <label>主 TeX<input value={mainTex} onChange={(e) => setMainTex(e.target.value)} /></label>
        <button className="button button--primary" disabled={!file || loading} onClick={convert}><CloudUpload size={17} /> 转换</button>
      </div>
      <pre className="logbox">{log || '等待上传...'}</pre>
    </section>
  );
}

function ConversionRecordsPanel({ api }: { api: Tex2DocApi }) {
  const [rows, setRows] = useState<ConversionJob[]>([]);
  const [message, setMessage] = useState('');

  async function load() {
    try {
      setRows(await api.conversions());
    } catch (err) {
      setMessage(errorMessage(err));
    }
  }

  async function download(kind: 'docx' | 'zip' | 'log', row: ConversionJob) {
    const id = jobIdOf(row);
    const blob = kind === 'docx' ? await api.downloadConversionDocx(id) : kind === 'zip' ? await api.downloadConversionZip(id) : await api.downloadConversionLog(id);
    downloadBlob(blob, `${id}.${kind}`);
  }

  useEffect(() => {
    void load();
  }, []);

  return (
    <section className="panel">
      <div className="panel__header"><h2>转换记录</h2><button className="button" onClick={load}><RefreshCw size={17} /> 刷新</button></div>
      {message && <div className="alert alert--error">{message}</div>}
      <DataTable
        rows={rows}
        columns={['job_id', 'main_tex', 'profile', 'quality', 'status', 'created_at']}
        actions={(row) => (
          <>
            <button className="icon-button" title="DOCX" onClick={() => download('docx', row)}><FileText size={16} /></button>
            <button className="icon-button" title="ZIP" onClick={() => download('zip', row)}><FileArchive size={16} /></button>
            <button className="icon-button" title="LOG" onClick={() => download('log', row)}><ClipboardList size={16} /></button>
          </>
        )}
      />
    </section>
  );
}

function RechargeRecordsPanel({ api }: { api: Tex2DocApi }) {
  const [rows, setRows] = useState<RechargeRecord[]>([]);
  useEffect(() => {
    api.recharges().then(setRows).catch(() => setRows([]));
  }, []);
  return <section className="panel"><div className="panel__header"><h2>充值记录</h2></div><DataTable rows={rows} columns={['id', 'recharge_id', 'recharge_type', 'package_id', 'amount_cents', 'provider', 'status', 'created_at']} /></section>;
}

function FeedbackPanel({ api }: { api: Tex2DocApi }) {
  const [threads, setThreads] = useState<FeedbackThread[]>([]);
  const [detail, setDetail] = useState<FeedbackThreadDetail>();
  const [title, setTitle] = useState('');
  const [content, setContent] = useState('');
  const [reply, setReply] = useState('');
  const [message, setMessage] = useState('');

  async function load() {
    try {
      setThreads(await api.feedbackThreads());
    } catch (err) {
      setMessage(errorMessage(err));
    }
  }

  async function createThread() {
    try {
      await api.createFeedbackThread({ title, content, feedbackType: 'issue', priority: 'normal' });
      setTitle('');
      setContent('');
      await load();
    } catch (err) {
      setMessage(errorMessage(err));
    }
  }

  async function open(thread: FeedbackThread) {
    setDetail(await api.feedbackThread(thread.id));
  }

  async function sendReply() {
    if (!detail) return;
    await api.addFeedbackMessage(detail.id, reply);
    setReply('');
    setDetail(await api.feedbackThread(detail.id));
  }

  useEffect(() => {
    void load();
  }, []);

  return (
    <div className="two-col">
      <section className="panel">
        <div className="panel__header"><h2>反馈</h2><button className="button" onClick={load}><RefreshCw size={17} /> 刷新</button></div>
        <div className="form">
          <input placeholder="标题" value={title} maxLength={100} onChange={(e) => setTitle(e.target.value)} />
          <textarea placeholder="描述" value={content} onChange={(e) => setContent(e.target.value)} />
          <button className="button button--primary" disabled={!title || !content} onClick={createThread}><Send size={17} /> 新建反馈</button>
        </div>
        {message && <div className="alert alert--error">{message}</div>}
        <div className="list">{threads.map((thread) => <button className="list-row" key={thread.id} onClick={() => open(thread)}><span>{thread.title}</span><StatusBadge value={thread.status ?? 'open'} /></button>)}</div>
      </section>
      <section className="panel">
        <div className="panel__header"><h2>会话详情</h2></div>
        {detail ? (
          <>
            <div className="messages">{(detail.messages ?? []).map((m, i) => <div className="message" key={m.id ?? i}><strong>{m.author_role ?? 'user'}</strong><p>{m.content}</p></div>)}</div>
            <div className="inline-form"><input value={reply} onChange={(e) => setReply(e.target.value)} placeholder="继续回复" /><button className="button" onClick={sendReply}><Send size={17} /> 发送</button></div>
          </>
        ) : <StateBox label="请选择一个反馈会话。" />}
      </section>
    </div>
  );
}

function AdminDashboardPanel({ api }: { api: Tex2DocApi }) {
  const [data, setData] = useState<AdminDashboardSummary>();
  const [message, setMessage] = useState('');

  async function load() {
    try {
      setData(await api.adminDashboard());
    } catch (err) {
      setMessage(errorMessage(err));
    }
  }

  useEffect(() => {
    void load();
  }, []);

  const counts = data?.counts ?? {};
  return (
    <section className="panel">
      <div className="panel__header"><h2>管理端仪表盘</h2><button className="button" onClick={load}><RefreshCw size={17} /> 刷新</button></div>
      {message && <div className="alert alert--error">{message}</div>}
      <div className="metric-grid">{Object.entries(counts).map(([k, v]) => <div className="metric" key={k}><span>{k}</span><strong>{v}</strong></div>)}</div>
      <div className="tag-row">{(data?.modules ?? []).map((m) => <span className="tag" key={m}>{m}</span>)}</div>
    </section>
  );
}

function AdminRedeemCreatePanel({ api }: { api: Tex2DocApi }) {
  const [packageId, setPackageId] = useState('count_3');
  const [quantity, setQuantity] = useState(10);
  const [channel, setChannel] = useState('web');
  const [note, setNote] = useState('');
  const [batch, setBatch] = useState<RedeemCodeBatch>();
  const [message, setMessage] = useState('');
  const [busy, setBusy] = useState(false);

  async function submit() {
    setBusy(true);
    try {
      const created = await api.createRedeemCodeBatch({ packageId, quantity, channel, note });
      setBatch(created);
      const blob = await api.exportRedeemCodeBatch(created.id);
      downloadBlob(blob, `redeem-codes-${created.batch_no ?? created.id}.xlsx`);
      setMessage('批次已生成，Excel 已开始下载。');
    } catch (err) {
      setMessage(errorMessage(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="panel">
      <div className="panel__header"><h2>兑换码生成</h2></div>
      <div className="form-row">
        <label>套餐<select value={packageId} onChange={(e) => setPackageId(e.target.value)}><option value="count_3">count_3</option><option value="count_10">count_10</option><option value="count_30">count_30</option></select></label>
        <label>数量<input type="number" min={1} value={quantity} onChange={(e) => setQuantity(Number(e.target.value))} /></label>
        <label>渠道<input value={channel} onChange={(e) => setChannel(e.target.value)} /></label>
      </div>
      <textarea placeholder="备注" value={note} onChange={(e) => setNote(e.target.value)} />
      <button className="button button--primary" disabled={busy} onClick={submit}><PackagePlus size={17} /> {busy ? '生成中...' : '生成并下载 Excel'}</button>
      {message && <div className="alert">{message}</div>}
      {batch && <DataTable rows={[batch]} columns={['id', 'batch_no', 'package_id', 'quantity', 'generated_count', 'status', 'created_at']} />}
    </section>
  );
}

function AdminRedeemBatchesPanel({ api }: { api: Tex2DocApi }) {
  const locale = useSessionStore((s) => s.locale);
  const [rows, setRows] = useState<RedeemCodeBatch[]>([]);
  const [detail, setDetail] = useState<RedeemCodeBatch>();
  const [batchNo, setBatchNo] = useState('');
  const [packageId, setPackageId] = useState('');
  const [channel, setChannel] = useState('');
  const [createdFrom, setCreatedFrom] = useState('');
  const [createdTo, setCreatedTo] = useState('');
  const [status, setStatus] = useState('');
  const [filters, setFilters] = useState({
    batchNo: '',
    packageId: '',
    channel: '',
    createdFrom: '',
    createdTo: '',
    status: '',
  });
  const [page, setPage] = useState(1);
  const [message, setMessage] = useState('');
  const pageSize = 10;
  const text = batchText[locale];

  async function load() {
    try {
      const nextRows = await api.redeemCodeBatches();
      setRows(nextRows);
      if (!detail && nextRows[0]) {
        await openDetail(nextRows[0].id);
      }
      setMessage('');
    } catch (err) {
      setMessage(errorMessage(err));
    }
  }

  async function openDetail(batchId: string) {
    try {
      setDetail(await api.redeemCodeBatchDetail(batchId));
      setMessage('');
    } catch (err) {
      setMessage(errorMessage(err));
    }
  }

  async function downloadBatch(batch: RedeemCodeBatch) {
    try {
      downloadBlob(await api.exportRedeemCodeBatch(batch.id), `redeem-codes-${batch.batch_no ?? batch.id}.xlsx`);
      setMessage(text.batchDownloadStarted);
    } catch (err) {
      setMessage(errorMessage(err));
    }
  }

  function applyFilters() {
    setFilters({ batchNo, packageId, channel, createdFrom, createdTo, status });
    setPage(1);
  }

  function resetFilters() {
    setBatchNo('');
    setPackageId('');
    setChannel('');
    setCreatedFrom('');
    setCreatedTo('');
    setStatus('');
    setFilters({ batchNo: '', packageId: '', channel: '', createdFrom: '', createdTo: '', status: '' });
    setPage(1);
  }

  useEffect(() => {
    void load();
  }, []);

  const packageOptions = uniqueSorted(rows.map((row) => row.package_id));
  const channelOptions = uniqueSorted(rows.map((row) => row.channel));
  const filteredRows = rows.filter((row) => {
    const createdAt = dateMs(row.created_at);
    const from = filters.createdFrom ? new Date(`${filters.createdFrom}T00:00:00`).getTime() : undefined;
    const to = filters.createdTo ? new Date(`${filters.createdTo}T23:59:59`).getTime() : undefined;
    return (
      (!filters.batchNo.trim() || String(row.batch_no ?? '').toLowerCase().includes(filters.batchNo.trim().toLowerCase())) &&
      (!filters.packageId || row.package_id === filters.packageId) &&
      (!filters.channel || row.channel === filters.channel) &&
      (!filters.status || row.status === filters.status) &&
      (from === undefined || (createdAt !== undefined && createdAt >= from)) &&
      (to === undefined || (createdAt !== undefined && createdAt <= to))
    );
  });
  const maxPage = Math.max(1, Math.ceil(filteredRows.length / pageSize));
  const safePage = Math.min(page, maxPage);
  const pageRows = filteredRows.slice((safePage - 1) * pageSize, safePage * pageSize);
  const batchColumns: EnhancedColumn<RedeemCodeBatch>[] = [
    { key: 'batch_no', label: text.columns.batchNo },
    { key: 'package_id', label: text.columns.packageId },
    { key: 'quantity', label: text.columns.quantity },
    { key: 'generated_count', label: text.columns.generatedCount },
    { key: 'exported_count', label: text.columns.exportedCount },
    { key: 'status', label: text.columns.status },
    { key: 'channel', label: text.columns.channel },
    { key: 'created_at', label: text.columns.createdAt, render: (row) => formatDateTime(row.created_at, locale) },
  ];
  const codeRows = (detail?.codes ?? []).map((code, index) => {
    const raw = typeof code === 'string' ? { code } : code;
    return {
      id: raw.id ?? raw.code_id ?? raw.code ?? String(index + 1),
      index: index + 1,
      code_preview: raw.code_preview ?? raw.code ?? '',
      code: raw.code ?? raw.code_preview ?? raw.code_id ?? '',
    };
  });

  function exportFilteredRows() {
    downloadCsv(filteredRows, batchColumns, `redeem-batches-${formatDateForFile(new Date())}.csv`);
    setMessage(text.listExportStarted);
  }

  return (
    <div className="two-col">
      <section className="panel">
        <div className="panel__header">
          <h2>{text.batchTableTitle}</h2>
          <div className="row-actions">
            <button className="button" onClick={exportFilteredRows}><ArrowDownToLine size={17} /> {text.exportList}</button>
            <button className="button" onClick={load}><RefreshCw size={17} /> {text.refresh}</button>
          </div>
        </div>
        <div className="filter-grid">
          <label>{text.filters.package}<select value={packageId} onChange={(e) => setPackageId(e.target.value)}>
            <option value="">{text.all}</option>
            {packageOptions.map((option) => <option value={option} key={option}>{option}</option>)}
          </select></label>
          <label>{text.filters.channel}<select value={channel} onChange={(e) => setChannel(e.target.value)}>
            <option value="">{text.all}</option>
            {channelOptions.map((option) => <option value={option} key={option}>{option}</option>)}
          </select></label>
          <label>{text.filters.batchNo}<input value={batchNo} onChange={(e) => setBatchNo(e.target.value)} /></label>
          <label>{text.filters.createdFrom}<input type="date" value={createdFrom} onChange={(e) => setCreatedFrom(e.target.value)} /></label>
          <label>{text.filters.createdTo}<input type="date" value={createdTo} onChange={(e) => setCreatedTo(e.target.value)} /></label>
          <label>{text.filters.status}<select value={status} onChange={(e) => setStatus(e.target.value)}>
            <option value="">{text.all}</option>
            <option value="active">active</option>
            <option value="voided">voided</option>
            <option value="expired">expired</option>
          </select></label>
        </div>
        <div className="toolbar">
          <button className="button button--primary" onClick={applyFilters}><Search size={17} /> {text.query}</button>
          <button className="button" onClick={resetFilters}>{text.reset}</button>
        </div>
        {message && <div className="alert">{message}</div>}
        <EnhancedDataTable
          rows={pageRows}
          columns={batchColumns}
          getRowId={(row) => row.id}
          selectedId={detail?.id}
          onSelectRow={(row) => void openDetail(row.id)}
          actions={(row) => (
            <>
              <button className="icon-button" title={text.detail} onClick={() => void openDetail(row.id)}><Search size={16} /></button>
              <button className="icon-button" title={text.downloadExcel} onClick={() => void downloadBatch(row)}><ArrowDownToLine size={16} /></button>
            </>
          )}
        />
        <div className="table-footer">
          <span className="muted">{text.total(filteredRows.length, safePage, maxPage)}</span>
          <button className="button" disabled={safePage <= 1} onClick={() => setPage(safePage - 1)}>{text.prev}</button>
          <button className="button" disabled={safePage >= maxPage} onClick={() => setPage(safePage + 1)}>{text.next}</button>
        </div>
      </section>
      <section className="panel">
        <div className="panel__header"><h2>{text.detailTitle}</h2>{detail && <button className="button" onClick={() => void downloadBatch(detail)}><ArrowDownToLine size={17} /> {text.downloadExcel}</button>}</div>
        {detail ? (
          <>
            <DataTable rows={[detail]} columns={['id', 'batch_no', 'package_id', 'quantity', 'generated_count', 'exported_count', 'status', 'note']} />
            <h3>{text.generatedCodes}</h3>
            <DataTable rows={codeRows} columns={['index', 'code_preview', 'code']} />
          </>
        ) : <StateBox label={text.pickBatch} />}
      </section>
    </div>
  );
}

function AdminRedeemStockPanel({ api }: { api: Tex2DocApi }) {
  const [status, setStatus] = useState('');
  const [batchId, setBatchId] = useState('');
  const [packageId, setPackageId] = useState('');
  const [search, setSearch] = useState('');
  const [rows, setRows] = useState<AdminRedeemCode[]>([]);
  const [selected, setSelected] = useState<string[]>([]);
  const [restock, setRestock] = useState('');
  const [message, setMessage] = useState('');
  const [page, setPage] = useState(1);
  const [total, setTotal] = useState(0);

  async function load(nextPage = page) {
    try {
      const result = await api.adminListRedeemCodes({ stockStatus: status, batchId, packageId, search, page: nextPage, pageSize: 50 });
      setRows(result.items ?? result.codes ?? result.records ?? []);
      setTotal(result.total ?? 0);
      setPage(result.page ?? nextPage);
      setSelected([]);
      setMessage('');
    } catch (err) {
      setMessage(errorMessage(err));
    }
  }

  async function bulkStock() {
    try {
      const result = await api.adminBulkStockRedeemCodes(selected);
      setMessage(`已上货 ${result.affected} 条。`);
      await load();
    } catch (err) {
      setMessage(errorMessage(err));
    }
  }

  async function doRestock() {
    try {
      const result = await api.adminRestockRedeemCodes(restock);
      setMessage(`已重置 ${result.affected} 条。`);
      setRestock('');
      await load(1);
    } catch (err) {
      setMessage(errorMessage(err));
    }
  }

  async function exportCodes() {
    try {
      downloadBlob(await api.adminExportRedeemCodesExcel({ stockStatus: status, batchId, packageId, search }), 'redeem-codes-list.xlsx');
      setMessage('库存 Excel 已开始下载。');
    } catch (err) {
      setMessage(errorMessage(err));
    }
  }

  useEffect(() => {
    void load(1);
  }, [status, batchId, packageId]);

  return (
    <section className="panel">
      <div className="panel__header"><h2>兑换码库存</h2><button className="button" onClick={() => void load()}><RefreshCw size={17} /> 刷新</button></div>
      <div className="toolbar">
        <Segmented value={status} onChange={setStatus} items={[['', '全部'], ['new', 'new'], ['stocked', 'stocked'], ['redeemed', 'redeemed'], ['restocked', 'restocked']]} />
        <input placeholder="搜索" value={search} onChange={(e) => setSearch(e.target.value)} onKeyDown={(e) => e.key === 'Enter' && void load()} />
        <input placeholder="批次 ID" value={batchId} onChange={(e) => setBatchId(e.target.value)} />
        <input placeholder="套餐 ID" value={packageId} onChange={(e) => setPackageId(e.target.value)} />
        <button className="button" onClick={() => void load(1)}><Search size={17} /> 检索</button>
        <button className="button" disabled={!selected.length} onClick={bulkStock}><Check size={17} /> 批量上货</button>
        <button className="button" onClick={exportCodes}><ArrowDownToLine size={17} /> 导出 Excel</button>
      </div>
      {message && <div className="alert">{message}</div>}
      <DataTable rows={rows} columns={['code_id', 'batch_no', 'code_preview', 'package_id', 'stock_status', 'stocked_at', 'redeemed_at', 'restocked_at', 'created_at']} select={{ selected, onSelected: setSelected }} />
      <div className="toolbar toolbar--compact">
        <span className="muted">共 {total} 条，第 {page} 页</span>
        <button className="button" disabled={page <= 1} onClick={() => void load(page - 1)}>上一页</button>
        <button className="button" disabled={rows.length < 50 || page * 50 >= total} onClick={() => void load(page + 1)}>下一页</button>
      </div>
      <div className="inline-form"><textarea placeholder="按行粘贴明文兑换码用于导入重置" value={restock} onChange={(e) => setRestock(e.target.value)} /><button className="button" onClick={doRestock}>导入重置</button></div>
    </section>
  );
}

function AdminFeedbackPanel({ api }: { api: Tex2DocApi }) {
  const [threads, setThreads] = useState<FeedbackThread[]>([]);
  const [reply, setReply] = useState<Record<string, string>>({});

  async function load() {
    setThreads(await api.adminFeedbackThreads());
  }

  useEffect(() => {
    void load();
  }, []);

  return (
    <section className="panel">
      <div className="panel__header"><h2>Feedback management</h2><button className="button" onClick={load}><RefreshCw size={17} /> 刷新</button></div>
      <div className="list">
        {threads.map((thread) => (
          <div className="card-row" key={thread.id}>
            <div><strong>{thread.title}</strong><p>{thread.feedback_type} · {thread.priority} · {thread.message_count ?? 0} messages</p></div>
            <select value={thread.status ?? 'open'} onChange={async (e) => { await api.adminUpdateFeedbackThread(thread.id, e.target.value); await load(); }}>
              <option value="open">open</option><option value="in_progress">in_progress</option><option value="resolved">resolved</option><option value="closed">closed</option>
            </select>
            <input placeholder="回复用户" value={reply[thread.id] ?? ''} onChange={(e) => setReply({ ...reply, [thread.id]: e.target.value })} />
            <button className="icon-button" title="发送" onClick={async () => { await api.adminReplyFeedbackThread(thread.id, reply[thread.id] ?? ''); setReply({ ...reply, [thread.id]: '' }); await load(); }}><Send size={16} /></button>
          </div>
        ))}
      </div>
    </section>
  );
}

function AdminReleasePanel({ api }: { api: Tex2DocApi }) {
  const [rows, setRows] = useState<ReleaseManifest[]>([]);
  const [form, setForm] = useState({ channel: 'beta', platform: 'windows', arch: 'x64', version: '', releaseTitle: '', downloadUrl: '', sha256: '' });
  const [message, setMessage] = useState('');

  async function load() {
    setRows(await api.adminReleases());
  }

  async function publish() {
    if (!form.version || !form.downloadUrl || !form.sha256) {
      setMessage('版本、下载地址和 SHA-256 必填。');
      return;
    }
    await api.adminPublishRelease(form);
    setMessage('发布清单已写入。');
    await load();
  }

  useEffect(() => {
    void load();
  }, []);

  return (
    <section className="panel">
      <div className="panel__header"><h2>发布管理</h2><button className="button" onClick={load}><RefreshCw size={17} /> 刷新</button></div>
      <div className="form-row">{(['channel', 'platform', 'arch', 'version'] as const).map((key) => <label key={key}>{key}<input value={form[key]} onChange={(e) => setForm({ ...form, [key]: e.target.value })} /></label>)}</div>
      <div className="form-row"><label>标题<input value={form.releaseTitle} onChange={(e) => setForm({ ...form, releaseTitle: e.target.value })} /></label><label>下载地址<input value={form.downloadUrl} onChange={(e) => setForm({ ...form, downloadUrl: e.target.value })} /></label><label>SHA-256<input value={form.sha256} onChange={(e) => setForm({ ...form, sha256: e.target.value })} /></label></div>
      <button className="button button--primary" onClick={publish}><Rocket size={17} /> 发布清单</button>
      {message && <div className="alert">{message}</div>}
      <DataTable rows={rows} columns={['channel', 'platform', 'arch', 'version', 'release_title', 'active', 'published_at', 'rolled_back_at']} actions={(row) => row.active !== false && <button className="button" onClick={async () => { await api.adminRollbackRelease(row.id!); await load(); }}>回滚</button>} />
    </section>
  );
}

function AdminAuditPanel({ api }: { api: Tex2DocApi }) {
  const [rows, setRows] = useState<Array<Record<string, unknown>>>([]);
  useEffect(() => {
    api.adminReleaseAudit().then(setRows).catch(() => setRows([]));
  }, []);
  return <section className="panel"><div className="panel__header"><h2>审计中心</h2></div><DataTable rows={rows} columns={['action', 'target_release_id', 'actor', 'details', 'created_at']} /></section>;
}

function AdminAutomationPanel({ api }: { api: Tex2DocApi }) {
  const [summary, setSummary] = useState<Record<string, number>>({});
  const [requests, setRequests] = useState<AutomationRequest[]>([]);
  const [agents, setAgents] = useState<AutomationAgent[]>([]);
  const [events, setEvents] = useState<AutomationEvent[]>([]);
  const [selected, setSelected] = useState<AutomationRequest>();
  const [status, setStatus] = useState('');
  const [risk, setRisk] = useState('');
  const [autoRefresh, setAutoRefresh] = useState(false);

  async function load() {
    const query: Record<string, string> = {};
    if (status) query.status = status;
    if (risk) query.risk_level = risk;
    const [nextSummary, nextRequests, nextAgents] = await Promise.all([
      api.adminAutomationSummary(),
      api.adminAutomationRequests(query),
      api.adminAutomationAgents(),
    ]);
    setSummary(nextSummary);
    setRequests(nextRequests);
    setAgents(nextAgents);
  }

  async function open(req: AutomationRequest) {
    setSelected(await api.adminAutomationRequest(req.id));
    setEvents(await api.adminAutomationEvents(req.id));
  }

  useEffect(() => {
    void load();
  }, [status, risk]);

  useEffect(() => {
    if (!autoRefresh) return;
    const id = window.setInterval(() => void load(), 10000);
    return () => window.clearInterval(id);
  }, [autoRefresh, status, risk]);

  const canApprove = selected && ['triaged', 'needs_approval'].includes(selected.status ?? '') && !['high', 'critical'].includes(selected.risk_level ?? '');

  return (
    <div className="automation">
      <section className="panel">
        <div className="panel__header"><h2>自动化</h2><button className="button" onClick={() => setAutoRefresh(!autoRefresh)}><RefreshCw size={17} /> Auto-refresh</button></div>
        <div className="metric-grid">{Object.entries(summary).map(([k, v]) => <div className="metric" key={k}><span>{k}</span><strong>{v}</strong></div>)}</div>
        <div className="toolbar"><select value={status} onChange={(e) => setStatus(e.target.value)}><option value="">All status</option><option value="triaged">Triaged</option><option value="needs_approval">Needs Approval</option><option value="local_failed">Local Failed</option><option value="ci_failed">CI Failed</option><option value="deployed">Deployed</option></select><select value={risk} onChange={(e) => setRisk(e.target.value)}><option value="">All risk</option><option value="low">Low</option><option value="medium">Medium</option><option value="high">High</option><option value="critical">Critical</option></select></div>
        <div className="list">{requests.map((r) => <button className="list-row" key={r.id} onClick={() => open(r)}><span>{r.short_id ?? r.id} · {r.title}</span><StatusBadge value={r.risk_level ?? 'unknown'} /></button>)}</div>
      </section>
      <section className="panel">
        <div className="panel__header"><h2>请求详情</h2></div>
        {selected ? <>
          <h3>{selected.short_id ?? selected.id} · {selected.title}</h3>
          <p>{selected.ai_summary}</p>
          <div className="actions">
            {canApprove && <button className="button" onClick={async () => { await api.adminAutomationApprove(selected.id); await open(selected); }}><Check size={17} /> Approve</button>}
            <button className="button" onClick={async () => { const reason = window.prompt('Reject reason') ?? ''; if (reason) { await api.adminAutomationReject(selected.id, reason); await open(selected); } }}>Reject</button>
            <button className="button" onClick={async () => { await api.adminAutomationRetry(selected.id); await open(selected); }}>Retry</button>
            <button className="button" onClick={async () => { const assignee = window.prompt('Assignee') ?? ''; if (assignee) { await api.adminAutomationEscalate(selected.id, assignee); await open(selected); } }}>Escalate</button>
          </div>
          <DataTable rows={events} columns={['event_type', 'message', 'created_at']} />
        </> : <StateBox label="选择请求查看详情。" />}
      </section>
      <section className="panel">
        <div className="panel__header"><h2>Agents</h2></div>
        <DataTable rows={agents} columns={['id', 'status', 'hostname', 'agent_version', 'last_heartbeat_at', 'completed_count', 'failed_count']} actions={(agent) => <button className="button" onClick={async () => { agent.status === 'paused' ? await api.adminAutomationResumeAgent(agent.id) : await api.adminAutomationPauseAgent(agent.id); await load(); }}>{agent.status === 'paused' ? 'Resume' : 'Pause'}</button>} />
      </section>
    </div>
  );
}

function AboutPanel({ admin = false }: { admin?: boolean }) {
  return (
    <section className="panel">
      <div className="panel__header"><h2>关于</h2></div>
      <p>Tex2Doc React Web 版本覆盖 Flutter {admin ? '管理端' : '用户端'}核心能力，保留 Flutter 作为验收基线与回滚备份。</p>
      <div className="tag-row"><span className="tag">React</span><span className="tag">TypeScript</span><span className="tag">Vite</span><span className="tag">WASM lazy loading</span></div>
    </section>
  );
}

type EnhancedColumn<T> = {
  key: keyof T | string;
  label: string;
  render?: (row: T) => React.ReactNode;
};

function EnhancedDataTable<T extends object>({
  rows,
  columns,
  getRowId,
  selectedId,
  onSelectRow,
  actions,
}: {
  rows: T[];
  columns: Array<EnhancedColumn<T>>;
  getRowId: (row: T) => string;
  selectedId?: string;
  onSelectRow?: (row: T) => void;
  actions?: (row: T) => React.ReactNode;
}) {
  return (
    <div className="table-wrap table-wrap--enhanced">
      <table>
        <thead>
          <tr>
            {onSelectRow && <th>选择</th>}
            {columns.map((column) => <th key={String(column.key)}>{column.label}</th>)}
            {actions && <th>操作</th>}
          </tr>
        </thead>
        <tbody>
          {rows.map((row) => {
            const id = getRowId(row);
            const selected = selectedId === id;
            return (
              <tr key={id} className={selected ? 'is-selected' : ''} onClick={() => onSelectRow?.(row)}>
                {onSelectRow && <td><input type="checkbox" checked={selected} onChange={() => onSelectRow(row)} onClick={(event) => event.stopPropagation()} /></td>}
                {columns.map((column) => (
                  <td key={String(column.key)}>{column.render ? column.render(row) : formatCell((row as Record<string, unknown>)[String(column.key)])}</td>
                ))}
                {actions && <td onClick={(event) => event.stopPropagation()}><div className="row-actions">{actions(row)}</div></td>}
              </tr>
            );
          })}
        </tbody>
      </table>
      {!rows.length && <StateBox label="暂无数据。" />}
    </div>
  );
}

function DataTable<T extends { id?: string } | Record<string, unknown>>({
  rows,
  columns,
  actions,
  select,
}: {
  rows: T[];
  columns: string[];
  actions?: (row: T) => React.ReactNode;
  select?: { selected: string[]; onSelected: (ids: string[]) => void };
}) {
  return (
    <div className="table-wrap">
      <table>
        <thead><tr>{select && <th><input type="checkbox" checked={rows.length > 0 && select.selected.length === rows.length} onChange={(e) => select.onSelected(e.target.checked ? rows.map((r) => String((r as { id?: string }).id)) : [])} /></th>}{columns.map((c) => <th key={c}>{c}</th>)}{actions && <th>操作</th>}</tr></thead>
        <tbody>
          {rows.map((row, index) => {
            const id = String((row as { id?: string }).id ?? index);
            return (
              <tr key={id}>
                {select && <td><input type="checkbox" checked={select.selected.includes(id)} onChange={(e) => select.onSelected(e.target.checked ? [...select.selected, id] : select.selected.filter((x) => x !== id))} /></td>}
                {columns.map((c) => <td key={c}>{formatCell((row as Record<string, unknown>)[c])}</td>)}
                {actions && <td><div className="row-actions">{actions(row)}</div></td>}
              </tr>
            );
          })}
        </tbody>
      </table>
      {!rows.length && <StateBox label="暂无数据。" />}
    </div>
  );
}

function Segmented({ value, onChange, items }: { value: string; onChange: (value: string) => void; items: Array<[string, string]> }) {
  return <div className="segmented">{items.map(([id, label]) => <button key={id} className={id === value ? 'is-active' : ''} onClick={() => onChange(id)}>{label}</button>)}</div>;
}

function StatusBadge({ value }: { value: string }) {
  return <span className={`status status--${value.replace(/[^a-z0-9_-]/gi, '-')}`}>{value}</span>;
}

function StateBox({ label }: { label: string }) {
  return <div className="state-box">{label}</div>;
}

function formatCell(value: unknown): string {
  if (value === undefined || value === null || value === '') return '-';
  if (typeof value === 'object') return JSON.stringify(value);
  return String(value);
}

function formatDateTime(value: unknown, locale: 'zh-CN' | 'en-US' = 'zh-CN'): string {
  const ms = dateMs(value);
  if (ms === undefined) return formatCell(value);
  const date = new Date(ms);
  const pad = (part: number) => String(part).padStart(2, '0');
  if (locale === 'en-US') {
    return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`;
  }
  return `${date.getFullYear()}年${pad(date.getMonth() + 1)}月${pad(date.getDate())}日 ${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`;
}

function dateMs(value: unknown): number | undefined {
  if (value === undefined || value === null || value === '') return undefined;
  if (typeof value === 'number') return value < 10_000_000_000 ? value * 1000 : value;
  const text = String(value);
  if (/^\d+$/.test(text)) {
    const secondsOrMs = Number(text);
    return secondsOrMs < 10_000_000_000 ? secondsOrMs * 1000 : secondsOrMs;
  }
  const parsed = Date.parse(text);
  return Number.isNaN(parsed) ? undefined : parsed;
}

function uniqueSorted(values: Array<string | null | undefined>): string[] {
  return Array.from(new Set(values.filter((value): value is string => !!value))).sort((a, b) => a.localeCompare(b));
}

function formatDateForFile(date: Date): string {
  const pad = (part: number) => String(part).padStart(2, '0');
  return `${date.getFullYear()}${pad(date.getMonth() + 1)}${pad(date.getDate())}-${pad(date.getHours())}${pad(date.getMinutes())}${pad(date.getSeconds())}`;
}

function downloadCsv<T extends object>(rows: T[], columns: Array<EnhancedColumn<T>>, fileName: string): void {
  const escape = (value: unknown) => `"${String(value ?? '').replace(/"/g, '""')}"`;
  const lines = [
    columns.map((column) => escape(column.label)).join(','),
    ...rows.map((row) => columns.map((column) => escape(column.render ? column.render(row) : (row as Record<string, unknown>)[String(column.key)])).join(',')),
  ];
  downloadBlob(new Blob([`\uFEFF${lines.join('\r\n')}`], { type: 'text/csv;charset=utf-8' }), fileName);
}

const batchText = {
  'zh-CN': {
    batchTableTitle: '兑换码批次表格',
    detailTitle: '批次详细信息',
    generatedCodes: '生成的兑换码列表',
    refresh: '刷新',
    exportList: '导出列表',
    query: '检索',
    reset: '重置',
    all: '全部',
    detail: '详情',
    downloadExcel: '下载 Excel',
    batchDownloadStarted: '批次 Excel 已开始下载。',
    listExportStarted: '批次列表已开始导出。',
    pickBatch: '选择批次查看详情。',
    prev: '上一页',
    next: '下一页',
    total: (count: number, page: number, maxPage: number) => `共 ${count} 条，第 ${page} / ${maxPage} 页`,
    filters: {
      package: '套餐',
      channel: '渠道',
      batchNo: '批次号',
      createdFrom: '生成日期起',
      createdTo: '生成日期止',
      status: '状态',
    },
    columns: {
      batchNo: '批次号',
      packageId: '套餐',
      quantity: '数量',
      generatedCount: '生成数',
      exportedCount: '已导出',
      status: '状态',
      channel: '渠道',
      createdAt: '生成时间',
    },
  },
  'en-US': {
    batchTableTitle: 'Redeem Code Batches',
    detailTitle: 'Batch Details',
    generatedCodes: 'Generated Redeem Codes',
    refresh: 'Refresh',
    exportList: 'Export List',
    query: 'Search',
    reset: 'Reset',
    all: 'All',
    detail: 'Details',
    downloadExcel: 'Download Excel',
    batchDownloadStarted: 'Batch Excel download has started.',
    listExportStarted: 'Batch list export has started.',
    pickBatch: 'Select a batch to view details.',
    prev: 'Previous',
    next: 'Next',
    total: (count: number, page: number, maxPage: number) => `${count} total, page ${page} / ${maxPage}`,
    filters: {
      package: 'Package',
      channel: 'Channel',
      batchNo: 'Batch No.',
      createdFrom: 'Created From',
      createdTo: 'Created To',
      status: 'Status',
    },
    columns: {
      batchNo: 'Batch No.',
      packageId: 'Package',
      quantity: 'Quantity',
      generatedCount: 'Generated',
      exportedCount: 'Exported',
      status: 'Status',
      channel: 'Channel',
      createdAt: 'Created At',
    },
  },
};

function errorMessage(error: unknown): string {
  if (error instanceof ApiError) return `HTTP ${error.status}: ${error.message}`;
  if (error instanceof Error) return error.message;
  if (typeof error === 'object' && error !== null && 'message' in error) {
    return String((error as { message?: unknown }).message);
  }
  return '操作失败';
}

function jobIdOf(job: ConversionJob): string {
  return String(job.job_id ?? job.id ?? '');
}

async function pollConversion(api: Tex2DocApi, jobId: string, log: (line: string) => void): Promise<ConversionJob> {
  for (let i = 0; i < 120; i += 1) {
    const job = await api.getConversion(jobId);
    log(`轮询 ${i + 1}/120：${job.status}`);
    if (job.status === 'completed') return job;
    if (job.status === 'failed' || job.status === 'expired') {
      throw new Error(job.error_message ?? job.error_code ?? job.status);
    }
    await new Promise((resolve) => window.setTimeout(resolve, 1000));
  }
  throw new Error('云端转换轮询超时。');
}

async function handleQuickZipSelection(
  nextFile: File | undefined,
  setFile: (file: File | undefined) => void,
  setMainTex: (mainTex: string) => void,
  log: (line: string) => void,
): Promise<void> {
  setFile(nextFile);
  if (!nextFile) return;
  try {
    const detected = await detectMainTex(nextFile);
    if (detected) {
      setMainTex(detected);
      log(`已自动识别主 TeX：${detected}`);
    } else {
      log('未能自动识别主 TeX，请手动填写 ZIP 内路径。');
    }
  } catch (error) {
    log(`读取 ZIP 文件列表失败：${errorMessage(error)}`);
  }
}

async function handleCloudZipSelection(
  nextFile: File | undefined,
  setFile: (file: File | undefined) => void,
  setMainTex: (mainTex: string) => void,
): Promise<void> {
  setFile(nextFile);
  if (!nextFile) return;
  const detected = await detectMainTex(nextFile).catch(() => undefined);
  if (detected) {
    setMainTex(detected);
  }
}

async function detectMainTex(file: File): Promise<string | undefined> {
  const bytes = new Uint8Array(await file.arrayBuffer());
  const entries = unzipSync(bytes, { filter: (fileInfo) => fileInfo.name.toLowerCase().endsWith('.tex') });
  const names = Object.keys(entries)
    .filter((name) => !name.endsWith('/') && name.toLowerCase().endsWith('.tex'))
    .map((name) => name.replace(/\\/g, '/'));
  const preferred = ['main-jos.tex', 'main.tex', 'main-zh.tex'];
  for (const item of preferred) {
    const exact = names.find((name) => name.toLowerCase() === item);
    if (exact) return exact;
  }
  return names.find((name) => /(^|\/)main[^/]*\.tex$/i.test(name)) ?? names[0];
}
