import { 
  getConfig, 
  getConnectionStatus, 
  disconnectStream,
  connectStream,
  updateMetadata,
  onConnectionStatus, 
  onStreamStats, 
  onAppError,
  onMetadataChanged,
  saveConfig,
  startRecording,
  stopRecording,
  type AppConfig, 
  type ConnectionStatusType, 
  type StreamStats, 
  type AppError 
} from './ipc';

class AppStateStore {
  // Global Svelte 5 state variables
  activeTab = $state<'main' | 'server' | 'audio' | 'recording'>('main');
  config = $state<AppConfig | null>(null);
  configLoaded = $state(false);
  connectionStatus = $state<ConnectionStatusType>('Idle');
  connectionStatusData = $state<any>(null);
  streamStats = $state<StreamStats>({ duration_secs: 0, bytes_sent: 0, kbps: 0 });
  lastError = $state<AppError | null>(null);
  showError = $state(false);

  // Connection-specific states
  songTitle = $state('');
  connecting = $state(false);
  connectError = $state<string | null>(null);

  // Local recorder states
  isRecording = $state(false);
  recordingDuration = $state(0);
  recordingStartTime = 0;
  private recordingIntervalId: ReturnType<typeof setInterval> | null = null;

  // Status/state derived properties
  isActiveStreaming = $derived(
    this.connectionStatus === 'Connected' || 
    this.connectionStatus === 'Connecting' || 
    this.connectionStatus === 'Reconnecting'
  );

  isConnected = $derived(this.connectionStatus === 'Connected');

  // Event handler registration cleanup
  private unlistens: (() => void)[] = [];

  async init() {
    // Load config
    try {
      this.config = await getConfig();
    } catch (e) {
      console.error('Failed to load config:', e);
    } finally {
      this.configLoaded = true;
    }

    // Get initial status
    try {
      const initialStatus = await getConnectionStatus();
      this.connectionStatus = initialStatus.type;
      this.connectionStatusData = initialStatus.data;
      if (initialStatus.type === 'Error') {
        const errMsg = (initialStatus.data as any)?.message || 'Unknown connection error';
        this.lastError = { message: errMsg, level: 'error' };
        this.showError = true;
        setTimeout(() => (this.showError = false), 6000);
      }
    } catch (e) {
      console.error('Failed to get connection status:', e);
    }

    // Register event listeners
    const statusUnlisten = await onConnectionStatus((s) => {
      this.connectionStatus = s.type;
      this.connectionStatusData = s.data;
      if (s.type === 'Error') {
        const errMsg = (s.data as any)?.message || 'Unknown connection error';
        this.lastError = { message: errMsg, level: 'error' };
        this.showError = true;
        setTimeout(() => (this.showError = false), 6000);
      }
      if (s.type === 'Connected' && this.songTitle.trim()) {
        updateMetadata(this.songTitle).catch((e) => 
          console.error('Failed to auto-update metadata on connect:', e)
        );
      }
    });
    this.unlistens.push(statusUnlisten);

    const statsUnlisten = await onStreamStats((s) => {
      this.streamStats = s;
    });
    this.unlistens.push(statsUnlisten);

    const errorUnlisten = await onAppError(async (err) => {
      this.lastError = err;
      this.showError = true;
      setTimeout(() => (this.showError = false), 5000);
      if (err.level === 'error') {
        try {
          await disconnectStream();
        } catch (e) {
          console.error('Failed to disconnect on fatal capture error:', e);
        }
      }
    });
    this.unlistens.push(errorUnlisten);

    const metadataUnlisten = await onMetadataChanged((data) => {
      this.songTitle = data.current;
    });
    this.unlistens.push(metadataUnlisten);
  }

  destroy() {
    for (const unlisten of this.unlistens) {
      unlisten();
    }
    this.unlistens = [];
    if (this.recordingIntervalId) {
      clearInterval(this.recordingIntervalId);
      this.recordingIntervalId = null;
    }
  }

  async handleConnect() {
    this.connecting = true;
    this.connectError = null;
    try {
      await connectStream();
    } catch (e: any) {
      console.error('Connect failed:', e);
      this.connectError = e?.toString() || 'Failed to start connection';
    } finally {
      this.connecting = false;
    }
  }

  async handleDisconnect() {
    try {
      await disconnectStream();
    } catch (e) {
      console.error('Disconnect failed:', e);
    }
  }

  async handleMetadataUpdate() {
    if (!this.songTitle.trim()) return;
    try {
      await updateMetadata(this.songTitle);
    } catch (e) {
      console.error('Metadata update failed:', e);
    }
  }

  async handleServerChange() {
    if (!this.config) return;
    try {
      await saveConfig(this.config);
    } catch (e) {
      console.error('Failed to save config on server change:', e);
    }
  }

  async toggleRecording() {
    const recordingPath = this.config?.recording?.output_path || '';
    const recordingFormat = this.config?.recording?.format || 'mp3';

    if (this.isRecording) {
      try {
        await stopRecording();
      } catch (e) {
        console.error('Stop recording failed:', e);
      }
      this.isRecording = false;
      if (this.recordingIntervalId) {
        clearInterval(this.recordingIntervalId);
        this.recordingIntervalId = null;
      }
      this.recordingDuration = 0;
    } else {
      const fullPath = recordingPath || `~/recording.${recordingFormat}`;
      try {
        await startRecording(fullPath);
        this.isRecording = true;
        this.recordingStartTime = Date.now();
        this.recordingIntervalId = setInterval(() => {
          this.recordingDuration = Math.floor((Date.now() - this.recordingStartTime) / 1000);
        }, 250);
      } catch (e) {
        console.error('Start recording failed:', e);
      }
    }
  }
}

export const appState = new AppStateStore();
