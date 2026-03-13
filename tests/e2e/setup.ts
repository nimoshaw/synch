import { execSync } from 'child_process';
import { resolve } from 'path';

const projectRoot = resolve(__dirname, '../../');

export async function setup() {
  console.log('🚀 Starting e2e test environment...');
  try {
    // 强制清理可能存在的旧容器
    console.log('Cleaning up old containers...');
    execSync('docker compose -f docker-compose.test.yml down -v', { cwd: projectRoot, stdio: 'inherit' });
    
    console.log('Building and starting new containers...');
    execSync('docker compose -f docker-compose.test.yml up -d --build', { cwd: projectRoot, stdio: 'inherit' });
    
    // 等待系统就绪
    console.log('Waiting for relay server to be healthy...');
    let retries = 10;
    let healthy = false;
    while (retries > 0 && !healthy) {
      try {
        const result = execSync('docker ps --filter "name=relay-test" --format "{{.Status}}"').toString();
        if (result.includes('healthy')) {
          healthy = true;
          break;
        }
      } catch (e) {
        // ignore
      }
      retries--;
      await new Promise(r => setTimeout(r, 2000));
    }
    
    if (!healthy) {
      throw new Error('Relay server failed to become healthy');
    }
    console.log('✅ Environment is ready');
  } catch (err) {
    console.error('Failed to start test environment:', err);
    throw err;
  }
}

export async function teardown() {
  console.log('🧹 Tearing down e2e test environment...');
  try {
    execSync('docker compose -f docker-compose.test.yml down -v', { cwd: projectRoot, stdio: 'inherit' });
    console.log('✅ Environment cleaned up');
  } catch (err) {
    console.error('Failed to teardown environment:', err);
  }
}
