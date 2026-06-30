import { FormEvent, useEffect, useMemo, useState } from 'react';
import { Link, Navigate, Outlet, Route, Routes, useLocation, useNavigate } from 'react-router-dom';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import {
  Alert,
  App as AntApp,
  Badge,
  Button,
  Card,
  Col,
  Collapse,
  ConfigProvider,
  Descriptions,
  Drawer,
  Empty,
  Flex,
  Form,
  Grid,
  Input,
  InputNumber,
  Layout,
  List,
  Menu,
  Modal,
  Progress,
  Result,
  Row,
  Segmented,
  Select,
  Space,
  Spin,
  Statistic,
  Steps,
  Table,
  Tabs,
  Tag,
  Tooltip,
  Typography,
  Upload,
  theme as antdTheme,
} from 'antd';
import type { ColumnsType } from 'antd/es/table';
import type { UploadProps } from 'antd';
import zhCN from 'antd/locale/zh_CN';
import enUS from 'antd/locale/en_US';
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
  UploadCloud,
  User,
  Zap,
} from 'lucide-react';
import { unzipSync } from 'fflate';
import { Tex2DocApi } from '../api/client';
import { ApiError, defaultApiBaseUrl, downloadBlob } from '../api/http';
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
import { AuthSession, initialApiBaseUrl, storedQuickCode, useSessionStore } from '../stores/session';
import { convertZipToDocx } from '../wasm/doc-engine';
import { copyText, fieldLabel, messages, statusLabel, type CopyKey, type Locale, type Messages } from '../i18n/messages';

const { Header, Sider, Content } = Layout;
const { Title, Text, Paragraph } = Typography;

type NavItem = { key: string; labelKey: keyof Messages['nav']; icon: React.ReactNode; path: string; group?: string };
type StatusKind = 'success' | 'processing' | 'warning' | 'error' | 'default';

const userNav: NavItem[] = [
  { key: 'quick', labelKey: 'quick', icon: <Zap size={16} />, path: '/app-react/quick' },
  { key: 'account', labelKey: 'account', icon: <User size={16} />, path: '/app-react/account' },
  { key: 'recharge', labelKey: 'recharge', icon: <ShoppingCart size={16} />, path: '/app-react/recharge' },
  { key: 'convert', labelKey: 'convert', icon: <CloudUpload size={16} />, path: '/app-react/convert' },
  { key: 'jobs', labelKey: 'jobs', icon: <ClipboardList size={16} />, path: '/app-react/jobs' },
  { key: 'billing', labelKey: 'billing', icon: <History size={16} />, path: '/app-react/billing' },
  { key: 'feedback', labelKey: 'feedback', icon: <MessageSquare size={16} />, path: '/app-react/feedback' },
  { key: 'settings', labelKey: 'settings', icon: <Settings size={16} />, path: '/app-react/settings' },
];

const adminNav: NavItem[] = [
  { key: 'dashboard', labelKey: 'adminDashboard', icon: <Activity size={16} />, path: '/admin-react/dashboard', group: 'overview' },
  { key: 'redeem-create', labelKey: 'redeemCreate', icon: <PackagePlus size={16} />, path: '/admin-react/redeem/create', group: 'redeem' },
  { key: 'redeem-batches', labelKey: 'redeemBatches', icon: <FileArchive size={16} />, path: '/admin-react/redeem/batches', group: 'redeem' },
  { key: 'redeem-stock', labelKey: 'redeemStock', icon: <KeyRound size={16} />, path: '/admin-react/redeem/stock', group: 'redeem' },
  { key: 'feedback', labelKey: 'feedback', icon: <MessageSquare size={16} />, path: '/admin-react/feedback', group: 'ops' },
  { key: 'releases', labelKey: 'releases', icon: <Rocket size={16} />, path: '/admin-react/releases', group: 'release' },
  { key: 'audit', labelKey: 'audit', icon: <ClipboardList size={16} />, path: '/admin-react/audit', group: 'release' },
  { key: 'automation', labelKey: 'automation', icon: <Bot size={16} />, path: '/admin-react/automation', group: 'dev' },
  { key: 'settings', labelKey: 'settings', icon: <Settings size={16} />, path: '/admin-react/settings', group: 'system' },
];

function useLocaleMessages() {
  const locale = useSessionStore((s) => s.locale);
  return { locale, t: messages[locale], tx: (key: CopyKey) => copyText(locale, key) };
}

export function App() {
  const themeMode = useSessionStore((s) => s.theme);
  const locale = useSessionStore((s) => s.locale);

  useEffect(() => {
    document.documentElement.dataset.theme = themeMode;
  }, [themeMode]);

  return (
    <ConfigProvider
      locale={locale === 'zh-CN' ? zhCN : enUS}
      theme={{
        algorithm: themeMode === 'dark' ? antdTheme.darkAlgorithm : antdTheme.defaultAlgorithm,
        token: {
          colorPrimary: themeMode === 'dark' ? '#6aa8ff' : '#1769d2',
          borderRadius: 6,
          fontFamily: 'Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif',
        },
      }}
    >
      <AntApp>
        <Routes>
          <Route path="/" element={<Navigate to="/react" replace />} />
          <Route path="/react" element={<HomePage />} />
          <Route path="/app-react" element={<Navigate to="/app-react/quick" replace />} />
          <Route path="/app-react/*" element={<UserShell />}>
            <Route index element={<Navigate to="quick" replace />} />
            <Route path="quick" element={<QuickAssistant />} />
            <Route path="account" element={<RequireUser><AccountPage /></RequireUser>} />
            <Route path="recharge" element={<RequireUser><RechargePanel /></RequireUser>} />
            <Route path="convert" element={<RequireUser><CloudConversionPanel /></RequireUser>} />
            <Route path="jobs" element={<RequireUser><ConversionRecordsPanel /></RequireUser>} />
            <Route path="billing" element={<RequireUser><RechargeRecordsPanel /></RequireUser>} />
            <Route path="feedback" element={<RequireUser><FeedbackPanel /></RequireUser>} />
            <Route path="settings" element={<SettingsPanel scope="user" />} />
            <Route path="*" element={<Navigate to="quick" replace />} />
          </Route>
          <Route path="/admin-react" element={<Navigate to="/admin-react/dashboard" replace />} />
          <Route path="/admin-react/*" element={<AdminGate />}>
            <Route index element={<Navigate to="dashboard" replace />} />
            <Route path="dashboard" element={<AdminDashboardPanel />} />
            <Route path="redeem/create" element={<AdminRedeemCreatePanel />} />
            <Route path="redeem/batches" element={<AdminRedeemBatchesPanel />} />
            <Route path="redeem/stock" element={<AdminRedeemStockPanel />} />
            <Route path="feedback" element={<AdminFeedbackPanel />} />
            <Route path="releases" element={<AdminReleasePanel />} />
            <Route path="audit" element={<AdminAuditPanel />} />
            <Route path="automation" element={<AdminAutomationPanel />} />
            <Route path="settings" element={<SettingsPanel scope="admin" />} />
            <Route path="*" element={<Navigate to="dashboard" replace />} />
          </Route>
          <Route path="*" element={<Navigate to="/react" replace />} />
        </Routes>
      </AntApp>
    </ConfigProvider>
  );
}

