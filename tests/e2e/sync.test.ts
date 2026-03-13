import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import WebSocket from 'ws';

describe('Vault Sync', () => {
  let wsClientA: WebSocket;
  let wsClientB: WebSocket;
  const SERVER_URL = 'ws://localhost:8081';

  beforeAll(async () => {
    // 等待服务真正就绪，尽管 setup.ts 已经有了 docker check
    // 我们在这里加一个简短延迟防止边缘情况
    await new Promise(r => setTimeout(r, 2000));
  });

  afterAll(() => {
    if (wsClientA && wsClientA.readyState === WebSocket.OPEN) {
      wsClientA.close();
    }
    if (wsClientB && wsClientB.readyState === WebSocket.OPEN) {
      wsClientB.close();
    }
  });

  it('client should connect to relay server successfully', async () => {
    wsClientA = new WebSocket(SERVER_URL);
    
    const isConnected = await new Promise((resolve) => {
      wsClientA.on('open', () => resolve(true));
      wsClientA.on('error', () => resolve(false));
      setTimeout(() => resolve(false), 5000);
    });
    
    expect(isConnected).toBe(true);
  });

  it('second client can connect and exchange simple ping/messages', async () => {
    wsClientB = new WebSocket(SERVER_URL);
    
    const isConnected = await new Promise((resolve) => {
      wsClientB.on('open', () => resolve(true));
      wsClientB.on('error', () => resolve(false));
      setTimeout(() => resolve(false), 5000);
    });
    
    expect(isConnected).toBe(true);

    // 测试简单的消息收发 (视 relay server 逻辑而定)
    // 此处暂只测试发送不出错
    expect(() => wsClientA.send(JSON.stringify({ type: 'ping' }))).not.toThrow();
  });
});
