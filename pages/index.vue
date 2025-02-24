<script setup lang="ts">

import DiscordIcon from '~/components/icons/Discord.vue';
import Cog from '~/components/icons/Cog.vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { type DownloadProgress, EventNames, type UnpackProgress, type UpdateProgress, UpdateStatus } from '~/types/types';
import config from '~/config';

const localVersion = ref('Загружаем...')
const remoteVersion = ref('Загружаем...')

const updateStarted = ref(false)

const updateDownloadStarted = ref(false)
const updateDownloadSpeed = ref('0')
const updateDownloadPercentage = ref(0)
const updateDownloaded = ref(false)

const updateUnpackStarted = ref(false)
const updateUnpackPercentage = ref(0)
const updateUnpacked = ref(false)

const updateAvailable = ref(false)

const updatePercentage = computed(() => {
  return (+(+updateDownloadPercentage.value.toFixed(0) / 2).toFixed(0)) + (+(+updateUnpackPercentage.value.toFixed(0) / 2.1).toFixed(0)) + (updateStarted ? 0 : 2)
})

onMounted(async () => {
  invoke<string>('get_local_version').then(res => {
    localVersion.value = res === 'NO_PATCH' ? '0.0' : res
  })
  invoke<string>('get_remote_version').then(res => {
    remoteVersion.value = res === 'NO_PATCH' ? '0.0' : res

    updateAvailable.value = remoteVersion.value !== localVersion.value
  })
})

const update = async () => {
  updateStarted.value = true

  const unlistenUpdate = await listen<UpdateProgress>(EventNames.UpdateProgress,  async(data) => {
    if (data.payload.status === UpdateStatus.DownloadStarted)
      updateDownloadStarted.value = true

    const unlistenDownload = await listenDownload(config.localUpdateFileName)

    if (data.payload.status === UpdateStatus.DownloadFinished) {
      updateDownloaded.value = true
      updateDownloadStarted.value = false
      unlistenDownload()
    }

    const unlistenUnpack = await listenUnpack()

    if (data.payload.status === UpdateStatus.UnpackStarted)
      updateUnpackStarted.value = true

    if (data.payload.status === UpdateStatus.UnpackFinished) {
      updateUnpacked.value = true
      updateUnpackStarted.value = false
      unlistenUnpack()
    }
  })

  await invoke('update')

  updateStarted.value = false

  invoke<string>('get_local_version').then(res => {
    localVersion.value = res === 'NO_PATCH' ? '0.0' : res
    updateAvailable.value = remoteVersion.value !== localVersion.value
  })

  unlistenUpdate()
}

const listenDownload = async (fileName: string) => {
  return await listen<DownloadProgress>(EventNames.DownloadProgress, (data) => {
    if (data.payload.fileName !== fileName)
      return;
    updateDownloadSpeed.value = (data.payload.speedBytesPerSec / 1024 / 1024).toFixed(1)
    updateDownloadPercentage.value = data.payload.percentage
  })
}

const listenUnpack = async () => {
  return await listen<UnpackProgress>(EventNames.UnpackProgress, (data) => {
    updateUnpackPercentage.value = data.payload.percentage
  })
}
</script>

<template>
  <div class="px-10 py-10 flex flex-row w-full h-full min-h-svh relative overflow-hidden">
    <div class="flex flex-row gap-6 z-40">
      <div class="flex flex-col justify-between min-h-full">
        <CircleButton v-for="i in 7">
          <DiscordIcon class="w-9 text-secondary"/>
        </CircleButton>
      </div>
      <div class="horizontal-divider">
      </div>
      <div class="flex flex-col justify-between h-full">
        <h1 class="text-5xl text-gradient font-semibold">RFAD SE 6.0</h1>
        <div class="flex flex-col gap-4 relative">
          <transition-group name="fade" tag="div" class="relative flex flex-col gap-4">
            <UpdatingMessage :percentage="updatePercentage" v-if="updateStarted" class="w-full"/>
            <UnpackingMessage :percentage="updateUnpackPercentage" v-if="updateUnpackStarted" class="w-full"/>
            <DownloadingMessage :speed="updateDownloadSpeed" :percentage="updateDownloadPercentage" v-if="updateDownloadStarted" class="w-full"/>
            <UpdateAvailableMessage :version="remoteVersion" v-if="updateAvailable && !updateStarted" class="w-full"/>
          </transition-group>
          <div class="flex flex-row gap-2.5">
            <Button @click="update" class="font-bold text-4xl text-primary tracking-wider">
              ОБНОВИТЬ
            </Button>
            <Button :same-padding="true" class="font-bold text-4xl text-primary">
              <Cog class="w-11 text-primary"/>
            </Button>
          </div>
          <div class="flex flex-col w-full">
            <div class="flex flex-row w-full">
              <span class="text-secondary font-medium w-24 mr-2 tracking-wide">Установлена:</span>
              <span class="text-primary font-semibold tracking-wide">{{ localVersion }}</span>
            </div>
            <div class="flex flex-row w-full">
              <span class="text-secondary font-medium w-24 mr-2 tracking-wide">Актуальная:</span>
              <span class="text-primary font-semibold tracking-wide">{{ remoteVersion }}</span>
            </div>
          </div>
        </div>
      </div>
    </div>
    <img alt="centurion" src="assets/image/centurion.webp" class="centurion z-10"/>
  </div>
</template>

<style scoped>
.horizontal-divider {
  background-image: radial-gradient(circle, theme('colors.secondaryDarker'), #000000);
  width: 1px;
  @apply h-full
}

.text-gradient {
  background: linear-gradient(120deg, rgba(13, 12, 10, 0) 30%, #0D0C0A 100%),
  linear-gradient(#FFEABF, #FFEABF);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  text-shadow: 0 0 2px rgba(255, 234, 191, 0.5);
}


.centurion {
  opacity: 50%;
  position: absolute;
  bottom: -30px;
  left: 20px;
  width: 620px;
  height: 620px;
}
</style>