<script lang="ts">
import { Button, Checkbox, CheckIcon, SettingRow } from 'components';
import { onDestroy, onMount } from 'svelte';
import { type AiModelId, type AiModelView, type AiSettings, Client } from '$lib/client';

let settings: AiSettings = { enabled: true, model: 'Accurate' };
let models: AiModelView[] = [];
let isLoading = true;
let error = '';
let poll: ReturnType<typeof setInterval> | undefined;

const DESCRIPTIONS: Record<AiModelId, string> = {
	Fast: 'About 500 MB. Quick on any PC and catches the most common mistakes.',
	Accurate:
		'About 1.1 GB. Catches noticeably more, like awkward phrasing and wrong word choices. Best with 8 GB of RAM or more.',
};

onMount(() => {
	void load();
	poll = setInterval(() => void refreshModels(), 1000);
});

onDestroy(() => {
	if (poll) clearInterval(poll);
});

async function load() {
	isLoading = true;
	error = '';
	try {
		settings = await Client.getAiSettings();
		await refreshModels();
	} catch (e) {
		error = `Unable to load AI settings: ${e}`;
	} finally {
		isLoading = false;
	}
}

async function refreshModels() {
	try {
		models = await Client.getAiModels();
	} catch (e) {
		error = `Unable to read model status: ${e}`;
	}
}

async function save(next: AiSettings) {
	const previous = settings;
	settings = next;
	try {
		await Client.setAiSettings(next);
	} catch (e) {
		settings = previous;
		error = `Unable to save AI settings: ${e}`;
	}
}

async function download(model: AiModelId) {
	error = '';
	await Client.downloadAiModel(model);
	await refreshModels();
}

async function remove(model: AiModelId) {
	error = '';
	try {
		await Client.deleteAiModel(model);
	} catch (e) {
		error = `Unable to delete model: ${e}`;
	}
	await refreshModels();
}

function percent(model: AiModelView): number {
	if (!model.state.totalBytes) return 0;
	return Math.min(100, Math.round((model.state.downloadedBytes / model.state.totalBytes) * 100));
}

$: selected = models.find((m) => m.id === settings.model);
</script>

<section>
  <div class="stanza">
    <div class="eyebrow">AI Suggestions</div>
    <p class="section-copy">
      A small AI model runs entirely on your computer to catch the mistakes rules can't, like
      wrong word choices and awkward grammar. It's free, works offline, and your writing never
      leaves your PC.
    </p>
    <div class="rows">
      <SettingRow top>
        <strong>Use AI suggestions</strong>
        <p>Suggestions appear a moment after you finish a sentence, next to Shah Re-Writer's instant checks.</p>
        <Checkbox
          slot="control"
          appearance="settings"
          checked={settings.enabled}
          disabled={isLoading}
          on:click={() => save({ ...settings, enabled: !settings.enabled })}
        >
          {#if settings.enabled}<CheckIcon className="control-icon" />{/if}
        </Checkbox>
      </SettingRow>
    </div>
  </div>

  <div class="divider"></div>

  <div class="stanza">
    <div class="eyebrow">Model</div>
    <div class="rows">
      {#each models as model}
        <SettingRow top>
          <strong>{model.displayName}</strong>
          <p>{DESCRIPTIONS[model.id]}</p>
          {#if model.state.downloading}
            <p class="result-summary">Downloading… {percent(model)}%</p>
          {:else if model.state.error}
            <p class="result-summary">Download failed: {model.state.error}</p>
          {:else if model.state.downloaded}
            <p class="result-summary">Downloaded</p>
          {/if}
          <div class="inline-row">
            {#if !model.state.downloaded && !model.state.downloading}
              <Button unstyled class="button" type="button" on:click={() => download(model.id)}>
                Download
              </Button>
            {/if}
            {#if model.state.downloaded}
              <Button unstyled class="button" type="button" on:click={() => remove(model.id)}>
                Delete
              </Button>
            {/if}
          </div>
          <Checkbox
            slot="control"
            appearance="settings"
            checked={settings.model === model.id}
            disabled={isLoading}
            title="Use this model"
            on:click={() => save({ ...settings, model: model.id })}
          >
            {#if settings.model === model.id}<CheckIcon className="control-icon" />{/if}
          </Checkbox>
        </SettingRow>
      {/each}
    </div>
    {#if selected && !selected.state.downloaded && !selected.state.downloading && settings.enabled}
      <p class="result-summary">Download the selected model to turn on AI suggestions.</p>
    {/if}
    {#if error}
      <p class="result-summary">{error}</p>
    {/if}
  </div>
</section>
