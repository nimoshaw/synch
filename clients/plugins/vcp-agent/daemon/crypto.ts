/**
 * crypto.ts — VCP-Agent E2EE 加密模块
 *
 * 使用 @noble/curves (X25519 ECDH) + @noble/ciphers (AES-256-GCM) 实现
 * 端到端加密消息的密钥派生、加密和解密。
 *
 * 当前阶段：静态密钥对 + ECDH，暂不实现 Double Ratchet 前向保密。
 */

import { x25519 } from '@noble/curves/ed25519.js';
import { gcm } from '@noble/ciphers/aes';
import { hkdf } from '@noble/hashes/hkdf.js';
import { sha256 } from '@noble/hashes/sha2.js';
import { randomBytes } from '@noble/hashes/utils.js';
import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';

// ─── Types ───────────────────────────────────────────────────────────────────

export interface KeyPair {
  publicKey: Uint8Array;   // 32 bytes X25519 public key
  secretKey: Uint8Array;   // 32 bytes X25519 secret key
}

export interface EncryptResult {
  ciphertext: Uint8Array;
  nonce: Uint8Array;       // 12 bytes
}

// ─── Key Management ──────────────────────────────────────────────────────────

const SYNCH_DIR = path.join(os.homedir(), '.synch');
const KEYS_FILE = path.join(SYNCH_DIR, 'exchange_keys.json');

/**
 * 生成新的 X25519 密钥对
 */
export function generateX25519KeyPair(): KeyPair {
  const secretKey = x25519.utils.randomSecretKey();
  const publicKey = x25519.getPublicKey(secretKey);
  return { publicKey, secretKey };
}

/**
 * 加载或创建持久化的密钥交换密钥对
 * 存储在 ~/.synch/exchange_keys.json
 */
export function loadOrCreateExchangeKeys(): KeyPair {
  try {
    if (fs.existsSync(KEYS_FILE)) {
      const data = JSON.parse(fs.readFileSync(KEYS_FILE, 'utf-8'));
      return {
        publicKey: Buffer.from(data.publicKey, 'hex'),
        secretKey: Buffer.from(data.secretKey, 'hex'),
      };
    }
  } catch (e) {
    console.warn('[Crypto] Failed to load exchange keys, generating new ones:', e);
  }

  const kp = generateX25519KeyPair();
  
  // Persist
  if (!fs.existsSync(SYNCH_DIR)) {
    fs.mkdirSync(SYNCH_DIR, { recursive: true });
  }
  fs.writeFileSync(KEYS_FILE, JSON.stringify({
    publicKey: Buffer.from(kp.publicKey).toString('hex'),
    secretKey: Buffer.from(kp.secretKey).toString('hex'),
  }, null, 2), { mode: 0o600 });
  
  console.log('[Crypto] Generated new X25519 exchange key pair');
  return kp;
}

// ─── Key Derivation ──────────────────────────────────────────────────────────

/**
 * X25519 ECDH + HKDF-SHA256 → 32-byte AES-256 key
 *
 * @param mySecretKey   本节点 X25519 私钥 (32 bytes)
 * @param theirPublicKey 对端 X25519 公钥 (32 bytes)
 * @param info          HKDF info 字符串 (用于上下文绑定)
 * @returns             32-byte symmetric key for AES-256-GCM
 */
export function deriveSharedKey(
  mySecretKey: Uint8Array,
  theirPublicKey: Uint8Array,
  info: string = 'synch-e2ee-v1'
): Uint8Array {
  // Step 1: X25519 ECDH → shared secret
  const sharedSecret = x25519.getSharedSecret(mySecretKey, theirPublicKey);
  
  // Step 2: HKDF-SHA256 (no salt, context-bound info)
  const infoBytes = new TextEncoder().encode(info);
  const derivedKey = hkdf(sha256, sharedSecret, undefined, infoBytes, 32);
  
  return derivedKey;
}

// ─── Encryption / Decryption ─────────────────────────────────────────────────

/**
 * AES-256-GCM 加密
 *
 * @param key        32-byte symmetric key
 * @param plaintext  明文字节
 * @returns          { ciphertext, nonce }
 */
export function encrypt(key: Uint8Array, plaintext: Uint8Array): EncryptResult {
  const nonce = randomBytes(12); // 96-bit nonce for AES-GCM
  const aes = gcm(key, nonce);
  const ciphertext = aes.encrypt(plaintext);
  return { ciphertext, nonce };
}

/**
 * AES-256-GCM 解密
 *
 * @param key        32-byte symmetric key
 * @param ciphertext 密文 (含 GCM tag)
 * @param nonce      12-byte nonce
 * @returns          解密后的明文
 * @throws           解密失败时抛出异常
 */
export function decrypt(key: Uint8Array, ciphertext: Uint8Array, nonce: Uint8Array): Uint8Array {
  const aes = gcm(key, nonce);
  return aes.decrypt(ciphertext);
}

// ─── Peer Key Store ──────────────────────────────────────────────────────────

/**
 * 管理已知对端节点的公钥映射
 */
export class PeerKeyStore {
  private keys = new Map<string, Uint8Array>();

  /**
   * 存储对端公钥
   */
  set(nodeId: string, publicKey: Uint8Array): void {
    this.keys.set(nodeId, publicKey);
  }

  /**
   * 获取对端公钥
   */
  get(nodeId: string): Uint8Array | undefined {
    return this.keys.get(nodeId);
  }

  /**
   * 检查是否有对端公钥
   */
  has(nodeId: string): boolean {
    return this.keys.has(nodeId);
  }
}