function HomePage() {
  const [code, setCode] = useState(storedQuickCode());
  const navigate = useNavigate();
  const { tx } = useLocaleMessages();

  return (
    <main className="commercial-home">
      <header className="home-nav">
        <Link to="/react" className="brand-mark">Tex2Doc</Link>
        <Space>
          <Link to="/app-react/account"><Button>{tx('会员登录')}</Button></Link>
          <Link to="/admin-react/dashboard"><Button icon={<Shield size={16} />}>{tx('管理端')}</Button></Link>
        </Space>
      </header>
      <section className="home-grid">
        <div className="home-copy">
          <Tag color="blue">{tx('LaTeX to DOCX Workspace')}</Tag>
          <Title>{tx('把 LaTeX 项目转换成可交付的 DOCX')}</Title>
          <Paragraph>
            {tx('面向学术作者、渠道运营和产品管理员的商业化转换工作台。支持兑换码权益、本地 WASM 转换、云端队列、记录追溯与运营后台。')}
          </Paragraph>
          <Space wrap>
            <Button type="primary" size="large" icon={<Zap size={18} />} onClick={() => navigate('/app-react/quick')}>
              {tx('立即转换')}
            </Button>
            <Button size="large" icon={<User size={18} />} onClick={() => navigate('/app-react/account')}>
              {tx('进入会员中心')}
            </Button>
          </Space>
        </div>
        <Card className="quick-start-card" title={tx('兑换码快速开始')}>
          <Space direction="vertical" size="middle" style={{ width: '100%' }}>
            <Input size="large" placeholder={tx('输入兑换码')} value={code} onChange={(event) => setCode(event.target.value)} />
            <Upload.Dragger
              accept=".zip"
              multiple={false}
              beforeUpload={() => false}
              showUploadList={false}
              className="home-upload"
            >
              <UploadCloud size={28} />
              <p>{tx('拖入 LaTeX ZIP 项目')}</p>
              <Text type="secondary">{tx('进入转换页后会自动识别主 TeX')}</Text>
            </Upload.Dragger>
            <Button
              type="primary"
              block
              size="large"
              onClick={() => {
                if (code.trim()) localStorage.setItem('tex2doc.quick.redeemCode', code.trim());
                navigate('/app-react/quick');
              }}
            >
              {tx('激活并开始')}
            </Button>
            <Row gutter={12}>
              <Col span={8}><TrustSignal title={tx('本地转换')} value={tx('ZIP 不上传')} /></Col>
              <Col span={8}><TrustSignal title={tx('云端队列')} value={tx('可追踪')} /></Col>
              <Col span={8}><TrustSignal title={tx('额度扣减')} value={tx('可查看')} /></Col>
            </Row>
          </Space>
        </Card>
      </section>
    </main>
  );
}

function TrustSignal({ title, value }: { title: string; value: string }) {
  return (
    <div className="trust-signal">
      <Text type="secondary">{title}</Text>
      <strong>{value}</strong>
    </div>
  );
}

function UserShell() {
  const { tx } = useLocaleMessages();
  return <WorkspaceShell title={tx('Tex2Doc 用户端')} subtitle={tx('userSubtitle')} nav={userNav} root="/app-react" />;
}

function AdminGate() {
  const adminSession = useSessionStore((s) => s.adminSession);
  const setAdminSession = useSessionStore((s) => s.setAdminSession);
  const { tx } = useLocaleMessages();

  if (!adminSession) {
    return (
      <WorkspaceFrame title={tx('Tex2Doc 管理端')} subtitle={tx('adminSubtitle')}>
        <AuthPanel kind="admin" />
      </WorkspaceFrame>
    );
  }

  return <AdminShell session={adminSession} onInvalid={() => setAdminSession(undefined)} />;
}

function AdminShell({ session, onInvalid }: { session: AuthSession; onInvalid: () => void }) {
  const api = useMemo(() => new Tex2DocApi(session.apiBaseUrl, session.accessToken), [session]);
  const { tx } = useLocaleMessages();
  const gate = useQuery({
    queryKey: ['adminMe', session.accessToken],
    queryFn: () => api.adminMe(),
    retry: false,
  });

  useEffect(() => {
    if (gate.isError) onInvalid();
  }, [gate.isError, onInvalid]);

  if (gate.isLoading) {
    return (
      <WorkspaceFrame title={tx('Tex2Doc 管理端')} subtitle={tx('正在校验管理员权限')}>
        <StateView state="loading" title={tx('正在校验管理员权限')} />
      </WorkspaceFrame>
    );
  }

  const role = gate.data?.user?.role ?? session.user.role;
  if (role !== 'admin') {
    return (
      <WorkspaceFrame title={tx('Tex2Doc 管理端')} subtitle={tx('权限不足')}>
        <Result
          status="403"
          title={tx('需要管理员权限')}
          subTitle={tx('当前账号没有访问管理端的角色权限。')}
          extra={<Button onClick={onInvalid}>{tx('切换账号')}</Button>}
        />
      </WorkspaceFrame>
    );
  }

  return <WorkspaceShell title={tx('Tex2Doc 管理端')} subtitle={tx('adminSubtitle')} nav={adminNav} root="/admin-react" admin />;
}

function WorkspaceFrame({ title, subtitle, children }: { title: string; subtitle: string; children: React.ReactNode }) {
  const navigate = useNavigate();
  const locale = useSessionStore((s) => s.locale);
  const themeMode = useSessionStore((s) => s.theme);
  const setPreferences = useSessionStore((s) => s.setPreferences);
  const t = messages[locale];

  return (
    <Layout className="workspace-shell">
      <Header className="workspace-header">
        <Button type="text" icon={<Home size={18} />} onClick={() => navigate('/react')} />
        <div className="workspace-title">
          <strong>{title}</strong>
          <span>{subtitle}</span>
        </div>
        <div className="workspace-spacer" />
        <Segmented
          size="small"
          value={themeMode}
          onChange={(next) => setPreferences(next as 'light' | 'dark', locale)}
          options={[
            { label: t.common.light, value: 'light' },
            { label: t.common.dark, value: 'dark' },
          ]}
        />
        <Select
          size="small"
          value={locale}
          style={{ width: 104 }}
          onChange={(next) => setPreferences(themeMode, next as Locale)}
          options={[
            { label: 'zh-CN', value: 'zh-CN' },
            { label: 'en-US', value: 'en-US' },
          ]}
        />
      </Header>
      {children}
    </Layout>
  );
}

function WorkspaceShell({ title, subtitle, nav, root, admin = false }: { title: string; subtitle: string; nav: NavItem[]; root: string; admin?: boolean }) {
  const navigate = useNavigate();
  const location = useLocation();
  const screens = Grid.useBreakpoint();
  const { t, tx } = useLocaleMessages();
  const active = useMemo(() => nav.find((item) => location.pathname.startsWith(item.path))?.key ?? nav[0]?.key, [location.pathname, nav]);
  const menuItems = nav.map((item) => ({ key: item.key, icon: item.icon, label: t.nav[item.labelKey] }));

  return (
    <WorkspaceFrame title={title} subtitle={subtitle}>
      <Layout className="workspace-body">
        {screens.md && (
          <Sider width={248} className="workspace-sider">
            <Menu
              mode="inline"
              selectedKeys={[active]}
              items={menuItems}
              onClick={({ key }) => navigate(nav.find((item) => item.key === key)?.path ?? root)}
            />
          </Sider>
        )}
        <Layout>
          {!screens.md && (
            <div className="mobile-nav">
              <Select
                value={active}
                style={{ width: '100%' }}
                options={nav.map((item) => ({ label: t.nav[item.labelKey], value: item.key }))}
                onChange={(key) => navigate(nav.find((item) => item.key === key)?.path ?? root)}
              />
              {admin && <Alert type="info" showIcon message={tx('复杂管理表格建议在桌面端使用。')} />}
            </div>
          )}
          <Content className="workspace-content">
            <Outlet />
          </Content>
        </Layout>
      </Layout>
    </WorkspaceFrame>
  );
}

function RequireUser({ children }: { children: React.ReactNode }) {
  const session = useSessionStore((s) => s.userSession);
  if (!session) return <AuthPanel kind="user" />;
  return <>{children}</>;
}

function useUserApi(): Tex2DocApi {
  const session = useSessionStore((s) => s.userSession)!;
  return useMemo(() => new Tex2DocApi(session.apiBaseUrl, session.accessToken), [session]);
}

function useAdminApi(): Tex2DocApi {
  const session = useSessionStore((s) => s.adminSession)!;
  return useMemo(() => new Tex2DocApi(session.apiBaseUrl, session.accessToken), [session]);
}

