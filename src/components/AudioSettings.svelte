<script lang="ts">
  import { onMount } from 'svelte';
  import {
    getAudioDevices,
    saveConfig,
    startMonitor,
    type AppConfig,
  } from '../lib/ipc';

  let { config = $bindable(), disabled = false }: { config: AppConfig | null; disabled?: boolean } = $props();

  let devices = $state<string[]>([]);
  let selectedDevice = $state('');
  let codec = $state<'mp3' | 'opus' | 'ogg_vorbis' | 'aac' | 'aac_plus'>('mp3');
  let bitrate = $state(128);
  let sampleRate = $state(44100);
  let channels = $state(2);
  let loading = $state(true);
  let saving = $state(false);
  let saved = $state(false);

  const bitrateOptions = [64, 96, 128, 160, 192, 224, 256, 320];
  const sampleRateOptions = [22050, 44100, 48000];

  const codecInfo: Record<string, { label: string; desc: string }> = {
    mp3: { label: 'MP3', desc: 'Universal compatibility' },
    opus: { label: 'Opus', desc: 'High quality, low bitrate' },
    aac: { label: 'AAC', desc: 'Sleek, standard compression' },
    aac_plus: { label: 'AAC+ / HE-AAC', desc: 'Low bitrate powerhouse' },
    ogg_vorbis: { label: 'OGG Vorbis', desc: 'Open source format' },
  };

  // Populate from config
  $effect(() => {
    if (config) {
      const a = config.audio;
      selectedDevice = a.device_name;
      codec = a.codec;
      bitrate = a.bitrate;
      sampleRate = a.sample_rate;
      channels = a.channels;
    }
  });

  // Opus strictly requires 48000 Hz sample rate
  $effect(() => {
    if (codec === 'opus' && sampleRate !== 48000) {
      sampleRate = 48000;
    }
  });

  // Automatically start monitoring the selected device when the user selects a new one
  $effect(() => {
    if (selectedDevice !== undefined) {
      startMonitor(selectedDevice || null).catch(e => console.error('Failed to change monitor device:', e));
    }
  });

  onMount(async () => {
    try {
      devices = await getAudioDevices();
    } catch (e) {
      console.error('Failed to get audio devices:', e);
      devices = [];
    } finally {
      loading = false;
    }
  });

  async function handleSave() {
    if (!config) return;
    saving = true;

    config.audio = {
      device_name: selectedDevice,
      codec,
      bitrate,
      sample_rate: sampleRate,
      channels,
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

  async function refreshDevices() {
    loading = true;
    try {
      devices = await getAudioDevices();
    } catch (e) {
      console.error('Failed to refresh devices:', e);
    } finally {
      loading = false;
    }
  }
</script>

<div class="animate-fade-in flex flex-col gap-4 h-full min-h-0 select-none">
  {#if disabled}
    <div class="px-4 py-2.5 rounded-lg bg-[rgba(245,158,11,0.05)] border border-[rgba(245,158,11,0.2)] text-[10px] text-[#f59e0b] font-bold uppercase tracking-wider flex items-center gap-2 animate-fade-in shrink-0">
      <span class="w-1.5 h-1.5 rounded-full bg-[#f59e0b] animate-pulse"></span>
      Broadcasting Active: Audio configuration settings are locked. Disconnect stream to edit.
    </div>
  {/if}

  <!-- Side-by-Side Audio Panel -->
  <div class="grid grid-cols-2 gap-5 flex-1 min-h-0 items-stretch">
    
    <!-- Left Column: Source Capture Setup -->
    <div class="flex flex-col gap-3.5 bg-bg-card border border-border-subtle p-5 rounded-xl h-full overflow-y-auto no-scrollbar justify-start">
      <h3 class="text-[11px] font-bold text-text-dim uppercase tracking-wider mb-2 flex items-center gap-1.5">
        <svg class="w-4 h-4 opacity-75 text-blue-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11a7 7 0 01-7 7m0 0a7 7 0 01-7-7m7 7v4m0 0H8m4 0h4m-4-8a3 3 0 01-3-3V5a3 3 0 116 0v6a3 3 0 01-3 3z" />
        </svg>
        Source Capture
      </h3>

      <!-- Input Device -->
      <div>
        <label for="input-device-select" class="form-label mb-1 text-[10px] font-bold text-text-muted uppercase tracking-wider">Input Device</label>
        <div class="flex gap-2">
          <select id="input-device-select" class="select-field py-2 px-3 text-xs bg-position-right-6 flex-1" bind:value={selectedDevice} disabled={disabled || loading}>
            {#if loading}
              <option value="">Scanning devices…</option>
            {:else}
              <option value="">System Default</option>
              {#each devices as device}
                <option value={device}>{device}</option>
              {/each}
            {/if}
          </select>
          <button
            class="btn-secondary py-2 px-3.5 flex-shrink-0"
            onclick={refreshDevices}
            disabled={disabled || loading}
            title="Scan devices"
            aria-label="Scan devices"
          >
            <svg
              class="w-4 h-4 transition-transform {loading ? 'animate-spin' : ''}"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
              />
            </svg>
          </button>
        </div>
      </div>

      <!-- Channels -->
      <div>
        <span class="form-label block mb-1 text-[10px] font-bold text-text-muted uppercase tracking-wider">Channels</span>
        <div class="flex gap-1.5 bg-black/10 p-1 rounded-lg border border-border-subtle">
          <button
            class="flex-1 py-1.5 rounded-md text-xs font-bold transition-all duration-200
              {channels === 1
                ? 'bg-gradient-to-r from-[#3b82f6] to-[#06b6d4] text-white shadow-[0_1px_5px_rgba(59,130,246,0.15)]'
                : 'text-text-muted hover:text-text-primary'} {disabled ? 'cursor-not-allowed opacity-50' : ''}"
            onclick={() => !disabled && (channels = 1)}
            disabled={disabled}
          >
            Mono
          </button>
          <button
            class="flex-1 py-1.5 rounded-md text-xs font-bold transition-all duration-200
              {channels === 2
                ? 'bg-gradient-to-r from-[#3b82f6] to-[#06b6d4] text-white shadow-[0_1px_5px_rgba(59,130,246,0.15)]'
                : 'text-text-muted hover:text-text-primary'} {disabled ? 'cursor-not-allowed opacity-50' : ''}"
            onclick={() => !disabled && (channels = 2)}
            disabled={disabled}
          >
            Stereo
          </button>
        </div>
      </div>

      <!-- Sample Rate -->
      <div>
        <label for="sample-rate-select" class="form-label mb-1 text-[10px] font-bold text-text-muted uppercase tracking-wider">Sample Rate</label>
        <select id="sample-rate-select" class="select-field py-2 px-3 text-xs bg-position-right-6" bind:value={sampleRate} disabled={disabled || codec === 'opus'}>
          {#each sampleRateOptions as sr}
            <option value={sr} disabled={codec === 'opus' && sr !== 48000}>
              {(sr / 1000).toFixed(1)} kHz {codec === 'opus' && sr !== 48000 ? '(Unsupported by Opus)' : ''}
            </option>
          {/each}
        </select>
        {#if codec === 'opus'}
          <p class="text-[9px] text-[#3b82f6] mt-2 italic leading-none font-semibold uppercase tracking-wider">
            Opus strictly requires a 48.0 kHz sample rate.
          </p>
        {/if}
      </div>
    </div>

    <!-- Right Column: Encoder Details Setup -->
    <div class="flex flex-col gap-3.5 bg-bg-card border border-border-subtle p-5 rounded-xl h-full overflow-y-auto no-scrollbar justify-start">
      <h3 class="text-[11px] font-bold text-text-dim uppercase tracking-wider mb-2 flex items-center gap-1.5">
        <svg class="w-4 h-4 opacity-75 text-cyan-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11a7 7 0 01-7 7m0 0a7 7 0 01-7-7m7 7v4m0 0H8m4 0h4m-4-8a3 3 0 01-3-3V5a3 3 0 116 0v6a3 3 0 01-3 3z" />
        </svg>
        Encoder Options
      </h3>

      <!-- Codecs selector (compact grid) -->
      <div>
        <span class="form-label block mb-1.5 text-[10px] font-bold text-text-muted uppercase tracking-wider">Audio Codec</span>
        <div class="grid grid-cols-2 gap-2">
          {#each Object.entries(codecInfo) as [key, info]}
            <button
              class="text-left py-2 px-3 rounded-lg transition-all duration-200 border
                {codec === key
                  ? 'bg-[rgba(59,130,246,0.05)] border-[rgba(59,130,246,0.25)] shadow-sm'
                  : 'bg-bg-card border-border-subtle hover:bg-bg-card-hover'} {disabled ? 'cursor-not-allowed opacity-50' : ''}"
              onclick={() => !disabled && (codec = key as typeof codec)}
              disabled={disabled}
            >
              <span class="text-xs font-bold {codec === key ? 'text-accent-blue' : 'text-text-primary'}">{info.label}</span>
              <p class="text-[8px] text-text-dim leading-none mt-0.5 uppercase tracking-wider font-semibold">{info.desc}</p>
            </button>
          {/each}
        </div>
      </div>

      <!-- Bitrates selection -->
      <div>
        <label for="bitrate-select" class="form-label mb-1 text-[10px] font-bold text-text-muted uppercase tracking-wider">Encoding Bitrate</label>
        <select id="bitrate-select" class="select-field py-2 px-3 text-xs bg-position-right-6" bind:value={bitrate} disabled={disabled}>
          {#each bitrateOptions as br}
            <option value={br}>{br} kbps</option>
          {/each}
        </select>
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
      Save Audio Settings
    </button>
  </div>
</div>
