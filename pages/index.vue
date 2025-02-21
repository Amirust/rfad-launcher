<script setup lang="ts">

import DiscordIcon from '~/components/icons/Discord.vue';
import Cog from '~/components/icons/Cog.vue';
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
        <div class="text-white flex flex-col gap-4">
          <UpdateAvailableMessage class="w-full"/>
          <div class="flex flex-row gap-2.5">
            <Button class="font-bold text-4xl text-primary tracking-wider">
              ОБНОВИТЬ
            </Button>
            <Button :same-padding="true" class="font-bold text-4xl text-primary">
              <Cog class="w-11 text-primary"/>
            </Button>
          </div>
          <div class="flex flex-col w-full">
            <div class="flex flex-row w-full">
              <span class="text-secondary font-medium w-24 tracking-wide">Установлена:</span>
              <span class="text-primary font-semibold tracking-wide">16.01</span>
            </div>
            <div class="flex flex-row w-full">
              <span class="text-secondary font-medium w-24 tracking-wide">Актуальная:</span>
              <span class="text-primary font-semibold tracking-wide">28.02</span>
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