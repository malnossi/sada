<script lang="ts">
  import { appState } from '../lib/state.svelte';

  let recordingPath = $derived(appState.config?.recording?.output_path || '');
  let recordingFormat = $derived(appState.config?.recording?.format || 'mp3');
  let bitrate = $derived(appState.config?.audio?.bitrate || 128);

  function formatDuration(secs: number): string {
    const h = Math.floor(secs / 3600);
    const m = Math.floor((secs % 3600) / 60);
    const s = Math.floor(secs % 60);
    return `${h.toString().padStart(2, '0')}:${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}`;
  }

  function formatFileSize(secs: number, bitrateKbps: number): string {
    const bytes = (bitrateKbps * 1000 * secs) / 8;
    if (bytes < 1048576) return `~${(bytes / 1024).toFixed(0)} KB`;
    return `~${(bytes / 1048576).toFixed(1)} MB`;
  }
</script>

<div class="glass-card-static p-5 flex flex-col justify-between h-full min-h-0 select-none">
  <div class="flex flex-col gap-4">
    <div class="flex items-center justify-between border-b border-border-subtle pb-3">
      <div class="flex items-center gap-2">
        <svg class="w-4 h-4 text-text-muted opacity-75" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 10l4.553-2.276A1 1 0 0121 8.618v6.764a1 1 0 01-1.447.894L15 14M5 18h8a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 00-2 2v8a2 2 0 002 2z" />
        </svg>
        <h3 class="text-[10px] font-bold text-text-muted uppercase tracking-wider">Local Recorder</h3>
      </div>
      
      <div class="flex items-center gap-2">
        {#if appState.isRecording}
          <div class="w-2 h-2 rounded-full bg-[#f43f5e] animate-recording shadow-[0_0_8px_rgba(244,63,94,0.5)]"></div>
          <span class="text-[10px] font-bold text-[#f43f5e] uppercase tracking-wide">Recording</span>
        {:else}
          <div class="w-2 h-2 rounded-full bg-text-dim"></div>
          <span class="text-[10px] font-bold text-text-dim uppercase tracking-wide">Stopped</span>
        {/if}
      </div>
    </div>

    <!-- Storage information details -->
    <div class="flex flex-col gap-3.5 bg-black/10 rounded-xl p-4 border border-border-subtle">
      <div class="flex items-center justify-between">
        <span class="text-[9px] text-text-dim uppercase font-bold tracking-wider">Output Format</span>
        <span class="text-xs font-bold text-text-primary">{recordingFormat.toUpperCase()} @ {bitrate}k</span>
      </div>
      
      <div class="flex items-center justify-between">
        <span class="text-[9px] text-text-dim uppercase font-bold tracking-wider">Current Duration</span>
        <span class="font-mono text-sm font-bold text-text-primary tracking-wide">
          {formatDuration(appState.recordingDuration)}
        </span>
      </div>
    </div>
  </div>

  <div class="flex flex-col gap-3.5">
    <button
      class="{appState.isRecording ? 'btn-danger bg-gradient-to-r from-red-500 to-rose-600 hover:shadow-red-500/10' : 'btn-secondary'} w-full flex items-center justify-center gap-2 py-2.5 text-xs font-bold transition-all duration-200"
      onclick={() => appState.toggleRecording()}
    >
      {#if appState.isRecording}
        <svg class="w-4 h-4 fill-current animate-pulse" viewBox="0 0 24 24">
          <rect x="6" y="6" width="12" height="12" rx="1" />
        </svg>
        Stop Recording
      {:else}
        <svg class="w-4 h-4 fill-current text-red-500" viewBox="0 0 24 24">
          <circle cx="12" cy="12" r="5" />
        </svg>
        Start Local Recording
      {/if}
    </button>

    {#if appState.isRecording}
      <div class="flex items-center justify-between text-[10px] text-text-dim px-0.5 animate-fade-in leading-none">
        <span class="truncate max-w-[170px]" title={recordingPath || `~/recording.${recordingFormat}`}>
          Dest: {recordingPath ? recordingPath.replace('/Users/mohamednossirat', '~') : `~/recording.${recordingFormat}`}
        </span>
        <span class="font-medium font-mono shrink-0">
          {formatFileSize(appState.recordingDuration, bitrate)}
        </span>
      </div>
    {/if}
  </div>
</div>
