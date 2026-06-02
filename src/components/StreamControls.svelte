<script lang="ts">
  import { appState } from '../lib/state.svelte';

  let isConnected = $derived(appState.isConnected);
  let isActive = $derived(appState.isActiveStreaming);

  let errorMessage = $derived.by(() => {
    if (appState.connectionStatus === 'Error') {
      if (appState.connectionStatusData && typeof appState.connectionStatusData === 'object' && 'message' in appState.connectionStatusData) {
        return appState.connectionStatusData.message;
      }
      return appState.connectionStatusData || 'Unknown connection error';
    }
    return '';
  });

  let reconnectInfo = $derived.by(() => {
    if (appState.connectionStatus === 'Reconnecting') {
      if (appState.connectionStatusData && typeof appState.connectionStatusData === 'object') {
        const attempt = appState.connectionStatusData.attempt ?? '?';
        const delay = appState.connectionStatusData.delay_secs ?? '?';
        return `Reconnecting (attempt ${attempt}, retrying in ${delay}s)`;
      }
      return 'Reconnecting…';
    }
    return '';
  });

  let activeError = $derived(appState.connectError || errorMessage);

  function formatDuration(secs: number): string {
    const h = Math.floor(secs / 3600);
    const m = Math.floor((secs % 3600) / 60);
    const s = Math.floor(secs % 60);
    return `${h.toString().padStart(2, '0')}:${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}`;
  }

  function formatBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1048576) return `${(bytes / 1024).toFixed(0)} KB`;
    if (bytes < 1073741824) return `${(bytes / 1048576).toFixed(1)} MB`;
    return `${(bytes / 1073741824).toFixed(2)} GB`;
  }

  const statusConfig = {
    Idle: { label: 'Idle', color: 'text-text-dim', dotClass: 'status-dot-idle' },
    Connecting: { label: 'Connecting…', color: 'text-[#f59e0b]', dotClass: 'status-dot-connecting' },
    Connected: { label: 'Connected', color: 'text-[#10b981]', dotClass: 'status-dot-connected' },
    Reconnecting: { label: 'Reconnecting…', color: 'text-[#f59e0b]', dotClass: 'status-dot-connecting' },
    Error: { label: 'Error', color: 'text-[#f43f5e]', dotClass: 'status-dot-error' },
  };

  let currentStatus = $derived(statusConfig[appState.connectionStatus]);
</script>

