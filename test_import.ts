import { SyncMessage } from './shared/ts-core/src/proto/v1/sync.ts';
console.log('SyncMessage export found:', typeof SyncMessage);
if (typeof SyncMessage === 'object') {
    console.log('Static methods available:', Object.keys(SyncMessage));
}
