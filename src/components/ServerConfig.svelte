<script lang="ts">
  import { saveConfig, type AppConfig } from '../lib/ipc';

  let { config = $bindable(), disabled = false }: { config: AppConfig | null; disabled?: boolean } = $props();

  let activeServerId = $state<string>('');
  let saving = $state(false);
  let saved = $state(false);
  let showSourcePass = $state(false);
  let showAdminPass = $state(false);

  let showAddModal = $state(false);
  let newServerName = $state('');
  let showDeleteModal = $state(false);
  let showRenameModal = $state(false);
  let renameServerName = $state('');

  // Synchronize activeServerId with config
  $effect(() => {
    if (config) {
      if (!activeServerId || !config.servers.some(s => s.id === activeServerId)) {
        activeServerId = config.selected_server_id || config.servers[0]?.id || '';
      }
    }
  });

  // Automatically save config when activeServerId/selected_server_id changes
  $effect(() => {
    if (config && activeServerId) {
      if (config.selected_server_id !== activeServerId) {
        config.selected_server_id = activeServerId;
        saveConfig(config).catch(e => console.error('Failed to auto-save selected server ID:', e));
      }
    }
  });

  // Derived active server profile
  let activeServer = $derived(config?.servers.find(s => s.id === activeServerId) || null);

  async function handleCreateServer() {
    if (!config) return;
    const name = newServerName.trim() || `Server ${config.servers.length + 1}`;
    const newId = `srv_${Date.now()}`;
    const newServer = {
      id: newId,
      name,
      server_type: 'icecast' as const,
      host: 'localhost',
      port: 8000,
      mount_point: '/stream',
      password: 'hackme',
      admin_password: 'admin',
      username: 'source',
      legacy_icecast: false,
      custom_listener_url: '',
      custom_listener_mount: '',
      stream_name: name,
      stream_description: '',
      stream_genre: '',
      stream_url: '',
      public_server: false,
      tls: false,
    };
    config.servers.push(newServer);
    activeServerId = newId;
    config.selected_server_id = newId;
    showAddModal = false;
    try {
      await saveConfig(config);
    } catch (e) {
      console.error('Failed to save new server profile:', e);
    }
  }

  async function handleConfirmDelete() {
    if (!config || !activeServerId) return;
    if (config.servers.length <= 1) return;

    const index = config.servers.findIndex(s => s.id === activeServerId);
    if (index !== -1) {
      config.servers.splice(index, 1);
      const newSelectedId = config.servers[0].id;
      activeServerId = newSelectedId;
      config.selected_server_id = newSelectedId;
      showDeleteModal = false;
      try {
        await saveConfig(config);
      } catch (e) {
        console.error('Failed to delete server profile:', e);
      }
    }
  }

  async function handleSaveRename() {
    if (!config || !activeServer) return;
    const name = renameServerName.trim();
    if (name) {
      activeServer.name = name;
      showRenameModal = false;
      await handleSave();
    }
  }

  async function handleSave() {
    if (!config) return;
    saving = true;
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

  function handleKeyDown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      showAddModal = false;
      showRenameModal = false;
      showDeleteModal = false;
    }
  }
</script>

<svelte:window onkeydown={handleKeyDown} />

