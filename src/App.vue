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
:root {
  font-family: Inter, Avenir, Helvetica, Arial, sans-serif;
  font-size: 16px;
  line-height: 24px;
  font-weight: 400;

  color: #0f0f0f;
  background-color: #f6f6f6;

  font-synthesis: none;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  -webkit-text-size-adjust: 100%;
}

.container {
  margin: 0;
  padding-top: 10vh;
  display: flex;
  flex-direction: column;
  justify-content: center;
  text-align: center;
}


.row {
  display: flex;
  justify-content: center;
}

a {
  font-weight: 500;
  color: #646cff;
  text-decoration: inherit;
}

a:hover {
  color: #535bf2;
}

h1 {
  text-align: center;
}

input,
button {
  border-radius: 8px;
  border: 1px solid transparent;
  padding: 0.6em 1.2em;
  font-size: 1em;
  font-weight: 500;
  font-family: inherit;
  color: #0f0f0f;
  background-color: #ffffff;
  transition: border-color 0.25s;
  box-shadow: 0 2px 2px rgba(0, 0, 0, 0.2);
}

button {
  cursor: pointer;
}

button:hover {
  border-color: #396cd8;
}
button:active {
  border-color: #396cd8;
  background-color: #e8e8e8;
}

input,
button {
  outline: none;
}

#greet-input {
  margin-right: 5px;
}

@media (prefers-color-scheme: dark) {
  :root {
    color: #f6f6f6;
    background-color: #2f2f2f;
  }

  a:hover {
    color: #24c8db;
  }

  input,
  button {
    color: #ffffff;
    background-color: #0f0f0f98;
  }
  button:active {
    background-color: #0f0f0f69;
  }
}

</style>