function AuthPanel({ kind, error }: { kind: 'user' | 'admin'; error?: string }) {
  const setUserSession = useSessionStore((s) => s.setUserSession);
  const setAdminSession = useSessionStore((s) => s.setAdminSession);
  const { tx } = useLocaleMessages();
  const [tab, setTab] = useState<'login' | 'register'>('login');
  const [apiBaseUrl, setApiBaseUrl] = useState(initialApiBaseUrl());
  const [message, setMessage] = useState(error ?? '');
  const [form] = Form.useForm();

  async function submit(values: { email: string; password: string; displayName?: string }) {
    setMessage('');
    const api = new Tex2DocApi(apiBaseUrl);
    const auth = tab === 'login' ? await api.login(values.email, values.password) : await api.register(values.email, values.password, values.displayName);
    const session = { apiBaseUrl: api.baseUrl, accessToken: auth.access_token, refreshToken: auth.refresh_token, user: auth.user };
    if (kind === 'admin') {
      const profile = await new Tex2DocApi(api.baseUrl, auth.access_token).adminMe();
      if ((profile.user?.role ?? auth.user.role) !== 'admin') throw new Error('Admin role required.');
      setAdminSession({ ...session, user: { ...auth.user, role: 'admin' } });
    } else {
      setUserSession(session);
    }
  }

  return (
    <Card className="auth-card">
      <Flex justify="space-between" align="center" gap={16} wrap="wrap">
        <div>
          <Text type="secondary">{kind === 'admin' ? tx('Admin Gate') : tx('Member Center')}</Text>
          <Title level={3}>{kind === 'admin' ? tx('管理员登录') : tx('登录或注册')}</Title>
        </div>
        <Segmented value={tab} onChange={(next) => setTab(next as 'login' | 'register')} options={[{ label: tx('登录'), value: 'login' }, { label: tx('注册'), value: 'register' }]} />
      </Flex>
      <Form layout="vertical" form={form} onFinish={(values) => submit(values).catch((err) => setMessage(errorMessage(err)))} requiredMark={false}>
        <Form.Item label="API Base URL">
          <Input value={apiBaseUrl} onChange={(event) => setApiBaseUrl(event.target.value)} />
        </Form.Item>
        <Form.Item label={tx('邮箱')} name="email" rules={[{ required: true, type: 'email' }]}>
          <Input autoComplete="email" />
        </Form.Item>
        <Form.Item label={tx('密码')} name="password" rules={[{ required: true, min: tab === 'register' ? 6 : undefined }]}>
          <Input.Password autoComplete={tab === 'login' ? 'current-password' : 'new-password'} />
        </Form.Item>
        {tab === 'register' && (
          <Form.Item label={tx('显示名')} name="displayName">
            <Input />
          </Form.Item>
        )}
        {message && <Alert className="block-alert" type="error" showIcon message={message} />}
        <Button type="primary" htmlType="submit" block icon={<KeyRound size={16} />}>
          {tab === 'login' ? tx('登录') : tx('注册')}
        </Button>
      </Form>
    </Card>
  );
}

