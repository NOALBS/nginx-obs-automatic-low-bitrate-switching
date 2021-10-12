<script>
  import { onMount } from "svelte";
  import { push } from "svelte-spa-router";
  import { isLoggedIn, user } from "../store";

  import InputSwitch from "../components/InputSwitch.svelte";
  import SettingsSection from "../components/SettingsSection.svelte";
  import Input from "../components/Input.svelte";

  $: if (!$isLoggedIn) {
    push("/login");
  }

  function toggleDarkmode() {
    document.body.classList.toggle("theme-dark");
  }
</script>

<div class="space-y-5 sm:px-10 sm:py-5 sm:max-w-5xl mx-auto">
  <h1
    class="text-4xl font-medium text-center text-skin-base"
    on:click={toggleDarkmode}
  >
    Settings
  </h1>

  <SettingsSection
    name="Switcher"
    description="Settings for that switching thing"
  >
    <div class="card">
      <form>
        <div class="space-y-5">
          <div class="space-y-4">
            <InputSwitch
              label="Enable switcher"
              bind:value={$user.config.switcher.bitrateSwitcherEnabled}
            />
            <div class="border border-b border-skin-divider" />

            <InputSwitch
              label="Only switch when streaming"
              description="Will not switch when not streaming from OBS"
              bind:value={$user.config.switcher.onlySwitchWhenStreaming}
            />
            <div class="border border-b border-skin-divider" />

            <InputSwitch
              label="Auto switch notification"
              bind:value={$user.config.switcher.autoSwitchNotification}
            />
          </div>
        </div>
      </form>
    </div>
  </SettingsSection>

  <SettingsSection name="Triggers" description="At what point to switch scenes">
    <div class="card">
      <form>
        <div class="space-y-5">
          <div class="space-y-4">
            <Input
              type="number"
              label="Low"
              description="When the bitrate drops below {$user.config.switcher
                .triggers.low} Kbps"
              bind:value={$user.config.switcher.triggers.low}
            />
            <div class="border border-b border-skin-divider" />

            <Input
              type="number"
              label="RTT"
              description="When the RTT is higher than {$user.config.switcher
                .triggers.rtt} ms"
              bind:value={$user.config.switcher.triggers.rtt}
            />
            <div class="border border-b border-skin-divider" />

            <Input
              type="number"
              label="Offline"
              description="When the bitrate drops below {$user.config.switcher
                .triggers.offline} Kbps"
              bind:value={$user.config.switcher.triggers.offline}
            />
          </div>
        </div>
      </form>
    </div>
  </SettingsSection>

  <SettingsSection
    name="Stream servers"
    description="All your ingest servers stats stuff you know"
  />

  <SettingsSection name="Stream software">
    <div class="card">
      <form>
        <div class="space-y-5">
          <div class="space-y-4">
            <Input
              type="text"
              label="Software"
              bind:value={$user.config.software.type}
              readonly
            />

            <div class="border border-b border-skin-divider" />

            <Input
              type="text"
              label="Host"
              bind:value={$user.config.software.host}
            />
            <div class="border border-b border-skin-divider" />

            <Input
              type="number"
              label="Port"
              bind:value={$user.config.software.port}
            />
            <div class="border border-b border-skin-divider" />

            <Input
              type="password"
              label="Password"
              bind:value={$user.config.software.password}
            />
          </div>
        </div>
      </form>
    </div>
  </SettingsSection>

  <SettingsSection name="Chat">
    <div class="card">
      <form>
        <div class="space-y-5">
          <div class="space-y-4">
            <Input
              type="text"
              label="Platform"
              bind:value={$user.config.chat.platform}
              readonly
            />
            <div class="border border-b border-skin-divider" />

            <Input
              type="text"
              label="Username"
              bind:value={$user.config.chat.username}
            />
            <div class="border border-b border-skin-divider" />

            <Input
              type="text"
              label="prefix"
              bind:value={$user.config.chat.prefix}
            />
            <div class="border border-b border-skin-divider" />

            <InputSwitch
              label="Enable auto stop"
              description="Automatically stop the stream on host or raid"
              bind:value={$user.config.chat.enableAutoStopStreamOnHostOrRaid}
            />
          </div>
        </div>
      </form>
    </div>
  </SettingsSection>

  <SettingsSection name="Optional scenes">
    <div class="card">
      <form>
        <div class="space-y-5">
          <div class="space-y-4">
            <Input
              type="text"
              label="Starting"
              bind:value={$user.config.optionalScenes.starting}
            />
            <div class="border border-b border-skin-divider" />

            <Input
              type="text"
              label="Ending"
              bind:value={$user.config.optionalScenes.ending}
            />
            <div class="border border-b border-skin-divider" />

            <Input
              type="text"
              label="Privacy"
              bind:value={$user.config.optionalScenes.privacy}
            />
          </div>
        </div>
      </form>
    </div>
  </SettingsSection>

  <SettingsSection name="Extra options">
    <div class="card">
      <form>
        <div class="space-y-5">
          <div class="space-y-4">
            <InputSwitch
              label="Twitch transcoding check"
              bind:value={$user.config.optionalOptions.twitchTranscodingCheck}
            />

            {#if $user.config.optionalOptions.twitchTranscodingCheck}
              <div class="border border-b border-skin-divider" />

              <Input
                type="number"
                label="Twitch transcoding retries"
                bind:value={$user.config.optionalOptions
                  .twitchTranscodingRetries}
              />
              <div class="border border-b border-skin-divider" />

              <Input
                type="number"
                label="Twitch transcoding delay seconds"
                bind:value={$user.config.optionalOptions
                  .twitchTranscodingDelaySeconds}
              />
            {/if}
          </div>
        </div>
      </form>
    </div>
  </SettingsSection>
</div>
