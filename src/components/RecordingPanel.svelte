<script lang="ts">
  import { saveConfig, type AppConfig } from '../lib/ipc';

  let { config = $bindable(), disabled = false }: { config: AppConfig | null; disabled?: boolean } = $props();

  let recordingPath = $state('');
  let recordingFormat = $state<'mp3' | 'wav' | 'ogg'>('mp3');
  let saving = $state(false);
  let saved = $state(false);

  const formatOptions: { value: string; label: string; ext: string }[] = [
    { value: 'wav', label: 'WAV', ext: '.wav' },
    { value: 'mp3', label: 'MP3', ext: '.mp3' },
    { value: 'ogg', label: 'OGG', ext: '.ogg' },
  ];

  // Populate from config when it loads
  $effect(() => {
    if (config) {
      recordingPath = config.recording.output_path;
      recordingFormat = config.recording.format;
    }
  });

  async function handleSave() {
    if (!config) return;
    saving = true;

    config.recording = {
      enabled: config.recording.enabled, // Preserve existing value
      output_path: recordingPath,
      format: recordingFormat,
    };

    try {
      await saveConfig(config);
      saved = true;
      setTimeout(() => (saved = false), 2000);
    } catch (e) {
      console.error('Failed to save config:', e);
    } finally {
      saving = false;
    }
  }
</script>

<div class="animate-fade-in flex flex-col gap-4 h-full min-h-0 select-none">
  {#if disabled}
    <div class="px-4 py-2.5 rounded-lg bg-[rgba(245,158,11,0.05)] border border-[rgba(245,158,11,0.2)] text-[10px] text-[#f59e0b] font-bold uppercase tracking-wider flex items-center gap-2 animate-fade-in shrink-0">
      <span class="w-1.5 h-1.5 rounded-full bg-[#f59e0b] animate-pulse"></span>
      Broadcasting Active: Recording configuration settings are locked. Disconnect stream to edit.
    </div>
  {/if}

  <!-- Side-by-Side Recording Setup -->
  <div class="grid grid-cols-2 gap-5 flex-1 min-h-0 items-stretch">
    
    <!-- Left Column: Encoding Format -->
    <div class="flex flex-col gap-3.5 bg-bg-card border border-border-subtle p-5 rounded-xl h-full overflow-y-auto no-scrollbar justify-start">
      <h3 class="text-[11px] font-bold text-text-dim uppercase tracking-wider mb-2 flex items-center gap-1.5">
        <svg class="w-4 h-4 opacity-75 text-blue-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
        </svg>
        File Format
      </h3>

      <div>
        <span class="form-label block mb-1.5 text-[10px] font-bold text-text-muted uppercase tracking-wider">Output Audio Format</span>
        <div class="grid grid-cols-2 gap-2">
          {#each formatOptions as opt}
            <button
              class="py-2 rounded-lg text-xs font-bold border text-center transition-all duration-200
                {recordingFormat === opt.value
                  ? 'bg-[rgba(59,130,246,0.06)] border-[rgba(59,130,246,0.25)] text-accent-blue shadow-sm'
                  : 'bg-bg-card border-border-subtle text-text-muted hover:bg-bg-card-hover'} {disabled ? 'cursor-not-allowed opacity-50' : ''}"
              onclick={() => !disabled && (recordingFormat = opt.value as typeof recordingFormat)}
              disabled={disabled}
            >
              {opt.label}
            </button>
          {/each}
        </div>
      </div>
    </div>

    <!-- Right Column: Storage Path -->
    <div class="flex flex-col gap-3.5 bg-bg-card border border-border-subtle p-5 rounded-xl h-full overflow-y-auto no-scrollbar justify-start">
      <h3 class="text-[11px] font-bold text-text-dim uppercase tracking-wider mb-2 flex items-center gap-1.5">
        <svg class="w-4 h-4 opacity-75 text-cyan-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
        </svg>
        Storage Destination
      </h3>

      <div>
        <label for="recording-output-path" class="form-label mb-1 text-[10px] font-bold text-text-muted uppercase tracking-wider">Output File Path</label>
        <div class="flex gap-2">
          <input
            id="recording-output-path"
            class="input-field py-2 px-3 text-xs font-mono flex-1"
            type="text"
            bind:value={recordingPath}
            placeholder="~/recordings/stream.mp3"
            disabled={disabled}
          />
          <button
            class="btn-secondary py-2 px-3.5 flex-shrink-0"
            title="Browse Directory"
            aria-label="Browse Directory"
            disabled={disabled}
          >
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
            </svg>
          </button>
        </div>
        <p class="text-[9px] text-text-dim mt-2 leading-normal italic uppercase tracking-wider font-semibold">
          Files are saved in your home folder (~). Extensions automatically match format choices.
        </p>
      </div>
    </div>

  </div>

  <!-- Save Action Footer -->
  <div class="flex items-center justify-end gap-3 pt-3 border-t border-border-subtle shrink-0">
    {#if saved}
      <span class="text-[11px] text-[#10b981] animate-fade-in flex items-center gap-1">
        <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
        </svg>
        Saved
      </span>
    {/if}
    <button class="btn-primary py-2 px-5 text-xs font-bold" onclick={handleSave} disabled={disabled || saving}>
      {#if saving}
        <span class="w-3.5 h-3.5 border-2 border-white/30 border-t-white rounded-full animate-spin"></span>
      {/if}
      Save Settings
    </button>
  </div>
</div>
