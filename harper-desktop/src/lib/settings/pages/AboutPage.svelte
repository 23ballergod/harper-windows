<script lang="ts">
import { onMount } from 'svelte';
import { Client } from '$lib/client';

let currentVersion = '';

onMount(() => {
	void loadCurrentVersion();
});

async function loadCurrentVersion() {
	try {
		currentVersion = await Client.getCurrentVersion();
	} catch (error) {
		console.error('Unable to load the app version.', error);
	}
}
</script>

<section class="about">
        <div class="about-mark">S</div>
        <h1>Shah Re-Writer</h1>
        <p class="muted">Version {currentVersion || 'unknown'}</p>
        <p>
          A grammar checker for Windows that works in every app and runs entirely on your PC. No
          accounts, no subscriptions, and your writing never leaves your computer.
        </p>
        <details class="licenses">
          <summary>Open-source licenses</summary>
          <p>
            Shah Re-Writer is built with open-source software. Their licenses require that we
            credit them here.
          </p>
          <ul>
            <li>
              <strong>Harper</strong>, the rule-based grammar engine. Copyright The Harper
              Contributors and Automattic. Apache License 2.0.
            </li>
            <li>
              <strong>candle</strong>, used to run the AI model. Copyright Hugging Face. MIT or
              Apache License 2.0.
            </li>
            <li>
              <strong>Qwen3</strong> AI models, downloaded on request. Copyright Alibaba Cloud.
              Apache License 2.0.
            </li>
            <li>
              <strong>Tauri</strong>, <strong>egui</strong>, <strong>wgpu</strong> and other
              libraries. MIT or Apache License 2.0.
            </li>
          </ul>
          <p class="muted">
            The full Apache License 2.0 text is at apache.org/licenses/LICENSE-2.0 and ships in
            the LICENSE file of the source code.
          </p>
        </details>
      </section>

<style>
.licenses {
	margin-top: 24px;
	text-align: left;
	max-width: 520px;
	font-size: 13px;
}
.licenses summary {
	cursor: pointer;
	font-weight: 600;
	text-align: center;
}
.licenses ul {
	padding-left: 18px;
}
.licenses li {
	margin: 6px 0;
}
</style>
