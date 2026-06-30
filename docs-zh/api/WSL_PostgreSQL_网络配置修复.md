# WSL PostgreSQL 从 Windows 访问配置

## 环境

| 项目 | 值 |
|------|-----|
| WSL 发行版 | Ubuntu 24.04 |
| PostgreSQL 版本 | 17 |
| 数据库 | docdb |
| 用户/密码 | postgres / postgres |
| WSL 网络模式 | mirrored（镜像网络） |

## 配置步骤

### 1. WSL 镜像网络模式（关键一步）

在 `C:\Users\<用户名>\.wslconfig` 中配置：

```ini
[wsl2]
networkingMode=mirrored
[experimental]
hostAddressLoopback=true
autoProxy=true
```

作用：使 WSL 与 Windows 共享同一 IP，WSL 中的端口直接在 `127.0.0.1` 上可达。

### 2. PostgreSQL 监听所有地址

修改 `/etc/postgresql/17/main/postgresql.conf`：

```ini
listen_addresses = '*'
port = 5432
```

### 3. pg_hba.conf 允许远程连接

在 `/etc/postgresql/17/main/pg_hba.conf` 中添加：

```ini
host all all 192.168.0.0/24 md5
```

### 4. 重启 WSL 和 PostgreSQL

```powershell
# PowerShell (管理员)
wsl --shutdown
wsl
wsl sudo systemctl restart postgresql
```

## 连接信息

| 参数 | 值 |
|------|-----|
| Host | `127.0.0.1` 或 `localhost` |
| Port | `5432` |
| User | `postgres` |
| Password | `postgres` |
| Database | `docdb` |

## 验证方法

```powershell
# PowerShell 验证端口可达
Test-NetConnection -ComputerName localhost -Port 5432
```

或使用 Python 测试：

```powershell
pip install psycopg2-binary
python -c "
import psycopg2
conn = psycopg2.connect(host='127.0.0.1', port=5432, user='postgres',
                        password='postgres', dbname='docdb')
cur = conn.cursor()
cur.execute('SELECT version()')
print('Connected!', cur.fetchone()[0])
cur.close()
conn.close()
"
```

## 关键点

- 核心是 WSL 使用 **mirrored** 网络模式（需 Windows 22H2+ 和 WSL 0.9+）
- 镜像模式下 WSL 与 Windows 共享 IP，本地回环地址 `127.0.0.1` 即可访问 WSL 服务
- 无需额外端口转发或防火墙规则
- 如果使用传统 NAT 模式，则需要通过 `wsl hostname -I` 获取 WSL IP，或配置端口转发
