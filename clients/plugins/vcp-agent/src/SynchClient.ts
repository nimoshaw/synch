type ConnectionState = "disconnected" | "connecting" | "connected";

class SynchClient {
  private ws: WebSocket | null = null;
  private url: string;
  private listeners: Set<() => void> = new Set();
  
  public state: ConnectionState = "disconnected";
  public nodeId: string = "pending...";

  constructor(url: string) {
    this.url = url;
  }

  public connect() {
    if (this.ws && (this.ws.readyState === WebSocket.CONNECTING || this.ws.readyState === WebSocket.OPEN)) {
        return;
    }

    this.state = "connecting";
    this.notify();
    try {
      this.ws = new WebSocket(this.url);
      
      this.ws.onopen = () => {
        this.state = "connected";
        // Generates a mock NodeID for demonstration. In a real scenario, this might come from the server or local storage.
        this.nodeId = "plugin://vcp-" + Math.random().toString(36).substring(2, 10);
        this.notify();
      };

      this.ws.onclose = () => {
        this.state = "disconnected";
        this.nodeId = "disconnected";
        this.notify();
      };

      this.ws.onerror = (error) => {
        console.error("SynchClient WebSocket error:", error);
        this.state = "disconnected";
        this.notify();
      };

      this.ws.onmessage = async (event) => {
        // Normally we decode protobuf here using @bufbuild/protobuf
        // e.g., const message = SyncMessage.decode(new Uint8Array(await event.data.arrayBuffer()));
        console.log("SynchClient received message format:", typeof event.data);
      };
    } catch (e) {
      console.error("Failed to connect:", e);
      this.state = "disconnected";
      this.notify();
    }
  }

  public disconnect() {
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
    this.state = "disconnected";
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

export const synchClient = new SynchClient("ws://localhost:8081");
