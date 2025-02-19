<script setup lang="ts">

import DiscordIcon from '~/components/icons/Discord.vue';
</script>

<template>
  <div class="px-10 py-10 flex flex-row w-full h-full min-h-svh relative overflow-hidden">
    <img alt="centurion" src="assets/image/centurion.webp" class="centurion"/>
    <div class="flex flex-row gap-6">
      <div class="flex flex-col justify-between min-h-full">
        <CircleButton v-for="i in 7">
          <DiscordIcon class="w-9 text-secondary"/>
        </CircleButton>
      </div>
      <div class="horizontal-divider">
      </div>
      <div class="flex flex-col justify-between h-full">
        <h1 class="text-5xl text-gradient font-semibold">RFAD SE 6.0</h1>
        <div class="text-white">
          Abobaa
        </div>
      </div>
    </div>
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