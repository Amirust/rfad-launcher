<script setup lang="ts">
import Box from '~/components/base/Box.vue';
import Clock from '~/components/icons/Clock.vue';
import User from '~/components/icons/User.vue';
</script>

<template>
  <Box>
    <div class="flex flex-col w-86 gap-4 items-center text-secondary font-semibold text-sm tracking-wide">
      <div class="flex flex-row w-full items-center gap-4">
        <img
          src=""
          alt=""
          class="rounded-2xl w-20 h-20"
        >
        <div class="flex flex-col gap-1.5">
          <div class="font-bold text-xl">
            Edge UI - RFAD Edition
          </div>
          <div class="flex flex-row gap-3 [&>div]:flex [&>div]:flex-row [&>div]:gap-2 [&>div]:items-center [&>div]:font-medium">
            <div>
              <Clock class="w-3.5 h-3.5"/>
              00.00.0000
            </div>
            <div>
              <User class="w-3.5 h-3.5"/>
              Amirust
            </div>
          </div>
        </div>
      </div>
      <div class="font-normal">
        Новый UI для Skyrim SE, адаптированный под RFAD SE.
        Здесь представлен полный пак, или просто патч под RFAD SE.
      </div>
    </div>
  </Box>
</template>

<style scoped>

</style>