<div class="animate-fade-in flex flex-col gap-4 h-full min-h-0 select-none">
  <!-- Status and Connect Card -->
  <div class="glass-card-static p-5 flex flex-col gap-4 flex-1 min-h-0 justify-between">
    <div class="flex flex-col gap-4">
      <div class="flex items-center justify-between border-b border-border-subtle pb-3">
        <div class="flex items-center gap-2 text-ellipsis overflow-hidden whitespace-nowrap">
          <div class="status-dot w-2.5 h-2.5 {currentStatus.dotClass}"></div>
          <span class="text-xs font-bold uppercase tracking-wider {currentStatus.color}">
            {appState.connectionStatus === 'Reconnecting' ? reconnectInfo : currentStatus.label}
          </span>
        </div>
        {#if isConnected}
          <div class="flex items-center gap-1.5 px-2.5 py-0.5 rounded-full bg-[rgba(16,185,129,0.08)] border border-[rgba(16,185,129,0.12)]">
            <span class="w-1.5 h-1.5 rounded-full bg-[#10b981] animate-pulse-dot"></span>
            <span class="text-[9px] font-bold text-[#10b981] tracking-wide uppercase">LIVE</span>
          </div>
        {/if}
      </div>

      {#if activeError}
        <div class="p-3 rounded-lg bg-[rgba(244,63,94,0.05)] border border-[rgba(244,63,94,0.18)] text-xs text-[#f43f5e] font-medium leading-relaxed break-words flex flex-col gap-1.5">
          <div class="flex items-center gap-1 font-bold uppercase tracking-wider text-[9px]">
            <svg class="w-3.5 h-3.5 text-[#f43f5e]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L4.082 16.5c-.77.833.192 2.5 1.732 2.5z" />
            </svg>
            Connection Failure
          </div>
          <p class="font-mono text-[10px] bg-black/15 p-2 rounded leading-normal select-text">
            {activeError}
          </p>
        </div>
      {/if}

      <!-- Target Server Profile Dropdown Selection -->
      {#if appState.config && appState.config.servers}
        <div class="flex flex-col gap-1.5">
          <label for="target-server-select" class="form-label mb-0.5 text-[10px] uppercase tracking-wider text-text-dim font-bold">Target Server</label>
          <select 
            id="target-server-select"
            class="select-field py-2 px-3 text-xs bg-position-right-6"
            bind:value={appState.config.selected_server_id}
            disabled={isActive}
            onchange={() => appState.handleServerChange()}
          >
            {#each appState.config.servers as s}
              <option value={s.id}>{s.name}</option>
            {/each}
          </select>
        </div>
      {/if}
    </div>

    <!-- Connect Action -->
    <div class="pt-3 border-t border-border-subtle">
      {#if isActive}
        <button class="btn-danger w-full py-2.5 text-xs font-bold bg-gradient-to-r from-red-500 to-rose-600 hover:shadow-red-500/10 transition-all duration-200" onclick={() => appState.handleDisconnect()}>
          <svg class="w-4 h-4 mr-1.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 10a1 1 0 011-1h4a1 1 0 011 1v4a1 1 0 01-1 1h-4a1 1 0 01-1-1v-4z" />
          </svg>
          Disconnect Stream
        </button>
      {:else}
        <button class="btn-primary w-full py-2.5 text-xs font-bold bg-gradient-to-r from-[#3b82f6] to-[#06b6d4] hover:shadow-blue-500/10 transition-all duration-200" onclick={() => appState.handleConnect()} disabled={appState.connecting}>
          {#if appState.connecting}
            <span class="w-4 h-4 border-2 border-white/30 border-t-white rounded-full animate-spin"></span>
          {:else}
            <svg class="w-4 h-4 mr-1.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5.636 18.364a9 9 0 010-12.728m12.728 0a9 9 0 010 12.728M9.172 15.828a5 5 0 010-7.072m5.656 0a5 5 0 010 7.072M12 12h.01" />
            </svg>
          {/if}
          Connect to Server
        </button>
      {/if}
    </div>
  </div>

  <!-- Real-time Stream Statistics (displays when connected) -->
  {#if isConnected}
    <div class="grid grid-cols-3 gap-3 animate-fade-in shrink-0">
      <div class="glass-card-static p-2.5 text-center">
        <p class="text-[8px] font-bold text-text-dim uppercase tracking-wider mb-1">Duration</p>
        <p class="text-xs font-bold text-text-primary font-mono tracking-wide">{formatDuration(appState.streamStats.duration_secs)}</p>
      </div>
      <div class="glass-card-static p-2.5 text-center">
        <p class="text-[8px] font-bold text-text-dim uppercase tracking-wider mb-1">Bitrate</p>
        <p class="text-xs font-bold text-text-primary font-mono tracking-wide">
          {appState.streamStats.kbps.toFixed(0)}
          <span class="text-[8px] font-normal text-text-dim">k</span>
        </p>
      </div>
      <div class="glass-card-static p-2.5 text-center">
        <p class="text-[8px] font-bold text-text-dim uppercase tracking-wider mb-1">Sent</p>
        <p class="text-xs font-bold text-text-primary font-mono tracking-wide">{formatBytes(appState.streamStats.bytes_sent)}</p>
      </div>
    </div>
  {/if}

  <!-- Metadata Injection / Now Playing -->
  <div class="glass-card-static p-4 flex flex-col gap-3 shrink-0">
    <div class="flex items-center gap-2">
      <svg class="w-4 h-4 text-text-muted opacity-75" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 19V6l12-3v13M9 19c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zm12-3c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zM9 10l12-3" />
      </svg>
      <h3 class="text-[10px] font-bold text-text-muted uppercase tracking-wider">Now Playing Metadata</h3>
    </div>
    
    <div class="flex gap-2">
      <input
        class="input-field flex-1 py-2 px-3 text-xs"
        type="text"
        bind:value={appState.songTitle}
        placeholder="Artist - Song Title"
        onkeydown={(e) => e.key === 'Enter' && isConnected && appState.songTitle.trim() && appState.handleMetadataUpdate()}
        disabled={!isConnected}
      />
      <button
        class="btn-secondary py-2 px-4 text-xs font-bold"
        onclick={() => appState.handleMetadataUpdate()}
        disabled={!isConnected || !appState.songTitle.trim()}
      >
        Update
      </button>
    </div>
    {#if !isConnected}
      <p class="text-[9px] text-text-dim leading-normal italic text-center uppercase tracking-wider font-semibold">
        Connect to stream to update now-playing info
      </p>
    {/if}
  </div>
</div>