function QuickAssistant() {
  const quickSession = useSessionStore((s) => s.quickSession);
  const setQuickSession = useSessionStore((s) => s.setQuickSession);
  const { tx } = useLocaleMessages();
  const [apiBaseUrl, setApiBaseUrl] = useState(defaultApiBaseUrl());
  const [code, setCode] = useState(storedQuickCode());
  const [mode, setMode] = useState<'local' | 'cloud'>('local');
  const [file, setFile] = useState<File>();
  const [fileSummary, setFileSummary] = useState('');
  const [mainTex, setMainTex] = useState('main.tex');
  const [profile, setProfile] = useState('jos');
  const [quality, setQuality] = useState('high');
  const [logs, setLogs] = useState<string[]>([]);
  const [lastDocx, setLastDocx] = useState<Blob>();
  const [busy, setBusy] = useState(false);
  const api = useMemo(() => (quickSession ? new Tex2DocApi(quickSession.apiBaseUrl, quickSession.accessToken) : undefined), [quickSession]);

  function log(line: string) {
    setLogs((old) => [`${new Date().toLocaleTimeString()} ${line}`, ...old].slice(0, 100));
  }

  async function activate(candidate = code) {
    if (!candidate.trim()) return;
    setBusy(true);
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
      setQuickSession({ apiBaseUrl: anonymous.baseUrl, accessToken: auth.access_token, refreshToken: auth.refresh_token, user: auth.user, usage, redeemCode: candidate.trim() });
      log('快捷助手已激活。');
    } catch (err) {
      log(`激活失败：${errorMessage(err)}`);
    } finally {
      setBusy(false);
    }
  }

  useEffect(() => {
    if (!quickSession && code) void activate(code);
  }, []);

  async function pickFile(nextFile?: File) {
    setFile(nextFile);
    setFileSummary('');
    if (!nextFile) return;
    try {
      const detected = await detectMainTex(nextFile);
      setFileSummary(`${nextFile.name} · ${(nextFile.size / 1024 / 1024).toFixed(2)} MB`);
      if (detected) {
        setMainTex(detected);
        log(`已自动识别主 TeX：${detected}`);
      }
    } catch (error) {
      log(`读取 ZIP 文件列表失败：${errorMessage(error)}`);
    }
  }

  async function runConvert() {
    if (!api || !quickSession || !file) return;
    if (file.size >= 10 * 1024 * 1024) {
      log('ZIP 文件需小于 10 MB。');
      return;
    }
    setBusy(true);
    try {
      if (mode === 'local') {
        log('检查本地转换额度。');
        const quota = await api.checkLocalConversion();
        if (!quota.allowed || (!quota.valid_until_active && (quota.count_balance ?? 0) <= 0)) {
          log(`额度不足：${quota.reason ?? 'not allowed'}`);
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
      setBusy(false);
    }
  }

  const uploadProps: UploadProps = {
    accept: '.zip',
    multiple: false,
    showUploadList: false,
    beforeUpload: (nextFile) => {
      void pickFile(nextFile);
      return false;
    },
  };

  return (
    <div className="page-stack">
      <PageHeader title={tx('开始转换')} description={tx('使用兑换码激活权益，上传 LaTeX ZIP，选择本地或云端转换并下载 DOCX。')} />
      <Steps
        current={quickSession ? (file ? 2 : 1) : 0}
        items={[
          { title: tx('激活权益'), description: quickSession ? tx('已激活') : tx('输入兑换码') },
          { title: tx('上传项目'), description: fileSummary || tx('ZIP 小于 10 MB') },
          { title: tx('转换与下载'), description: mode === 'local' ? tx('本地 WASM') : tx('云端队列') },
        ]}
      />
      <Row gutter={[16, 16]}>
        <Col xs={24} lg={8}>
          <Card title={tx('1. 激活权益')} extra={<StatusTag value={quickSession ? 'activated' : 'idle'} />}>
            <Space direction="vertical" style={{ width: '100%' }}>
              <Input value={apiBaseUrl} onChange={(event) => setApiBaseUrl(event.target.value)} disabled={!!quickSession} addonBefore="API" />
              <Input value={code} onChange={(event) => setCode(event.target.value)} disabled={!!quickSession} placeholder={tx('兑换码')} />
              <Space wrap>
                <Button type="primary" icon={<KeyRound size={16} />} disabled={busy || !!quickSession} onClick={() => activate()}>
                  {tx('激活权益')}
                </Button>
                {quickSession && <Button icon={<LogOut size={16} />} onClick={() => setQuickSession(undefined)}>{tx('清除激活')}</Button>}
              </Space>
              {quickSession && <UsageCards usage={quickSession.usage} compact />}
            </Space>
          </Card>
        </Col>
        <Col xs={24} lg={8}>
          <Card title={tx('2. 上传项目')}>
            <Upload.Dragger {...uploadProps} disabled={!quickSession}>
              <UploadCloud size={30} />
              <p>{fileSummary || tx('拖入或选择 LaTeX ZIP')}</p>
              <Text type="secondary">{tx('会自动识别 main.tex / main-jos.tex')}</Text>
            </Upload.Dragger>
            <Form layout="vertical" className="form-after-upload">
              <Form.Item label={tx('主 TeX')}>
                <Input value={mainTex} onChange={(event) => setMainTex(event.target.value)} />
              </Form.Item>
            </Form>
          </Card>
        </Col>
        <Col xs={24} lg={8}>
          <Card title={tx('3. 转换与下载')}>
            <Space direction="vertical" style={{ width: '100%' }}>
              <Segmented
                block
                value={mode}
                onChange={(next) => setMode(next as 'local' | 'cloud')}
                options={[{ label: tx('快捷版'), value: 'local' }, { label: tx('专业版'), value: 'cloud' }]}
              />
              <Row gutter={12}>
                <Col span={12}>
                  <Select value={profile} onChange={setProfile} style={{ width: '100%' }} options={[{ value: 'jos', label: 'JOS' }, { value: 'standard', label: 'Standard' }]} />
                </Col>
                <Col span={12}>
                  <Select value={quality} onChange={setQuality} style={{ width: '100%' }} options={[{ value: 'high', label: tx('High') }, { value: 'medium', label: tx('Medium') }]} />
                </Col>
              </Row>
              <Button type="primary" block loading={busy} disabled={!quickSession || !file} icon={<Zap size={16} />} onClick={runConvert}>
                {tx('转换并下载')}
              </Button>
              {lastDocx && <Button block icon={<ArrowDownToLine size={16} />} onClick={() => downloadBlob(lastDocx, 'tex2doc.docx')}>{tx('再次下载')}</Button>}
            </Space>
          </Card>
        </Col>
      </Row>
      <Collapse items={[{ key: 'logs', label: tx('技术详情'), children: <pre className="logbox">{logs.join('\n') || tx('等待操作...')}</pre> }]} />
    </div>
  );
}

function AccountPage() {
  const session = useSessionStore((s) => s.userSession)!;
  return <AccountPanel session={session} />;
}

function AccountPanel({ session, admin = false }: { session: AuthSession; admin?: boolean }) {
  const setUserSession = useSessionStore((s) => s.setUserSession);
  const setAdminSession = useSessionStore((s) => s.setAdminSession);
  const { tx } = useLocaleMessages();
  const api = useMemo(() => new Tex2DocApi(session.apiBaseUrl, session.accessToken), [session]);
  const usage = useQuery({ queryKey: ['usage', session.accessToken], queryFn: () => api.usage(), initialData: session.usage });

  return (
    <div className="page-stack">
      <PageHeader
        title={session.user.email}
        description={admin ? tx('管理员账号与会话信息') : tx('会员权益、额度和最近工作入口')}
        extra={<Button icon={<LogOut size={16} />} onClick={() => (admin ? setAdminSession(undefined) : setUserSession(undefined))}>{tx('退出登录')}</Button>}
      />
      <UsageCards usage={usage.data} />
      {usage.isError && <Alert type="error" showIcon message={errorMessage(usage.error)} />}
    </div>
  );
}

function UsageCards({ usage, compact = false }: { usage?: UsageSummary; compact?: boolean }) {
  const { tx } = useLocaleMessages();
  const cards = [
    [tx('套餐'), usage?.plan_id ?? '-'],
    [tx('云端转换'), `${usage?.cloud_conversions_used ?? 0} / ${usage?.cloud_conversions_limit ?? '-'}`],
    [tx('按次余额'), usage?.count_balance ?? 0],
    [tx('有效期'), usage?.date_valid_until ?? '-'],
  ];
  return (
    <Row gutter={[12, 12]}>
      {cards.map(([label, value]) => (
        <Col xs={compact ? 12 : 24} sm={compact ? 12 : 12} lg={compact ? 12 : 6} key={label}>
          <Card size="small">
            <Statistic title={label} value={String(value)} />
          </Card>
        </Col>
      ))}
    </Row>
  );
}

function RechargePanel() {
  const api = useUserApi();
  const queryClient = useQueryClient();
  const { tx } = useLocaleMessages();
  const [code, setCode] = useState('');
  const records = useQuery({ queryKey: ['recharges', 'redeemRecords'], queryFn: () => api.redeemCodeRecords() });
  const redeem = useMutation({
    mutationFn: () => api.redeemCode(code.trim()),
    onSuccess: () => {
      setCode('');
      void queryClient.invalidateQueries({ queryKey: ['recharges'] });
      void queryClient.invalidateQueries({ queryKey: ['usage'] });
    },
  });

  return (
    <div className="page-stack">
      <PageHeader title={tx('充值兑换')} description={tx('兑换卡密权益，或进入购买页获取新卡片。')} extra={<a href="https://pay.ldxp.cn/item/ns8i2g" target="_blank" rel="noreferrer"><Button icon={<ShoppingCart size={16} />}>{tx('购买卡片')}</Button></a>} />
      <Card>
        <Space.Compact style={{ width: '100%' }}>
          <Input placeholder={tx('输入兑换码')} value={code} onChange={(event) => setCode(event.target.value)} />
          <Button type="primary" loading={redeem.isPending} disabled={!code.trim()} icon={<Check size={16} />} onClick={() => redeem.mutate()}>
            {tx('提交兑换码')}
          </Button>
        </Space.Compact>
        {redeem.isSuccess && <Alert className="block-alert" type="success" showIcon message={tx('兑换成功，权益已更新。')} />}
        {redeem.isError && <Alert className="block-alert" type="error" showIcon message={errorMessage(redeem.error)} />}
      </Card>
      <DataTable rows={records.data ?? []} loading={records.isLoading} columns={['id', 'code_preview', 'package_id', 'quantity', 'redeemed_at', 'created_at']} />
    </div>
  );
}

function CloudConversionPanel() {
  const api = useUserApi();
  const { tx } = useLocaleMessages();
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
      const job = await pollConversion(api, jobIdOf(created), setLog);
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
    <div className="page-stack">
      <PageHeader title={tx('云端转换')} description={tx('适合大文件、复杂模板和需要队列追踪的专业转换。')} />
      <Card>
        <Row gutter={[16, 16]}>
          <Col xs={24} md={12}>
            <Upload.Dragger
              accept=".zip"
              multiple={false}
              showUploadList={!!file}
              beforeUpload={(nextFile) => {
                setFile(nextFile);
                void detectMainTex(nextFile).then((detected) => detected && setMainTex(detected));
                return false;
              }}
            >
              <UploadCloud size={30} />
              <p>{file?.name ?? tx('选择 LaTeX ZIP 项目')}</p>
            </Upload.Dragger>
          </Col>
          <Col xs={24} md={12}>
            <Form layout="vertical">
              <Form.Item label={tx('主 TeX')}>
                <Input value={mainTex} onChange={(event) => setMainTex(event.target.value)} />
              </Form.Item>
              <Button type="primary" loading={loading} disabled={!file} icon={<CloudUpload size={16} />} onClick={convert}>
                {tx('创建云端任务')}
              </Button>
            </Form>
          </Col>
        </Row>
      </Card>
      <Card title={tx('任务日志')}><pre className="logbox">{log || tx('等待上传...')}</pre></Card>
    </div>
  );
}

function ConversionRecordsPanel() {
  const api = useUserApi();
  const { tx } = useLocaleMessages();
  const rows = useQuery({ queryKey: ['conversions'], queryFn: () => api.conversions() });

  async function download(kind: 'docx' | 'zip' | 'log', row: ConversionJob) {
    const id = jobIdOf(row);
    const blob = kind === 'docx' ? await api.downloadConversionDocx(id) : kind === 'zip' ? await api.downloadConversionZip(id) : await api.downloadConversionLog(id);
    downloadBlob(blob, `${id}.${kind}`);
  }

  return (
    <div className="page-stack">
      <PageHeader title={tx('转换记录')} description={tx('追踪云端任务、重新下载结果或日志。')} extra={<Button icon={<RefreshCw size={16} />} onClick={() => rows.refetch()}>{tx('刷新')}</Button>} />
      {rows.isError && <Alert type="error" showIcon message={errorMessage(rows.error)} />}
      <DataTable
        rows={rows.data ?? []}
        loading={rows.isLoading}
        columns={['job_id', 'main_tex', 'profile', 'quality', 'status', 'created_at']}
        actions={(row) => (
          <Space>
            <Tooltip title="DOCX"><Button icon={<FileText size={16} />} onClick={() => download('docx', row)} /></Tooltip>
            <Tooltip title="ZIP"><Button icon={<FileArchive size={16} />} onClick={() => download('zip', row)} /></Tooltip>
            <Tooltip title="LOG"><Button icon={<ClipboardList size={16} />} onClick={() => download('log', row)} /></Tooltip>
          </Space>
        )}
      />
    </div>
  );
}

