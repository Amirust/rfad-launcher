<script setup lang="ts">

import DiscordIcon from '~/components/icons/Discord.vue';
import Cog from '~/components/icons/Cog.vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { type DownloadProgress, EventNames, type UnpackProgress, type UpdateProgress, UpdateStatus } from '~/types/types';
import config from '~/config';
import Telegram from '~/components/icons/Telegram.vue';
import Vk from '~/components/icons/Vk.vue';
import Boosty from '~/components/icons/Boosty.vue';
import FolderSmallStroke from '~/components/icons/FolderSmallStroke.vue';
import UpdateConfirmationMessage from '~/components/UpdateConfirmationMessage.vue';
import ModComponent from '~/components/ModComponent.vue';

const firstStart = ref(true)

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

const additionalProgress = ref(0)

const dirError = ref(false)
const isGameStarting = ref(false)

const updatePercentage = computed(() => {
  return (+(+updateDownloadPercentage.value.toFixed(0) / 2).toFixed(0)) + (+(+updateUnpackPercentage.value.toFixed(0) / 2.1).toFixed(0)) + additionalProgress.value
})

const showConfirmation = ref(false)

const wait = (ms = 1000) => new Promise(resolve => setTimeout(resolve, ms))

onMounted(async () => {
  firstStart.value = !localStorage.getItem('lastUpdate')

  invoke<string>('get_local_version').then(res => {
    localVersion.value = res === 'NO_PATCH' ? '0.0' : res
  })
  invoke<string>('get_remote_version').then(res => {
    if (res === 'NO_DIR') {
      remoteVersion.value = '0.0'
      dirError.value = true
      return
    }
    remoteVersion.value = res === 'NO_PATCH' ? '0.0' : res

    updateAvailable.value = remoteVersion.value !== localVersion.value
  })
})

const update = async (isFirstStart: boolean = false) => {
  if (!isFirstStart && !showConfirmation.value) {
    showConfirmation.value = true
    return
  }

  showConfirmation.value = false

  updateDownloadPercentage.value = 0
  updateUnpackPercentage.value = 0
  additionalProgress.value = 0
  updateDownloadSpeed.value = '0'
  updateDownloaded.value = false
  updateUnpacked.value = false

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

    if (data.payload.status === UpdateStatus.LoadOrderUpdateStarted)
      additionalProgress.value += 1

    if (data.payload.status === UpdateStatus.LoadOrderUpdateFinished)
      additionalProgress.value += 1
  })

  await invoke('update')

  await wait(300)

  localStorage.setItem('lastUpdate', Date.now().toString())
  firstStart.value = !localStorage.getItem('lastUpdate')

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

const processButtonClick = async () => {
  if (firstStart.value)
    await update(true)
  else {
    isGameStarting.value = true
    await invoke('start_game')

    await wait(30_000)
    isGameStarting.value = false
  }
}

const openExplorer = async () => {
  await invoke('open_explorer')
}

const openMo2 = async () => {
  await invoke('open_mo2')
}
</script>

<template>
  <div class="px-10 py-10 flex flex-row w-full h-full min-h-svh relative overflow-hidden">
    <div class="flex flex-row gap-6 z-40 w-full">
      <div class="flex flex-col justify-between min-h-full">
        <a :href="config.discord" target="_blank">
          <CircleButton>
            <DiscordIcon class="w-9 text-secondary"/>
          </CircleButton>
        </a>
        <a :href="config.telegram" target="_blank">
          <CircleButton>
            <Telegram class="w-7 mr-1 mt-[2px] text-secondary"/>
          </CircleButton>
        </a>
        <a :href="config.vk" target="_blank">
          <CircleButton>
            <Vk class="w-8 text-secondary"/>
          </CircleButton>
        </a>
        <a :href="config.boosty" target="_blank">
          <CircleButton>
            <Boosty class="w-8 mb-[2px] ml-[2px] text-secondary"/>
          </CircleButton>
        </a>
        <a :href="config.db" target="_blank">
          <CircleButton>
            <DiscordIcon class="w-9 text-secondary"/>
          </CircleButton>
        </a>
        <CircleButton @click="openMo2">
          <DiscordIcon class="w-9 text-secondary"/>
        </CircleButton>
        <CircleButton @click="openExplorer">
          <FolderSmallStroke class="w-8 h-8 text-secondary"/>
        </CircleButton>
      </div>
      <div class="horizontal-divider">
      </div>
      <div class="flex flex-col justify-between h-full">
        <h1 class="text-5xl text-gradient font-semibold">RFAD SE 6.0</h1>
        <div class="flex flex-col gap-4 relative">
          <transition-group name="fade" tag="div" class="relative flex flex-col gap-4">
            <UpdateConfirmationMessage v-if="showConfirmation" class="w-full">
              <div class="flex flex-row justify-between w-full mt-2.5">
                <div class="font-bold hover:opacity-80 transition-opacity cursor-pointer" @click="update()">
                  Продолжить
                </div>
                <div class="font-bold text-secondary hover:opacity-80 transition-opacity cursor-pointer" @click="showConfirmation = false">
                  Отменить
                </div>
              </div>
            </UpdateConfirmationMessage>
            <DirErrorMessage v-if="dirError" class="w-full"/>
            <UpdatingMessage :percentage="updatePercentage" v-if="updateStarted" class="w-full"/>
            <UnpackingMessage :percentage="updateUnpackPercentage" v-if="updateUnpackStarted" class="w-full"/>
            <DownloadingMessage :speed="updateDownloadSpeed" :percentage="updateDownloadPercentage" v-if="updateDownloadStarted" class="w-full"/>
            <UpdateAvailableMessage :version="remoteVersion" v-if="updateAvailable && !updateStarted" class="w-full"/>
          </transition-group>
          <div class="flex flex-row gap-2.5">
            <Button
              @click="processButtonClick"
              class="font-bold text-4xl text-primary tracking-wider uppercase min-w-72"
              :class="{
                'cursor-pointer': !isGameStarting && !updateStarted,
                'cursor-not-allowed text-secondary pointer-events-none': isGameStarting || updateStarted
              }"
            >
              {{ firstStart ? 'Обновить' : 'Играть' }}
            </Button>
            <DropdownButton
              :same-padding="true"
              class="font-bold text-4xl text-primary"
              @update="update(false)"
              @open-mo2="openMo2"
              @open-explorer="openExplorer"
            >
              <Cog class="w-11 text-primary"/>
            </DropdownButton>
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
      <div class="w-full flex flex-row justify-end">
        <div class="flex flex-col">
          <ModComponent/>
        </div>
      </div>
    </div>
    <img alt="centurion" src="assets/image/centurion.webp" class="centurion z-10"/>
  </div>
</template>

<style lang="scss">
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