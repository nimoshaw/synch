import { execSync, spawn, ChildProcess } from 'child_process';
import { resolve } from 'path';

const projectRoot = resolve(__dirname, '../../');
let relayProcess: ChildProcess | null = null;

export async function setup() {
  console.log('🚀 Starting e2e test environment...');
  
  // Try Docker first
  try {
    console.log('Checking for docker...');
    execSync('docker --version', { stdio: 'ignore' });
    
    console.log('Cleaning up old containers...');
    execSync('docker compose -f docker-compose.test.yml down -v', { cwd: projectRoot, stdio: 'inherit' });
    
    console.log('Building and starting new containers...');
    execSync('docker compose -f docker-compose.test.yml up -d --build', { cwd: projectRoot, stdio: 'inherit' });
    
    // Wait for system
    console.log('Waiting for relay server to be healthy...');
    let retries = 10;
    while (retries > 0) {
      try {
        const result = execSync('docker ps --filter "name=relay-test" --format "{{.Status}}"').toString();
        if (result.includes('healthy') || result.includes('Up')) {
          console.log('✅ Environment is ready (Docker)');
          return;
        }
      } catch (e) {}
      retries--;
      await new Promise(r => setTimeout(r, 2000));
    }
  } catch (err) {
    console.log('⚠️ Docker not available or failed, falling back to manual start...');
  }

  // Fallback: Start relay server manually
  try {
    console.log('Cleaning up existing processes on port 8081...');
    try {
      execSync('npx kill-port 8081');
    } catch (e) {}

    console.log('Starting relay server via go run...');
    relayProcess = spawn('go', ['run', 'cmd/relay/main.go', '-addr', ':8081', '-log', 'debug'], {
      cwd: resolve(projectRoot, 'server'),
      stdio: 'inherit',
      shell: true,
    });
    
    // Wait for it to start listening
    await new Promise(r => setTimeout(r, 3000));
    console.log('✅ Environment is ready (Manual)');
  } catch (err) {
    console.error('❌ Failed to start relay server manually:', err);
    throw err;
  }
}

export async function teardown() {
  console.log('🧹 Tearing down e2e test environment...');
  
  if (relayProcess) {
    console.log('Killing manual relay server...');
    relayProcess.kill();
  }

  try {
    execSync('docker --version', { stdio: 'ignore' });
    execSync('docker compose -f docker-compose.test.yml down -v', { cwd: projectRoot, stdio: 'inherit' });
  } catch (e) {
    // Ignore if docker fails
  }
  
  console.log('✅ Environment cleaned up');
}
