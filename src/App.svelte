<script lang="ts">
  import { onMount } from 'svelte';
  import VuMeter from './components/VuMeter.svelte';
  import ServerConfig from './components/ServerConfig.svelte';
  import AudioSettings from './components/AudioSettings.svelte';
  import RecordingPanel from './components/RecordingPanel.svelte';
  import RecordingLive from './components/RecordingLive.svelte';
  import StreamControls from './components/StreamControls.svelte';
  import { appState } from './lib/state.svelte';
  import logo from "./lib/assets/logo.png";

  /* ── Tabs ──────────────────────────────── */
  type TabId = 'main' | 'server' | 'audio' | 'recording';

  interface Tab {
    id: TabId;
    label: string;
    icon: string;
  }

  const tabs: Tab[] = [
    { 
      id: 'main', 
      label: 'Stream', 
      icon: 'M12 12h.01M14.83 9.17a4 4 0 010 5.66M9.17 14.83a4 4 0 010-5.66M17.66 6.34a8 8 0 010 11.32M6.34 17.66a8 8 0 010-11.32'
    },
    { id: 'server', label: 'Server Connection', icon: 'M5 12h14M5 12a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v4a2 2 0 01-2 2M5 12a2 2 0 00-2 2v4a2 2 0 002 2h14a2 2 0 002-2v-4a2 2 0 00-2-2m-2-4h.01M17 16h.01' },
    { id: 'audio', label: 'Audio Source', icon: 'M19 11a7 7 0 01-7 7m0 0a7 7 0 01-7-7m7 7v4m0 0H8m4 0h4m-4-8a3 3 0 01-3-3V5a3 3 0 116 0v6a3 3 0 01-3 3z' },
    { id: 'recording', label: 'Local Recording', icon: 'M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z' },
  ];

  function formatDuration(secs: number): string {
    const h = Math.floor(secs / 3600);
    const m = Math.floor((secs % 3600) / 60);
    const s = Math.floor(secs % 60);
    return `${h.toString().padStart(2, '0')}:${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}`;
  }

  const statusColors = {
    Idle: '#64748b',
    Connecting: '#f59e0b',
    Connected: '#10b981',
    Reconnecting: '#f59e0b',
    Error: '#f43f5e',
  };

  const statusDotClasses = {
    Idle: 'status-dot-idle',
    Connecting: 'status-dot-connecting',
    Connected: 'status-dot-connected',
    Reconnecting: 'status-dot-connecting',
    Error: 'status-dot-error',
  };

  /* ── Lifecycle ─────────────────────────── */
  onMount(() => {
    appState.init();
    return () => {
      appState.destroy();
    };
  });
</script>

