<script setup lang="ts">
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from '@tauri-apps/api/event';

const greetMsg = ref("");
const downloadRes = ref("");
const name = ref("");
const fileId = ref('');
const downloadLog = ref('');

async function greet() {
  greetMsg.value = await invoke("greet", { name: name.value });
}

async function download() {
   downloadRes.value = await invoke("download", {
    id: fileId.value
  });
}

listen<{
  downloadedBytes:  number,
  percentage: number,
  speedBytesPerSec: number,
}>('download-progress', (event) => {
  downloadLog.value = `${event.payload.percentage.toFixed(0)}% / ${(event.payload.speedBytesPerSec / 1024 / 1024).toFixed(1)} MB/s`;
});

</script>

<template>
  <main class="container">
    <h1>Welcome to Tauri + Vue</h1>
    <form class="row" @submit.prevent="greet">
      <input id="greet-input" v-model="name" placeholder="Enter a name..." />
      <button type="submit">Greet</button>
    </form>
    <form style="margin-top: 1em" class="row" @submit.prevent="download">
      <input id="fileid-input" v-model="fileId" placeholder="File ID" />
      <button type="submit">Greet</button>
    </form>
    <p>{{ greetMsg }}</p>
    <p>{{ downloadRes }}</p>
    <p>{{ downloadLog }}</p>
  </main>
</template>

<style>
</style>