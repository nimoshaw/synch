type ConnectionState = "disconnected" | "connecting" | "connected" | "reconnecting";

class SynchClient {
  private ws: WebSocket | null = null;
  private url: string;
  private listeners: Set<() => void> = new Set();
  private reconnectAttempts: number = 0;
  private maxReconnectDelay: number = 30000; // 30s
  private baseReconnectDelay: number = 1000; // 1s
  private reconnectTimer: number | null = null;
  
  public state: ConnectionState = "disconnected";
  public nodeId: string = "pending...";

  constructor(url: string) {
    this.url = url;
  }

  public connect() {
    if (this.ws && (this.ws.readyState === WebSocket.CONNECTING || this.ws.readyState === WebSocket.OPEN)) {
        return;
    }

    if (this.state !== "reconnecting") {
        this.state = "connecting";
    }
    
    this.notify();
    console.log(`[SynchClient] Connecting to ${this.url} (Attempt ${this.reconnectAttempts + 1})...`);

    try {
      this.ws = new WebSocket(this.url);
      this.ws.binaryType = "arraybuffer";
      
      this.ws.onopen = () => {
        console.log("[SynchClient] WebSocket connected");
        this.state = "connected";
        this.reconnectAttempts = 0;
        this.nodeId = "plugin://vcp-" + Math.random().toString(36).substring(2, 10);
        this.notify();
      };

      this.ws.onclose = (event) => {
        console.log(`[SynchClient] WebSocket closed: ${event.code} ${event.reason}`);
        this.state = "disconnected";
        this.nodeId = "disconnected";
        this.notify();
        this.scheduleReconnect();
      };

      this.ws.onerror = (error) => {
        console.error("[SynchClient] WebSocket error:", error);
      };

      this.ws.onmessage = async (event) => {
        console.log("[SynchClient] Received message, size:", event.data.byteLength);
        // Handled via protobuf in real implementation
      };
    } catch (e) {
      console.error("[SynchClient] Failed to create WebSocket:", e);
      this.state = "disconnected";
      this.scheduleReconnect();
    }
  }

  private scheduleReconnect() {
    if (this.reconnectTimer) return;

    this.state = "reconnecting";
    const delay = Math.min(
        this.baseReconnectDelay * Math.pow(2, this.reconnectAttempts),
        this.maxReconnectDelay
    );
    
    console.log(`[SynchClient] Scheduling reconnect in ${delay}ms`);
    this.reconnectTimer = window.setTimeout(() => {
        this.reconnectTimer = null;
        this.reconnectAttempts++;
        this.connect();
    }, delay);
    this.notify();
  }

  public disconnect() {
    if (this.reconnectTimer) {
        clearTimeout(this.reconnectTimer);
        this.reconnectTimer = null;
    }
    if (this.ws) {
      this.ws.onclose = null; // Prevent auto-reconnect on manual disconnect
      this.ws.close();
      this.ws = null;
    }
    this.state = "disconnected";
    this.reconnectAttempts = 0;
    this.notify();
  }

  public subscribe(listener: () => void) {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  private notify() {
    this.listeners.forEach((l) => l());
  }
}

export const synchClient = new SynchClient("ws://localhost:8080/ws");
