import requests
try:
    resp = requests.post(
        'https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions',
        headers={'Authorization': 'Bearer sk-REDACTED-DASHSCOPE'},
        json={'model': 'deepseek-v3', 'messages': [{'role': 'user', 'content': 'hi'}]},
        proxies={'http': 'http://127.0.0.1:7890', 'https': 'http://127.0.0.1:7890'},
        timeout=10
    )
    print("Proxy 7890 status:", resp.status_code)
    print("Proxy 7890 response:", resp.text)
    resp.raise_for_status()
except Exception as e:
    print("Proxy 7890 error type:", type(e).__name__)
    print("Proxy 7890 error:", repr(e))
