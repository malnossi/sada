/**
 * Sada IPC — Typed wrappers for Tauri 2 commands & events
 */

import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

/* ── Types ───────────────────────────────── */

export interface ServerConfig {
  id: string;
  name: string;
  server_type: 'icecast' | 'shoutcast';
  host: string;
  port: number;
  password: string;
  admin_password?: string;
  username?: string;
  legacy_icecast?: boolean;
  custom_listener_url?: string;
  custom_listener_mount?: string;
  mount_point: string;
  stream_name: string;
  stream_description: string;
  stream_genre: string;
  stream_url: string;
  public_server: boolean;
  tls?: boolean;
}

export interface AudioConfig {
  device_name: string;
  codec: 'mp3' | 'opus' | 'ogg_vorbis' | 'aac' | 'aac_plus';
  bitrate: number;
  sample_rate: number;
  channels: number;
}

export interface RecordingConfig {
  enabled: boolean;
  output_path: string;
  format: 'mp3' | 'wav' | 'ogg';
}

export interface MetadataConfig {
  update_from_file: boolean;
  file_path: string;
  poll_interval_secs: number;
}

export interface AppConfig {
  servers: ServerConfig[];
  selected_server_id: string;
  audio: AudioConfig;
  recording: RecordingConfig;
  metadata: MetadataConfig;
}

export interface VuMeterData {
  left: number;
  right: number;
  peak_left: number;
  peak_right: number;
}

export type ConnectionStatusType = 'Idle' | 'Connecting' | 'Connected' | 'Reconnecting' | 'Error';

export interface ConnectionStatus {
  type: ConnectionStatusType;
  data?: string;
}

export interface StreamStats {
  duration_secs: number;
  bytes_sent: number;
  kbps: number;
}

export interface AppError {
  message: string;
  level: 'warn' | 'error';
}

/* ── Commands ────────────────────────────── */

export async function getConfig(): Promise<AppConfig> {
  return invoke<AppConfig>('get_config');
}

export async function saveConfig(config: AppConfig): Promise<void> {
  return invoke('save_config_cmd', { config });
}

export async function getAudioDevices(): Promise<string[]> {
  return invoke<string[]>('get_audio_devices');
}

export async function connectStream(): Promise<void> {
  return invoke('connect');
}

export async function disconnectStream(): Promise<void> {
  return invoke('disconnect');
}

export async function getConnectionStatus(): Promise<ConnectionStatus> {
  return invoke<ConnectionStatus>('get_connection_status');
}

export async function updateMetadata(song: string): Promise<void> {
  return invoke('update_metadata', { song });
}

export async function startRecording(path: string): Promise<void> {
  return invoke('start_recording', { path });
}

export async function stopRecording(): Promise<void> {
  return invoke('stop_recording');
}

export async function startMonitor(deviceName: string | null): Promise<void> {
  return invoke('start_monitor', { deviceName });
}

export async function stopMonitor(): Promise<void> {
  return invoke('stop_monitor');
}

/* ── Event Listeners ─────────────────────── */

export function onVuMeter(callback: (data: VuMeterData) => void): Promise<UnlistenFn> {
  return listen<VuMeterData>('vu-meter', (event) => {
    callback(event.payload);
  });
}

export function onConnectionStatus(callback: (status: ConnectionStatus) => void): Promise<UnlistenFn> {
  return listen<ConnectionStatus>('connection-status', (event) => {
    callback(event.payload);
  });
}

export function onStreamStats(callback: (stats: StreamStats) => void): Promise<UnlistenFn> {
  return listen<StreamStats>('stream-stats', (event) => {
    callback(event.payload);
  });
}

export function onAppError(callback: (error: AppError) => void): Promise<UnlistenFn> {
  return listen<AppError>('app-error', (event) => {
    callback(event.payload);
  });
}

export function onMetadataChanged(callback: (data: { current: string }) => void): Promise<UnlistenFn> {
  return listen<{ current: string }>('metadata-changed', (event) => {
    callback(event.payload);
  });
}
