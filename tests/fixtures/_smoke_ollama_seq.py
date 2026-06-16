import requests, time
s = requests.Session()
url = 'http://127.0.0.1:11434/v1/chat/completions'
headers = {'Content-Type': 'application/json'}
proxies = {'http': None, 'https': None}
# 模拟预热
print('warmup...', flush=True)
t = time.time()
r1 = s.post(url, headers=headers, json={'model': 'gemma4:e4b', 'messages': [{'role': 'user', 'content': 'ok'}], 'max_tokens': 4, 'temperature': 0, 'stream': False}, timeout=300, proxies=proxies)
r1.raise_for_status()
c1 = r1.json()['choices'][0]['message']['content']
print(f'warmup done in {time.time()-t:.1f}s: {c1!r}', flush=True)
# 主调用:模拟真实场景
print('main...', flush=True)
t = time.time()
r2 = s.post(url, headers=headers, json={'model': 'gemma4:e4b', 'messages': [{'role': 'system', 'content': '你是简洁的中文 commit 助手'}, {'role': 'user', 'content': '请描述 1+1=?'}], 'max_tokens': 50, 'temperature': 0.4, 'stream': False}, timeout=300, proxies=proxies)
r2.raise_for_status()
c2 = r2.json()['choices'][0]['message']['content']
print(f'main done in {time.time()-t:.1f}s: {c2!r}', flush=True)