<div class="animate-fade-in flex flex-col gap-4 h-full min-h-0 select-none">
  {#if disabled}
    <div class="px-4 py-2.5 rounded-lg bg-[rgba(245,158,11,0.05)] border border-[rgba(245,158,11,0.2)] text-[10px] text-[#f59e0b] font-bold uppercase tracking-wider flex items-center gap-2 animate-fade-in shrink-0">
      <span class="w-1.5 h-1.5 rounded-full bg-[#f59e0b] animate-pulse"></span>
      Broadcasting Active: Server configuration settings are locked. Disconnect stream to edit.
    </div>
  {/if}

  <!-- Profile Management Toolbar -->
  <div class="glass-card-static p-4 flex items-center justify-between gap-4 shrink-0">
    <div class="flex items-center gap-3 flex-1 min-w-0">
      <label for="server-profile-select" class="text-[10px] font-bold text-text-dim uppercase tracking-wider whitespace-nowrap">Server Profile:</label>
      <select 
        id="server-profile-select"
        class="select-field py-1.5 px-3 text-xs bg-position-right-4 max-w-[150px] shrink-0" 
        bind:value={activeServerId}
        disabled={disabled}
      >
        {#each config?.servers || [] as s}
          <option value={s.id}>{s.name}</option>
        {/each}
      </select>
      
      {#if activeServer}
        <button
          class="btn-secondary py-1.5 px-3 text-xs font-semibold flex items-center gap-1.5"
          onclick={() => {
            renameServerName = activeServer.name;
            showRenameModal = true;
          }}
          disabled={disabled}
          title="Rename Profile"
        >
          <svg class="w-3.5 h-3.5 opacity-70 text-blue-400" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" d="M16.862 4.487l1.687-1.688a1.875 1.875 0 112.652 2.652L10.582 16.07a4.5 4.5 0 01-1.897 1.13L6 18l.8-2.685a4.5 4.5 0 011.13-1.897l8.932-8.931zm0 0L19.5 7.125M18 14v4.75A2.25 2.25 0 0115.75 21H5.25A2.25 2.25 0 013 18.75V8.25A2.25 2.25 0 015.25 6H10" />
          </svg>
          Rename Profile
        </button>
      {/if}
    </div>
    
    <div class="flex items-center gap-2 shrink-0">
      <button 
        class="btn-secondary py-1.5 px-3.5 text-xs font-semibold" 
        onclick={() => {
          newServerName = '';
          showAddModal = true;
        }}
        disabled={disabled}
      >
        Add Server
      </button>
      <button 
        class="btn-danger py-1.5 px-3.5 text-xs font-semibold bg-rose-600/80 hover:bg-rose-600 disabled:opacity-30 disabled:hover:bg-transparent" 
        onclick={() => {
          showDeleteModal = true;
        }} 
        disabled={disabled || (config?.servers || []).length <= 1}
      >
        Delete
      </button>
    </div>
  </div>

  {#if activeServer}
    <!-- Side-by-Side Configuration Panel -->
    <div class="grid grid-cols-2 gap-5 flex-1 min-h-0 items-stretch">
      
      <!-- Left Column: Connection Settings -->
      <div class="flex flex-col gap-3.5 bg-bg-card border border-border-subtle p-5 rounded-xl h-full overflow-y-auto no-scrollbar justify-start">
        <h3 class="text-[11px] font-bold text-text-dim uppercase tracking-wider mb-2 flex items-center gap-1.5">
          <svg class="w-4 h-4 opacity-75 text-blue-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11a7 7 0 01-7 7m0 0a7 7 0 01-7-7m7 7v4m0 0H8m4 0h4m-4-8a3 3 0 01-3-3V5a3 3 0 116 0v6a3 3 0 01-3 3z" />
          </svg>
          Connection Settings
        </h3>

        <!-- Server Type & TLS Grid -->
        <div class="grid grid-cols-[1fr_auto] gap-3 items-end">
          <div>
            <label for="server-type-select" class="form-label mb-1 text-[10px] font-bold text-text-muted uppercase tracking-wider">Server Type</label>
            <select id="server-type-select" class="select-field py-2 px-3 text-xs bg-position-right-6" bind:value={activeServer.server_type} disabled={disabled}>
              <option value="icecast">Icecast</option>
              <option value="shoutcast">Shoutcast</option>
            </select>
          </div>
          <div class="flex items-center gap-2 pb-2 h-9">
            <input
              id="tls-toggle"
              type="checkbox"
              bind:checked={activeServer.tls}
              class="w-3.5 h-3.5 rounded border-border-medium bg-black/10 text-accent-blue focus:ring-[#3b82f6] focus:ring-offset-0 cursor-pointer"
              disabled={disabled}
            />
            <label for="tls-toggle" class="text-[10px] font-bold text-text-muted uppercase tracking-wider cursor-pointer select-none">Use TLS / SSL</label>
          </div>
        </div>

        <!-- Hostname & Port Grid -->
        <div class="grid grid-cols-[1fr_90px] gap-3">
          <div>
            <label for="hostname-input" class="form-label mb-1 text-[10px] font-bold text-text-muted uppercase tracking-wider">Hostname / IP</label>
            <input id="hostname-input" class="input-field py-2 px-3 text-xs" type="text" bind:value={activeServer.host} placeholder="localhost" disabled={disabled} />
          </div>
          <div>
            <label for="port-input" class="form-label mb-1 text-[10px] font-bold text-text-muted uppercase tracking-wider">Port</label>
            <input id="port-input" class="input-field py-2 px-3 text-xs" type="number" bind:value={activeServer.port} disabled={disabled} />
          </div>
        </div>

        {#if activeServer.server_type === 'icecast'}
          <!-- Icecast Password (Full-Width) -->
          <div>
            <label for="icecast-pass-input" class="form-label mb-1 text-[10px] font-bold text-text-muted uppercase tracking-wider">Password</label>
            <div class="relative">
              <input id="icecast-pass-input" class="input-field py-2 pl-3 pr-9 text-xs font-mono w-full" type={showSourcePass ? "text" : "password"} bind:value={activeServer.password} placeholder="password" disabled={disabled} />
              <button
                type="button"
                class="absolute right-2.5 top-1/2 -translate-y-1/2 text-text-dim hover:text-text-muted focus:outline-none"
                onclick={() => (showSourcePass = !showSourcePass)}
                disabled={disabled}
              >
                {#if showSourcePass}
                  <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" d="M3.98 8.223A10.477 10.477 0 001.934 12C3.226 16.338 7.244 19.5 12 19.5c.993 0 1.953-.138 2.863-.395M6.228 6.228A10.45 10.45 0 0112 4.5c4.756 0 8.773 3.162 10.065 7.498a10.523 10.523 0 01-4.293 5.774M6.228 6.228L3 3m3.228 3.228l3.65 3.65m7.894 7.894L21 21m-3.228-3.228l-3.65-3.65m0 0a3 3 0 10-4.243-4.243m4.242 4.242L9.88 9.88" />
                  </svg>
                {:else}
                  <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" d="M2.036 12.322a1.012 1.012 0 010-.639C3.423 7.51 7.36 4.5 12 4.5c4.638 0 8.573 3.007 9.963 7.178.07.207.07.431 0 .639C20.577 16.49 16.64 19.5 12 19.5c-4.638 0-8.573-3.007-9.963-7.178z" />
                    <path stroke-linecap="round" stroke-linejoin="round" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                  </svg>
                {/if}
              </button>
            </div>
          </div>

          <!-- Icecast Mountpoint & User Grid -->
          <div class="grid grid-cols-2 gap-3">
            <div>
              <label for="icecast-mount-input" class="form-label mb-1 text-[10px] font-bold text-text-muted uppercase tracking-wider">Icecast Mountpoint</label>
              <input id="icecast-mount-input" class="input-field py-2 px-3 text-xs" type="text" bind:value={activeServer.mount_point} placeholder="stream" disabled={disabled} />
            </div>
            <div>
              <label for="icecast-user-input" class="form-label mb-1 text-[10px] font-bold text-text-muted uppercase tracking-wider">Icecast User</label>
              <input id="icecast-user-input" class="input-field py-2 px-3 text-xs" type="text" bind:value={activeServer.username} placeholder="source" disabled={disabled} />
            </div>
          </div>

          <!-- Use legacy Icecast protocol -->
          <div class="flex items-center gap-2 mt-1">
            <input
              id="legacy-icecast-toggle"
              type="checkbox"
              bind:checked={activeServer.legacy_icecast}
              class="w-3.5 h-3.5 rounded border-border-medium bg-black/10 text-accent-blue focus:ring-[#3b82f6] focus:ring-offset-0 cursor-pointer"
              disabled={disabled}
            />
            <label for="legacy-icecast-toggle" class="text-[10px] font-bold text-text-muted uppercase tracking-wider cursor-pointer select-none">Use legacy Icecast protocol</label>
          </div>

          <!-- Custom listener URL (optional) -->
          <div>
            <label for="icecast-custom-url-input" class="form-label mb-1 text-[10px] font-bold text-text-muted uppercase tracking-wider">Custom listener URL (optional)</label>
            <input id="icecast-custom-url-input" class="input-field py-2 px-3 text-xs" type="text" bind:value={activeServer.custom_listener_url} placeholder="http://example.com/listen" disabled={disabled} />
          </div>

          <!-- Custom listener mountpoint (optional) -->
          <div>
            <label for="icecast-custom-mount-input" class="form-label mb-1 text-[10px] font-bold text-text-muted uppercase tracking-wider">Custom listener mountpoint (optional)</label>
            <input id="icecast-custom-mount-input" class="input-field py-2 px-3 text-xs" type="text" bind:value={activeServer.custom_listener_mount} placeholder="/custom_mount" disabled={disabled} />
          </div>
        {:else if activeServer.server_type === 'shoutcast'}
          <!-- Shoutcast Passwords Grid (Source Pass & Admin Pass) -->
          <div class="grid grid-cols-2 gap-3">
            <div>
              <label for="shoutcast-source-pass-input" class="form-label mb-1 text-[10px] font-bold text-text-muted uppercase tracking-wider">Source Pass</label>
              <div class="relative">
                <input id="shoutcast-source-pass-input" class="input-field py-2 pl-3 pr-9 text-xs font-mono" type={showSourcePass ? "text" : "password"} bind:value={activeServer.password} placeholder="password" disabled={disabled} />
                <button
                  type="button"
                  class="absolute right-2.5 top-1/2 -translate-y-1/2 text-text-dim hover:text-text-muted focus:outline-none"
                  onclick={() => (showSourcePass = !showSourcePass)}
                  disabled={disabled}
                >
                  {#if showSourcePass}
                    <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" d="M3.98 8.223A10.477 10.477 0 001.934 12C3.226 16.338 7.244 19.5 12 19.5c.993 0 1.953-.138 2.863-.395M6.228 6.228A10.45 10.45 0 0112 4.5c4.756 0 8.773 3.162 10.065 7.498a10.523 10.523 0 01-4.293 5.774M6.228 6.228L3 3m3.228 3.228l3.65 3.65m7.894 7.894L21 21m-3.228-3.228l-3.65-3.65m0 0a3 3 0 10-4.243-4.243m4.242 4.242L9.88 9.88" />
                    </svg>
                  {:else}
                    <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" d="M2.036 12.322a1.012 1.012 0 010-.639C3.423 7.51 7.36 4.5 12 4.5c4.638 0 8.573 3.007 9.963 7.178.07.207.07.431 0 .639C20.577 16.49 16.64 19.5 12 19.5c-4.638 0-8.573-3.007-9.963-7.178z" />
                      <path stroke-linecap="round" stroke-linejoin="round" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                    </svg>
                  {/if}
                </button>
              </div>
            </div>
            <div>
              <label for="shoutcast-admin-pass-input" class="form-label mb-1 text-[10px] font-bold text-text-muted uppercase tracking-wider">Admin Pass <span class="text-[8px] text-text-dim font-normal">(meta)</span></label>
              <div class="relative">
                <input id="shoutcast-admin-pass-input" class="input-field py-2 pl-3 pr-9 text-xs font-mono" type={showAdminPass ? "text" : "password"} bind:value={activeServer.admin_password} placeholder="admin" disabled={disabled} />
                <button
                  type="button"
                  class="absolute right-2.5 top-1/2 -translate-y-1/2 text-text-dim hover:text-text-muted focus:outline-none"
                  onclick={() => (showAdminPass = !showAdminPass)}
                  disabled={disabled}
                >
                  {#if showAdminPass}
                    <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" d="M3.98 8.223A10.477 10.477 0 001.934 12C3.226 16.338 7.244 19.5 12 19.5c.993 0 1.953-.138 2.863-.395M6.228 6.228A10.45 10.45 0 0112 4.5c4.756 0 8.773 3.162 10.065 7.498a10.523 10.523 0 01-4.293 5.774M6.228 6.228L3 3m3.228 3.228l3.65 3.65m7.894 7.894L21 21m-3.228-3.228l-3.65-3.65m0 0a3 3 0 10-4.243-4.243m4.242 4.242L9.88 9.88" />
                    </svg>
                  {:else}
                    <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" d="M2.036 12.322a1.012 1.012 0 010-.639C3.423 7.51 7.36 4.5 12 4.5c4.638 0 8.573 3.007 9.963 7.178.07.207.07.431 0 .639C20.577 16.49 16.64 19.5 12 19.5c-4.638 0-8.573-3.007-9.963-7.178z" />
                      <path stroke-linecap="round" stroke-linejoin="round" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                    </svg>
                  {/if}
                </button>
              </div>
            </div>
          </div>

          <!-- Shoutcast Stream ID -->
          <div>
            <label for="shoutcast-stream-id-input" class="form-label mb-1 text-[10px] font-bold text-text-muted uppercase tracking-wider">Stream ID / Mount</label>
            <div class="flex gap-3 items-center">
              <input id="shoutcast-stream-id-input" class="input-field py-2 px-3 text-xs flex-1" type="text" bind:value={activeServer.mount_point} placeholder="1" disabled={disabled} />
            </div>
          </div>
        {/if}

        <!-- Public Checkbox -->
        <div class="mt-2 flex items-center">
          <label class="flex items-center gap-2 cursor-pointer select-none">
            <input
              type="checkbox"
              bind:checked={activeServer.public_server}
              class="w-4 h-4 rounded border-border-medium bg-bg-card accent-accent-blue cursor-pointer"
              disabled={disabled}
            />
            <span class="text-[10px] font-bold text-text-muted uppercase tracking-wider">List in Public Directory</span>
          </label>
        </div>
      </div>

      <!-- Right Column: Station Metadata Details -->
      <div class="flex flex-col gap-3.5 bg-bg-card border border-border-subtle p-5 rounded-xl h-full overflow-y-auto no-scrollbar justify-start">
        <h3 class="text-[11px] font-bold text-text-dim uppercase tracking-wider mb-2 flex items-center gap-1.5">
          <svg class="w-4 h-4 opacity-75 text-cyan-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
          </svg>
          Station Metadata
        </h3>

        <!-- Stream Name -->
        <div>
          <label for="stream-name-input" class="form-label mb-1 text-[10px] font-bold text-text-muted uppercase tracking-wider">Stream Name</label>
          <input id="stream-name-input" class="input-field py-2 px-3 text-xs" type="text" bind:value={activeServer.stream_name} placeholder="My Online Radio" disabled={disabled} />
        </div>

        <!-- Genre -->
        <div>
          <label for="stream-genre-input" class="form-label mb-1 text-[10px] font-bold text-text-muted uppercase tracking-wider">Genre</label>
          <input id="stream-genre-input" class="input-field py-2 px-3 text-xs" type="text" bind:value={activeServer.stream_genre} placeholder="Jazz, Electronic" disabled={disabled} />
        </div>

        <!-- Description -->
        <div>
          <label for="stream-description-input" class="form-label mb-1 text-[10px] font-bold text-text-muted uppercase tracking-wider">Description</label>
          <input id="stream-description-input" class="input-field py-2 px-3 text-xs" type="text" bind:value={activeServer.stream_description} placeholder="Live stream description" disabled={disabled} />
        </div>

        <!-- Website URL -->
        <div>
          <label for="stream-url-input" class="form-label mb-1 text-[10px] font-bold text-text-muted uppercase tracking-wider">Website URL</label>
          <input id="stream-url-input" class="input-field py-2 px-3 text-xs" type="url" bind:value={activeServer.stream_url} placeholder="https://mywebsite.com" disabled={disabled} />
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
        Save Server Config
      </button>
    </div>
  {/if}
</div>

{#if showAddModal}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="modal-overlay animate-fade-in" onclick={() => (showAddModal = false)}>
    <div class="modal-content p-6 w-full max-w-sm flex flex-col gap-4" onclick={(e) => e.stopPropagation()}>
      <h3 class="text-xs font-bold text-text-dim uppercase tracking-wider">Create Server Profile</h3>
      <div class="flex flex-col gap-1.5">
        <label for="add-profile-name-input" class="form-label text-[10px] font-bold text-text-muted uppercase tracking-wider">Profile Name</label>
        <!-- svelte-ignore a11y_autofocus -->
        <input 
          id="add-profile-name-input"
          class="input-field py-2 px-3 text-xs w-full" 
          type="text" 
          bind:value={newServerName} 
          placeholder="e.g. My Radio" 
          autofocus 
          onkeydown={(e) => e.key === 'Enter' && handleCreateServer()}
        />
      </div>
      <div class="flex justify-end gap-2.5 mt-2">
        <button class="btn-secondary py-1.5 px-4 text-xs font-semibold" onclick={() => (showAddModal = false)}>Cancel</button>
        <button class="btn-primary py-1.5 px-4 text-xs font-bold" onclick={handleCreateServer}>Create Profile</button>
      </div>
    </div>
  </div>
{/if}

{#if showRenameModal}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="modal-overlay animate-fade-in" onclick={() => (showRenameModal = false)}>
    <div class="modal-content p-6 w-full max-w-sm flex flex-col gap-4" onclick={(e) => e.stopPropagation()}>
      <h3 class="text-xs font-bold text-text-dim uppercase tracking-wider">Rename Server Profile</h3>
      <div class="flex flex-col gap-1.5">
        <label for="rename-profile-name-input" class="form-label text-[10px] font-bold text-text-muted uppercase tracking-wider">Profile Name</label>
        <!-- svelte-ignore a11y_autofocus -->
        <input 
          id="rename-profile-name-input"
          class="input-field py-2 px-3 text-xs w-full" 
          type="text" 
          bind:value={renameServerName} 
          placeholder="e.g. My Radio" 
          autofocus 
          onkeydown={(e) => e.key === 'Enter' && handleSaveRename()}
        />
      </div>
      <div class="flex justify-end gap-2.5 mt-2">
        <button class="btn-secondary py-1.5 px-4 text-xs font-semibold" onclick={() => (showRenameModal = false)}>Cancel</button>
        <button class="btn-primary py-1.5 px-4 text-xs font-bold" onclick={handleSaveRename}>Save Name</button>
      </div>
    </div>
  </div>
{/if}

{#if showDeleteModal}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="modal-overlay animate-fade-in" onclick={() => (showDeleteModal = false)}>
    <div class="modal-content p-6 w-full max-w-sm flex flex-col gap-4 border-danger/20 bg-danger/5" onclick={(e) => e.stopPropagation()}>
      <div class="flex items-center gap-2 text-rose-500">
        <svg class="w-5 h-5 shrink-0" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
        </svg>
        <h3 class="text-xs font-bold uppercase tracking-wider text-rose-500">Delete Server Profile</h3>
      </div>
      <p class="text-[11px] text-text-muted leading-relaxed">
        Are you sure you want to delete profile <strong class="text-text-primary">"{activeServer?.name}"</strong>? 
        This action will permanently erase these connection details and cannot be undone.
      </p>
      <div class="flex justify-end gap-2.5 mt-2">
        <button class="btn-secondary py-1.5 px-4 text-xs font-semibold" onclick={() => (showDeleteModal = false)}>Cancel</button>
        <button class="btn-danger py-1.5 px-4 text-xs font-bold bg-rose-600/80 hover:bg-rose-600" onclick={handleConfirmDelete}>Confirm Delete</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.65);
    backdrop-filter: blur(8px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
    padding: 1rem;
  }

  .modal-content {
    background: var(--modal-bg);
    backdrop-filter: blur(20px) saturate(1.3);
    -webkit-backdrop-filter: blur(20px) saturate(1.3);
    border: 1px solid var(--color-border-medium);
    box-shadow: 
      0 12px 40px rgba(0, 0, 0, 0.25),
      inset 0 1px 0 var(--color-border-subtle);
    border-radius: 12px;
  }
</style>
