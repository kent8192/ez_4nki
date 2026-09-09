import assert from 'node:assert/strict';
import { spawn, execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';

assert.equal(process.env.CI, 'true', 'Run only in an isolated CI account.');
assert(['linux', 'win32'].includes(process.platform));
const windows = process.platform === 'win32';
const temporary = mkdtempSync(join(tmpdir(), 'kotoba-native-'));
const env = { ...process.env };
if (!windows) env.XDG_DATA_HOME = join(temporary, 'data');
if (windows) env.TAURI_WEBVIEW_AUTOMATION = 'true';
const dataRoot = windows ? env.LOCALAPPDATA : env.XDG_DATA_HOME;
assert(dataRoot);
const directory = join(dataRoot, 'local.kotoba.desktop');
assert(!existsSync(directory), 'Refusing to use an existing application data directory.');
const fixture = (mode) => execFileSync('cargo', ['run', '--locked', '--release', '-p', 'kotoba-core', '--example', 'native_fixture', '--', mode, directory], { env, stdio: 'inherit' });
fixture('seed');
const application = process.env.KOTOBA_TEST_APPLICATION;
assert(application && existsSync(application), 'Specify the installed application path.');
const artifacts = resolve('artifacts/native');
mkdirSync(artifacts, { recursive: true });
const driverArgs = ['--port', '4444', '--native-port', '4445'];
const driver = windows
  ? spawn(resolve('src-tauri/runtime/driver/msedgedriver.exe'), ['--port=4444', '--verbose', `--log-path=${join(artifacts, 'msedgedriver.log')}`], { env, stdio: ['ignore', 'pipe', 'pipe'] })
  : spawn('strace', ['--kill-on-exit', '-f', '-s', '1', '-e', 'trace=network', '-o', join(artifacts, 'network-linux.log'), 'tauri-driver', ...driverArgs], { env, detached: true, stdio: ['ignore', 'pipe', 'pipe'] });
let driverLog = '';
driver.stdout.on('data', (data) => { driverLog += data; });
driver.stderr.on('data', (data) => { driverLog += data; });
driver.on('error', (error) => { driverLog += error.message; });
let session;
let applicationProcess;
let applicationLog = '';

function stopWindowsProcess(child) {
  if (!child) return;
  if (child.pid && child.exitCode === null) {
    try {
      execFileSync('taskkill', ['/PID', String(child.pid), '/T', '/F'], { stdio: 'ignore', timeout: 10000 });
    } catch (error) {
      applicationLog += `Process cleanup: ${error.message}\n`;
    }
  }
  child.stdout?.destroy();
  child.stderr?.destroy();
  child.unref();
}

async function request(method, path, body, timeout = 45000) {
  try {
    const response = await fetch(`http://127.0.0.1:4444${path}`, {
      method,
      headers: { 'content-type': 'application/json' },
      body: body === undefined ? undefined : JSON.stringify(body),
      signal: AbortSignal.timeout(timeout),
    });
    const result = await response.json();
    if (!response.ok || result.value?.error) throw new Error(JSON.stringify(result));
    return result.value;
  } catch (error) {
    throw new Error(`${method} ${path}: ${error.message}`, { cause: error });
  }
}
async function until(operation, description, timeout = 20000) {
  const deadline = Date.now() + timeout;
  let error;
  do {
    try {
      const value = await operation();
      if (value) return value;
    } catch (caught) { error = caught; }
    await delay(200);
  } while (Date.now() < deadline);
  throw new Error(`Timed out: ${description}`, { cause: error });
}
const route = (path) => `/session/${session}${path}`;
const elementKey = 'element-6066-11e4-a52e-4f735466cecf';
const elements = (selector) => request('POST', route('/elements'), { using: 'css selector', value: selector });
const element = (selector) => until(async () => (await elements(selector))[0], selector);
const text = (value) => request('GET', route(`/element/${value[elementKey]}/text`));
const click = (value) => request('POST', route(`/element/${value[elementKey]}/click`), {});
const button = (name) => until(async () => {
  for (const value of await elements('button')) {
    if ((await text(value)).includes(name)) return value;
  }
}, `button ${name}`);
const clickButton = async (name) => click(await button(name));
const fill = async (selector, value) => {
  const input = await element(selector);
  await request('POST', route(`/element/${input[elementKey]}/clear`), {});
  await request('POST', route(`/element/${input[elementKey]}/value`), { text: value });
};
const bodyContains = (value) => until(async () => (await text(await element('body'))).includes(value), value);
const screenshot = async (name) => writeFileSync(join(artifacts, `${name}.png`), Buffer.from(await request('GET', route('/screenshot')), 'base64'));
async function openSession() {
  if (windows) {
    // Attach to an explicit loopback port: the driver's launch mode cannot
    // discover DevToolsActivePort with this installed WebView2 application.
    // These arguments exist only in the isolated CI process environment.
    applicationProcess = spawn(application, [], {
      env: {
        ...env,
        WEBVIEW2_USER_DATA_FOLDER: join(temporary, 'webview-profile'),
        WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=9222 --remote-debugging-address=127.0.0.1 --enable-logging --log-file="${join(artifacts, 'webview2.log')}"`,
      },
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    applicationProcess.stdout.on('data', (data) => { applicationLog += data; });
    applicationProcess.stderr.on('data', (data) => { applicationLog += data; });
    applicationProcess.on('error', (error) => { applicationLog += `${error.message}\n`; });
    applicationProcess.on('exit', (code, signal) => { applicationLog += `Application exited: code=${code}, signal=${signal}\n`; });
    await until(async () => {
      assert.equal(applicationProcess.exitCode, null, `Application exited before exposing its WebView2: ${applicationLog}`);
      const response = await fetch('http://127.0.0.1:9222/json/version', { signal: AbortSignal.timeout(2000) });
      if (!response.ok) return false;
      const version = await response.json();
      writeFileSync(join(artifacts, 'webview2-debug-version.json'), JSON.stringify(version, null, 2));
      return true;
    }, 'installed application WebView2 startup');
  }
  const capabilities = windows
    ? { browserName: 'webview2', 'ms:edgeChromium': true, 'ms:edgeOptions': { debuggerAddress: '127.0.0.1:9222' } }
    : { 'tauri:options': { application } };
  const result = await request('POST', '/session', { capabilities: { alwaysMatch: capabilities } }, 120000);
  session = result.sessionId;
  assert(session);
  if (windows) {
    const runtime = JSON.parse(readFileSync('scripts/webview2-runtime.json', 'utf8'));
    assert.equal(result.capabilities.browserVersion, runtime.version);
    writeFileSync(join(artifacts, 'webview2-capabilities.json'), JSON.stringify(result.capabilities, null, 2));
  }
  await bodyContains('Synthetic native verification');
  await until(() => request('POST', route('/execute/sync'), {
    script: 'return Array.from(document.fonts).some(font => font.family.includes("Kotoba Noto Sans JP") && font.status === "loaded");',
    args: [],
  }), 'bundled Japanese font');
}
async function closeSession() {
  try {
    if (session) await request('DELETE', route(''));
  } finally {
    session = undefined;
    if (windows) {
      applicationLog += 'Stopping the CI application process after its session.\n';
      stopWindowsProcess(applicationProcess);
      applicationProcess = undefined;
    }
  }
}

try {
  await until(() => request('GET', '/status'), 'native driver startup');
  await openSession();
  await clickButton('学習をはじめる');
  await bodyContains('What is 2 + 3?');
  assert.equal((await elements('.answer-area')).length, 0, 'Answer must stay hidden before reveal.');
  await clickButton('答えを見る');
  assert.equal(await text(await element('.answer-text')), '5 <img src=x>');
  assert.equal((await elements('.answer-area img')).length, 0, 'CSV HTML must remain text.');
  await screenshot('answer');
  await clickButton('普通');
  await bodyContains('Second question');
  await clickButton('直前の自己評価を取り消す');
  await bodyContains('What is 2 + 3?');
  assert.equal((await elements('.answer-area')).length, 0);
  await clickButton('答えを見る');
  await clickButton('普通');
  await bodyContains('Second question');
  await click(await element('button[aria-label="閉じる"]'));
  await clickButton('今日だけ上乗せ');
  await fill('[role="dialog"] input[type="number"]', '10');
  await clickButton('上乗せする');
  await bodyContains('今日の新規上限 30枚');
  await clickButton('学習の分析');
  await element('svg[aria-label="30日分の新規学習と復習枚数の予測"]');
  const option = await element('select[aria-label="忘却曲線のカード"] option:nth-child(2)');
  await click(option);
  await element('svg[aria-label="今から30日間の推定忘却曲線"]');
  await screenshot('analytics');
  await closeSession();
  fixture('check');
  await openSession();
  await bodyContains('今日の新規上限 30枚');
  await clickButton('学習をはじめる');
  await bodyContains('Second question');
  await screenshot('restart');
  await closeSession();
  fixture('check');
  console.log(`Installed native UI passed on ${process.platform}: reveal, literal text, grade, undo, bonus, forecast, forgetting curve, restart persistence.`);
} catch (error) {
  if (windows && applicationProcess?.pid) {
    try {
      execFileSync('pwsh.exe', ['-NoProfile', '-NonInteractive', '-File', 'scripts/capture-native-windows.ps1', '-ApplicationProcessId', String(applicationProcess.pid), '-Destination', artifacts], { env, stdio: 'pipe', timeout: 25000 });
    } catch (diagnosticError) {
      applicationLog += `Native diagnostics: ${diagnosticError.message}\n`;
    }
  }
  if (session) {
    try { await screenshot('failure'); } catch {}
    try { writeFileSync(join(artifacts, 'failure.html'), await request('GET', route('/source'))); } catch {}
  }
  throw error;
} finally {
  try { await closeSession(); } catch {}
  if (windows) {
    stopWindowsProcess(driver);
  } else if (!windows && driver.pid) {
    try { process.kill(-driver.pid, 'SIGTERM'); } catch (error) {
      if (error.code !== 'ESRCH') throw error;
    }
  }
  if (driver.exitCode === null) {
    await Promise.race([new Promise((done) => driver.once('exit', done)), delay(3000)]);
  }
  if (!windows && driver.pid) {
    try { process.kill(-driver.pid, 'SIGKILL'); } catch (error) {
      if (error.code !== 'ESRCH') throw error;
    }
  }
  // Native helpers may retain inherited pipe handles after the driver exits.
  // The test has already closed its session; release those handles explicitly.
  driver.stdout.destroy();
  driver.stderr.destroy();
  driver.unref();
  writeFileSync(join(artifacts, 'driver.log'), driverLog);
  if (windows) writeFileSync(join(artifacts, 'application.log'), applicationLog);
}

if (!windows) {
  const trace = readFileSync(join(artifacts, 'network-linux.log'), 'utf8');
  const outbound = trace.split('\n').filter((line) => {
    if (!/\b(connect|sendto|sendmsg)\(/.test(line)) return false;
    const addresses = [...line.matchAll(/(?:inet_addr\("|inet_pton\(AF_INET6, ")([^"]+)/g)].map((match) => match[1]);
    return addresses.some((address) => !address.startsWith('127.') && address !== '::1');
  });
  assert.deepEqual(outbound, [], 'Observed an external network destination in the traced native process tree.');
  console.log('No external IP destination observed in the traced Linux driver/application process tree.');
}