function RechargeRecordsPanel() {
  const api = useUserApi();
  const { tx } = useLocaleMessages();
  const rows = useQuery({ queryKey: ['recharges'], queryFn: () => api.recharges() });
  return (
    <div className="page-stack">
      <PageHeader title={tx('充值记录')} description={tx('查看会员充值、购买和兑换流水。')} />
      <DataTable rows={rows.data ?? []} loading={rows.isLoading} columns={['id', 'recharge_id', 'recharge_type', 'package_id', 'amount_cents', 'provider', 'status', 'created_at']} />
    </div>
  );
}

function FeedbackPanel() {
  const api = useUserApi();
  const queryClient = useQueryClient();
  const { locale, t, tx } = useLocaleMessages();
  const [selected, setSelected] = useState<string>();
  const [form] = Form.useForm();
  const [reply, setReply] = useState('');
  const threads = useQuery({ queryKey: ['feedbackThreads'], queryFn: () => api.feedbackThreads() });
  const detail = useQuery({ queryKey: ['feedbackThread', selected], queryFn: () => api.feedbackThread(selected!), enabled: !!selected });
  const create = useMutation({
    mutationFn: (values: { title: string; content: string }) => api.createFeedbackThread({ title: values.title, content: values.content, feedbackType: 'issue', priority: 'normal' }),
    onSuccess: () => {
      form.resetFields();
      void queryClient.invalidateQueries({ queryKey: ['feedbackThreads'] });
    },
  });

  async function sendReply() {
    if (!detail.data || !reply.trim()) return;
    await api.addFeedbackMessage(detail.data.id, reply.trim());
    setReply('');
    await queryClient.invalidateQueries({ queryKey: ['feedbackThread', selected] });
  }

  return (
    <div className="page-stack">
      <PageHeader title={tx('反馈')} description={tx('提交问题并跟进支持会话，可关联转换任务。')} />
      <Row gutter={[16, 16]}>
        <Col xs={24} lg={9}>
          <Card title={tx('新建反馈')}>
            <Form form={form} layout="vertical" onFinish={(values) => create.mutate(values)}>
              <Form.Item name="title" label={tx('标题')} rules={[{ required: true }]}>
                <Input maxLength={100} />
              </Form.Item>
              <Form.Item name="content" label={tx('描述')} rules={[{ required: true }]}>
                <Input.TextArea rows={4} />
              </Form.Item>
              <Button type="primary" htmlType="submit" loading={create.isPending} icon={<Send size={16} />}>{tx('提交反馈')}</Button>
            </Form>
          </Card>
          <List
            className="support-list"
            loading={threads.isLoading}
            dataSource={threads.data ?? []}
            locale={{ emptyText: <Empty description={tx('暂无反馈')} /> }}
            renderItem={(item) => (
              <List.Item className={selected === item.id ? 'is-selected' : ''} onClick={() => setSelected(item.id)}>
                <List.Item.Meta title={item.title} description={t.dynamic.feedbackMeta(localizeStatus(locale, item.feedback_type), localizeStatus(locale, item.priority ?? 'normal'))} />
                <StatusTag value={item.status ?? 'open'} />
              </List.Item>
            )}
          />
        </Col>
        <Col xs={24} lg={15}>
          <Card title={tx('会话详情')}>
            {detail.isLoading ? <Spin /> : detail.data ? (
              <Space direction="vertical" style={{ width: '100%' }}>
                {(detail.data.messages ?? []).map((m, index) => <MessageBubble key={m.id ?? index} role={m.author_role ?? 'user'} content={m.content} />)}
                <Space.Compact style={{ width: '100%' }}>
                  <Input value={reply} onChange={(event) => setReply(event.target.value)} placeholder={tx('继续回复')} />
                  <Button icon={<Send size={16} />} onClick={sendReply}>{tx('发送')}</Button>
                </Space.Compact>
              </Space>
            ) : <StateView state="empty" title={tx('选择一个反馈会话')} />}
          </Card>
        </Col>
      </Row>
    </div>
  );
}

function AdminDashboardPanel() {
  const api = useAdminApi();
  const { locale, tx } = useLocaleMessages();
  const dashboard = useQuery({ queryKey: ['adminDashboard'], queryFn: () => api.adminDashboard() });
  const data = dashboard.data;
  const counts = data?.counts ?? {};
  const countEntries = Object.entries(counts);

  return (
    <div className="page-stack">
      <PageHeader title={tx('仪表盘')} description={tx('经营指标、待办风险和最近活动。')} extra={<Button icon={<RefreshCw size={16} />} onClick={() => dashboard.refetch()}>{tx('刷新')}</Button>} />
      {dashboard.isError && <Alert type="error" showIcon message={errorMessage(dashboard.error)} />}
      <Row gutter={[16, 16]}>
        {(countEntries.length ? countEntries : [['conversion_tasks', 0], ['redeem_codes', 0], ['feedback_threads', 0], ['automation_requests', 0]]).map(([key, value]) => (
          <Col xs={24} sm={12} lg={6} key={key}>
            <Card loading={dashboard.isLoading}><Statistic title={fieldLabel(locale, String(key)) ?? humanizeKey(String(key))} value={Number(value) || 0} /></Card>
          </Col>
        ))}
      </Row>
      <Row gutter={[16, 16]}>
        <Col xs={24} lg={8}>
          <Card title={tx('待处理')}><StateList items={[tx('高优先级反馈'), tx('失败转换任务'), tx('待审批自动化请求')]} /></Card>
        </Col>
        <Col xs={24} lg={8}>
          <Card title={tx('风险概览')}><StateList danger items={[tx('高风险自动化需人工复核'), tx('发布回滚需审计记录')]} /></Card>
        </Col>
        <Col xs={24} lg={8}>
          <Card title={tx('模块状态')}>
            <Space wrap>{(data?.modules ?? ['redeem', 'feedback', 'release', 'automation']).map((item) => <Tag color="blue" key={item}>{item}</Tag>)}</Space>
          </Card>
        </Col>
      </Row>
    </div>
  );
}

function StateList({ items, danger = false }: { items: string[]; danger?: boolean }) {
  return (
    <List
      size="small"
      dataSource={items}
      renderItem={(item) => <List.Item><Badge status={danger ? 'warning' : 'processing'} text={item} /></List.Item>}
    />
  );
}

function AdminRedeemCreatePanel() {
  const api = useAdminApi();
  const { tx } = useLocaleMessages();
  const [batch, setBatch] = useState<RedeemCodeBatch>();
  const create = useMutation({
    mutationFn: (values: { packageId: string; quantity: number; channel?: string; note?: string }) => api.createRedeemCodeBatch(values),
    onSuccess: async (created) => {
      setBatch(created);
      const blob = await api.exportRedeemCodeBatch(created.id);
      downloadBlob(blob, `redeem-codes-${created.batch_no ?? created.id}.xlsx`);
    },
  });

  return (
    <div className="page-stack">
      <PageHeader title={tx('生成兑换码')} description={tx('分步创建批次，生成后立即下载 Excel。')} />
      <Card>
        <Form layout="vertical" initialValues={{ packageId: 'count_3', quantity: 10, channel: 'web' }} onFinish={(values) => create.mutate(values)}>
          <Row gutter={16}>
            <Col xs={24} md={8}><Form.Item name="packageId" label={tx('套餐')} rules={[{ required: true }]}><Select options={[{ value: 'count_3' }, { value: 'count_10' }, { value: 'count_30' }]} /></Form.Item></Col>
            <Col xs={24} md={8}><Form.Item name="quantity" label={tx('数量')} rules={[{ required: true }]}><InputNumber min={1} max={10000} style={{ width: '100%' }} /></Form.Item></Col>
            <Col xs={24} md={8}><Form.Item name="channel" label={tx('渠道')}><Input /></Form.Item></Col>
          </Row>
          <Form.Item name="note" label={tx('备注')}><Input.TextArea rows={3} /></Form.Item>
          <Button type="primary" htmlType="submit" loading={create.isPending} icon={<PackagePlus size={16} />}>{tx('生成并下载 Excel')}</Button>
        </Form>
        {create.isError && <Alert className="block-alert" type="error" showIcon message={errorMessage(create.error)} />}
        {create.isSuccess && <Alert className="block-alert" type="success" showIcon message={tx('批次已生成，Excel 已开始下载。')} />}
      </Card>
      {batch && <DataTable rows={[batch]} columns={['id', 'batch_no', 'package_id', 'quantity', 'generated_count', 'status', 'created_at']} />}
    </div>
  );
}