<div class="flex flex-col h-screen overflow-hidden bg-gradient-to-br from-bg-base via-bg-surface to-bg-gradient-end">
  <div class="h-[2px] w-full bg-gradient-to-r from-blue-500 via-cyan-400 to-indigo-500 shrink-0"></div>
  <!-- ── Header ───────────────────────────── -->
  <header class="flex items-center justify-between px-4 pt-1.5 pb-1 border-b border-border-subtle bg-black/5 flex-shrink-0">
    <div class="flex items-center gap-3">
      <!-- Logo mark -->
      <div class="w-10 h-10 flex items-center justify-center">
        <img src={logo} alt="logo"/>
      </div>
      <div>
        <h1 class="text-xl font-bold text-text-primary tracking-tight">Sada</h1>
      </div>
    </div>

    <!-- Navigation tabs inside the header to optimize screen real estate -->
    {#if appState.configLoaded}
      <nav class="nav-capsule ml-auto mr-4">
        {#each tabs as tab}
          <button
            class="tab-button-capsule {appState.activeTab === tab.id ? 'active' : ''}"
            onclick={() => (appState.activeTab = tab.id)}
          >
            {#if tab.id === 'main'}
              <svg class="w-3.5 h-3.5 transition-colors duration-200" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <circle cx="12" cy="12" r="2" />
                <path d="M16.24 7.76a6 6 0 0 1 0 8.49m-8.48-.01a6 6 0 0 1 0-8.49m11.31-2.82a10 10 0 0 1 0 14.14m-14.14 0a10 10 0 0 1 0-14.14" />
              </svg>
            {:else if tab.id === 'server'}
              <svg class="w-3.5 h-3.5 transition-colors duration-200" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <rect x="2" y="2" width="20" height="8" rx="2" ry="2" />
                <rect x="2" y="14" width="20" height="8" rx="2" ry="2" />
                <line x1="6" y1="6" x2="6.01" y2="6" />
                <line x1="6" y1="18" x2="6.01" y2="18" />
                <line x1="10" y1="6" x2="14" y2="6" />
                <line x1="10" y1="18" x2="14" y2="18" />
              </svg>
            {:else if tab.id === 'audio'}
              <svg class="w-3.5 h-3.5 transition-colors duration-200" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M12 2a3 3 0 0 0-3 3v7a3 3 0 0 0 6 0V5a3 3 0 0 0-3-3Z" />
                <path d="M19 10v1a7 7 0 0 1-14 0v-1" />
                <line x1="12" y1="19" x2="12" y2="22" />
              </svg>
            {:else if tab.id === 'recording'}
              <svg class="w-3.5 h-3.5 transition-colors duration-200" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <circle cx="12" cy="12" r="10" />
                <circle cx="12" cy="12" r="3" />
                <circle cx="12" cy="12" r="1" />
              </svg>
            {/if}
            <span>{tab.label}</span>
          </button>
        {/each}
      </nav>
    {/if}

    <div class="text-[9px] text-[#64748b] font-mono">v0.1.0</div>
  </header>

  <!-- ── Main Workspace ───────────────────── -->
  <main class="flex-1 p-3 flex flex-col overflow-hidden min-h-0 bg-bg-base/20 animate-fade-in">
    {#if !appState.configLoaded}
      <!-- Loading state -->
      <div class="flex items-center justify-center h-full">
        <div class="flex flex-col items-center gap-3">
          <div class="w-8 h-8 border-2 border-[rgba(59,130,246,0.2)] border-t-[#3b82f6] rounded-full animate-spin"></div>
          <span class="text-xs text-[#64748b]">Loading configuration…</span>
        </div>
      </div>
    {:else}
      <!-- Tab Content Area -->
      <div class="flex-1 h-full min-h-0 flex flex-col">
        {#if appState.activeTab === 'main'}
          <!-- ── Main Tab: Operational Desk Layout ── -->
          <div class="grid grid-cols-[1fr_60px] gap-4 h-full min-h-0 items-stretch">
            
            <!-- Left area: Stream Controls & Recording Cards -->
            <div class="grid grid-cols-2 gap-4 h-full min-h-0">
              <!-- Left Sub-column: Stream Connection Panel -->
              <div class="flex flex-col h-full min-h-0">
                <StreamControls />
              </div>
              
              <!-- Right Sub-column: Local Recording Panel -->
              <div class="flex flex-col gap-3 h-full min-h-0">
                <RecordingLive />
              </div>
            </div>
            
            <!-- Right area: Vertical VU Meter Panel -->
            <div class="flex flex-col h-full min-h-0 bg-black/10 rounded-lg p-2 border border-border-subtle">
              <h3 class="text-[8px] font-bold text-[#64748b] uppercase tracking-wider mb-2 text-center select-none leading-none">VU LEVEL</h3>
              <div class="flex-1 min-h-0">
                <VuMeter vertical={true} />
              </div>
            </div>

          </div>
        {:else if appState.activeTab === 'server'}
          <ServerConfig bind:config={appState.config} disabled={appState.isActiveStreaming} />
        {:else if appState.activeTab === 'audio'}
          <AudioSettings bind:config={appState.config} disabled={appState.isActiveStreaming} />
        {:else if appState.activeTab === 'recording'}
          <RecordingPanel bind:config={appState.config} disabled={appState.isActiveStreaming} />
        {/if}
      </div>
    {/if}
  </main>

  <!-- ── Error Toast ──────────────────────── -->
  {#if appState.showError && appState.lastError}
    <div class="fixed bottom-12 left-3.5 z-50 w-[280px] animate-fade-in">
      <div class="glass-card-static p-2.5 flex items-start gap-2.5 shadow-lg
        {appState.lastError.level === 'error'
          ? 'border-[rgba(244,63,94,0.25)] bg-[rgba(244,63,94,0.06)]'
          : 'border-[rgba(245,158,11,0.25)] bg-[rgba(245,158,11,0.06)]'}">
        <svg class="w-3.5 h-3.5 flex-shrink-0 mt-0.5
          {appState.lastError.level === 'error' ? 'text-[#f43f5e]' : 'text-[#f59e0b]'}"
          fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
            d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L4.082 16.5c-.77.833.192 2.5 1.732 2.5z" />
        </svg>
        <p class="text-[11px] text-text-primary flex-1 leading-normal">{appState.lastError.message}</p>
        <button aria-label="Close error message" class="text-text-dim hover:text-text-primary transition-colors" onclick={() => (appState.showError = false)}>
          <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>
    </div>
  {/if}

  <!-- ── Status Bar ───────────────────────── -->
  <footer class="px-4 py-1.5 flex items-center justify-between border-t border-border-subtle bg-black/10 flex-shrink-0">
    <div class="flex items-center gap-1.5">
      <div class="status-dot w-1.5 h-1.5 {statusDotClasses[appState.connectionStatus]}"></div>
      <span class="text-[9px] font-bold uppercase tracking-wider" style="color: {statusColors[appState.connectionStatus]}">
        {appState.connectionStatus}
      </span>
    </div>

    {#if appState.connectionStatus === 'Connected'}
      <div class="flex items-center gap-3">
        <div class="flex items-center gap-1.2">
          <svg class="w-3 h-3 text-text-dim" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
          </svg>
          <span class="text-[9px] font-bold font-mono text-text-muted">{formatDuration(appState.streamStats.duration_secs)}</span>
        </div>
        <div class="flex items-center gap-1.2">
          <svg class="w-3 h-3 text-text-dim" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 7h8m0 0v8m0-8l-8 8-4-4-6 6" />
          </svg>
          <span class="text-[9px] font-bold font-mono text-text-muted">{appState.streamStats.kbps.toFixed(0)} kbps</span>
        </div>
      </div>
    {:else}
      <span class="text-[9px] text-[#64748b] font-semibold font-mono">Sada v0.1.0</span>
    {/if}
  </footer>
</div>
