import { useEffect, useSyncExternalStore } from 'react';
import './App.css';
import { synchClient } from './SynchClient';

function App() {
  const state = useSyncExternalStore(
    (listener) => synchClient.subscribe(listener),
    () => synchClient.state
  );
  
  const nodeId = useSyncExternalStore(
    (listener) => synchClient.subscribe(listener),
    () => synchClient.nodeId
  );

  useEffect(() => {
    synchClient.connect();
    return () => synchClient.disconnect();
  }, []);

  return (
    <div className="app-container">
      <div className="glass-panel">
        <header className="panel-header">
          <div className="logo">
            <div className="logo-icon"></div>
            <h1>Synch Agent</h1>
          </div>
          <div className={`status-badge status-${state}`}>
            <span className="pulse-dot"></span>
            {state.charAt(0).toUpperCase() + state.slice(1)}
          </div>
        </header>

        <main className="panel-body">
          <div className="info-card">
            <div className="card-label">Node Identity</div>
            <div className="card-value font-mono">{nodeId}</div>
          </div>

          <div className="info-card">
            <div className="card-label">Target Server</div>
            <div className="card-value">ws://localhost:8081</div>
          </div>
        </main>

        <footer className="panel-footer">
          <button 
            className={`action-btn ${state === 'connected' ? 'btn-disconnect' : 'btn-connect'}`}
            onClick={() => state === 'disconnected' ? synchClient.connect() : synchClient.disconnect()}
          >
            {state === 'disconnected' ? 'Connect to Server' : 'Disconnect'}
          </button>
        </footer>
      </div>
    </div>
  );
}

export default App;