function AdminRedeemBatchesPanel() {
  const api = useAdminApi();
  const { tx } = useLocaleMessages();
  const [detailId, setDetailId] = useState<string>();
  const [open, setOpen] = useState(false);
  const rows = useQuery({ queryKey: ['redeemBatches'], queryFn: () => api.redeemCodeBatches() });
  const detail = useQuery({ queryKey: ['redeemBatch', detailId], queryFn: () => api.redeemCodeBatchDetail(detailId!), enabled: !!detailId });

  async function exportBatch(row: RedeemCodeBatch) {
    const blob = await api.exportRedeemCodeBatch(row.id);
    downloadBlob(blob, `redeem-codes-${row.batch_no ?? row.id}.xlsx`);
  }

  return (
    <div className="page-stack">
      <PageHeader title={tx('兑换码批次')} description={tx('筛选、查看、导出兑换码批次。')} extra={<Button icon={<RefreshCw size={16} />} onClick={() => rows.refetch()}>{tx('刷新')}</Button>} />
      <DataTable
        rows={rows.data ?? []}
        loading={rows.isLoading}
        columns={['batch_no', 'package_id', 'quantity', 'generated_count', 'exported_count', 'status', 'channel', 'created_at']}
        actions={(row) => (
          <Space>
            <Button icon={<Search size={16} />} onClick={() => { setDetailId(row.id); setOpen(true); }}>{tx('详情')}</Button>
            <Button icon={<ArrowDownToLine size={16} />} onClick={() => exportBatch(row)}>{tx('Excel')}</Button>
          </Space>
        )}
      />
      <Drawer title={tx('批次详情')} width={720} open={open} onClose={() => setOpen(false)}>
        {detail.isLoading ? <Spin /> : detail.data ? (
          <Space direction="vertical" style={{ width: '100%' }}>
            <Descriptions bordered size="small" column={1} items={describeRecord(detail.data, ['id', 'batch_no', 'package_id', 'quantity', 'generated_count', 'exported_count', 'status', 'note'])} />
            <DataTable rows={codeRowsOf(detail.data)} columns={['index', 'code_preview', 'code']} />
          </Space>
        ) : <Empty />}
      </Drawer>
    </div>
  );
}

function AdminRedeemStockPanel() {
  const api = useAdminApi();
  const { locale, t, tx } = useLocaleMessages();
  const [status, setStatus] = useState('');
  const [batchId, setBatchId] = useState('');
  const [packageId, setPackageId] = useState('');
  const [search, setSearch] = useState('');
  const [selected, setSelected] = useState<string[]>([]);
  const [page, setPage] = useState(1);
  const rows = useQuery({
    queryKey: ['redeemCodes', status, batchId, packageId, search, page],
    queryFn: () => api.adminListRedeemCodes({ stockStatus: status, batchId, packageId, search, page, pageSize: 50 }),
  });
  const records = rows.data?.items ?? rows.data?.codes ?? rows.data?.records ?? [];

  async function bulkStock() {
    await api.adminBulkStockRedeemCodes(selected);
    setSelected([]);
    await rows.refetch();
  }

  async function exportCodes() {
    const blob = await api.adminExportRedeemCodesExcel({ stockStatus: status, batchId, packageId, search });
    downloadBlob(blob, 'redeem-codes-list.xlsx');
  }

  return (
    <div className="page-stack">
      <PageHeader title={tx('兑换码库存')} description={tx('库存筛选、批量上货和导出。')} />
      <Card>
        <Flex gap={8} wrap="wrap">
          <Select value={status} onChange={setStatus} style={{ width: 140 }} options={['', 'new', 'stocked', 'redeemed', 'restocked'].map((value) => ({ value, label: value ? localizeStatus(locale, value) : tx('全部') }))} />
          <Input placeholder={tx('搜索')} value={search} onChange={(event) => setSearch(event.target.value)} style={{ width: 180 }} />
          <Input placeholder={tx('批次 ID')} value={batchId} onChange={(event) => setBatchId(event.target.value)} style={{ width: 180 }} />
          <Input placeholder={tx('套餐 ID')} value={packageId} onChange={(event) => setPackageId(event.target.value)} style={{ width: 160 }} />
          <Button icon={<Search size={16} />} onClick={() => rows.refetch()}>{tx('检索')}</Button>
          <Button disabled={!selected.length} icon={<Check size={16} />} onClick={() => Modal.confirm({ title: t.dynamic.bulkStockConfirm(selected.length), onOk: bulkStock })}>{tx('批量上货')}</Button>
          <Button icon={<ArrowDownToLine size={16} />} onClick={exportCodes}>{tx('导出 Excel')}</Button>
        </Flex>
      </Card>
      <DataTable
        rows={records}
        loading={rows.isLoading}
        columns={['code_id', 'batch_no', 'code_preview', 'package_id', 'stock_status', 'stocked_at', 'redeemed_at', 'restocked_at', 'created_at']}
        select={{ selected, onSelected: setSelected }}
      />
      <Flex justify="end" gap={8}>
        <Text type="secondary">{t.dynamic.pageSummary(rows.data?.total ?? 0, page)}</Text>
        <Button disabled={page <= 1} onClick={() => setPage(page - 1)}>{tx('上一页')}</Button>
        <Button disabled={records.length < 50 || page * 50 >= (rows.data?.total ?? 0)} onClick={() => setPage(page + 1)}>{tx('下一页')}</Button>
      </Flex>
    </div>
  );
}

function AdminFeedbackPanel() {
  const api = useAdminApi();
  const queryClient = useQueryClient();
  const { locale, t, tx } = useLocaleMessages();
  const [selected, setSelected] = useState<FeedbackThread>();
  const [reply, setReply] = useState('');
  const rows = useQuery({ queryKey: ['adminFeedbackThreads'], queryFn: () => api.adminFeedbackThreads() });

  async function updateStatus(thread: FeedbackThread, status: string) {
    await api.adminUpdateFeedbackThread(thread.id, status);
    await queryClient.invalidateQueries({ queryKey: ['adminFeedbackThreads'] });
  }

  async function sendReply() {
    if (!selected || !reply.trim()) return;
    await api.adminReplyFeedbackThread(selected.id, reply.trim(), false);
    setReply('');
    await queryClient.invalidateQueries({ queryKey: ['adminFeedbackThreads'] });
  }

  return (
    <div className="page-stack">
      <PageHeader title={tx('客户支持')} description={tx('按状态和优先级处理用户反馈。')} extra={<Button icon={<RefreshCw size={16} />} onClick={() => rows.refetch()}>{tx('刷新')}</Button>} />
      <Row gutter={[16, 16]}>
        <Col xs={24} lg={10}>
          <List
            className="support-list"
            loading={rows.isLoading}
            dataSource={rows.data ?? []}
            renderItem={(thread) => (
              <List.Item className={selected?.id === thread.id ? 'is-selected' : ''} onClick={() => setSelected(thread)}>
                <List.Item.Meta title={thread.title} description={t.dynamic.adminFeedbackMeta(localizeStatus(locale, thread.feedback_type), localizeStatus(locale, thread.priority), thread.message_count ?? 0)} />
                <StatusTag value={thread.status ?? 'open'} />
              </List.Item>
            )}
          />
        </Col>
        <Col xs={24} lg={14}>
          <Card title={tx('处理面板')}>
            {selected ? (
              <Space direction="vertical" style={{ width: '100%' }}>
                <Descriptions bordered size="small" column={1} items={describeRecord(selected, ['id', 'title', 'feedback_type', 'priority', 'status', 'conversion_job_id', 'created_at'])} />
                <Select value={selected.status ?? 'open'} onChange={(next) => updateStatus(selected, next)} options={['open', 'in_progress', 'resolved', 'closed'].map((value) => ({ value, label: localizeStatus(locale, value) }))} />
                <Space.Compact style={{ width: '100%' }}>
                  <Input value={reply} onChange={(event) => setReply(event.target.value)} placeholder={tx('公开回复用户')} />
                  <Button icon={<Send size={16} />} onClick={sendReply}>{tx('发送')}</Button>
                </Space.Compact>
              </Space>
            ) : <StateView state="empty" title={tx('选择一个反馈会话')} />}
          </Card>
        </Col>
      </Row>
    </div>
  );
}

function AdminReleasePanel() {
  const api = useAdminApi();
  const queryClient = useQueryClient();
  const { t, tx } = useLocaleMessages();
  const rows = useQuery({ queryKey: ['releases'], queryFn: () => api.adminReleases() });
  const publish = useMutation({
    mutationFn: (values: { channel: string; platform: string; arch: string; version: string; releaseTitle?: string; downloadUrl: string; sha256: string }) => api.adminPublishRelease(values),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['releases'] }),
  });

  async function rollback(row: ReleaseManifest) {
    if (!row.id) return;
    await api.adminRollbackRelease(row.id);
    await queryClient.invalidateQueries({ queryKey: ['releases'] });
  }

  return (
    <div className="page-stack">
      <PageHeader title={tx('发布管理')} description={tx('发布清单、渠道策略和回滚入口。')} />
      <Card title={tx('新建发布')}>
        <Form layout="vertical" initialValues={{ channel: 'beta', platform: 'windows', arch: 'x64' }} onFinish={(values) => publish.mutate(values)}>
          <Row gutter={16}>
            {(['channel', 'platform', 'arch', 'version'] as const).map((key) => <Col xs={24} md={6} key={key}><Form.Item name={key} label={key} rules={[{ required: true }]}><Input /></Form.Item></Col>)}
            <Col xs={24} md={8}><Form.Item name="releaseTitle" label={tx('标题')}><Input /></Form.Item></Col>
            <Col xs={24} md={8}><Form.Item name="downloadUrl" label={tx('下载地址')} rules={[{ required: true, type: 'url' }]}><Input /></Form.Item></Col>
            <Col xs={24} md={8}><Form.Item name="sha256" label="SHA-256" rules={[{ required: true, len: 64 }]}><Input /></Form.Item></Col>
          </Row>
          <Button type="primary" htmlType="submit" loading={publish.isPending} icon={<Rocket size={16} />}>{tx('发布清单')}</Button>
        </Form>
      </Card>
      <DataTable
        rows={rows.data ?? []}
        loading={rows.isLoading}
        columns={['channel', 'platform', 'arch', 'version', 'release_title', 'active', 'published_at', 'rolled_back_at']}
        actions={(row) => row.active !== false && <Button danger onClick={() => Modal.confirm({ title: t.dynamic.rollbackConfirm(row.version ?? '-'), onOk: () => rollback(row) })}>{tx('回滚')}</Button>}
      />
    </div>
  );
}

function AdminAuditPanel() {
  const api = useAdminApi();
  const { tx } = useLocaleMessages();
  const rows = useQuery({ queryKey: ['auditLogs'], queryFn: () => api.adminReleaseAudit() });
  return (
    <div className="page-stack">
      <PageHeader title={tx('审计中心')} description={tx('查看发布、回滚和管理员操作记录。')} />
      <DataTable rows={rows.data ?? []} loading={rows.isLoading} columns={['action', 'target_release_id', 'actor', 'details', 'created_at']} />
    </div>
  );
}

function AdminAutomationPanel() {
  const api = useAdminApi();
  const queryClient = useQueryClient();
  const { locale, tx } = useLocaleMessages();
  const [status, setStatus] = useState('');
  const [risk, setRisk] = useState('');
  const [selected, setSelected] = useState<AutomationRequest>();
  const summary = useQuery({ queryKey: ['automationSummary'], queryFn: () => api.adminAutomationSummary() });
  const requests = useQuery({ queryKey: ['automationRequests', status, risk], queryFn: () => api.adminAutomationRequests({ ...(status ? { status } : {}), ...(risk ? { risk_level: risk } : {}) }) });
  const detail = useQuery({ queryKey: ['automationRequest', selected?.id], queryFn: () => api.adminAutomationRequest(selected!.id), enabled: !!selected });
  const events = useQuery({ queryKey: ['automationEvents', selected?.id], queryFn: () => api.adminAutomationEvents(selected!.id), enabled: !!selected });
  const agents = useQuery({ queryKey: ['automationAgents'], queryFn: () => api.adminAutomationAgents() });

  async function approve() {
    if (!selected) return;
    await api.adminAutomationApprove(selected.id);
    await queryClient.invalidateQueries({ queryKey: ['automationRequest', selected.id] });
  }

  const current = detail.data ?? selected;
  const highRisk = ['high', 'critical'].includes(current?.risk_level ?? '');

  return (
    <div className="page-stack">
      <PageHeader title={tx('自动化')} description={tx('风险优先的自动化请求审批与 Agent 状态。')} />
      <Row gutter={[16, 16]}>
        {Object.entries(summary.data ?? {}).map(([key, value]) => <Col xs={12} lg={6} key={key}><Card><Statistic title={fieldLabel(locale, key) ?? humanizeKey(key)} value={value} /></Card></Col>)}
      </Row>
      <Row gutter={[16, 16]}>
        <Col xs={24} lg={10}>
          <Card title={tx('请求队列')}>
            <Flex gap={8} wrap="wrap" className="toolbar-row">
              <Select value={status} onChange={setStatus} style={{ width: 170 }} options={['', 'triaged', 'needs_approval', 'local_failed', 'ci_failed', 'deployed'].map((value) => ({ value, label: value ? localizeStatus(locale, value) : tx('All status') }))} />
              <Select value={risk} onChange={setRisk} style={{ width: 150 }} options={['', 'low', 'medium', 'high', 'critical'].map((value) => ({ value, label: value ? localizeStatus(locale, value) : tx('All risk') }))} />
            </Flex>
            <List
              loading={requests.isLoading}
              dataSource={requests.data ?? []}
              renderItem={(item) => (
                <List.Item className={selected?.id === item.id ? 'is-selected' : ''} onClick={() => setSelected(item)}>
                  <List.Item.Meta title={`${item.short_id ?? item.id} · ${item.title ?? '-'}`} description={`${localizeStatus(locale, item.status ?? '-')} · ${item.assigned_agent_id ?? 'unassigned'}`} />
                  <StatusTag value={item.risk_level ?? 'unknown'} />
                </List.Item>
              )}
            />
          </Card>
        </Col>
        <Col xs={24} lg={14}>
          <Card title={tx('请求详情')}>
            {current ? (
              <Space direction="vertical" style={{ width: '100%' }}>
                {highRisk && <Alert type="warning" showIcon message={tx('高风险请求需要人工复核，前端禁止直接 Approve。')} />}
                <Descriptions bordered size="small" column={1} items={describeRecord(current, ['id', 'short_id', 'title', 'status', 'risk_level', 'branch_name', 'pr_url', 'updated_at'])} />
                <Paragraph>{current.ai_summary}</Paragraph>
                <Space wrap>
                  <Button type="primary" disabled={highRisk || !['triaged', 'needs_approval'].includes(current.status ?? '')} icon={<Check size={16} />} onClick={approve}>{tx('Approve')}</Button>
                  <Button onClick={() => Modal.confirm({ title: tx('Reject reason'), content: tx('确认拒绝该自动化请求？'), onOk: async () => selected && api.adminAutomationReject(selected.id, 'Rejected from admin panel') })}>{tx('Reject')}</Button>
                  <Button onClick={async () => selected && api.adminAutomationRetry(selected.id)}>{tx('Retry')}</Button>
                </Space>
                <DataTable rows={events.data ?? []} loading={events.isLoading} columns={['event_type', 'message', 'created_at']} />
              </Space>
            ) : <StateView state="empty" title={tx('选择请求查看详情')} />}
          </Card>
        </Col>
      </Row>
      <Card title={tx('Agents')}>
        <DataTable
          rows={agents.data ?? []}
          loading={agents.isLoading}
          columns={['id', 'status', 'hostname', 'agent_version', 'last_heartbeat_at', 'completed_count', 'failed_count']}
          actions={(agent) => <Button onClick={async () => { agent.status === 'paused' ? await api.adminAutomationResumeAgent(agent.id) : await api.adminAutomationPauseAgent(agent.id); await agents.refetch(); }}>{agent.status === 'paused' ? tx('Resume') : tx('Pause')}</Button>}
        />
      </Card>
    </div>
  );
}

function SettingsPanel({ scope }: { scope: 'user' | 'admin' }) {
  const { theme, locale, setPreferences } = useSessionStore();
  const { tx } = useLocaleMessages();
  const session = useSessionStore((s) => (scope === 'admin' ? s.adminSession : s.userSession));
  return (
    <div className="page-stack">
      <PageHeader title={tx('设置')} description={tx('主题、语言和当前 API 连接信息。')} />
      <Card>
        <Descriptions bordered column={1} items={[
          { key: 'theme', label: tx('主题'), children: <Segmented value={theme} onChange={(next) => setPreferences(next as 'light' | 'dark', locale)} options={[{ label: tx('浅色'), value: 'light' }, { label: tx('深色'), value: 'dark' }]} /> },
          { key: 'locale', label: tx('语言'), children: <Select value={locale} onChange={(next) => setPreferences(theme, next as Locale)} options={[{ value: 'zh-CN' }, { value: 'en-US' }]} /> },
          { key: 'api', label: 'API Base URL', children: session?.apiBaseUrl ?? defaultApiBaseUrl() },
        ]} />
      </Card>
    </div>
  );
}

function PageHeader({ title, description, extra }: { title: string; description?: string; extra?: React.ReactNode }) {
  return (
    <Flex className="page-header" justify="space-between" align="flex-start" gap={16} wrap="wrap">
      <div>
        <Title level={2}>{title}</Title>
        {description && <Text type="secondary">{description}</Text>}
      </div>
      {extra}
    </Flex>
  );
}

function StateView({ state, title }: { state: 'loading' | 'empty' | 'error' | 'permission'; title: string }) {
  if (state === 'loading') return <Spin tip={title}><div className="state-pad" /></Spin>;
  if (state === 'permission') return <Result status="403" title={title} />;
  if (state === 'error') return <Result status="error" title={title} />;
  return <Empty description={title} />;
}

function MessageBubble({ role, content }: { role: string; content: string }) {
  return (
    <div className="message-bubble">
      <Text strong>{role}</Text>
      <Paragraph>{content}</Paragraph>
    </div>
  );
}

function StatusTag({ value }: { value: string }) {
  const kind = statusKind(value);
  const locale = useSessionStore((s) => s.locale);
  const color: Record<StatusKind, string> = { success: 'success', processing: 'processing', warning: 'warning', error: 'error', default: 'default' };
  return <Badge status={color[kind] as StatusKind} text={localizeStatus(locale, value)} />;
}

function statusKind(value: string): StatusKind {
  const normalized = value.toLowerCase();
  if (['completed', 'active', 'resolved', 'deployed', 'activated', 'success', 'stocked'].includes(normalized)) return 'success';
  if (['queued', 'running', 'open', 'in_progress', 'triaged', 'needs_approval'].includes(normalized)) return 'processing';
  if (['medium', 'high', 'warning', 'expired', 'paused'].includes(normalized)) return 'warning';
  if (['failed', 'critical', 'error', 'closed', 'local_failed', 'ci_failed'].includes(normalized)) return 'error';
  return 'default';
}

type EnhancedColumn<T> = {
  key: keyof T | string;
  label: string;
  render?: (row: T) => React.ReactNode;
};

function DataTable<T extends { id?: string } | Record<string, unknown>>({
  rows,
  columns,
  actions,
  select,
  loading = false,
}: {
  rows: T[];
  columns: string[];
  actions?: (row: T) => React.ReactNode;
  select?: { selected: string[]; onSelected: (ids: string[]) => void };
  loading?: boolean;
}) {
  const { locale, tx } = useLocaleMessages();
  const tableColumns: ColumnsType<T> = columns.map((key) => ({
    title: fieldLabel(locale, key) ?? humanizeKey(key),
    dataIndex: key,
    key,
    ellipsis: true,
    render: (value: unknown) => renderCell(key, value, locale),
  }));
  if (actions) {
    tableColumns.push({ title: tx('操作'), key: 'actions', fixed: 'right', render: (_, row) => actions(row) });
  }
  return (
    <Table
      size="middle"
      rowKey={(row) => String((row as { id?: string }).id ?? (row as Record<string, unknown>).code_id ?? (row as Record<string, unknown>).job_id ?? JSON.stringify(row))}
      dataSource={rows}
      columns={tableColumns}
      loading={loading}
      scroll={{ x: 900 }}
      locale={{ emptyText: <Empty description={tx('暂无数据')} /> }}
      rowSelection={select ? {
        selectedRowKeys: select.selected,
        onChange: (keys) => select.onSelected(keys.map(String)),
      } : undefined}
    />
  );
}

function renderCell(key: string, value: unknown, locale: Locale = 'zh-CN'): React.ReactNode {
  if (value === undefined || value === null || value === '') return '-';
  if (key.includes('status') || key === 'risk_level') return <StatusTag value={String(value)} />;
  if (key.endsWith('_at') || key.includes('created') || key.includes('updated')) return formatDateTime(value, locale);
  if (typeof value === 'boolean') return value ? <Tag color="success">{copyText(locale, 'true')}</Tag> : <Tag>{copyText(locale, 'false')}</Tag>;
  if (typeof value === 'object') return <Tooltip title={JSON.stringify(value)}><Text ellipsis>{JSON.stringify(value)}</Text></Tooltip>;
  const text = String(value);
  return text.length > 36 ? <Tooltip title={text}><Text copyable ellipsis>{text}</Text></Tooltip> : text;
}

function formatDateTime(value: unknown, locale: 'zh-CN' | 'en-US' = 'zh-CN'): string {
  const ms = dateMs(value);
  if (ms === undefined) return formatCell(value);
  const date = new Date(ms);
  const pad = (part: number) => String(part).padStart(2, '0');
  if (locale === 'en-US') return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`;
  return `${date.getFullYear()}年${pad(date.getMonth() + 1)}月${pad(date.getDate())}日 ${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`;
}

function formatCell(value: unknown): string {
  if (value === undefined || value === null || value === '') return '-';
  if (typeof value === 'object') return JSON.stringify(value);
  return String(value);
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

function humanizeKey(key: string): string {
  return key.replace(/_/g, ' ').replace(/\b\w/g, (letter) => letter.toUpperCase());
}

function describeRecord(record: object, keys: string[]) {
  const locale = useSessionStore.getState().locale;
  const source = record as Record<string, unknown>;
  return keys.map((key) => ({ key, label: fieldLabel(locale, key) ?? humanizeKey(key), children: renderCell(key, source[key], locale) }));
}

function localizeStatus(locale: Locale, value?: string): string {
  if (!value) return '-';
  return statusLabel(locale, value) ?? value;
}

function codeRowsOf(detail: RedeemCodeBatch) {
  return (detail.codes ?? []).map((code, index) => {
    const raw = typeof code === 'string' ? { code } : code;
    return {
      id: raw.id ?? raw.code_id ?? raw.code ?? String(index + 1),
      index: index + 1,
      code_preview: raw.code_preview ?? raw.code ?? '',
      code: raw.code ?? raw.code_preview ?? raw.code_id ?? '',
    };
  });
}

function errorMessage(error: unknown): string {
  if (error instanceof ApiError) return `HTTP ${error.status}: ${error.message}`;
  if (error instanceof Error) return error.message;
  if (typeof error === 'object' && error !== null && 'message' in error) return String((error as { message?: unknown }).message);
  return copyText(useSessionStore.getState().locale, '操作失败');
}

function jobIdOf(job: ConversionJob): string {
  return String(job.job_id ?? job.id ?? '');
}

async function pollConversion(api: Tex2DocApi, jobId: string, log: (line: string) => void): Promise<ConversionJob> {
  const locale = useSessionStore.getState().locale;
  const t = messages[locale];
  for (let i = 0; i < 120; i += 1) {
    const job = await api.getConversion(jobId);
    log(t.dynamic.polling(i + 1, localizeStatus(locale, job.status)));
    if (job.status === 'completed') return job;
    if (job.status === 'failed' || job.status === 'expired') throw new Error(job.error_message ?? job.error_code ?? job.status);
    await new Promise((resolve) => window.setTimeout(resolve, 1000));
  }
  throw new Error(locale === 'zh-CN' ? '云端转换轮询超时。' : 'Cloud conversion polling timed out.');
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
